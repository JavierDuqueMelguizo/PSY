# PIPEWIRE_YOLO_STREAMING — Índice de análisis (2026-08-24)

Notas por fuente:
- [workspace-propio.md](workspace-propio.md) — análisis completo del repo Rust (módulos, flujo, bugs).
- [pipewire-1.6.md](pipewire-1.6.md) — PipeWire v1.6: stream.c, spa_data/chunk, negociación DMA-BUF, explicit sync.
- [kde-kwin-6.6.4.md](kde-kwin-6.6.4.md) — KWin 6.6.4 productor screencast + WindowsRunner; plasma-desktop 6.6.4.
- [nvidia-drm.md](nvidia-drm.md) — NVIDIA open-gpu-kernel-modules: nv-dmabuf (mmap, can_mmap), modificadores.

## Conclusiones globales (cadena de captura KDE→PipeWire→app)
1. Portal XDG ScreenCast → node_id + fd → pw_context_connect_fd (hecho y funcionando en el proyecto).
2. KWin produce: DmaBuf (con modifier + sync explícita SyncTimeline/syncobj si se negocia) o MemFd SHM ARGB8888
   (SPA_VIDEO_FORMAT_BGRA) como fallback. Formatos DRM↔SPA ver kde-kwin-6.6.4.md.
3. Consumidor (proyecto) debe en param_changed: spa_format_video_raw_parse + detectar SPA_FORMAT_VIDEO_modifier →
   dataType DmaBuf o MemFd, y negociar SPA_META_Header (pts) y opcional SyncTimeline. Patrón: video-play-sync.c (1.6).
4. **Siempre queue_buffer** al final de process (hoy falta → el stream se congela).
5. NVIDIA: DmaBuf VRAM NO es mapeable en CPU (can_mmap=FALSE, ENOTSUP) y DMA_BUF_IOCTL_SYNC es no-op.
   Para YOLO en CPU lo más simple y robusto: forzar MemFd (solo anunciar 1<<SPA_DATA_MemFd). Para GPU: EGL/Vulkan +
   SyncTimeline (o fences).
6. NO hardcodear NVIDIA_MODIFIERS: consultar en runtime (eglQueryDmaBufModifiersEXT / drm_info).

## Bugs críticos del proyecto (detalle en workspace-propio.md)
1. Falta queue_buffer en process. 2. audio_stream mal tipada. 3. Respuestas del portal sin validar (panic en cancel).
4. DmaBuf: mmap sin munmap/sync y sin usar mapoffset/MAPPABLE. 5. RT_PROCESS + println/mmap → xruns.
6. RawKRunnerMatch: 3º=icono, 4º=type (ExactMatch=100). 7. wayland/ muerto. 8. VideoModifier hardcodeado/sobrescrito.
