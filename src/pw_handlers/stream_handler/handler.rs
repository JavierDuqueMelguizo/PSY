
use std::collections::HashMap ;
use std::{io::Write};


pub struct UserData<T>{
    info : T 
}

use pipewire::spa::sys::off_t;

use crate::{pw_handlers::{ToSpaProperties, stream_handler::StreamPipewireBuilder}, utils::logging::LOGGER};

pub struct StreamPipewireHandler{
    pub(super) thread_loop : pipewire::thread_loop::ThreadLoopRc,//pipewire::main_loop::MainLoopRc,
    pub(super) context : pipewire::context::ContextRc,
    pub(super) core : pipewire::core::CoreRc,
    pub(super) streams : HashMap<&'static str, pipewire::stream::StreamRc>,
    pub(super) streams_listeners : HashMap<&'static str, Vec<pipewire::stream::StreamListener<UserData<pipewire::spa::param::video::VideoInfoRaw>>>>
}

impl Drop for StreamPipewireHandler{
    fn drop(&mut self) {
        
        {
            let _guard = self.thread_loop.lock();
        
            let mut logger = LOGGER.try_lock().ok();

            self.streams_listeners.clear();
            for stream in self.streams.values(){
                if let Err(err) = stream.flush(true) {
                    if let Some(logger) = logger.as_mut(){
                        let _ = writeln!(logger, "[ERROR] Error en vaciado del buffer de Stream: {err:#?}");
                    }
                }

                if let Err(err) = stream.disconnect() {
                    if let Some(logger) = logger.as_mut() {
                        let _ = writeln!(logger, "[ERROR]. Error desconectando Stream. Error:{:#?}", err);
                    }    
                }
            }
            self.streams.clear();
        }
        self.thread_loop.stop();
        
    }
}

impl StreamPipewireHandler{
    
    pub fn stream(
        &mut self,
        name : &'static str,
        node_id : Option<u32>,
        flags : pipewire::stream::StreamFlags,
        properties : pipewire::properties::PropertiesBox, // Parametros para Gestor de Sesión de PW (WirePlumber)
        media_type :  pipewire::spa::param::format::MediaType,
        media_subtype : pipewire::spa::param::format::MediaSubtype,
        format : impl ToSpaProperties // Parametros para Motor Grafo/DSP de PW
    ) -> Result<pipewire::stream::StreamRc, Box<dyn std::error::Error>> {

        let stream = StreamPipewireBuilder::new()
        .set_flags(flags)
        .next().set_properties( properties)
        .next().set_formats(media_type, media_subtype, format)
        .next().build(self.core.clone(), name, node_id)?;

        self.streams.insert(name, stream.clone());

        Ok(stream)
    }

    pub fn video_stream(
        &mut self,
        name : &'static str,
        node_id : Option<u32>, 
        category : &'static str,
        role : &'static str,
        subtype : pipewire::spa::param::format::MediaSubtype,
        video_properties : pipewire::spa::param::video::VideoInfoRaw
    ) -> Result<pipewire::stream::StreamRc, Box<dyn std::error::Error>>{

        use pipewire::stream::StreamFlags;
        use pipewire::spa::param::format::{MediaType};

        self.stream(name, node_id, 
            StreamFlags::AUTOCONNECT | StreamFlags::MAP_BUFFERS | StreamFlags::RT_PROCESS,
            pipewire::properties::properties! {
                *pipewire::keys::MEDIA_TYPE => "Video",
                *pipewire::keys::MEDIA_CATEGORY => category,
                *pipewire::keys::MEDIA_ROLE => role,
            },
            MediaType::Video, subtype, video_properties
        )
    } 

