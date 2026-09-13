# Análisis workspace PIPEWIRE_YOLO_STREAMING (2026-08-24)

Proyecto: cliente Rust de captura de pantalla/ventana en KDE Plasma/Wayland.
Objetivo final: portal XDG ScreenCast → stream PipeWire → procesar frames → YOLO (aún no implementado).
  DECISIÓN 2026-08-25: el usuario quiere YOLO en GPU (CUDA) ⇒ camino DMA-BUF cero-copias es el objetivo real;
  MemFd queda como fallback/inicio rápido. Plan sugerido: dataType=1<<SPA_DATA_DmaBuf, modifiers consultados en
  runtime (no NVIDIA_MODIFIERS hardcodeado), SPA_META_SyncTimeline obligatorio (NVIDIA), import dma-buf a CUDA
  (EGL→GL interop o Vulkan→cust::ExternalMemory, como waycap-rs NvencEncoder), inferencia CUDA (ort CUDA EP /
  tch / TensorRT). Quitar RT_PROCESS o mover trabajo GPU fuera del callback RT; añadir queue_buffer.
Flujo: D-Bus portal → fd PipeWire → PipewireHandlerBuilder (core) → StreamPipewireHandler (stream+listener) → process de buffers.

## Dependencias (Cargo.toml)
- tokio 1.52.1 (full), tokio-stream 0.1.19, tokio-util 0.7 (MUERTA), uuid 1.10 (v4)
- serde 1.0.229 (solo vía serde_json), serde_json 1.0.151, zbus 5.18.0
- pipewire 0.10.1, libc 0.2 (mmap/ioctl DMA-BUF NVIDIA). edition 2024 (Rust >=1.85)
- Comentadas: pipewire-native (path ./dependencies, descartada), wayland-client 0.31.15/backend/scanner (descartadas junto a mod wayland/)

## Módulos
- main.rs: 3 fases: (1 opcional) WindowsRunnerClient::get_active_windows(); (2) ScreenCastRunnerClient::get_pipewire_node_id() -> (u32 node_id, OwnedFd); (3) PipewireHandlerBuilder::new(None)?.context(None)?.core(fd,None)?.build()? -> default_video_stream("pipewire-yolo-client", Some(node)) -> start() -> await JoinHandle. Cierre: connection.close().await; unsafe pipewire::deinit(). `// mod wayland;` comentado.
- dbus/krunner: proxy org.kde.krunner1 (default_service org.kde.KWin, path /WindowsRunner), Match(query)->Vec<RawKRunnerMatch>. RawKRunnerMatch = (id:String, title:String, icon:String, type:i32, relevance:f64, properties:HashMap<String,OwnedValue>). Filtra title no vacío, excluye xwaylandvideobridge, type==100 (ExactMatch).
- dbus/screencast: proxy org.freedesktop.portal.ScreenCast (default_service org.freedesktop.portal.Desktop, path /org/freedesktop/portal/desktop): CreateSession, SelectSources, Start, OpenPipeWireRemote, props AvailableSourceTypes/CursorModes/Version. Proxy org.freedesktop.portal.Request con señal Response(u32, a{sv}).
  - SelectSources: types=2 (WINDOW), cursor_mode=1, persist_mode=2; restore_token persistido en restore_data.bin (zvariant to_bytes/Context::new_dbus(LE,0)).
  - node_id extraído de results["streams"]: Array→Structure→fields()[0]→U32.
- pw_handlers/mod.rs: macro add_if; trait ToSpaProperties -> HashMap<u32, pod::Property>; impls para VideoInfoRaw y AudioInfoRaw (mapea format, interlace, size, framerate, max_framerate, pixel_aspect_ratio, modifier, views, multiview*, chroma_site, color_range/matrix, transfer, color_primaries).
- pw_handlers/pod.rs: enum PodValue (None,Bool,Id,Int,Long,Float,Double,String,Bytes,Rectangle,Fraction,Fd,ValueArray,Struct,Object,Choice,Pointer); macro kv!; KeyValue(FormatProperties, PodValue) -> pod::Property.
- pw_handlers/stream_handler/builder.rs: type-state builders. PipewireHandlerBuilder (NeedMainLoop/NeedContext/NeedCore/Ready): ThreadLoopRc::new(Some("jdm-stream")), ContextRc::new(&thread_loop), core = context.connect_fd_rc(fd) (socket PW del portal). StreamPipewireBuilder (NeedFlags/NeedProperties/NeedFormatting/StreamReady): build() serializa 2 PODs con PodSerializer: EnumFormat (SpaTypes::ObjectParamFormat; media type/subtype + props VideoInfoRaw + VideoModifier NVIDIA forzado) y Buffers (SPA_PARAM_BUFFERS_dataType = (1<<SPA_DATA_MemFd)|(1<<SPA_DATA_DmaBuf)); StreamRc::new + stream.connect(Direction::Input, node_id, flags, pods). POD Meta SPA_META_Header comentado.
- pw_handlers/stream_handler/handler.rs: StreamPipewireHandler { thread_loop, context (sin leer), core, streams: HashMap<&'static str, StreamRc>, streams_listeners: HashMap<&'static str, Vec<StreamListener<UserData<VideoInfoRaw>>>> }. UserData<T>{info:T} nunca leído. Métodos: stream(), video_stream() (flags AUTOCONNECT|MAP_BUFFERS|RT_PROCESS; props MEDIA_TYPE="Video", MEDIA_CATEGORY, MEDIA_ROLE), default_video_stream() (VideoInfoRaw + set_format(VideoFormat::BGRA); listener con user_data; register), audio_stream() (ROTA), start() -> (oneshot::Sender<bool>, JoinHandle). Drop: lock loop, clear listeners, flush(true), disconnect, clear, stop.
- utils/mod.rs: generate_uuid_v4; save_data/load_data; NVIDIA_MODIFIERS: &[i64; 13] (12 valores 0x03xx... + lineal 0x00FFFFFFFFFFFFFF=72057594037927935); build_nvidia_mod_property() -> Property(VideoModifier, Value::Choice(ChoiceValue::Long(Choice::Enum{default, alternatives}))).
- utils/logging: LogWriter<W1,W2> tee stdout+fichero salida.log; static LOGGER: LazyLock<Mutex<Logger>> (tokio Mutex).
- wayland/ (ABANDONADO): zkde_screencast vía wayland_scanner generate_interfaces!("protocols/not-available/zkde-screencast-unstable-v1.xml") — ruta no existe en crate root; dispatcher con Dispatch impls vacíos; NO COMPILA si se reactiva.

