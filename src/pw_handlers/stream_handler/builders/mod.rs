
use std::collections::HashMap;
use std::os::fd::AsRawFd;
use std::os::raw::c_void;
use pipewire::spa::pod::{
    CanonicalFixedSizedPod, ChoiceValue as PwPODChoiceValue, Object as PwPODObject, Pod, PropertyFlags as PwPropertyFlags, Value as PwPODValue, ValueArray as PwPODValueArray
};
use pipewire::spa::utils::{
    Choice as PwPODChoice, 
    ChoiceEnum as PwPODChoiceEnum, 
    ChoiceFlags as PwPODChoiceFlags, 
    Fd as PwPODFD, 
    Fraction as PwPODFraction, 
    Id as PwPODId, 
    Rectangle as PwPODRectangle
};

use super::{KeyValue};

pub mod pipewire_handler_builder;
pub use pipewire_handler_builder::*;

pub mod stream_pipewire_builder;
pub use stream_pipewire_builder::*;

pub mod stream_processor_pipewire_builder;
pub use stream_processor_pipewire_builder::*;


#[macro_export]
macro_rules! add_if {
    ($map:expr, $key:ident, $cond:expr, $val:expr) => {
        if $cond {
            $map.insert(
                FormatProperties::$key.as_raw(),
                kv!(FormatProperties::$key => $val),
            );
        }
    };

    ($map:expr, $key:ident, $cond:expr, $val:expr, $flags:expr) => {
        if $cond {
            $map.insert(
                FormatProperties::$key.as_raw(),
                kv!(FormatProperties::$key => $val, $flags ),
            );
        }
    };
	
}

pub trait ToSpaProperties {
    fn to_spa_properties(&self) -> HashMap<u32, KeyValue>;
}

impl ToSpaProperties for pipewire::spa::param::video::VideoInfoRaw {
    fn to_spa_properties(&self) -> HashMap<u32, KeyValue> {

        use pipewire::spa::{
            utils::Id,
            param::{
                video::{VideoFormat, VideoInterlaceMode}, 
                format::FormatProperties
            }
        };

        let mut properties = HashMap::new();

        // FORMAT
        let fmt = self.format();
        add_if!(properties, VideoFormat, fmt != VideoFormat::Unknown, Id(fmt.as_raw()));

        // FLAGS (omitidos, no se insertan, se recibe de PW de los eventos)

        // MODIFIER
        // De la documentación:
        /*
            https://eh5.pages.freedesktop.org/pipewire/page_dma_buf.html
            """
            Query the list of all supported modifiers from your graphics API of choice. Add a SPA_FORMAT_VIDEO_modifier property to the first stream parameter 
            with the flags SPA_POD_PROP_FLAG_MANDATORY | SPA_POD_PROP_FLAG_DONT_FIXATE. The value of the property should be set to a SPA_CHOICE_Enum containing 
            one long choice per supported modifier, plus DRM_FORMAT_MOD_INVALID if the graphics API supports modifier-less buffers.
            """
            Por eso le añado los flags del final
         */
        let modifier = self.modifier(); // DRM_FORMAT_MOD_INVALID = 0x00ffffffffffffff
        add_if!(properties, VideoModifier, modifier != u64::from(drm_fourcc::DrmModifier::Invalid),  self.modifier(), PwPropertyFlags::MANDATORY | PwPropertyFlags::DONT_FIXATE);

        // SIZE
        let size = self.size();
        add_if!(properties, VideoSize, size.width > 0 && size.height > 0, size);

        // FRAMERATE
        let fps = self.framerate();
        add_if!(properties, VideoFramerate, fps.num > 0 && fps.denom > 0, fps);

        // MAX FRAMERATE
        let max_fps = self.max_framerate();
        add_if!(properties, VideoMaxFramerate, max_fps.num > 0 && max_fps.denom > 0, max_fps);

        // VIEWS
        let views = self.views();
        add_if!(properties, VideoViews, views != 0, views);

        // INTERLACE MODE
        let interlace = self.interlace_mode();
        add_if!(properties, VideoInterlaceMode, interlace != VideoInterlaceMode::Progressive, Id(interlace.as_raw()));

        // PIXEL ASPECT RATIO
        let par = self.pixel_aspect_ratio();
        add_if!(properties, VideoPixelAspectRatio, par.num > 0 && par.denom > 0, par);

        // MULTIVIEW MODE
        let mv_mode = self.multiview_mode();
        add_if!(properties, VideoMultiviewMode, mv_mode != 0, Id(mv_mode as u32));

        // MULTIVIEW FLAGS
        let mv_flags = self.multiview_flags();
        add_if!(properties, VideoMultiviewFlags, mv_flags != 0, Id(mv_flags));

        // CHROMA SITE
        let chroma = self.chroma_site();
        add_if!(properties, VideoChromaSite, chroma != 0, Id(chroma));

        // COLOR RANGE
        let range = self.color_range();
        add_if!(properties, VideoColorRange, range != 0, Id(range));

        // COLOR MATRIX
        let matrix = self.color_matrix();
        add_if!(properties, VideoColorMatrix, matrix != 0, Id(matrix));

        // TRANSFER FUNCTION
        let transfer = self.transfer_function();
        add_if!(properties, VideoTransferFunction, transfer != 0, Id(transfer));

        //COLOR PRIMARIES
        let primaries = self.color_primaries();
        add_if!(properties, VideoColorPrimaries, primaries != 0, Id(primaries));

        properties
    }
}

