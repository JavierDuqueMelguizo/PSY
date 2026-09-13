# PipeWire v1.6 (GitHub PipeWire/pipewire, tag 1.6) — 2026-08-24

Fuentes: src/pipewire/stream.{c,h}, spa/include/spa/buffer/{buffer.h,meta.h}, spa/param/video/raw.h,
doc/dox/internals/dma-buf.dox, src/examples/video-play-sync.c.

## Estructuras SPA clave
- spa_data: type (Invalid,MemPtr,MemFd,DmaBuf,MemId,SyncObj), flags (READABLE=1,WRITABLE=2,DYNAMIC=4,MAPPABLE=8),
  fd:int64, mapoffset:uint32 (¡alineado a página!, base del mmap), maxsize, data:void*, chunk:spa_chunk*.
- spa_chunk: offset (se toma MODULO maxsize), size (clamp a maxsize), stride:int32, flags (CORRUPTED=1,EMPTY=2).
- spa_buffer: n_metas/n_datas + arrays. spa_buffer_find_meta_data(buf, type, size).
- spa_video_info_raw: format, flags (VARIABLE_FPS, PREMULTIPLIED_ALPHA, **MODIFIER=1<<2**, MODIFIER_FIXATION_REQUIRED=1<<3),
  modifier:u64 ("only used with DMA-BUF"), size, framerate, max_framerate, views, interlace_mode, pixel_aspect_ratio,
  multiview_mode/flags, chroma_site, color_range/matrix, transfer_function, color_primaries.
- SPA_VIDEO_FORMAT_BGRA existe; DSP_F32 = RGBA_F32 (alias).

## Internos de pw_stream (stream.c 1.6)
- MAX_BUFFERS=64, dos colas ring: dequeued (disponibles para app) y queued (devueltos).
- pw_stream_dequeue_buffer(): pop de dequeued, marca DEQUEUED. NULL si cola vacía.
- pw_stream_queue_buffer(): exige flag DEQUEUED, push a queued. pw_stream_return_buffer() devuelve SIN marcar DEQUEUED.
- PATRÓN RECOMENDADO (video-play-sync.c on_process):
  ```
  b=NULL; while((t=pw_stream_dequeue_buffer(stream))){ if(b) pw_stream_queue_buffer(stream,b); b=t; }
  ... procesar b ...  done: pw_stream_queue_buffer(stream, b);   // SIEMPRE requeue
  ```
- impl_port_use_buffers: si flag MAP_BUFFERS, para cada data con data==NULL && MAPPABLE → map_data():
  pw_map_range_init(mapoffset,maxsize,pagesize) → mmap(NULL, size, PROT_READ|WRITE, MAP_SHARED, fd, range.offset);
  data->data = ptr + range.start. **PipeWire YA mapea por ti las datas MAPPABLE** (usa mapoffset, no chunk.offset).
  DmaBuf NO mappable (típico NVIDIA) → data->data == NULL: importar vía GPU API (EGL/Vulkan/VA-API), NO mmap manual.
- fix_datatype(): si dataType tiene MemPtr añade automáticamente MemFd.
- pw_stream_connect: añade automático SPA_PARAM_IO (SPA_IO_Buffers) y SPA_PARAM_Meta (SPA_META_Busy) LOCKED.
  Si NO RT_PROCESS: node.loop.class="main" y node.async=true. RT_PROCESS ⇒ callback process en hilo RT (nada de println/mmap/alloc).
- impl_node_process_input: push del buffer a dequeued + call_process (solo si n_buffers>0); recicla queued al final del ciclo.
- flags stream: AUTOCONNECT, MAP_BUFFERS, RT_PROCESS, INACTIVE (activar luego con pw_stream_set_active), ASYNC (1<<10),
  EARLY_PROCESS (1<<11, 0.3.81). Los ejemplos oficiales usan AUTOCONNECT|INACTIVE|MAP_BUFFERS (sin RT_PROCESS) y activan
  en state PAUSED con pw_stream_set_active(true).

## Negociación DMA-BUF (dma-buf.dox — GUÍA OFICIAL)
1. Anunciar DOS EnumFormat por formato: (a) con prop SPA_FORMAT_VIDEO_modifier flags MANDATORY|DONT_FIXATE y valor
   SPA_CHOICE_Enum de longs (modifiers soportados + DRM_FORMAT_MOD_INVALID si se soporta sin modifier) — ponerlo PRIMERO
   para priorizar DMA-BUF; (b) segundo sin modifier (fallback SHM).
