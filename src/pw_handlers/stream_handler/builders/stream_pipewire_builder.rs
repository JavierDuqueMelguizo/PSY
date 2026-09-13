
///////////////////////////////////////////////////////////////////////////////////////
// Builder para configurar un objecto Stream (para grabar Pantalla, Audio, Camara, ...)
// Requiere una instancia de pipewire::core::CoreRc instanciado

use std::{collections::HashMap, marker::PhantomData};
use pipewire::{spa::{
    param::{
        ParamType as PwParamType, 
        format::FormatProperties as PwFormatProperties}, 
    pod::{
        Object as PwPodObject, 
        Pod as PwPod,
        Property as PwProperty,
        PropertyFlags as PwPropertyFlags,
        Value as PwValue,
        serialize::PodSerializer as PwPodSerializer},
    sys as PwSys, 
    utils::{Id as PwID, SpaTypes as PwSpaTypes}
}, stream::StreamRc};


use crate::{kv, pw_handlers::stream_handler::{AsChoice, BufferMaskTypes, KeyValue, builders::PodValue}};

pub struct NeedStreamFlags;
pub struct NeedStreamProperties;
pub struct NeedMediaType;
pub struct NeedMediaSubtype;
pub struct NeedFormatParams;
pub struct NeedBufferParams;

pub struct StreamReady;

pub struct StreamPipewireBuilder<State>{
    stream_flags: pipewire::stream::StreamFlags,
    stream_properties: pipewire::properties::PropertiesBox,//HashMap<&'static str, Vec<u8> >,
    format_params : HashMap<u32, Vec<KeyValue>>,
    built_format_params: HashMap<u32, PwProperty>,
    buffer_params : HashMap<u32, PwProperty>,
    _state: PhantomData<State>
}
impl<State> StreamPipewireBuilder<State> {
    // Método privado que realiza la conversión de estado
    fn transition<NewState>(self) -> StreamPipewireBuilder<NewState> {
        StreamPipewireBuilder { 
            stream_flags: self.stream_flags, 
            stream_properties: self.stream_properties,
            format_params: self.format_params, 
            built_format_params: self.built_format_params,
            buffer_params: self.buffer_params,
            _state: PhantomData,
        }
    }
}

impl StreamPipewireBuilder<NeedStreamFlags> {

    pub fn new() -> Self {

        use pipewire::stream::StreamFlags;

        Self { 
            stream_properties: pipewire::properties::PropertiesBox::new(), 
            stream_flags: StreamFlags::empty(), 
            format_params: HashMap::new(),
            built_format_params: HashMap::new(),
            buffer_params: HashMap::new(),
            _state: PhantomData
        }
    }

    pub fn add_flag(mut self, flag : pipewire::stream::StreamFlags) -> Self { self.stream_flags |= flag; self }
    pub fn remove_flags(mut self, flags : pipewire::stream::StreamFlags) -> Self { self.stream_flags &= !flags; self}
    pub fn set_flags(mut self, flags : pipewire::stream::StreamFlags) -> Self  { self.stream_flags = flags; self }

    pub fn next(self) -> StreamPipewireBuilder<NeedStreamProperties>{ self.transition() }
}

impl StreamPipewireBuilder<NeedStreamProperties> {
    
    pub fn add_property(mut self, key : impl Into<Vec<u8>>, value : impl Into<Vec<u8>>) -> Self  { self.stream_properties.insert(key, value); self}
    pub fn set_properties(mut self, properties : pipewire::properties::PropertiesBox) -> Self  { self.stream_properties = properties; self}

    pub fn next(self) -> StreamPipewireBuilder<NeedMediaType>{ self.transition() }
}

impl StreamPipewireBuilder<NeedMediaType>{
    
