
use std::{collections::HashMap, marker::PhantomData};

use crate::pw_handlers::StreamPipewireHandler;



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

        use pipewire::thread_loop::ThreadLoopRc;

        let dict = match properties.as_ref(){
            None => None,
            Some(value) => Some(value.dict())
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