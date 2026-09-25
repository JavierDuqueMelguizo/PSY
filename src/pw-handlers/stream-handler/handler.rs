
use std::collections::HashMap;
use std::error::Error;

use pipewire::stream::StreamListener;

use crate::pw_handlers::{StreamEvents, StreamEventsHandlerMetadata};
use crate::pw_handlers::stream_handler::BufferMaskTypes;
use crate::{pw_handlers::stream_handler::{ToSpaProperties,StreamBuilder}};
use crate::{log_writeln, log_writeln_async};


pub trait VideoStreamPipeWireHandler{

    fn video_stream( &mut self,
        name : &str,
        node_id : Option<u32>, 
        category : &str,
        role : &str,
        subtype : pipewire::spa::param::format::MediaSubtype,
        buffer : BufferMaskTypes,
        video_properties : pipewire::spa::param::video::VideoInfoRaw) 
        -> Result<pipewire::stream::StreamRc, Box<dyn std::error::Error>>;

    fn default_video_stream(&mut self,
        name : &str,
        node_id : Option<u32>,
        processor_handler : &impl StreamEventsHandlerMetadata
    ) -> Result<pipewire::stream::StreamRc, Box<dyn std::error::Error>>;

}

pub trait AudioStreamPipeWireHandler{

    fn audio_stream(
        &mut self,
        name : &'static str,
        node_id : Option<u32>, 
        category : &'static str,
        role : &'static str,
        subtype : pipewire::spa::param::format::MediaSubtype,
        audio_properties : pipewire::spa::param::video::VideoInfoRaw
    ) -> Result<pipewire::stream::StreamRc, Box<dyn std::error::Error>>;

}

pub struct StreamPipewireHandler{
    pub(crate) thread_loop : pipewire::thread_loop::ThreadLoopRc,//pipewire::main_loop::MainLoopRc,
    pub(crate) context : pipewire::context::ContextRc,
    pub(crate) core : pipewire::core::CoreRc,
    pub(crate) streams : HashMap<String, pipewire::stream::StreamRc>,
    pub(crate) streams_listeners : HashMap<String, Vec<Box<dyn std::any::Any> > >
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
    
    /*
        Construye un stream generico.
     */
    pub fn stream(
        &mut self,
        name : &str,
        node_id : Option<u32>,
        flags : pipewire::stream::StreamFlags,
        properties : pipewire::properties::PropertiesBox, // Parametros para Gestor de Sesión de PW (WirePlumber)
        media_type :  pipewire::spa::param::format::MediaType,
        media_subtype : pipewire::spa::param::format::MediaSubtype,
        format : impl ToSpaProperties, // Parametros para Motor Grafo/DSP de PW
        buffer_params : super::BufferMaskTypes,
        buffer_flags :pipewire::spa::pod::PropertyFlags
    ) -> Result<pipewire::stream::StreamRc, Box<dyn std::error::Error>> {

        
        let stream = StreamBuilder::new()
        .set_flags(flags)
        .next().set_properties(properties)
        .next().set_media_type(media_type)
        .next().set_media_subtype(media_subtype)    
        .next().set_buffer_type(buffer_params, buffer_flags)
        .next().set_formats_params( format)
        .next().build(self.core.clone(), name, node_id)?;

        self.streams.insert(name.into(), stream.clone());

        Ok(stream)
    }

