# NVIDIA open-gpu-kernel-modules (rama main) — 2026-08-24

Fuentes: kernel-open/nvidia/nv-dmabuf.c, kernel-open/nvidia-drm/nvidia-drm-{gem-dma-buf.c,helper.h,drv.c}.

## Exportación DMA-BUF NVIDIA (nv-dmabuf.c)
- nv_dma_buf_create(): dma_buf_export con exp_name "nv_dmabuf"; priv contiene handles de memoria RM duplicados
  (rm_dma_buf_dup_mem_handle), attrs: can_mmap (del RM), cache_type, read_only_mem, memory_type (SYSTEM vs VRAM),
  mapping_type DEFAULT/FORCE_PCIE, static_phys_addrs.
- struct dma_buf_ops nv_dma_buf_ops = { attach, map_dma_buf, unmap_dma_buf, release, mmap,
  .map=STUB devuelve NULL, .unmap=STUB, .map_atomic/.unmap_atomic=STUB }.
  **NO implementa begin_cpu_access/end_cpu_access** → DMA_BUF_IOCTL_SYNC sobre dmabuf NVIDIA es NO-OP:
  la sincronización CPU↔GPU NO se puede hacer con ese ioctl; hay que usar fences EGL/syncobj.
- .map stub NULL ⇒ el mmap dinámico del framework dma-buf queda deshabilitado; el mmap userspace va por
  nv_dma_buf_mmap().

## nv_dma_buf_mmap (por qué mmap en VRAM falla o es lento)
1. Si !map_attrs.can_mmap → -ENOTSUPP ("mmap is not allowed"). Para memoria de framebuffer (VRAM) el RM
   normalmente marca can_mmap=FALSE ⇒ el mmap del código del usuario fallará SIEMPRE con fb en VRAM.
2. Si está permitido (memoria de sistema / si el exportador pidió bAllowMmap): mapea vía BAR1
   ("If the GPU has BAR1 < FB... map the whole buffer to BAR1"), usa nv_remap_page_range (PFN, VM_PFNMAP),
   no vm_insert_pages; NO soporta pin_user_pages sobre ese mapeo. Acceso CPU vía BAR1 = lentísimo en GPU discreta.
3. cache_type del RM (normalmente uncached) se aplica al prot de página.
4. Exige dma-buf completamente attachado (num_objects==total_objects) para mapear.

## Importación y topology
- nv_dma_buf_attach(): comprueba topología PCI (nv_grdma_pci_topology_supported); con FORCE_PCIE salta IOMMU
  (peer-to-peer). import: nv_dma_import_dma_buf → dma_buf_map_attachment.
- Modo coherente (CDMM): usa páginas MEMORY_DEVICE_COHERENT de UVM + dma_map_sg_attrs.

## Modificadores NVIDIA
- nvidia-drm-helper.h define DRM_FORMAT_MOD_NVIDIA_BLOCK_LINEAR_2D(c,s,g,k,h) =
  fourcc_mod_code(NVIDIA, 0x10 | (h&0xf) | ((k&0xff)<<12) | ((g&0x3)<<20) | ((s&1)<<22) | ((c&0x7)<<23));
  vendor NVIDIA=0x03 → prefijo 0x0300000000000000.
- Ejemplos: g=1 → 0x0300000000100010; g=2 (16BPT) → 0x0300000000200010. La lista hardcodeada del proyecto
  (6 valores 0x0300000000606010..15 y 6 valores 0x0300000000E08010..15 + DRM_FORMAT_MOD_INVALID) decodifica con la
  macro BLOCK_LINEAR_2D(c,s,g,k,h) como (c=0,s=1,g=2,k=6,h=0..5) y (c=1,s=1,g=2,k=8,h=0..5). CORRECCIÓN 2026-08-25:
  antes anoté k=96/224 (mal); los campos correctos son k=6 (c=0) y k=8 (c=1). h solo llega a 5 (6 valores por grupo).
  ORIGEN: lista de fallback hardcodeada de OBS Studio (época sin eglQueryDmaBufModifiersEXT en drivers NVIDIA),
  copiada a waycap-rs (comentario del autor: "Literally stole these by looking at what OBS uses") y de ahí al proyecto.
  DRM_FORMAT_MOD_INVALID = 0x00FFFFFFFFFFFFFF = 72057594037927935 = ((1<<56)-1).
- RECOMENDACIÓN: NO hardcodear modificadores; consultarlos en runtime (eglQueryDmaBufModifiersEXT /
  vkGetPhysicalDeviceImageFormatProperties2 / drm_info / drmGetFormatModifierName). KWin anuncia los suyos
  (backend->supportedFormats()) y fija uno; el consumidor solo tiene que ofrecer la intersección real de su driver.

## Resumen para el proyecto del usuario (camino GPU NVIDIA)
- mmap a pelo del DmaBuf NVIDIA: NO funcionará para buffers de VRAM (can_mmap=FALSE → ENOTSUP) y, si funcionara,
  sería lento y SIN sincronización (DMA_BUF_IOCTL_SYNC es no-op aquí). El código actual del proyecto es inviable tal cual.
- Camino correcto GPU: importar el DmaBuf con EGL (EGL_PLATFORM_DEVICE_EXT + eglCreateImageKHR + modifiers) o Vulkan,
  y sincronizar con SyncTimeline (syncobj) o fences EGL. KWin ya lo hace así (ver notas kde-kwin).
- Camino correcto CPU para YOLO: negociar SOLO MemFd (anunciar dataType 1<<SPA_DATA_MemFd sin EnumFormat con
  modifier) → KWin hace render a SHM ARGB8888 (BGRA) y da memoria CPU mapeable con stride en chunk->stride.
