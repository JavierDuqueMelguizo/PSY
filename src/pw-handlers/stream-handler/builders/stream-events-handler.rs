

use std::{marker::PhantomData, os::fd::FromRawFd};

use drm_fourcc::DrmModifier;
use pipewire::{spa::{param::{format::MediaSubtype, video::VideoFormat}, pod::Pod}, stream::Stream as PwStream};
use pipewire::spa::buffer::Data as PwData;
use tokio::sync::mpsc::Sender as TokioSender;
use crate::{log_writeln, pw_handlers::{BufferMaskTypes, PodValue, VideoData}};

pub trait StreamEvents<Data>{

    fn get_io_changed_handler(&self) -> impl FnMut(&PwStream, &mut Data, u32, *mut std::os::raw::c_void, u32) + 'static {
        |_stream, _user_data, io_area_id, area, size|{
            log_writeln!("Io Changed: [ io_area_id: {io_area_id:?}, Puntero area: {area:?}, size: {size:?} ]");
        }
    }
    fn get_control_info_handler(&self) -> impl FnMut(&PwStream, &mut Data, u32, *const pipewire::sys::pw_stream_control) + 'static{
        |_stream, _user_data, id_control, control|{
            log_writeln!("Control Info: [ id: {id_control:?}, control : {control:?} ]");
        }
    }
    fn get_state_changed_handler(&self) -> impl FnMut(&PwStream, &mut Data, pipewire::stream::StreamState, pipewire::stream::StreamState) + 'static{
        |_stream, _user_data, old_state, new_state|{
            log_writeln!("State Changed: [ From: {old_state:?} --> To: {new_state:?} ]");
        }
    }
    fn get_param_changed_handler(&self) -> impl FnMut(&PwStream, &mut Data, u32, Option<&pipewire::spa::pod::Pod>) + 'static{
        |_stream, _user_data, id, _param|{
            use pipewire::spa::param::ParamType;
            log_writeln!(r#"Param Changed: [
                ID: {:?}({id:?})
            ]"#,ParamType::from_raw(id));
        }
    }
    fn get_add_buffer_handler(&self) -> impl FnMut(&PwStream, &mut Data, *mut pipewire::sys::pw_buffer) + 'static{
        |_stream, _user_data, buffer|{
            log_writeln!("Buffer Added: [ buffer:{buffer:?} ]");
        }
    }
    fn get_remove_buffer_handler(&self) -> impl FnMut(&PwStream, &mut Data, *mut pipewire::sys::pw_buffer) + 'static{
         |_stream, _user_data, buffer|{
            log_writeln!("Buffer Removed: [ buffer: {buffer:?} ]");
         }
    }
    fn get_drained_handler(&self) -> impl FnMut(&PwStream, &mut Data) + 'static{
        |_stream, _user_data|{
            log_writeln!("Drained: []");
        }
    }

    type ProcessorHandler : FnMut(&PwStream, &mut Data) + 'static;
    fn get_processor_handler(&self) -> Self::ProcessorHandler;

  
}

pub trait StreamEventsHandlerMetadata{
    fn get_media_subtype(&self) -> MediaSubtype;
    fn get_buffer_mode(&self) -> BufferMaskTypes;
    fn get_modifier(&self) -> DrmModifier{
        DrmModifier::Invalid // Este valor por defecto no se añade 
    }
    fn get_video_format(&self) -> VideoFormat;
}
pub struct VideoEventsHandlerCPU;
pub struct VideoEventsHandlerGPU;

pub struct StreamEventsHandler<Processor>{
    _state: PhantomData<Processor>
}

impl<Processor> StreamEventsHandler<Processor>{
    pub fn new() -> Self{
        Self { 
            _state: PhantomData 
        }
    }
}

// Sobre los tipos de BufferMaskTYpes:
// https://chatgpt.com/s/t_6aa997eb58448191831026be5769959bs
impl StreamEventsHandler<VideoEventsHandlerCPU>{
    pub fn hola(&self){}
}