impl ToSpaProperties for pipewire::spa::param::audio::AudioInfoRaw {
    fn to_spa_properties(&self) -> HashMap<u32, KeyValue> {

        use pipewire::spa::{
            sys::SPA_AUDIO_MAX_CHANNELS,
            utils::Id,
            param::{
                audio::{AudioFormat, AudioInfoRawFlags}, 
                format::FormatProperties
            }
        };
        
        let mut properties = HashMap::new();

        // FORMAT
        let fmt = self.format();
        add_if!(properties, AudioFormat, fmt != AudioFormat::Unknown, Id(fmt.as_raw()));

        // FLAGS
        let flags = self.flags();
        add_if!(properties, AudioFlags, flags != AudioInfoRawFlags::empty(), flags.bits());

        // RATE
        let rate = self.rate();
        add_if!(properties, AudioRate, rate != 0, rate);

        // CHANNELS
        let channels = self.channels();
        add_if!(properties, AudioChannels, channels != 0, channels);

        // POSITION
        let position = self.position();
        add_if!(
            properties,
            AudioPosition,
            position != [0; SPA_AUDIO_MAX_CHANNELS as usize],
            Id(position[0])
        );

        properties
    }
}


bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct BufferMaskTypes: i32 {
        const INVALID  = 1 << pipewire::spa::sys::SPA_DATA_Invalid;
        const MEM_PTR  = 1 << pipewire::spa::sys::SPA_DATA_MemPtr;
        const MEM_FD   = 1 << pipewire::spa::sys::SPA_DATA_MemFd;
        const DMA_BUF  = 1 << pipewire::spa::sys::SPA_DATA_DmaBuf;
        const MEM_ID   = 1 << pipewire::spa::sys::SPA_DATA_MemId;
        const SYNC_OBJ = 1 << pipewire::spa::sys::SPA_DATA_SyncObj;
    }
}

//////////////////////////////////////////////////
/// POD

/*
 La idea de este Enum POD es poder facilitar la instanción de clases POD mediante tuplas , asi como controlar la posterior conversión
 a los POD que PipeWire maneja.
*/
#[derive(Debug, Clone)]
pub(crate) enum PodValue {
    None,
    Bool(bool),
    Id(PwPODId),
    Int(i32),
    Long(i64),
    Float(f32),
    Double(f64),
    String(String),
    Bytes(Vec<u8>),
    Rectangle(PwPODRectangle),
    Fraction(PwPODFraction),
    Fd(PwPODFD),
    ValueArray(PwPODValueArray),
    Struct(Vec<PodValue>),
    Object(PwPODObject),
    Choice(PwPODChoiceValue),
    Pointer(u32, *const c_void),
}
impl PodValue{
    pub fn is_simple_value(&self) -> bool{
        !matches!(
            self,
            PodValue::ValueArray(_)
            | PodValue::Struct(_)
            | PodValue::Object(_)
            | PodValue::Choice(_)
            | PodValue::Pointer(_, _)
        )
    }

