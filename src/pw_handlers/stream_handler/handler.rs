
use std::any::Any;
use std::collections::HashMap;


pub struct UserData<T>{
    info : T 
}


use crate::pw_handlers::ToStreamProcessor;
use crate::pw_handlers::stream_handler::BufferMaskTypes;
use crate::utils::print_pod;
use crate::{pw_handlers::stream_handler::{ToSpaProperties,StreamPipewireBuilder}};
use crate::{log_writeln, log_writeln_async};

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

            self.streams_listeners.clear();
            for stream in self.streams.values(){
                if let Err(err) = stream.flush(true) {
                    let _ = log_writeln!("[ERROR] Error en vaciado del buffer de Stream: {err:#?}");
                }

                if let Err(err) = stream.disconnect() {
                    let _ = log_writeln!("[ERROR] Error en vaciado del buffer de Stream: {err:#?}");   
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
        format : impl ToSpaProperties, // Parametros para Motor Grafo/DSP de PW
        buffer_params : super::BufferMaskTypes,
        buffer_flags :pipewire::spa::pod::PropertyFlags 
    ) -> Result<pipewire::stream::StreamRc, Box<dyn std::error::Error>> {

        
        let stream = StreamPipewireBuilder::new()
        .set_flags(flags)
        .next().set_properties( properties)
        .next().set_media_type(media_type)
        .next().set_media_subtype(media_subtype)    
        .next().set_buffer_type(buffer_params, buffer_flags)
        .next().set_formats_params( format)
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
        buffer : BufferMaskTypes,
        video_properties : pipewire::spa::param::video::VideoInfoRaw
    ) -> Result<pipewire::stream::StreamRc, Box<dyn std::error::Error>>{

        use pipewire::stream::StreamFlags;
        use pipewire::spa::param::format::{MediaType};
        use pipewire::spa::pod::PropertyFlags;

        // A algún genio se le ocurrió que el valor '0x0' (DRM_FORMAT_MOD_LINEAR) es un valor con sentido para BufferMaskTypes::DMA_BUF
        // Y el que no guarda nada e indica que se debe ignorar es '0x00ffffffffffffff' (DRM_FORMAT_MOD_INVALID)
        // if buffer.contains( BufferMaskTypes::MEM_FD) && video_properties.modifier() == 0 {
        //     video_properties.set_modifier(0x00ffffffffffffff);
        // };
        
        self.stream(name, node_id, 
            StreamFlags::AUTOCONNECT | StreamFlags::MAP_BUFFERS | StreamFlags::RT_PROCESS,
            pipewire::properties::properties! {
                *pipewire::keys::MEDIA_TYPE => "Video",
                *pipewire::keys::MEDIA_CATEGORY => category,
                *pipewire::keys::MEDIA_ROLE => role,
            },
            MediaType::Video, subtype, 
            video_properties,
            buffer, 
            PropertyFlags::MANDATORY
        )
    } 

    pub fn default_video_stream(
        &mut self,
        name : &'static str,
        node_id : Option<u32>,
        processor_handler : impl ToStreamProcessor
    ) -> Result<pipewire::stream::StreamRc, Box<dyn std::error::Error>>{

        use pipewire::spa::param::{
            video::{VideoInfoRaw},
            format::MediaSubtype,
            
        };

        let mut format = VideoInfoRaw::new();
        format.set_format(processor_handler.get_video_format());
        
        let user_data = UserData{
            info : format
        };
        let stream = self.video_stream(
            name, node_id, 
            "Capture", 
            "Screen", 
            processor_handler.get_media_subtype(), 
            processor_handler.get_buffer_mode() ,
            format 
        );

        let listener: pipewire::stream::StreamListener<UserData<VideoInfoRaw>> = stream.as_ref().expect("Error recuperando stream")
            .add_local_listener_with_user_data(user_data)
            .io_changed(|stream, user_data: &mut UserData<VideoInfoRaw>, io_area_id, area, size|{
                log_writeln!("Io Changed: Data: [ io_area_id: {io_area_id:?}, Puntero area: {area:?}, size: {size:?} ]");
            })
            .control_info(|stream, user_data: &mut UserData<VideoInfoRaw>, id_control, control|{
                log_writeln!("Control Info: [ id: {id_control:?}, control :  ]");
            })
            .state_changed(|stream, user_data: &mut UserData<VideoInfoRaw>, old_state, new_state|{
                log_writeln!("Old State: {old_state:?} --> New State: {new_state:?}");
            })
            .param_changed(|stream, user_data: &mut UserData<VideoInfoRaw>, id, param|{ 

                use pipewire::spa::param::ParamType;
                
                log_writeln!("BEGIN: param_changed. Args: UserData: {:?}", user_data.info);
                log_writeln!("ID({:?}) : {:?}",id, ParamType::from_raw(id));
                match param {
                    None => log_writeln!("PARAM: NULL"),
                    Some(value) =>  {
                        if let Ok(obj) = value.as_object() {
                            print!("\x1b[48;2;0;70;0m"); // Pintar fondo verde 
                            log_writeln!("POD Type: {:?}, Id: {:?}, Type Id: {:?}", obj.type_(), obj.id(), obj.type_id());
                            obj.props().into_iter().for_each(|prop| print_pod(prop));
                            log_writeln!("\x1b[0m\n"); // Resetear color
                        }
                    }
                };
                log_writeln!("FIN: param_changed");
            })
            .process(processor_handler.get_processor())
            .add_buffer(|stream, user_data: &mut UserData<VideoInfoRaw>, buffer|{
                log_writeln!("Buffer Added: {buffer:?}");
            })
            .remove_buffer(|stream, user_data: &mut UserData<VideoInfoRaw>, buffer|{
                log_writeln!("Buffer Removed: {buffer:?}");
            })
            .drained(|stream: &pipewire::stream::Stream, user_data: &mut UserData<VideoInfoRaw>|{
                log_writeln!("Hola desde drained");
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
        use pipewire::spa::pod::PropertyFlags;

        self.stream(name, node_id, 
            StreamFlags::AUTOCONNECT | StreamFlags::MAP_BUFFERS | StreamFlags::RT_PROCESS,
            pipewire::properties::properties! {
                *pipewire::keys::MEDIA_TYPE => "Audio",
                *pipewire::keys::MEDIA_CATEGORY => category,
                *pipewire::keys::MEDIA_ROLE => role,
            },
            MediaType::Video, subtype, audio_properties,
            BufferMaskTypes::all(), PropertyFlags::MANDATORY
        )
    } 

    pub fn start(&self) -> (tokio::sync::oneshot::Sender<bool>, tokio::task::JoinHandle<()>){

        use tokio::sync::oneshot;

        let (sx, rx) = oneshot::channel::<bool>(); 
        let join_handle = tokio::spawn(async move{
            tokio::select! {
                _ = tokio::signal::ctrl_c() => {
                    let _ = log_writeln_async!("[INFO] Apagado solicitado por Ctrl+C");
                }
                res = rx => { 
                    match res {
                        Ok(val) => {
                            let _ = log_writeln_async!("[INFO] Apagado solicitado vía canal (valor: {val})");
                        }
                        Err(_) => {
                            // Sucede si el extremo emisor (tx) se destruye sin enviar nada
                            let _ = log_writeln_async!("[WARN] El emisor del canal fue destruido");
                        }
                    }
                }
            }
        });

        self.thread_loop.start();

        (sx, join_handle)
        
    }

}