# KDE Plasma: KWin v6.6.4 + plasma-desktop v6.6.4 — 2026-08-24

## KWin v6.6.4 — src/plugins/screencast/screencaststream.cpp (PRODUCTOR del stream)
- supportedFormats DRM↔SPA: ARGB8888↔BGRA, XRGB8888↔BGRx, RGBA8888↔ABGR, RGBX8888↔xBGR,
  ABGR8888↔RGBA, XBGR8888↔RGBx, BGRA8888↔ARGB, BGRX8888↔xRGB, NV12↔NV12, RGB888↔BGR, BGR888↔RGB.
  Fallback SHM usa DRM_FORMAT_ARGB8888 (= SPA_VIDEO_FORMAT_BGRA).
- createStream: pw_stream_new("kwin-screencast-*"), flags PW_STREAM_FLAG_DRIVER|PW_STREAM_FLAG_ALLOC_BUFFERS,
  direction OUTPUT. m_modifiers = backend->supportedFormats()[sourceDrmFormat]; si el formato no soporta dmabuf
  prefiere convertir a ARGB8888 antes que caer a memfd. m_hasDmaBuf = testCreateDmaBuf() real.
- buildFormats(fixate): si m_hasDmaBuf → EnumFormat dmabuf con SPA_FORMAT_VIDEO_modifier flags
  MANDATORY|DONT_FIXATE (lista completa); si fixate → además EnumFormat con UN solo modifier flags MANDATORY.
  Siempre añade EnumFormat SHM (ARGB8888) SIN modifier como fallback. BGRA/RGBA se anuncian con
  SPA_POD_CHOICE_ENUM_Id incluyendo variante sin alfa (BGRx/RGBx).
- onStreamParamChanged (param_changed): spa_format_video_raw_parse → m_videoFormat;
  si hay prop SPA_FORMAT_VIDEO_modifier → parsea lista (values[0] = preferido), quita DRM_FORMAT_MOD_INVALID
  si hay >1, testCreateDmaBuf → si OK responde con buildFormats(fixate=true) + pw_stream_update_params
  (fija UN par formato+modifier, re-dispara param_changed). Si no hay modifier → m_dmabufParams.reset() → MemFd.
- newStreamParams (Buffers): si dmabuf && supportsSyncObj → Buffers CHOICE_RANGE(3,2,4), dataType 1<<SPA_DATA_DmaBuf,
  blocks=planeCount+2, metaType MANDATORY 1<<SPA_META_SyncTimeline; fallback Buffers sin metaType
  (dmabuf: blocks=planeCount; memfd: blocks=1 + size + stride + align 16). Metas: Cursor, VideoDamage (16 regiones),
  Header, y SyncTimeline si hay syncobj. pw_stream_update_params.
- add_buffer: type DmaBuf → DmaBufScreenCastBuffer (GraphicsBufferOptions{format,modifiers}); MemFd →
  MemFdScreenCastBuffer (software=true). user_data=pwBuffer->user_data.
- record(): dequeueBuffer() (espera release_point materializado en buffers con synctimeline); render al framebuffer;
  DmaBuf+synctmeta: espera EGLNativeFence (exportSyncFile(release_point)) antes de render y al final
  acquire_point=release_point+1; release_point=acquire_point+1; moveInto(acquire_point, fence fd).
  Sync implícita: "Implicit sync is broken on Nvidia and with llvmpipe" → glFinish() en NVIDIA/software, si no glFlush().
- chunk->flags = SPA_CHUNK_FLAG_CORRUPTED si el frame no tiene contenido ("do not look at the frame contents").
- addHeader: pts = source->clock().count(), seq; addDamage: SPA_META_VideoDamage rects (bounding rect si >15).
- Cursor: SPA_META_Cursor + SPA_META_Bitmap RGBA (solo modo Metadata envía meta; Embedded lo renderiza).
- pipewirecore.cpp: KWin corre su propio pw_loop con QSocketNotifier (pw_loop_get_fd + pw_loop_iterate).

## KWin v6.6.4 — WindowsRunner (src/plugins/krunner-integration/)
- windowsrunnerinterface.cpp: registra /WindowsRunner en servicio org.kde.KWin con adaptor org.kde.krunner1.
- XML org.kde.krunner1.xml: Match(query s) → out a(sssuda{sv}): (Id:s, Text:s, IconName:s, Type:u, Relevance:d,
  Properties:a{sv}). Type = PlasmaQuery::Type (ver KRunner framework; ExactMatch=100).
- windowsMatch: id = "<action>_<window internalId uuid>"; text = caption(); iconName = window->icon().name();
  relevance 0.8 (exacto), 0.7 (contiene), 0.5 (por desktop); categoryRelevance Highest/Low; properties{"subtext"}.
  Run(id,actionId): id split '_' → action + uuid → findWindow.
- keywords: window, activate, close, min(imize), max(imize), fullscreen, shade, keep above/below, desktop.
- CONSECUENCIA para el código del usuario: en RawKRunnerMatch el campo 3º es el ICONO y el 4º es el TYPE
  (categoryRelevance*100), NO app_id/category. WindowInfo.app_id recibe realmente el icono. Filtrar type==100
  (ExactMatch) funciona por casualidad.

## plasma-desktop v6.6.4 — relevancia limitada para screencast
- No contiene portal ni PipeWire (el backend del portal ScreenCast está en xdg-desktop-portal-kde, repo aparte).
- Único punto tocando screencast: qml/ContextMenu.qml "Hide from Screencast" en taskmanager (propiedad de ventana
  que KWin respeta al ocultar ventanas en capturas — filteredsceneview.cpp en KWin).
- runners/ en plasma-desktop: solo kwin-runner (consola de debug), keys, plasma-desktop. El runner de ventanas
  REAL está en KWin (arriba), no aquí.

## Notas cruzadas PipeWire↔KWin para el proyecto del usuario
- El usuario (CONSUMIDOR) debe: anunciar EnumFormat con modifiers MANDATORY|DONT_FIXATE (lista NVIDIA real del
  sistema, p.ej. de drm_info) y otro EnumFormat sin modifier; en param_changed, si llega modifier → dataType DmaBuf
  (+SyncTimeline si se quiere sync explícita), si no → MemFd. KWin hace la fijación y fallback él mismo.
- KWin produce MemFd (ARGB8888→BGRA) si el consumidor solo anuncia MemFd: camino CPU garantizado para YOLO.
- Con DmaBuf en NVIDIA: sync implícita rota → KWin hace glFinish (latencia) o SyncTimeline (syncobj) si se negocia.
- El consumidor NO debe asumir stride=width*4 ni maxsize válido en DmaBuf; con MemFd el stride está en chunk->stride.
- KWin marca SPA_CHUNK_FLAG_CORRUPTED en frames sin contenido: comprobar antes de procesar.