    pub fn set_media_type(mut self, media_type : pipewire::spa::param::format::MediaType) -> Self{

        use pipewire::spa::{
            utils::Id,
            param::format::FormatProperties
        };

        self.format_params.insert(
            FormatProperties::MediaType.as_raw(), 
            vec![kv!(FormatProperties::MediaType => Id(media_type.as_raw()))]
        );

        self
    }

    pub fn next(self) -> StreamPipewireBuilder<NeedMediaSubtype>{ self.transition() }
}

impl StreamPipewireBuilder<NeedMediaSubtype>{
    
    pub fn set_media_subtype(mut self, media_subtype : pipewire::spa::param::format::MediaSubtype) -> Self {

        self.format_params.insert(
            PwFormatProperties::MediaSubtype.as_raw(), 
            vec![kv!(PwFormatProperties::MediaSubtype => PwID(media_subtype.as_raw())).into()]
        );

        self
    }

    pub fn next(self) -> StreamPipewireBuilder<NeedBufferParams>{ self.transition() }
}

impl StreamPipewireBuilder<NeedBufferParams>{

    pub fn set_buffer_type(
        mut self,
        buffer_param : BufferMaskTypes,
        flags : pipewire::spa::pod::PropertyFlags
    ) -> Self {

        const KEY : u32 = PwSys::SPA_PARAM_BUFFERS_dataType;
        self.buffer_params.insert(
         KEY,
            PwProperty{
                key : KEY,
                value: PwValue::Int(buffer_param.bits() ),
                flags: flags
            }
        );
        
        self
    }

    pub fn next(self) -> StreamPipewireBuilder<NeedFormatParams>{ self.transition() }

}

impl StreamPipewireBuilder<NeedFormatParams>{

    pub fn add_complex_format_param(mut self, property: KeyValue) -> Self{
        // COMPROBACIONES
        if property.1.is_simple_value() {
            panic!("Intentando introducir valor simple ({:?}) en valor complejo", property.1);
        }

        // INSERCIÓN (sobreescribe)
        self.format_params.insert(property.0.as_raw(), vec![property]);

        self
    }
    pub fn add_simple_format_param(mut self, property: KeyValue ) -> Self {
        // COMPROBACIONES
        // De momento solo admito añadir valores PodValue catalogados como 'simples'
        if !property.1.is_simple_value() {
            panic!("Intentando introducir valor complejo ({:?}) en valor simple para clave: {:?}", property.1, property.0);
        }
        // Si no tiene nada, inicializó el contenedor. Si tiene algo, miro que sea del mismo tipo de PodValue
        match self.format_params.get( &property.0.as_raw() ){
            None => { self.format_params.insert(property.0.as_raw(), Vec::new()); },
            Some(value) => {
                let first = value.first().unwrap();
                if first.1 != property.1 {
                    panic!("EL tipo PodValue introducido ({:?}) no coincide con sus semejantes para la clave {:?}", first.1, first.0)
                }
            }
        };

        // INSERCIÓN
        self.format_params.get_mut(&property.0.as_raw()).unwrap().push(property);

        self
    }

    pub fn set_formats_params(
        mut self,
        properties : impl crate::pw_handlers::stream_handler::ToSpaProperties
    ) -> Self {

        // NOTA: Sobreescribe si existe
        self.format_params.extend(
            properties.to_spa_properties()
            .into_iter().map(|(id, value): (u32, KeyValue)|{
                (id,vec![value])
            })
        );

        self
    }

