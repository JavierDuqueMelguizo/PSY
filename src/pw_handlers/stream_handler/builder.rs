use std::{collections::HashMap, marker::PhantomData};

use crate::{kv, pw_handlers::StreamPipewireHandler};


///////////////////////////////////////////////////////////////////////////////////////
/// Builder para construir el core de comunicacion con Pipewire 
pub struct NeedMainLoop;
pub struct NeedContext;
pub struct NeedCore;
pub struct Ready;

pub struct PipewireHandlerBuilder<State>{
    thread_loop : pipewire::thread_loop::ThreadLoopRc,//pipewire::main_loop::MainLoopRc,
    context : Option<pipewire::context::ContextRc>,
    core : Option<pipewire::core::CoreRc>,
    _state: PhantomData<State>,
}

impl PipewireHandlerBuilder<NeedMainLoop> {

    pub fn new(properties : Option<pipewire::properties::PropertiesBox>) -> Result<PipewireHandlerBuilder<NeedContext>, Box<dyn std::error::Error>> {

        use pipewire::main_loop::MainLoopRc;
        use pipewire::thread_loop::ThreadLoopRc;

        let dict = match properties.as_ref(){
            None => None,
            Some(value) => Some(value.dict())
        };
        let main_loop = match MainLoopRc::new(dict){
            Err(err) => return Err(format!("Ha ocurrido un error iniciando MainLoop. Error {}", err).into()),
            Ok(value) => value
        };

        let thread_loop = unsafe{
              let thread_loop = match ThreadLoopRc::new(Some("jdm-stream"), dict){
                Err(err) => return Err(format!("Ha ocurrido un error iniciando MainLoop. Error {}", err).into()),
                Ok(value) => value
              };
              thread_loop
        };
      
        
        Ok(PipewireHandlerBuilder{
            thread_loop: thread_loop,
            context: None,
            core: None,
            _state: PhantomData
        })
    }
}

impl PipewireHandlerBuilder<NeedContext> {

    pub fn context(self, properties : Option<pipewire::properties::PropertiesBox>) -> Result<PipewireHandlerBuilder<NeedCore>, Box<dyn std::error::Error>>{
        
        use pipewire::context::ContextRc;
        
        let thread_loop = self.thread_loop;
        let context  = match ContextRc::new(&thread_loop, properties) {
            Err(error) => return Err(format!("Error construyendo Contexto. Error : {:#?}", error).into()),
            Ok(value) => value
        };

        Ok(PipewireHandlerBuilder {
            thread_loop: thread_loop,
            context: Some(context),
            core: None,
            _state : PhantomData
        })
    }
}

impl PipewireHandlerBuilder<NeedCore> {

    pub fn core(self, fd : std::os::fd::OwnedFd, properties : Option<pipewire::properties::PropertiesBox>) -> Result<PipewireHandlerBuilder<Ready>, Box<dyn std::error::Error>> {

        let context = match self.context.as_ref() {
            None => return Err("Error construyendo contexto. Contexto es NULL".into()),
            Some(value) => value
        };
        let core = match context.connect_fd_rc(fd, properties){
            Err(error) => return Err(format!("Error construyendo Core. Error : {}", error).into()),
            Ok(value) => value
        };

        Ok(PipewireHandlerBuilder {
            thread_loop: self.thread_loop,
            context: self.context,
            core: Some(core),
            _state : PhantomData
        })
    }
}

impl PipewireHandlerBuilder<Ready> {
    pub fn build(self) -> Result<StreamPipewireHandler, Box<dyn std::error::Error>>  {

        let context = match self.context {
            None => return Err("Valor NULL en Contexto".into()),
            Some(value) => value
        };

        let core = match self.core {
            None => return Err("Valor NULL en Core".into()),
            Some(value) => value
        };
        Ok(StreamPipewireHandler{
            thread_loop:  self.thread_loop,
            context: context,
            core: core,
            streams: HashMap::new(),
            streams_listeners: HashMap::new()
        })
    }
}


///////////////////////////////////////////////////////////////////////////////////////
// Builder para configurar un objecto Stream (para grabar Pantalla, Audio, Camara, ...)
// Requiere una instancia de pipewire::core::CoreRc instanciado

pub struct NeedFlags;
pub struct NeedProperties;
pub struct NeedFormatting;

pub struct StreamReady;

pub struct StreamPipewireBuilder<State>{
    flags: pipewire::stream::StreamFlags,
    properties: pipewire::properties::PropertiesBox,//HashMap<&'static str, Vec<u8> >,
    formatting : HashMap<u32, pipewire::spa::pod::Property>,
    _state: PhantomData<State>
}
impl<State> StreamPipewireBuilder<State> {
    // Método privado que realiza la conversión de estado
    fn transition<NewState>(self) -> StreamPipewireBuilder<NewState> {
        StreamPipewireBuilder { 
            flags: self.flags, 
            properties: self.properties,
            formatting: self.formatting, 
            _state: PhantomData,
        }
    }
}

impl StreamPipewireBuilder<NeedFlags> {

    pub fn new() -> Self {

        use pipewire::stream::StreamFlags;

        Self { 
            properties: pipewire::properties::PropertiesBox::new(), 
            flags: StreamFlags::empty(), 
            formatting: HashMap::new(),
            _state: PhantomData
        }
    }