    pub fn default_video_stream(
        &mut self,
        name : &'static str,
        node_id : Option<u32>
    ) -> Result<pipewire::stream::StreamRc, Box<dyn std::error::Error>>{

        use pipewire::spa::param::{
            video::{VideoInfoRaw, VideoFormat},
            format::MediaSubtype
        };

        let mut format = VideoInfoRaw::new();
        format.set_format(VideoFormat::BGRA);
        
        let user_data = UserData{
            info : format
        };
        let stream = self.video_stream(
            name, node_id, "Capture", "Screen", MediaSubtype::Raw,format 
        );

        let listener = stream.as_ref().expect("Error recuperando stream")
            .add_local_listener_with_user_data(user_data)
            .add_buffer(|stream, user_data, buffer|{
                println!("Hola desde add_buffer");
            })
            .remove_buffer(|stream, user_data, buffer|{
                println!("Hola desde remove_buffer");
            })
            .control_info(|stream, user_data, id_control, control|{
                println!("Hola desde control_info");
            })
            .io_changed(|stream, user_data, io_area_id, area, size|{
                println!("Hola desde io_changed");
            })
            .state_changed(|stream, user_data, old_state, new_state|{
                println!("BEGIN: state_changed");
                println!("Old State: {:#?}",old_state);
                println!("New State: {:#?}",new_state);
                println!("FIN: state_changed");
            })
            .param_changed(|stream, user_data, id, param|{ 
                println!("BEGIN: param_changed");
                println!("ID : {id}");
                match param {
                    None => println!("PARAM: NULL"),
                    Some(value) => println!("PARAM: {:?}", value.as_bytes())
                };
                println!("FIN: param_changed");
            })
            .process(|stream, user_data|{
                println!("BEGIN: process");
                // 1. Desencolar el buffer disponible desde la cola de PipeWire
                let mut buffer = match stream.dequeue_buffer() {
                    Some(value) => value,
                    None => return
                };

                let datas = buffer.datas_mut();
                if datas.is_empty(){
                    return;
                }

                // 2 Obtener el primer plano de datos del buffer
                let data = &mut datas[0];
                match data.type_() { // Dependiendo de los parametros, puede que se mapee en CPU (MemFD) o GPU (DmaFD)
                    pipewire::spa::buffer::DataType::MemFd => { 
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
                                println!(
                                    "Frame procesado | Tamaño: {} bytes | Stride: {} px/bytes",
                                    frame_bytes.len(),
                                    stride
                                );
                                println!("{:#?}",data);
                            }
                        }
                    }
                    pipewire::spa::buffer::DataType::DmaBuf => { 
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
                    }
                    _ => {
                        println!("Eh?");
                    }
                }
               
                println!("FIN: process");

            })
            .drained(|stream, user_data|{
                println!("Hola desde drained");
            })
            .register()?;


        let lista = self.streams_listeners.entry(name).or_default();
        lista.push(listener);

        stream
    } 

    pub fn audio_stream(
        &mut self,
        name : &'static str,
        node_id : Option<u32>, 
        category : &'static str,
        role : &'static str,
        subtype : pipewire::spa::param::format::MediaSubtype,
        audio_properties : pipewire::spa::param::video::VideoInfoRaw
    ) -> Result<pipewire::stream::StreamRc, Box<dyn std::error::Error>>{

        use pipewire::stream::StreamFlags;
        use pipewire::spa::param::format::{MediaType};

        self.stream(name, node_id, 
            StreamFlags::AUTOCONNECT | StreamFlags::MAP_BUFFERS | StreamFlags::RT_PROCESS,
            pipewire::properties::properties! {
                *pipewire::keys::MEDIA_TYPE => "Audio",
                *pipewire::keys::MEDIA_CATEGORY => category,
                *pipewire::keys::MEDIA_ROLE => role,
            },
            MediaType::Video, subtype, audio_properties
        )
    } 

    pub fn start(&self) -> (tokio::sync::oneshot::Sender<bool>, tokio::task::JoinHandle<()>){

        use tokio::sync::oneshot;

        let (sx, rx) = oneshot::channel::<bool>(); 
        let join_handle = tokio::spawn(async move{
            tokio::select! {
                _ = tokio::signal::ctrl_c() => {
                    let mut logger = LOGGER.lock().await;
                    let _ = writeln!(logger, "[INFO] Apagado solicitado por Ctrl+C");
                }
                res = rx => {
                    let mut logger = LOGGER.lock().await;   
                    match res {
                        Ok(val) => {
                            let _ = writeln!(logger, "[INFO] Apagado solicitado vía canal (valor: {val})");
                        }
                        Err(_) => {
                            // Sucede si el extremo emisor (tx) se destruye sin enviar nada
                            let _ = writeln!(logger, "[WARN] El emisor del canal fue destruido");
                        }
                    }
                }
            }
        });

        self.thread_loop.start();

        (sx, join_handle)
        
    }

}