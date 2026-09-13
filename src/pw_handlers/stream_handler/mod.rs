use pipewire::spa::pod::{
    Property as PwProperty, 
    PropertyFlags as PwPropertyFlags
};

pub struct KeyValue(pub pipewire::spa::param::format::FormatProperties, pub(crate) PodValue, pub Option<PwPropertyFlags>);

#[macro_export]
macro_rules! kv {
    ($key:expr => $val:expr) => {
        KeyValue($key, $val.into(), None)
    };

    ($key:expr => $val:expr, $flags:expr) => {
        KeyValue($key, $val.into(), Some($flags))
    };
}
impl From<KeyValue> for PwProperty {
    fn from(kv: KeyValue) -> Self {
        PwProperty{
            key : kv.0.as_raw(),
            value: kv.1.into(),
            flags: match kv.2{
                None => PwPropertyFlags::empty(),
                Some(value) => value
            } 
        }
    }
}

#[macro_use]
pub mod builders;
pub use builders::*;

pub mod handler;
pub use handler::*;

pub mod data;
pub use data::*;




