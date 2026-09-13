

use std::marker::PhantomData;

use pipewire::{spa::param::{format::MediaSubtype, video::VideoFormat}, stream::Stream};

use crate::{log_writeln, pw_handlers::BufferMaskTypes};

pub trait ToStreamProcessor{
    fn get_processor<D>(&self) -> impl FnMut(&Stream, &mut D) + 'static;

    fn get_media_subtype(&self) -> MediaSubtype;
    fn get_buffer_mode(&self) -> BufferMaskTypes;
    fn get_video_format(&self) -> VideoFormat;
}
pub struct VideoProcessorCPU;
pub struct VideoProcessorGPU;

pub struct StreamProcessorPipewireBuilder<Processor>{
    _state: PhantomData<Processor>
}

impl<Processor> StreamProcessorPipewireBuilder<Processor>{
    pub fn new() -> Self{
        Self { _state: PhantomData }
    }
}

impl ToStreamProcessor for StreamProcessorPipewireBuilder<VideoProcessorCPU>{
    fn get_processor<D>(&self) -> impl FnMut(&Stream, &mut D) + 'static {

        |stream : &Stream, user_data : &mut D |{
            let mut buffer = match stream.dequeue_buffer() {
                Some(value) => value,
                None => return
            };

            let datas = buffer.datas_mut();
            if datas.is_empty(){
                return;
            }

            datas.into_iter().for_each(|data|{
                let chunk = data.chunk();
                let offset = chunk.offset() as usize;
                let size = chunk.size() as usize;
                let stride = chunk.stride() as usize; // Salto de bytes por fila (padding)

                if size == 0{
                    return ;
                }

                // 3. Acceder a los bytes planos en CPU (requiere StreamFlags::MAP_BUFFERS)
                if let Some(slice) = data.data() {
                    if offset + size <= slice.len() {
                        let frame_bytes = &slice[offset..offset+size];

                        // `frame_bytes` contiene los píxeles crudos (RAW) del fotograma
                        log_writeln!(
                            "Frame procesado | Tamaño: {} bytes | Stride: {} px/bytes",
                            frame_bytes.len(),
                            stride
                        );
                        log_writeln!("{:#?}",data);
                    }
                }
            });
        }
    }

    fn get_media_subtype(&self) -> MediaSubtype{
        MediaSubtype::Raw
    }
    fn get_buffer_mode(&self) -> BufferMaskTypes {
        BufferMaskTypes::MEM_FD
    }
    fn get_video_format(&self) -> VideoFormat {
        VideoFormat::BGRA
    }
}


impl ToStreamProcessor for StreamProcessorPipewireBuilder<VideoProcessorGPU>{
    fn get_processor<D>(&self) -> impl FnMut(&Stream, &mut D) + 'static {

        |stream : &Stream, user_data : &mut D |{
            let mut buffer = match stream.dequeue_buffer() {
                Some(value) => value,
                None => return
            };

            let datas = buffer.datas_mut();
            if datas.is_empty(){
                return;
            }

            datas.into_iter().for_each(|data|{
                let fd: std::os::fd::RawFd = data.fd();
                if fd >= 0 {
                    let chunk = data.chunk();
                    let offset = chunk.offset() as usize;
                    let size   = chunk.size()   as usize;
                    let len: usize    = (offset + size + 4095) & !4095; // alinear a página a multiplos de 4095
                    
                    // 1) mmap del dma-buf (solo lectura)
                    let ptr = unsafe {
                        libc::mmap(std::ptr::null_mut(), len,
                                libc::PROT_READ, libc::MAP_SHARED, fd, 0)
                    };
                    log_writeln!("{:#?}", ptr);
                    if ptr != libc::MAP_FAILED {
                        
                    }
                }
                 /* fallback: mmap manual + DMA_BUF_IOCTL_SYNC */ 
                        // Hay que traerse 'libc = "0.2" ' y manejarlo manualmente. Algo del siguiente estilo:
                        /*
                        let fd: RawFd = data.fd();
                        if fd >= 0 {
                            let chunk = data.chunk();
                            let offset = chunk.offset() as usize;
                            let size   = chunk.size()   as usize;
                            let len    = (offset + size + 4095) & !4095; // alinear a página

                            // 1) mmap del dma-buf (solo lectura)
                            let ptr = unsafe {
                                libc::mmap(std::ptr::null_mut(), len,
                                        libc::PROT_READ, libc::MAP_SHARED, fd, 0)
                            };
                            if ptr != libc::MAP_FAILED {
                                // 2) sincronización explícita (obligatoria en NVIDIA)
                                let sync = libc::dma_buf_sync {           // struct del kernel
                                    flags: libc::DMA_BUF_SYNC_START | libc::DMA_BUF_SYNC_READ,
                                };
                                unsafe { libc::ioctl(fd, libc::DMA_BUF_IOCTL_SYNC, &sync) };

                                // 3) leer el frame
                                let frame_bytes = unsafe {
                                    std::slice::from_raw_parts(ptr.add(offset) as *const u8, size)
                                };
                                // ... procesar frame_bytes ...

                                let sync_end = libc::dma_buf_sync {
                                    flags: libc::DMA_BUF_SYNC_END | libc::DMA_BUF_SYNC_READ,
                                };
                                unsafe { libc::ioctl(fd, libc::DMA_BUF_IOCTL_SYNC, &sync_end) };
                                unsafe { libc::munmap(ptr, len) };
                            }
                        }
                        */
            });
        }
    }

    fn get_media_subtype(&self) -> MediaSubtype{
        MediaSubtype::Raw
    }
    fn get_buffer_mode(&self) -> BufferMaskTypes {
        BufferMaskTypes::DMA_BUF
    }
    fn get_video_format(&self) -> VideoFormat {
        VideoFormat::BGRA
    }
}
