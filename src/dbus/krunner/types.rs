use std::collections::HashMap;
use zbus::zvariant::{OwnedValue};

/// Representa una coincidencia cruda devuelta por cualquier servicio `org.kde.krunner1`
/// https://invent.kde.org/frameworks/krunner/-/blob/master/src/data/org.kde.krunner1.xml#L103
pub type RawKRunnerMatch = (
    String,                 // id
    String,                 // title
    String,                 // icon
    i32,                    // category? o app_id?
    f64,                    // relevance
    HashMap<String, OwnedValue>, // properties
);

/// Estructura limpia para consumir en tu aplicación
#[derive(Debug, Clone)]
pub struct WindowInfo {
    pub id: String,
    pub title: String,
    pub app_id: String,
    pub category: i32,
    pub relevance: f64,
    pub properties: HashMap<String, OwnedValue>,
}