    pub fn is_choice(&self) -> bool {
        println!("{:#?}", self);
        matches!(
            self,
            PodValue::Choice(_)
        )
    }
}
/// Impl to take value
// impl PodValue{
//     pub fn to_bool(&self) -> Option<&bool>{
//         match self {
//             Self::Bool(v) => Some(v),
//             _ => None
//         }
//     }
//     pub fn to_id(&self) -> Option<PwPODId>{
//         match self {
//             Self::Id(v) => Some(v.clone()),
//             _ => None
//         }
//     }
//     pub fn to_int(&self) -> Option<i32>{
//         match self {
//             Self::Int(v) => Some(v.clone()),
//             _ => None
//         }
//     }
//     pub fn to_long(&self) -> Option<i64>{
//         match self {
//             Self::Long(v) => Some(v.clone()),
//             _ => None
//         }
//     }
//     pub fn to_float(&self) -> Option<f32>{
//         match self {
//             Self::Float(v) => Some(v.clone()),
//             _ => None
//         }
//     }
//     pub fn to_double(&self) -> Option<f64>{
//         match self {
//             Self::Double(v) => Some(v.clone()),
//             _ => None
//         }
//     }
//     pub fn to_string(&self) -> Option<String>{
//         match self {
//             Self::String(v) => Some(v.clone()),
//             _ => None
//         }
//     }
//     pub fn to_bytes(&self) -> Option<Vec<u8>>{
//         match self {
//             Self::Bytes(v) => Some(v.clone()),
//             _ => None
//         }
//     }
//     pub fn to_rectangle(&self) -> Option<PwPODRectangle>{
//         match self {
//             Self::Rectangle(v) => Some(v.clone()),
//             _ => None
//         }
//     }
//     pub fn to_fraction(&self) -> Option<PwPODFraction>{
//         match self {
//             Self::Fraction(v) => Some(v.clone()),
//             _ => None
//         }
//     }
//     pub fn to_fd(&self) -> Option<PwPODFD>{
//         match self {
//             Self::Fd(v) => Some(v.clone()),
//             _ => None
//         }
//     }
//     pub fn to_array(&self) -> Option<PwPODValueArray>{
//         match self {
//             Self::ValueArray(v) => Some(v.clone()),
//             _ => None
//         }
//     }
//     pub fn to_struct(&self) -> Option<Vec<PodValue>>{
//         match self {
//             Self::Struct(v) => Some(v.clone()),
//             _ => None
//         }
//     }
//     pub fn to_object(&self) -> Option<PwPODObject>{
//         match self {
//             Self::Object(v) => Some(v.clone()),
//             _ => None
//         }
//     }
//     pub fn to_pointer(&self) -> Option<(u32, *const c_void)>{
//         match self {
//             Self::Pointer(v,p) => Some((v.clone(),*p).clone()),
//             _ => None
//         }
//     }
//      pub fn to_choice(self) -> PodValue{
//         match self{
//             PodValue::None => panic!("No es posible transformer de PodValue::None a PodValue::Choice"),
//             PodValue::Bool(v) => (AsChoice, v).into(),
//             PodValue::Id(v) => (AsChoice, v).into(),
//             PodValue::Int(v) => (AsChoice, v).into(),
//             PodValue::Long(v) => (AsChoice, v).into(),
//             PodValue::Float(v) => (AsChoice, v).into(),
//             PodValue::Double(v) => (AsChoice, v).into(),
//             PodValue::Rectangle(v) => (AsChoice, AsRectangle, v.width,v.height).into(),
//             PodValue::Fraction(v) => (AsChoice, AsFraction, v.num, v.denom).into(),
//             PodValue::Fd(fd) => (AsChoice, AsFD, fd.0).into(),
//             value => panic!("No es posible transformar {:?} a PodValue::Choice", value)
//         }
//     }
// }

impl PartialEq for PodValue{
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::None, Self::None) => true,
            (Self::Bool(_), Self::Bool(_)) => true,
            (Self::Id(_), Self::Id(_)) => true,
            (Self::Int(_), Self::Int(_)) => true,
            (Self::Long(_), Self::Long(_)) => true,
            (Self::Float(_), Self::Float(_)) => true,
            (Self::Double(_), Self::Double(_)) => true,
            (Self::String(_), Self::String(_)) => true,
            (Self::Bytes(_), Self::Bytes(_)) => true,
            (Self::Rectangle(_), Self::Rectangle(_)) => true,
            (Self::Fraction(_), Self::Fraction(_)) => true,
            (Self::Fd(_), Self::Fd(_)) => true,
            (Self::ValueArray(_), Self::ValueArray(_)) => true,
            (Self::Struct(_), Self::Struct(_)) => true,
            (Self::Object(_), Self::Object(_)) => true,
            (Self::Choice(_), Self::Choice(_)) => true,
            (Self::Pointer(_, _), Self::Pointer(_, _)) => true,
            _ => false, //core::mem::discriminant(self) == core::mem::discriminant(other),
        }
    }
}

