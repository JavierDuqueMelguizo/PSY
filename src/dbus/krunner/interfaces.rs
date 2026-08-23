use zbus::proxy;

use crate::dbus::krunner::types::RawKRunnerMatch;

#[proxy(
    interface = "org.kde.krunner1",
    default_service = "org.kde.KWin",
    default_path = "/WindowsRunner"
)]
pub trait WindowsRunnerApi {
    #[zbus(name = "Match")]
    async fn match_query(&self, query: &str) -> zbus::Result<Vec<RawKRunnerMatch>>;
}