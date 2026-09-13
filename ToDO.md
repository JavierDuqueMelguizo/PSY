# ToDO — Captura PipeWire → YOLO en GPU (CUDA)

*Generado: 2026-08-25*

## Decisión de arquitectura

- Objetivo: **YOLO en GPU (CUDA)** consumiendo el stream de captura de KWin.
- Camino elegido: **DMA-BUF cero-copias** (los píxeles nunca salen de la GPU; solo las
  bounding boxes bajan a CPU).
- `MemFd` (SHM) queda solo como fallback de negociación.
- Por qué no mmap de CPU sobre DmaBuf: en NVIDIA los buffers de VRAM no son mapeables
  (`can_mmap=FALSE` → `ENOTSUPP`) y `DMA_BUF_IOCTL_SYNC` es no-op (sin
  `begin_cpu_access/end_cpu_access`). El mmap actual en `handler.rs` es inviable.

## 1. Negociación dinámica (`builder.rs` + `param_changed`)

- [ ] `SPA_PARAM_BUFFERS_dataType` = `1 << SPA_DATA_DmaBuf` (línea 253). Quitar el mix
      `MemFd | DmaBuf`.
- [ ] Sustituir `build_nvidia_mod_property()` por consulta de modifiers **en runtime**:
      `eglQueryDmaBufModifiersEXT` (DRM_FORMAT_ARGB8888) o
      `vkGetPhysicalDeviceImageFormatProperties2`. Mantener `DRM_FORMAT_MOD_INVALID`
      solo como último recurso.
- [ ] Anunciar DOS `EnumFormat` por formato:
      1. con `SPA_FORMAT_VIDEO_modifier`, flags `MANDATORY | DONT_FIXATE`
         (Choice::Enum de longs) — prioriza DMA-BUF;
      2. sin modifier (fallback SHM).
- [ ] Mover la fijación a `param_changed`: parsear el formato negociado
      (`spa_format_video_raw_parse`), guardar formato + modifier en `user_data`, y
      responder con `pw_stream_update_params` (patrón de OBS `on_param_changed_cb`).

## 2. Sincronización explícita (obligatoria en NVIDIA)

- [ ] Negociar en `param_changed`:
      - `SPA_PARAM_Meta` con `SPA_META_SyncTimeline` (size de `spa_meta_sync_timeline`).
      - `SPA_PARAM_Buffers` con `metaType = 1 << SPA_META_SyncTimeline`, flag `MANDATORY`
        → buffers con `blocks = planos + 2`; las 2 últimas datas son `SyncObj`
        (fds acquire/release).
- [ ] Abrir render node (`/dev/dri/renderD128`) y en `process`:
      - `drmSyncobjTimelineWait(acquire_handle, acquire_point)` ANTES de leer;
      - `drmSyncobjTimelineSignal(release_handle, release_point)` al terminar.
- [ ] Dependencias: libdrm (crate `drm` / `libdrm-sys`, o ioctls con `libc`).

## 3. Importar el dma-buf en CUDA

- [ ] Ruta A (EGL→GL→CUDA, como `NvencEncoder` de waycap-rs):
      `eglCreateImageKHR` (EGL_DMA_BUF_PLANE0_FD/OFFSET/PITCH/MODIFIER_{LO,HI}) →
      textura GL → `cuGraphicsGLRegisterImage` → `cuGraphicsMapResources` →
      `cuMemcpy2D_v2` a buffer CUDA.
- [ ] Ruta B (más limpia): `cust::external::ExternalMemory::import(fd)` +
      `mapped_buffer()`. Si CUDA no acepta el fd del dma-buf, importar en Vulkan
      (`VkImportMemoryFdInfoKHR`) y exportar fd OPAQUE para CUDA (como
      `persistent_memory_fd` de waycap-rs).
- [ ] BGRA = 1 plano; stride de `chunk.stride()`. En DmaBuf NO fiarse de
      `maxsize`/`size`: obtener el tamaño localmente desde el fd.

## 4. Inferencia YOLO en CUDA

- [ ] Opciones (en orden de esfuerzo):
      1. `ort` (ONNX Runtime) con CUDA EP — revisar soporte de IO binding para tensores
         que ya viven en GPU.
      2. `tch` (libtorch CUDA) — `Tensor::to_device(Cuda)`.
      3. TensorRT — máxima velocidad, máximo esfuerzo.
- [ ] Salida: solo bounding boxes a CPU (datos diminutos).

## 5. Arreglos colaterales (`handler.rs`)

- [ ] **Añadir `queue_buffer` al final de `process`** (crítico: sin esto la cola se
      agota y el stream se congela).
- [ ] Quitar `RT_PROCESS` de los flags si se hace trabajo GPU bloqueante en el
      callback (usar `AUTOCONNECT | MAP_BUFFERS` y activar en `PAUSED` con
      `set_active(true)`, patrón de OBS), o mover el trabajo a un hilo dedicado.
- [ ] Comprobar `chunk.flags() & SPA_CHUNK_FLAG_CORRUPTED` antes de procesar.
- [ ] Guardar formato/modifier negociados en `user_data` (`UserData.info`).
- [ ] Eliminar la rama `mmap` del DmaBuf (inviable en NVIDIA).

## Referencias

- waycap-rs: `src/encoders/nvenc_encoder.rs` (EGL/Vulkan → CUDA, copia GPU→GPU).
- OBS Studio: `plugins/linux-pipewire/pipewire.c` (negociación, SyncTimeline,
  `queue_buffer`).
- PipeWire doc oficial: `dma-buf.dox` (negociación DMA-BUF).
- Notas locales: `memories/nvidia-drm.md`, `memories/pipewire-1.6.md`,
  `memories/kde-kwin-6.6.4.md`.