// -------------------------------------------------------------------
// Conversiones FROM (tipos primitivos/comunes -> PodValue)
// -------------------------------------------------------------------

/// BOOLEAN
impl From<bool> for PodValue {
    fn from(v: bool) -> Self {
        PodValue::Bool(v)
    }
}

/// ID
impl From<PwPODId> for PodValue {
    fn from(v: PwPODId) -> Self {
        PodValue::Id(v)
    }
}

/// INTEGER
impl From<u16> for PodValue{
    fn from(v: u16) -> Self {
        PodValue::Int(v.into())
    }
}


impl From<i16> for PodValue{
    fn from(v: i16) -> Self {
        PodValue::Int(v.into())
    }
}

impl From<u32> for PodValue {
    fn from(v: u32) -> Self {
        PodValue::Id(PwPODId(v))
    }
}

impl From<i32> for PodValue {
    fn from(v: i32) -> Self {
        PodValue::Int(v)
    }
}

/// LONG
impl From<i64> for PodValue {
    fn from(v: i64) -> Self {
        PodValue::Long(v)
    }
}

impl From<u64> for PodValue {
    fn from(v: u64) -> Self {
        PodValue::Long(v as i64)
    }
}

/// FLOAT
impl From<f32> for PodValue {
    fn from(v: f32) -> Self {
        PodValue::Float(v)
    }
}

/// DOUBLE
impl From<f64> for PodValue {
    fn from(v: f64) -> Self {
        PodValue::Double(v)
    }
}

/// STRING
impl From<String> for PodValue {
    fn from(v: String) -> Self {
        PodValue::String(v)
    }
}

impl From<&str> for PodValue {
    fn from(v: &str) -> Self {
        PodValue::String(v.to_string())
    }
}

/// BYTES
impl From<Vec<u8>> for PodValue {
    fn from(v: Vec<u8>) -> Self {
        PodValue::Bytes(v)
    }
}

/// RECTANGLE
pub struct AsRectangle;
impl From<(AsRectangle, u32, u32)> for PodValue {
    fn from((_, width, height): (AsRectangle, u32, u32)) -> Self {
        PodValue::Rectangle(PwPODRectangle { width, height })
    }
}
impl From<PwPODRectangle> for PodValue {
    fn from(v: PwPODRectangle) -> Self {
        PodValue::Rectangle(v)
    }
}

/// FRACTION
pub struct AsFraction;
impl From<(AsFraction, u32, u32)> for PodValue {
    fn from((_, num, denom): (AsFraction, u32, u32)) -> Self {
        PodValue::Fraction(PwPODFraction { num, denom })
    }
}
impl From<PwPODFraction> for PodValue {
    fn from(v: PwPODFraction) -> Self {
        PodValue::Fraction(v)
    }
}

/// FILE DESCRIPTOR
pub struct AsFD;
impl From<(AsFD, i64)> for PodValue {
    fn from((_, fd): (AsFD, i64)) -> Self {
        PodValue::Fd(pipewire::spa::utils::Fd(fd))
    }
}
impl<'a> From<zbus::zvariant::Fd<'a>> for PodValue {
    fn from(v: zbus::zvariant::Fd<'a>) -> Self {
        PodValue::Fd(pipewire::spa::utils::Fd(v.as_raw_fd().into()))
    }
}

