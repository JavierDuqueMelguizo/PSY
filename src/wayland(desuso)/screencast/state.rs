
use super::super::zkde_screencast::{
    zkde_screencast_unstable_v1::ZkdeScreencastUnstableV1,
    zkde_screencast_stream_unstable_v1::ZkdeScreencastStreamUnstableV1
};


pub struct ScreenCastState {
    pub screencast_manager: Option<ZkdeScreencastUnstableV1>,
    pub stream: Option<ZkdeScreencastStreamUnstableV1>
} 