    pub fn setup_listener<T :'static>(&mut self,
        stream_name : &str,
        user_data : T,
        events_handler: &impl StreamEvents<T>
    ) -> Result<(), Box<dyn std::error::Error>>
    {
            // .param_changed(|stream, user_data: &mut UserData<VideoInfoRaw>, id, param|{ 

            //     use pipewire::spa::param::ParamType;
                
            //     log_writeln!("BEGIN: param_changed. Args: UserData: {:?}", user_data.info);
            //     log_writeln!("ID({:?}) : {:?}",id, ParamType::from_raw(id));
            //     match param {
            //         None => log_writeln!("PARAM: NULL"),
            //         Some(value) =>  {
            //             if let Ok(obj) = value.as_object() {
            //                 print!("\x1b[48;2;0;70;0m"); // Pintar fondo verde 
            //                 log_writeln!("POD Type: {:?}, Id: {:?}, Type Id: {:?}", obj.type_(), obj.id(), obj.type_id());
            //                 obj.props().into_iter().for_each(|prop| print_pod(prop));
            //                 log_writeln!("\x1b[0m\n"); // Resetear color
            //             }
            //         }
            //     };
            //     log_writeln!("FIN: param_changed");
            // })

        
        let stream = match self.streams.get(stream_name) {
            Some(value) => value,
            None => return Err(format!("STREAM_NOT_FOUND_EXCEPTION. No existe el stream con nombre : {}", stream_name).into())
        };

        let listener_builder =  stream.add_local_listener_with_user_data(user_data);

        let listener: Result<StreamListener<T>, Box<dyn Error>> =  match listener_builder
        .io_changed(events_handler.get_io_changed_handler())
        .control_info(events_handler.get_control_info_handler())
        .state_changed(events_handler.get_state_changed_handler())
        .param_changed(events_handler.get_param_changed_handler())
        .add_buffer(events_handler.get_add_buffer_handler())
        .remove_buffer(events_handler.get_remove_buffer_handler())
        .drained(events_handler.get_drained_handler())
        .process(events_handler.get_processor_handler())
        .register(){
            Ok(value) => Ok(value),
            Err(error) => Err(format!("LISTENER_BUILDER_EXCEPTION. HA ocurrido un error en la generación del escuchador. Error : {:?}", error).into()),
        };

        let lista= self.streams_listeners.entry(stream_name.into()).or_default();
        lista.push(Box::new(listener.unwrap()));

        Ok(())
    }

    pub fn start(&self) -> (tokio_util::sync::CancellationToken, tokio::task::JoinHandle<()>){

        use tokio_util::sync::CancellationToken;

        let cancel_token = CancellationToken::new();
        let cancel_token_clone = cancel_token.clone();
        let join_handle = tokio::spawn(async move{
            tokio::select! {
                _ = tokio::signal::ctrl_c() => {
                    let _ = log_writeln_async!("[INFO] Apagado solicitado por Ctrl+C");
                }
                _ = cancel_token_clone.cancelled() => {  
                    let _ = log_writeln_async!("[INFO] Apagado solicitado vía cancel token");  
                }
            }
        });

        self.thread_loop.start();

        (cancel_token, join_handle)

        /*
        use tokio_util::sync::CancellationToken;

let cancel_token = CancellationToken::new();
let token_clone = cancel_token.clone();

let join_handle = tokio::spawn(async move {
    tokio::select! {
        _ = token_clone.cancelled() => {
            // Limpieza si la tarea fue cancelada
        }
        _ = realizar_trabajo() => {}
    }
});

// Desde el hilo principal puedes forzar la parada cuando quieras:
cancel_token.cancel();
         */
        
        
    }
}

impl VideoStreamPipeWireHandler for StreamPipewireHandler{
    /*
        Abstracción para construir un stream de video (sin audio)
     */
    fn video_stream(
        &mut self,
        name : &str,
        node_id : Option<u32>, 
        category : &str,
        role : &str,
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

    /*
        Abstracción para construir un stream de video con fuente una pantalla del monitor o una ventana del escritorio
        Simplifica la construcción de un video stream y sirve de ejemplo de como llamar a 
     */
    fn default_video_stream(
        &mut self,
        name : &str,
        node_id : Option<u32>,
        processor_metadata : &impl StreamEventsHandlerMetadata
    ) -> Result<pipewire::stream::StreamRc, Box<dyn std::error::Error>>{

        use pipewire::spa::param::{
            video::{VideoInfoRaw},
        };

        let mut format = VideoInfoRaw::new();
        format.set_format(processor_metadata.get_video_format());
        format.set_modifier(u64::from(processor_metadata.get_modifier()));
        
        // 1) Construimos el Video Stream
        let stream = self.video_stream(
            name,
            node_id, 
            "Capture", 
            "Screen", 
            processor_metadata.get_media_subtype(), 
            processor_metadata.get_buffer_mode() ,
            format 
        );

        stream
    } 
}

impl AudioStreamPipeWireHandler for StreamPipewireHandler{

    // ToDO: No he desarrollado ni testeado nada con Audio aún
    fn audio_stream(
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

}