/////////////////////////////////////////////////////////////////////
////// VALUES ARRAY
/// NONE
impl From<Vec<()>> for PodValue {
    fn from(v: Vec<()>) -> Self {
        PodValue::ValueArray(PwPODValueArray::None(v))
    }
}
/// BOOL
impl From<Vec<bool>> for PodValue {
    fn from(v: Vec<bool>) -> Self {
        PodValue::ValueArray(PwPODValueArray::Bool(v))
    }
}
/// ID
impl From<Vec<PwPODId>> for PodValue {
    fn from(v: Vec<PwPODId>) -> Self {
        PodValue::ValueArray(PwPODValueArray::Id(v))
    }
}
/// INT
impl From<Vec<i32>> for PodValue {
    fn from(v: Vec<i32>) -> Self {
        PodValue::ValueArray(PwPODValueArray::Int(v))
    }
}
/// LONG
impl From<Vec<i64>> for PodValue {
    fn from(v: Vec<i64>) -> Self {
        PodValue::ValueArray(PwPODValueArray::Long(v))
    }
}
/// FLOAT
impl From<Vec<f32>> for PodValue {
    fn from(v: Vec<f32>) -> Self {
        PodValue::ValueArray(PwPODValueArray::Float(v))
    }
}
/// DOUBLE
impl From<Vec<f64>> for PodValue {
    fn from(v: Vec<f64>) -> Self {
        PodValue::ValueArray(PwPODValueArray::Double(v))
    }
}
/// RECTANGLE
impl From<Vec<(AsRectangle, u32, u32)>> for PodValue {
    fn from(v: Vec<(AsRectangle, u32, u32)>) -> Self {
        PodValue::ValueArray(PwPODValueArray::Rectangle(
            v.into_iter()
                .map(|(_, width, height)| PwPODRectangle { width, height })
                .collect(),
        ))
    }
}
/// FRACTION
impl From<Vec<(AsFraction, u32, u32)>> for PodValue {
    fn from(v: Vec<(AsFraction, u32, u32)>) -> Self {
        PodValue::ValueArray(PwPODValueArray::Fraction(
            v.into_iter()
                .map(|(_, num, denom)| PwPODFraction { num, denom })
                .collect(),
        ))
    }
}
/// FILE DESCRIPTOR
impl From<Vec<(AsFD, i64)>> for PodValue {
    fn from(v: Vec<(AsFD, i64)>) -> Self {
        PodValue::ValueArray(PwPODValueArray::Fd(
            v.into_iter()
                .map(|(_, fd)| pipewire::spa::utils::Fd(fd))
                .collect(),
        ))
    }
}

/////////////////////////////////////////////////////////////////////
// VALUES CHOICE
pub struct AsChoice;
pub struct AsRange<T> {
    pub default: T,
    pub min: T,
    pub max: T,
}
impl<T> From<AsRange<T>> for PwPODChoice<T>
where
    T: CanonicalFixedSizedPod,
{
    fn from(v: AsRange<T>) -> Self {
        PwPODChoice(
            PwPODChoiceFlags::empty(),
            PwPODChoiceEnum::Range {
                default: v.default,
                min: v.min,
                max: v.max,
            },
        )
    }
}
pub struct AsSteps<T> {
    pub default: T,
    pub min: T,
    pub max: T,
    pub step: T,
}
impl<T> From<AsSteps<T>> for PwPODChoice<T>
where
    T: CanonicalFixedSizedPod,
{
    fn from(v: AsSteps<T>) -> Self {
        PwPODChoice(
            PwPODChoiceFlags::empty(),
            PwPODChoiceEnum::Step {
                default: v.default,
                min: v.min,
                max: v.max,
                step: v.step,
            },
        )
    }
}

/// BOOL
// impl From<(AsChoice, bool)> for PodValue { // Discreto
//     fn from((_, v): (AsChoice, bool)) -> Self {
//         PodValue::Choice(PwPODChoiceValue::Bool(PwPODChoice(
//             PwPODChoiceFlags::empty(),
//             PwPODChoiceEnum::None(v),
//         )))
//     }
// }

/// ID
// impl From<(AsChoice, PwPODId)> for PodValue { // Discreto
//     fn from((_, v): (AsChoice, PwPODId)) -> Self {
//         PodValue::Choice(PwPODChoiceValue::Id(PwPODChoice(
//             PwPODChoiceFlags::empty(),
//             PwPODChoiceEnum::None(v),
//         )))
//     }
// }
impl From<(AsChoice, Vec<PwPODId>)> for PodValue { // Enumerable
    fn from((_, v): (AsChoice, Vec<PwPODId>)) -> Self {
        if v.is_empty() {
            panic!("ERROR en conversión Vec<Id> a PodValue::Choice(PwPODChoiceValue::Int). Vec<Id> esta vacio");
        }
        PodValue::Choice(PwPODChoiceValue::Id(PwPODChoice(
            PwPODChoiceFlags::empty(),
            PwPODChoiceEnum::Enum {
                default: v[0],
                alternatives: v.to_vec(),
            },
        )))
    }
}