    pub fn next(mut self) -> StreamPipewireBuilder<StreamReady>{ 

        // REGLAS.
        // 1) De la documentación:
        /*
            https://eh5.pages.freedesktop.org/pipewire/page_dma_buf.html
            """
            Query the list of all supported modifiers from your graphics API of choice. Add a SPA_FORMAT_VIDEO_modifier property to the first stream parameter 
            with the flags SPA_POD_PROP_FLAG_MANDATORY | SPA_POD_PROP_FLAG_DONT_FIXATE. The value of the property should be set to a SPA_CHOICE_Enum containing 
            one long choice per supported modifier, plus DRM_FORMAT_MOD_INVALID if the graphics API supports modifier-less buffers.
            """
            Al flag modifier hay que añadirle la opción de "DRM_FORMAT_MOD_INVALID", que siempre estará presente
         */
        self = self.add_simple_format_param(kv!(PwFormatProperties::VideoModifier => u64::from(drm_fourcc::DrmModifier::Invalid) ));

        // PREPARAMOS LOS PARAMETROS PARA HACERLOS VALIDOS
        self.built_format_params = self.format_params.drain().map(
            |(id, mut list): (u32, Vec<KeyValue>)| {
                if list.is_empty(){
                    panic!("ERROR. Lista vacia para key : {:?}", PwFormatProperties::from_raw(id));
                }

    
                if list.len() == 1 {
                    let KeyValue(key,value, option_flag ) = list.remove(0);
                    let flags = option_flag.unwrap_or(PwPropertyFlags::empty());

                    // PwPodValue::Choice(PwChoiceValue::Long(Choice::<i64>(
                    //         PwChoiceFlags::empty(),
                    //         PwChoiceEnum::Enum {
                    //             default: modifier as i64,
                    //             alternatives: vec![modifier as i64], // repetido: así lo hace KWin al meter un solo valor 
                    //         },
                    // )));
                    if value.is_simple_value(){
                        (id, kv!(key => value, flags).into()) // Yo he decidido que lo meto en un Choice::None(value)  
                    }
                    else{
                        (id, kv!(key => value, flags).into()) // Doy por hecho que esta bien construido
                    }
                }
                else{
                    let key = list[0].0;
                    let flags = list[0].2.unwrap_or(PwPropertyFlags::empty());

                    let values : Vec<PodValue> = list.into_iter().map(|v|{ v.1 }).collect();

                    (id, kv!(key => (AsChoice, values), flags).into()) // Agrupo en Choice::Enum
                }

            }).collect();

        // AVANZAMOS
        self.transition() 
    }

}

impl StreamPipewireBuilder<StreamReady>{

    pub fn build(self, core : pipewire::core::CoreRc, name : &str, node_id : Option<u32> ) -> Result<pipewire::stream::StreamRc, Box<dyn std::error::Error>> {

        let format_params_obj: PwPodObject = PwPodObject {
            id: PwParamType::EnumFormat.as_raw(),
            type_ : PwSpaTypes::ObjectParamFormat.as_raw(),
            properties: self.built_format_params.into_values().collect()
        };
        let format_params_data: Vec<u8> = PwPodSerializer::serialize(
            std::io::Cursor::new(Vec::new()), 
            &PwValue::Object(format_params_obj)
        )?.0.into_inner();
        let format_params_pod: &PwPod = match PwPod::from_bytes(&format_params_data) {
            None => return Err("Error en la construcción del objeto POD".into()),
            Some(value) => value,
        };
            
        // Parametros para el buffer de memoria del cual se leerán los datos (en memoria CPU o la de GPU)
        let buffers_params_obj = PwPodObject{
            id: PwParamType::Buffers.as_raw(),
            type_ : PwSpaTypes::ObjectParamBuffers.as_raw(),
            properties: self.buffer_params.into_values().collect()
        };
        let buffers_params_data = PwPodSerializer::serialize(
            std::io::Cursor::new(Vec::new()), 
            &PwValue::Object(buffers_params_obj)
        )?.0.into_inner();
        let buffers_params_pod =  match PwPod::from_bytes(&buffers_params_data) {
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

        let stream = StreamRc::new(core, name, self.stream_properties)?;
        stream.connect(
            pipewire::spa::utils::Direction::Input,
            node_id,
            self.stream_flags,
            &mut [format_params_pod, buffers_params_pod /*, Pod::from_bytes(&metas_values).unwrap()*/]
        )?;

        Ok(stream)
    }
   
}