## Callbacks del StreamListener (default_video_stream)
add_buffer/remove_buffer/control_info/io_changed/drained: solo println. state_changed y param_changed: println debug. process: ÚNICO con lógica:
1. dequeue_buffer() (None -> return)
2. datas_mut()[0]
3. DataType::MemFd: chunk offset/size/stride; size==0 return; data.data() (requiere MAP_BUFFERS); si offset+size<=len imprime "Frame procesado".
4. DataType::DmaBuf: fd; libc::mmap(NULL, len=(offset+size+4095)&!4095, PROT_READ, MAP_SHARED, fd, 0); imprime ptr. SIN munmap, SIN DMA_BUF_IOCTL_SYNC, sin leer datos.
5. _ => "Eh?"
**NUNCA llama queue_buffer → CRÍTICO.**

## Estado
Terminado: flujo D-Bus portal completo (session_handle, node_id, restore_token), listado KRunner, logging tee, builders type-state, serialización POD correcta, Drop limpio, lectura MemFd parcial.
A medias: process MemFd solo imprime; DmaBuf esqueleto (mmap sin cleanup); YOLO inexistente; wayland/ muerto; audio_stream rota; POD Meta comentado.
Muerto: tokio-util, serde, MainLoopRc en builder, NeedMainLoop, add_flag/remove_flags/add_property/add_format, UserData.info, context, off_t import.

## Bugs clave (ver detalle si hace falta)
1. **Falta queue_buffer** en process → cola se agota, stream se congela/xruns.
2. audio_stream recibe VideoInfoRaw y MediaType::Video (debe ser AudioInfoRaw + MediaType::Audio).
3. Comentario cursor_mode: 1=Hidden (no Embedded). Spec: 1 Hidden, 2 Embedded, 4 Metadata.
4. Response de SelectSources ignorada; códigos de Response nunca validados (0 ok, 1 cancel, 2 error) → panic! con expect.
5. DmaBuf: mmap sin munmap (fuga), sin DMA_BUF_IOCTL_SYNC (obligatorio NVIDIA), no usa offset, alineación real 4096 no 4095.
6. RT_PROCESS + println/mmap/logger en callback RT → riesgo xruns. No se comprueba io.status()/PW_STATUS_NEED_DATA.
7. RawKRunnerMatch: 3er campo es icon, 4º es type (no app_id/category) → WindowInfo.app_id recibe el icono.
8. wayland/ no compilaría (ruta XML inexistente + deps comentadas).
9. unsafe innecesario en ThreadLoopRc::new (builder.rs).
10. VideoModifier hardcodeado y sobrescrito en set_formats; lista NVIDIA_MODIFIERS (0x0300000000606010..15, 0x0300000000E08010..15) NO coincide con listas difundidas de KWin/wlroots/PipeWire (0x0300000000000010..15, 0x0300000000000C10..C15). Verificar con drm_info.
11. LOGGER.lock().await retenido a través de awaits largos.
12. default_video_stream hace expect() (panic) sobre stream.
13. stride no se usa para recorrer filas (YOLO lo necesitará: stride bytes/fila no width*4).
14. stride() cast a usize sin validar.
15. Drop flush(true) puede bloquear sin requeue (relacionado bug 1).
16. load_data usa eprintln (fuera del logger).
17. types.rs de dbus/screencast vacío.

## /doc
- doc/dbus/protocols/tipos.csv: tipos D-Bus ↔ Rust/zvariant.
- properties_mainloop.csv: props mainloop PW (loop.name, library.name=support/libspa-support, clock.name, loop.cancel, cpu.zero-denormals).
- available/client/org.freedesktop.portal.ScreenCast.xml y Request.xml.
- available/server/org.freedesktop.impl.portal.{ScreenCast,PermissionStore}.xml.
- not-available/zkde-screencast-unstable-v1.xml (protocolo privado KDE; requests stream_output/stream_window/stream_virtual_output/stream_region/stream_virtual_output_with_description; events created/failed/closed/serial; pointer enum hidden=1 embedded=2 metadata=4) y plasma-window-management.xml.
- doc/pipewire/: 3 PDFs (MEDIA_SUBTYPE, Opciones Audio, Opciones Video) — binarios.
- Raíz: restore_data.bin (token persistido), salida.log.

## Nombres clave
StreamFlags AUTOCONNECT|MAP_BUFFERS|RT_PROCESS; MediaType::Video, MediaSubtype::Raw, VideoFormat::BGRA; node "pipewire-yolo-client"; ThreadLoop "jdm-stream".