/// INT
// impl From<(AsChoice, i32)> for PodValue { // Discreto
//     fn from((_, v): (AsChoice, i32)) -> Self {
//         PodValue::Choice(PwPODChoiceValue::Int(PwPODChoice(
//             PwPODChoiceFlags::empty(),
//             PwPODChoiceEnum::None(v),
//         )))
//     }
// }
impl From<(AsChoice, Vec<i32>)> for PodValue { // Enumerable
    fn from((_, v): (AsChoice, Vec<i32>)) -> Self {
        if v.is_empty() {
            panic!("ERROR en conversión Vec<i32> a PodValue::Choice(PwPODChoiceValue::Int). Vec<i32> esta vacio");
        }
        PodValue::Choice(PwPODChoiceValue::Int(PwPODChoice(
            PwPODChoiceFlags::empty(),
            PwPODChoiceEnum::Enum {
                default: v[0],
                alternatives: v.to_vec(),
            },
        )))
    }
}
impl From<(AsChoice, AsRange<i32>)> for PodValue { // Rango (continuo)
    fn from((_, range): (AsChoice, AsRange<i32>)) -> Self {
        PodValue::Choice(PwPODChoiceValue::Int(range.into()))
    }
}
impl From<(AsChoice, AsSteps<i32>)> for PodValue { // Rango (discreto)
    fn from((_, steps): (AsChoice, AsSteps<i32>)) -> Self {
        PodValue::Choice(PwPODChoiceValue::Int(steps.into()))
    }
}

/// LONG
// impl From<(AsChoice, i64)> for PodValue { // Discreto
//     fn from((_, v): (AsChoice, i64)) -> Self {
//         PodValue::Choice(PwPODChoiceValue::Long(PwPODChoice(
//             PwPODChoiceFlags::empty(),
//             PwPODChoiceEnum::None(v),
//         )))
//     }
// }
// impl From<(AsChoice, u64)> for PodValue { // Discreto
//     fn from((_, v): (AsChoice, u64)) -> Self {
//         PodValue::Choice(PwPODChoiceValue::Long(PwPODChoice(
//             PwPODChoiceFlags::empty(),
//             PwPODChoiceEnum::None(v.cast_signed()),
//         )))
//     }
// }
impl From<(AsChoice, Vec<i64>)> for PodValue { // Enumerable
    fn from((_, v): (AsChoice, Vec<i64>)) -> Self {
        if v.is_empty() {
            panic!("ERROR en conversión Vec<i64> a PodValue::Choice(PwPODChoiceValue::Long). Vec<i64> esta vacio");
        }
        PodValue::Choice(PwPODChoiceValue::Long(PwPODChoice(
            PwPODChoiceFlags::empty(),
            PwPODChoiceEnum::Enum {
                default: v[0],
                alternatives: v.to_vec(),
            },
        )))
    }
}
impl From<(AsChoice, Vec<u64>)> for PodValue { // Enumerable
    fn from((_, v): (AsChoice, Vec<u64>)) -> Self {
        if v.is_empty() {
            panic!("ERROR en conversión Vec<i64> a PodValue::Choice(PwPODChoiceValue::Long). Vec<i64> esta vacio");
        }
        PodValue::Choice(PwPODChoiceValue::Long(PwPODChoice(
            PwPODChoiceFlags::empty(),
            PwPODChoiceEnum::Enum {
                default: v[0].cast_signed(),
                alternatives: v.iter().map(|x| x.cast_signed()).collect(),
            },
        )))
    }
}
impl From<(AsChoice, AsRange<i64>)> for PodValue { // Rango (continuo)
    fn from((_, range): (AsChoice, AsRange<i64>)) -> Self {
        PodValue::Choice(PwPODChoiceValue::Long(range.into()))
    }
}
impl From<(AsChoice, AsSteps<i64>)> for PodValue { // Rango (discreto)
    fn from((_, steps): (AsChoice, AsSteps<i64>)) -> Self {
        PodValue::Choice(PwPODChoiceValue::Long(steps.into()))
    }
}

/// FLOAT
// impl From<(AsChoice, f32)> for PodValue { // Discreto
//     fn from((_, v): (AsChoice, f32)) -> Self {
//         PodValue::Choice(PwPODChoiceValue::Float(PwPODChoice(
//             PwPODChoiceFlags::empty(),
//             PwPODChoiceEnum::None(v),
//         )))
//     }
// }
impl From<(AsChoice, Vec<f32>)> for PodValue { // Enumerable
    fn from((_, v): (AsChoice, Vec<f32>)) -> Self {
        if v.is_empty() {
            panic!("ERROR en conversión Vec<f32> a PodValue::Choice(PwPODChoiceValue::Float). Vec<f32> esta vacio");
        }
        PodValue::Choice(PwPODChoiceValue::Float(PwPODChoice(
            PwPODChoiceFlags::empty(),
            PwPODChoiceEnum::Enum {
                default: v[0],
                alternatives: v.to_vec(),
            },
        )))
    }
}
impl From<(AsChoice, AsRange<f32>)> for PodValue { // Rango (continuo)
    fn from((_, range): (AsChoice, AsRange<f32>)) -> Self {
        PodValue::Choice(PwPODChoiceValue::Float(range.into()))
    }
}
impl From<(AsChoice, AsSteps<f32>)> for PodValue { // Rango (discreto)
    fn from((_, steps): (AsChoice, AsSteps<f32>)) -> Self {
        PodValue::Choice(PwPODChoiceValue::Float(steps.into()))
    }
}