    pub fn add_flag(mut self, flag : pipewire::stream::StreamFlags) -> Self { self.flags |= flag; self }
    pub fn remove_flags(mut self, flags : pipewire::stream::StreamFlags) -> Self { self.flags &= !flags; self}
    pub fn set_flags(mut self, flags : pipewire::stream::StreamFlags) -> Self  { self.flags = flags; self }

    pub fn next(self) -> StreamPipewireBuilder<NeedProperties>{ self.transition() }
}

impl StreamPipewireBuilder<NeedProperties> {
    
    pub fn add_property(mut self, key : impl Into<Vec<u8>>, value : impl Into<Vec<u8>>) -> Self  { self.properties.insert(key, value); self}
    pub fn set_properties(mut self, properties : pipewire::properties::PropertiesBox) -> Self  { self.properties = properties; self}

    pub fn next(self) -> StreamPipewireBuilder<NeedFormatting>{ self.transition() }
}

impl StreamPipewireBuilder<NeedFormatting>{

    pub fn add_format(mut self, property : impl Into<pipewire::spa::pod::Property>) -> Self {
        let prop = property.into();
        self.formatting.insert(prop.key, prop);
        self
    }

    pub fn set_formats(
        mut self,
        media_type :  pipewire::spa::param::format::MediaType,
        media_subtype : pipewire::spa::param::format::MediaSubtype,
        properties : impl crate::pw_handlers::ToSpaProperties
    ) -> Self {

        use pipewire::spa::{
            utils::Id,
            param::format::FormatProperties
        };
        use crate::pw_handlers::KeyValue;

        self.formatting = properties.to_spa_properties();
        self.formatting.insert(
            FormatProperties::MediaType.as_raw(), 
            kv!(FormatProperties::MediaType => Id(media_type.0)).into()
        );
        self.formatting.insert(
            FormatProperties::MediaSubtype.as_raw(), 
            kv!(FormatProperties::MediaSubtype => Id(media_subtype.0)).into()
        );

        self
    }

    pub fn next(self) -> StreamPipewireBuilder<StreamReady>{ self.transition() }
}

impl StreamPipewireBuilder<StreamReady>{

    pub fn build(self, core : pipewire::core::CoreRc, name : &str, node_id : Option<u32> ) -> Result<pipewire::stream::StreamRc, Box<dyn std::error::Error>> {

        use pipewire::{
            stream::StreamRc,
            spa::{
                param::ParamType,
                pod::{ 
                    Object, Value, Pod, Property,
                    serialize::PodSerializer
                }, 
                utils::SpaTypes,
                sys
            }
        };
        use pipewire::spa::buffer::DataType;

        let format_params_obj = Object {
            id: ParamType::EnumFormat.as_raw(),
            type_ : SpaTypes::ObjectParamFormat.as_raw(),
            properties: self.formatting.into_values().collect()
        };
        let format_params_data = PodSerializer::serialize(
            std::io::Cursor::new(Vec::new()), 
            &Value::Object(format_params_obj)
        )?.0.into_inner();
        let format_params_pod = match Pod::from_bytes(&format_params_data) {
            None => return Err("Error en la construcción del objeto POD".into()),
            Some(value) => value,
        };


        let mut buffer_params : Vec<Property> = Vec::new();
        buffer_params.push(Property::new(
            sys::SPA_PARAM_BUFFERS_dataType, Value::Int(((1 << sys::SPA_DATA_MemFd) | (1 << sys::SPA_DATA_DmaBuf)) as i32) // Permitimos CPU o GPU
        ) 
        );
        let buffers_params_obj = Object{
            id: ParamType::Buffers.as_raw(),
            type_ : SpaTypes::ObjectParamBuffers.as_raw(),
            properties: buffer_params
        };
        let buffers_params_data = PodSerializer::serialize(
            std::io::Cursor::new(Vec::new()), 
            &Value::Object(buffers_params_obj)
        )?.0.into_inner();
        let buffers_params_pod =  match Pod::from_bytes(&buffers_params_data) {
            None => return Err("Error en la construcción del objeto POD".into()),
            Some(value) => value,
        };

        // let metas_obj = pipewire::spa::pod::object!(
        //     pipewire::spa::utils::SpaTypes::ObjectParamMeta,
        //     ParamType::Meta,
        //     pipewire::spa::pod::Property::new(
        //         pipewire::spa::sys::SPA_PARAM_META_type,
        //         pipewire::spa::pod::Value::Id(pipewire::spa::utils::Id(pipewire::spa::sys::SPA_META_Header))
        //     ),
        //     pipewire::spa::pod::Property::new(
        //         pipewire::spa::sys::SPA_PARAM_META_size,
        //         pipewire::spa::pod::Value::Int(size_of::<pipewire::spa::sys::spa_meta_header>() as i32)
        //     ),
        // );

        // let metas_values: Vec<u8> = pipewire::spa::pod::serialize::PodSerializer::serialize(
        //     std::io::Cursor::new(Vec::new()),
        //     &pipewire::spa::pod::Value::Object(metas_obj),
        // )
        // .unwrap()
        // .0
        // .into_inner();

        let stream = StreamRc::new(core, name, self.properties)?;
        stream.connect(
            pipewire::spa::utils::Direction::Input,
            node_id,
            self.flags,
            &mut [format_params_pod, buffers_params_pod /*, Pod::from_bytes(&metas_values).unwrap()*/]
        )?;

        Ok(stream)
    }
   
}


///////////////////////////////////////////////////////////////////////////////////////
// Builder para configurar el Oyente de un Stream (para manejar los datos del Stream)
// 