impl StreamEventsHandlerMetadata for StreamEventsHandler<VideoEventsHandlerCPU>{

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
impl StreamEvents<TokioSender<VideoData<4>>> for StreamEventsHandler<VideoEventsHandlerCPU>{

    type ProcessorHandler = fn(&PwStream, &mut TokioSender<VideoData<4>>);
    fn get_processor_handler(&self) -> Self::ProcessorHandler {
    
        |stream, tx |{
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

                if size == 0{ return; }
                // Acceder a los bytes planos en CPU (requiere StreamFlags::MAP_BUFFERS)
                if let Some(slice) = data.data() {
                    if offset + size <= slice.len() {
                        // `frame_bytes` contiene los píxeles crudos (RAW) del fotograma
                        let frame_bytes: &[u8] = &slice[offset..offset+size];

                        match tx.try_send(VideoData::new(
                            bytes::Bytes::copy_from_slice(frame_bytes),
                            stride)
                        ){
                            Ok(_) => {},
                            Err(error) => eprintln!("Error mandando frame bytes. Error : {:?}", error),
                        }
                        // Debugeo
                        // log_writeln!(
                        //     "Frame procesado | Tamaño: {} bytes | Stride: {} px/bytes",
                        //     frame_bytes.len(),
                        //     stride
                        // );
                        // log_writeln!("{:#?}",data);
                    }
                }
            });
        }
    }
    
}

impl StreamEventsHandlerMetadata for StreamEventsHandler<VideoEventsHandlerGPU>{
    fn get_media_subtype(&self) -> MediaSubtype{
        MediaSubtype::Raw
    }
    fn get_buffer_mode(&self) -> BufferMaskTypes {
        BufferMaskTypes::DMA_BUF
    }
    fn get_modifier(&self) -> DrmModifier {
        DrmModifier::Linear
    }
    fn get_video_format(&self) -> VideoFormat {
        VideoFormat::BGRA
    }
}

impl StreamEvents<TokioSender<bytes::Bytes>> for StreamEventsHandler<VideoEventsHandlerGPU>{

    type ProcessorHandler = fn(&PwStream, &mut TokioSender<bytes::Bytes>);
    fn get_processor_handler(&self) -> Self::ProcessorHandler {
        |stream , tx |{
            let mut buffer = match stream.dequeue_buffer() {
                Some(value) => value,
                None => return
            };

            let datas = buffer.datas_mut();
            if datas.is_empty(){ return; }

            for data in datas{
                // 1) Duplicar FD para no quitar a PipeWire el suyo
                let fd: std::os::fd::RawFd = unsafe {libc::dup(data.fd())};
                if fd < 0 { continue;}
                    
                // ToDo: Obviar mapeo(+ lento) e implementar procesamiento imagen en GPU directamente con Vulkan
                // Ahora mismo no consigo ninguna ventaja respecto a MEM_FD
                // Todo esto de aqui abajo sobra...:
                // 2) Mapeo de la memoria del Hardware a memoria accesible por CPU mediante DMA
                
                if let Ok(data) = unsafe{ dma_buf::DmaBuf::from_raw_fd(fd) }.memory_map() {
                    
                    // 3) DMA_BUF_SYNC_START -> Lectura -> DMA_BUF_SYNC_END
                    let _ = data.read(|bytes, args|{
                        println!("Bytes leidos: {:}, Args: {:?}",bytes.len(), args);
                        // 4) Procesamiento de frames...
                        match tx.try_send(bytes::Bytes::copy_from_slice(bytes)){
                            Ok(_) =>{},
                            Err(error) => eprintln!("Error mandando frame bytes. Error : {:?}", error),
                        }
                        return Ok(());
                    }, Some(0));
                    
                    // 5) Desmapeo y liberación de la memoria
                    drop(data);
                }
            }   
        }
    }
}