/// DOUBLE
// impl From<(AsChoice, f64)> for PodValue { // Discreto
//     fn from((_, v): (AsChoice, f64)) -> Self {
//         PodValue::Choice(PwPODChoiceValue::Double(PwPODChoice(
//             PwPODChoiceFlags::empty(),
//             PwPODChoiceEnum::None(v),
//         )))
//     }
// }
impl From<(AsChoice, Vec<f64>)>  for PodValue { // Enumerable
    fn from((_, v): (AsChoice, Vec<f64>)) -> Self {
         if v.is_empty() {
            panic!("ERROR en conversión Vec<f64> a PodValue::Choice(PwPODChoiceValue::Double). Vec<f64> esta vacio");
        }
        PodValue::Choice(PwPODChoiceValue::Double(
            PwPODChoice(
                PwPODChoiceFlags::empty(), 
                PwPODChoiceEnum::Enum { default: v[0], alternatives: v.to_vec() }
            )
        ))
    }
}
impl From<(AsChoice, AsRange<f64>)>  for PodValue { // Rango (continuo)
    fn from((_,range): (AsChoice, AsRange<f64>)) -> Self {
        PodValue::Choice(PwPODChoiceValue::Double(range.into()))
    }
}
impl From<(AsChoice, AsSteps<f64>)>  for PodValue { // Rango (discreto)
    fn from((_, steps): (AsChoice, AsSteps<f64>)) -> Self {
        PodValue::Choice(PwPODChoiceValue::Double(steps.into()))
    }
}

/// RECTANGLE
// impl From<(AsChoice, AsRectangle,u32,u32)>  for PodValue {  // Discreto
//     fn from((_,_, width, height): (AsChoice, AsRectangle,u32,u32)) -> Self {
//         PodValue::Choice(PwPODChoiceValue::Rectangle(PwPODChoice(PwPODChoiceFlags::empty(), PwPODChoiceEnum::None(PwPODRectangle{
//             width: width,
//             height: height,
//         }))))
//     }
// }
impl From<(AsChoice, Vec<PwPODRectangle>)>  for PodValue { // Enumerable
    fn from((_, v): (AsChoice, Vec<PwPODRectangle>)) -> Self {
         if v.is_empty() {
            panic!("ERROR en conversión Vec<PwPODRectangle> a PodValue::Choice(PwPODChoiceValue::Double). Vec<PwPODRectangle> esta vacio");
        }
        PodValue::Choice(PwPODChoiceValue::Rectangle(
            PwPODChoice(
                PwPODChoiceFlags::empty(), 
                PwPODChoiceEnum::Enum { default: v[0], alternatives: v.to_vec() }
            )
        ))
    }
}



/// FRACTION
// impl From<(AsChoice, AsFraction,u32,u32)>  for PodValue {  // Discreto
//     fn from((_,_,num,denom): (AsChoice, AsFraction,u32,u32)) -> Self {
//         PodValue::Choice(PwPODChoiceValue::Fraction(PwPODChoice(PwPODChoiceFlags::empty(), PwPODChoiceEnum::None(PwPODFraction{
//             num: num,
//             denom: denom
//         }))))
//     }
// }
impl From<(AsChoice, Vec<PwPODFraction>)>  for PodValue { // Enumerable
    fn from((_, v): (AsChoice, Vec<PwPODFraction>)) -> Self {
         if v.is_empty() {
            panic!("ERROR en conversión Vec<PwPODFraction> a PodValue::Choice(PwPODChoiceValue::Double). Vec<PwPODFraction> esta vacio");
        }
        PodValue::Choice(PwPODChoiceValue::Fraction(
            PwPODChoice(
                PwPODChoiceFlags::empty(), 
                PwPODChoiceEnum::Enum { default: v[0], alternatives: v.to_vec() }
            )
        ))
    }
}

