use std::collections::HashMap;

use zbus::{proxy, zvariant::{OwnedObjectPath, Value}};

// NOTA DEV: Con backend era mucho mas sencillo... La comunicación desde el lado cliente
// añade muchos mas procesos...
// Versión si fuera un compositor:
// #[proxy(
//     default_service = "org.freedesktop.impl.portal.desktop.kde",
//     default_path = "/org/freedesktop/portal/desktop",
//     interface = "org.freedesktop.impl.portal.ScreenCast"
// )]
// Debo utilizar la habilitada para clientes:
#[proxy(
    default_service = "org.freedesktop.portal.Desktop",
    default_path = "/org/freedesktop/portal/desktop",
    interface = "org.freedesktop.portal.ScreenCast"
)]
pub trait IScreenCast{
    
    #[zbus(name = "CreateSession")]
    async fn create_session<'a>(
        &self,
        options : HashMap<&str, Value<'a>>,
    ) -> zbus::Result<OwnedObjectPath>;

    #[zbus(name = "SelectSources")]
    async fn select_sources<'a>(
        &self,
        session_handle: OwnedObjectPath,
        options : HashMap<&str, Value<'a>>,
    ) -> zbus::Result<OwnedObjectPath>;
   
    #[zbus(name = "Start")]
    async fn start<'a>(
        &self,
        session_handle: OwnedObjectPath,
        parent_window: &str,
        options : HashMap<&str, Value<'a>>
    ) -> zbus::Result<OwnedObjectPath>;

    #[zbus(name = "OpenPipeWireRemote")]
    async fn open_pipewire_remote<'a>(
        &self,
        session_handle: OwnedObjectPath,
        options : HashMap<&str, Value<'a>>
    ) -> zbus::Result<zbus::zvariant::OwnedFd>;

    #[zbus(property, name = "AvailableSourceTypes")]
    fn get_available_source_types(&self) -> zbus::Result<u32>;

    #[zbus(property, name = "AvailableCursorModes")]
    fn get_available_cursor_modes(&self) -> zbus::Result<u32>;

    #[zbus(property, name = "Version")]
    fn get_version(&self) -> zbus::Result<u32>;

}

#[proxy(
    default_service = "org.freedesktop.portal.Desktop",
    interface = "org.freedesktop.portal.Request",
)]
pub trait IRequest {

    #[zbus(name="Close")]
    fn close(&self) -> zbus::Result<()>;

    #[zbus(signal, name="Response")]
    fn response(&self, response: u32, results: HashMap<String, Value<'_>>) -> zbus::Result<()>;
}