2. param_changed SPA_PARAM_Format (consumidor): spa_format_video_raw_parse; si hay modifier → SPA_PARAM_BUFFERS_dataType =
   1<<SPA_DATA_DmaBuf; si no → MemFd/MemPtr. El productor (KWin) fija un único par formato+modifier.
3. on_process: si type MemFd/MemPtr → SHM; si DmaBuf → importar FDs (n_datas = nº planos; puede haber fds extra de sync).
4. ADVERTENCIA OFICIAL: mmap sobre DMA-BUF NO da vista lineal con tiling/compresión (modifiers), no sincroniza con la GPU,
   y en GPU discreta es lentísimo. Usar EGL/Vulkan/VA-API.
5. Tamaño: para DmaBuf ignorar maxsize/size del chunk (productor puede poner 0); obtener tamaño localmente vía fd.
6. v4l2 excepción: DmaBuf SIN modifier y mappeable por mmap.
7. Explicit sync (NVIDIA): SPA_META_SyncTimeline + 2 fds extra (syncobj acquire/release) como ÚLTIMOS 2 datas
   (blocks = planos + 2). Negociar ParamMeta SPA_META_SyncTimeline y Buffers con metaType bit 1<<SPA_META_SyncTimeline
   flags MANDATORY. struct spa_meta_sync_timeline {flags, padding, acquire_point:u64, release_point:u64};
   SPA_META_FEATURE_SYNC_TIMELINE_RELEASE=1<<0; flag UNSCHEDULED_RELEASE=1<<0.
   Consumidor: drmSyncobjFDToHandle(drm_fd, datas[n_datas-2].fd, &acquire_handle) / datas[n_datas-1] → release_handle;
   drmSyncobjTimelineWait(acquire_handle, acquire_point) antes de leer; drmSyncobjTimelineSignal(release_handle, release_point)
   al terminar. Requiere DRM_CAP_SYNCOBJ / DRM_CAP_SYNCOBJ_TIMELINE.
8. Device ID negotiation: PW_CAPABILITY_DEVICE_ID_NEGOTIATION, SPA_FORMAT_VIDEO_deviceId (bytes dev_t, MANDATORY),
   usar EGL_PLATFORM_DEVICE_EXT con el dispositivo negociado.

## Metas disponibles (meta.h)
Header (flags/offset/pts/dts_offset/seq), VideoCrop, VideoDamage (array regiones), Cursor, Busy, VideoTransform, SyncTimeline.
SPA_META_Header: pts int64 ns — usar para timestamps YOLO.

## Implicaciones para el proyecto del usuario
1. **Falta queue_buffer** — crítico, cola de 8-64 buffers se agota y el stream para. Añadir al final de process
   (patrón de video-play-sync.c) o pw_stream_return_buffer si no se procesa.
2. En rama DmaBuf: comprobar flag SPA_DATA_FLAG_MAPPABLE ANTES de mmap manual; si está, data.data ya es válido.
   mmap debe usar data.mapoffset (+ maxsize), no chunk.offset.
3. NVIDIA: DmaBuf NO suele ser mappable → vía correcta = importar con GPU API (EGL/Vulkan) o forzar MemFd
   (anunciar solo 1<<SPA_DATA_MemFd) para CPU. El mmap a pelo + DMA_BUF_IOCTL_SYNC del código actual es antipatrón.
4. El VideoModifier debe negociarse: anunciar 2 EnumFormat (con/sin modifier) y en param_changed fijar dataType según
   resultado; además poner SPA_VIDEO_FLAG_MODIFIER en los flags de VideoInfoRaw.
5. Negociar SPA_META_Header (pts) y opcional SPA_META_SyncTimeline (sync explícita NVIDIA) en param_changed con
   pw_stream_update_params, como hace video-play-sync.c.
6. Quitar RT_PROCESS si se va a hacer trabajo bloqueante (YOLO en CPU): usar AUTOCONNECT|INACTIVE|MAP_BUFFERS y activar
   en state PAUSED; así el process corre en loop "main" no-RT.