/// FILE DESCRIPTOR
// impl From<(AsChoice, AsFD,i64)>  for PodValue {  // Discreto
//     fn from((_,_, v): (AsChoice, AsFD,i64)) -> Self {
//         PodValue::Choice(PwPODChoiceValue::Fd(PwPODChoice(PwPODChoiceFlags::empty(), PwPODChoiceEnum::None(pipewire::spa::utils::Fd(v)))))
//     }
// }
impl From<(AsChoice, Vec<PwPODFD>)>  for PodValue { // Enumerable
    fn from((_, v): (AsChoice, Vec<PwPODFD>)) -> Self {
         if v.is_empty() {
            panic!("ERROR en conversión Vec<PwPODFD> a PodValue::Choice(PwPODChoiceValue::Double). Vec<PwPODFD> esta vacio");
        }
        PodValue::Choice(PwPODChoiceValue::Fd(
            PwPODChoice(
                PwPODChoiceFlags::empty(), 
                PwPODChoiceEnum::Enum { default: v[0], alternatives: v.to_vec() }
            )
        ))
    }
}


// Generico ( Adhoc para StreamPipewireBuilder<NeedFormatParams>::next )
impl From<(AsChoice, Vec<PodValue>)> for PodValue { // Transforma en ChoiceEnum
    fn from((_,list): (AsChoice, Vec<PodValue>)) -> Self {
        // CONTROLES
        if list.iter().any(|x|{x.is_choice() | !x.is_simple_value()}) {
            panic!("La lista contiene valores complejos o variantes Choice anidadas no permitidas");
        }

        macro_rules! collect_choice {
            ($list:expr, $variant:path, $type:ty) => {{
                let values: Vec<$type> = $list
                    .into_iter()
                    .map(|value| match value {
                        $variant(value) => value,
                        other => panic!("Lista heterogénea: esperaba {}, pero encontré {:?}",stringify!($variant),other),
                    })
                    .collect();

                (AsChoice, values).into()
            }};
        }
        // TRADUCCION
        let value = match list.first() {
            None => panic!("La lista no puede estar vacía"),
            Some(PodValue::Id(_)) => collect_choice!(list, PodValue::Id, PwPODId),
            Some(PodValue::Int(_)) => collect_choice!(list, PodValue::Int, i32),
            Some(PodValue::Long(_)) => collect_choice!(list, PodValue::Long, i64),
            Some(PodValue::Float(_)) => collect_choice!(list, PodValue::Float, f32),
            Some(PodValue::Double(_)) => collect_choice!(list, PodValue::Double, f64),
            Some(PodValue::Rectangle(_)) => collect_choice!(list, PodValue::Rectangle, PwPODRectangle),
            Some(PodValue::Fraction(_)) => collect_choice!(list, PodValue::Fraction, PwPODFraction),
            Some(PodValue::Fd(_)) => collect_choice!(list, PodValue::Fd, PwPODFD),
            value => panic!("Tipo entregado: {:?}. Tipo no soportado para PodValue::Choice", value),
        };

        value
    }
}



// -------------------------------------------------------------------
// Conversión hacia el Value original de PipeWire
// -------------------------------------------------------------------

impl Into<PwPODValue> for PodValue{
    fn into(self) -> PwPODValue {
        match self {
            PodValue::None => PwPODValue::None,
            PodValue::Bool(v) => PwPODValue::Bool(v),
            PodValue::Id(v) => PwPODValue::Id(v),
            PodValue::Int(v) => PwPODValue::Int(v),
            PodValue::Long(v) => PwPODValue::Long(v),
            PodValue::Float(v) => PwPODValue::Float(v),
            PodValue::Double(v) => PwPODValue::Double(v),
            PodValue::String(v) => PwPODValue::String(v),
            PodValue::Bytes(v) => PwPODValue::Bytes(v),
            PodValue::Rectangle(v) => PwPODValue::Rectangle(v),
            PodValue::Fraction(v) => PwPODValue::Fraction(v),
            PodValue::Fd(v) => PwPODValue::Fd(v),
            PodValue::ValueArray(v) => PwPODValue::ValueArray(v),
            PodValue::Struct(vec) => PwPODValue::Struct(
                vec.into_iter().map(|item| item.into()).collect(),
            ),
            PodValue::Object(v) => PwPODValue::Object(v),
            PodValue::Choice(v) => PwPODValue::Choice(v),
            PodValue::Pointer(t, p) => PwPODValue::Pointer(t, p),
        }
    }
}