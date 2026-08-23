use std::os::fd::AsRawFd;
use std::os::raw::c_void;
use pipewire::spa::pod::{
    ChoiceValue, Object, Property as PwProperty, Value as PwValue, ValueArray
};
use pipewire::spa::utils::{Id, Rectangle, Fraction, Fd};

pub enum PodValue {
    None,
    Bool(bool),
    Id(Id),
    Int(i32),
    Long(i64),
    Float(f32),
    Double(f64),
    String(String),
    Bytes(Vec<u8>),
    Rectangle(Rectangle),
    Fraction(Fraction),
    Fd(Fd),
    ValueArray(ValueArray),
    Struct(Vec<PodValue>),
    Object(Object),
    Choice(ChoiceValue),
    Pointer(u32, *const c_void),
}

// -------------------------------------------------------------------
// Conversiones FROM (tipos primitivos/comunes -> PodValue)
// -------------------------------------------------------------------

// BOOLEAN
impl From<bool> for PodValue {
    fn from(v: bool) -> Self {
        PodValue::Bool(v)
    }
}

// ID
impl From<Id> for PodValue {
    fn from(v: Id) -> Self {
        PodValue::Id(v)
    }
}

// INTEGER
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
        PodValue::Id(Id(v))
    }
}

impl From<i32> for PodValue {
    fn from(v: i32) -> Self {
        PodValue::Int(v)
    }
}

// LONG
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

// FLOAT
impl From<f32> for PodValue {
    fn from(v: f32) -> Self {
        PodValue::Float(v)
    }
}

// DOUBLE
impl From<f64> for PodValue {
    fn from(v: f64) -> Self {
        PodValue::Double(v)
    }
}

// STRING
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

// BYTES
impl From<Vec<u8>> for PodValue {
    fn from(v: Vec<u8>) -> Self {
        PodValue::Bytes(v)
    }
}

// RECTANGLE
impl From<Rectangle> for PodValue {
    fn from(v: Rectangle) -> Self {
        PodValue::Rectangle(v)
    }
}
pub struct AsRectangle;
impl From<(AsRectangle, u32,u32)> for PodValue {
    fn from(v: (AsRectangle, u32,u32)) -> Self {
        PodValue::Rectangle(Rectangle{width:v.1, height:v.2})
    }
}

// FRACTION
impl From<Fraction> for PodValue {
    fn from(v: Fraction) -> Self {
        PodValue::Fraction(v)
    }
}
pub struct AsFraction;
impl From<(AsFraction, u32,u32)> for PodValue {
    fn from(v: (AsFraction, u32,u32)) -> Self {
        PodValue::Fraction(Fraction{num:v.1, denom:v.2})
    }
}

// FILE DESCRIPTOR
impl From<Fd> for PodValue {
    fn from(v: Fd) -> Self {
        PodValue::Fd(v)
    }
}

impl<'a> From<zbus::zvariant::Fd<'a>> for PodValue {
    fn from(v: zbus::zvariant::Fd<'a>) -> Self {
        PodValue::Fd(pipewire::spa::utils::Fd(v.as_raw_fd().into()))
    }
}
// -------------------------------------------------------------------
// Conversión hacia el Value original de PipeWire
// -------------------------------------------------------------------

impl Into<pipewire::spa::pod::Value> for PodValue{
    fn into(self) -> PwValue {
        match self {
            PodValue::None => PwValue::None,
            PodValue::Bool(v) => PwValue::Bool(v),
            PodValue::Id(v) => PwValue::Id(v),
            PodValue::Int(v) => PwValue::Int(v),
            PodValue::Long(v) => PwValue::Long(v),
            PodValue::Float(v) => PwValue::Float(v),
            PodValue::Double(v) => PwValue::Double(v),
            PodValue::String(v) => PwValue::String(v),
            PodValue::Bytes(v) => PwValue::Bytes(v),
            PodValue::Rectangle(v) => PwValue::Rectangle(v),
            PodValue::Fraction(v) => PwValue::Fraction(v),
            PodValue::Fd(v) => PwValue::Fd(v),
            PodValue::ValueArray(v) => PwValue::ValueArray(v),
            PodValue::Struct(vec) => PwValue::Struct(
                vec.into_iter().map(|item| item.into()).collect(),
            ),
            PodValue::Object(v) => PwValue::Object(v),
            PodValue::Choice(v) => PwValue::Choice(v),
            PodValue::Pointer(t, p) => PwValue::Pointer(t, p),
        }
    }
}

pub struct KeyValue(pub pipewire::spa::param::format::FormatProperties, pub PodValue);
#[macro_export]
macro_rules! kv {
    ($key:expr => $val:expr) => {
        KeyValue($key, $val.into())
    };
}
impl From<KeyValue> for PwProperty {
    fn from(kv: KeyValue) -> Self {
        PwProperty::new(kv.0.as_raw(), kv.1.into())
    }
}