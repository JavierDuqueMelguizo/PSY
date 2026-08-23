
use super::interfaces::WindowsRunnerApiProxy;
use super::types::{WindowInfo};

pub struct WindowsRunnerClient<'a> {
    proxy: WindowsRunnerApiProxy<'a>,
}

impl<'a> WindowsRunnerClient<'a> {
    pub async fn new(connection: &'a zbus::Connection) -> zbus::Result<Self> {
        let proxy = WindowsRunnerApiProxy::new(connection).await?;
        Ok(Self { proxy })
    }

    /// Obtiene únicamente las ventanas reales del usuario, aplicando los filtros requeridos.
    pub async fn get_active_windows(&self) -> zbus::Result<Vec<WindowInfo>> {
        let raw_matches = self.proxy.match_query("").await?;

        let windows = raw_matches
            .into_iter()
            .map(|(id, title, app_id, category, relevance, properties)| WindowInfo { id, title, app_id, category, relevance, properties })
            // 1. Filtrar títulos vacíos
            .filter(|win| !win.title.trim().is_empty())
            // 2. Filtrar la pasera de Wayland a X para aplicaciones viejas que esperan X11 (xwaylandvideobridge)
            .filter(|win| win.app_id != "xwaylandvideobridge" && !win.title.contains("xwaylandvideobridge"))
            // 3. Mantener solo las coincidencias primarias exactas para evitar duplicidades
            .filter(|win| win.category == 100)
            .collect();

        Ok(windows)
    }
}