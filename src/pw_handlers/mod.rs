#[macro_use]
pub mod pod;
pub use pod::*;

use std::collections::HashMap;

// Re-export pod module contents as needed from the pod module directly.

pub mod stream_handler;
pub use stream_handler::StreamPipewireHandler;


macro_rules! add_if {
    ($map:expr, $key:ident, $cond:expr, $val:expr) => {
        if $cond {
            $map.insert(
                FormatProperties::$key.as_raw(),
                kv!(FormatProperties::$key => $val).into(),
            );
        }
    };
}

pub trait ToSpaProperties {
    fn to_spa_properties(&self) -> HashMap<u32, pipewire::spa::pod::Property>;
}

impl ToSpaProperties for pipewire::spa::param::video::VideoInfoRaw {
    fn to_spa_properties(&self) -> HashMap<u32, pipewire::spa::pod::Property> {

        use pipewire::spa::{
            utils::Id,
            param::{
                video::{VideoFormat, VideoInterlaceMode}, 
                format::FormatProperties
            }
        };
        use crate::pw_handlers::pod::KeyValue;

        let mut properties = HashMap::new();

        let fmt = self.format();
        add_if!(properties, VideoFormat, fmt != VideoFormat::Unknown, Id(fmt.as_raw()));

        let interlace = self.interlace_mode();
        add_if!(properties, VideoInterlaceMode, interlace != VideoInterlaceMode::Progressive, Id(interlace.as_raw()));

        let size = self.size();
        add_if!(properties, VideoSize, size.width > 0 && size.height > 0, size);

        let fps = self.framerate();
        add_if!(properties, VideoFramerate, fps.num > 0 && fps.denom > 0, fps);

        let max_fps = self.max_framerate();
        add_if!(properties, VideoMaxFramerate, max_fps.num > 0 && max_fps.denom > 0, max_fps);

        let par = self.pixel_aspect_ratio();
        add_if!(properties, VideoPixelAspectRatio, par.num > 0 && par.denom > 0, par);

        let modifier = self.modifier();
        add_if!(properties, VideoModifier, modifier != 0, modifier);

        let views = self.views();
        add_if!(properties, VideoViews, views != 0, views);

        let mv_mode = self.multiview_mode();
        add_if!(properties, VideoMultiviewMode, mv_mode != 0, Id(mv_mode as u32));

        let mv_flags = self.multiview_flags();
        add_if!(properties, VideoMultiviewFlags, mv_flags != 0, Id(mv_flags));

        let chroma = self.chroma_site();
        add_if!(properties, VideoChromaSite, chroma != 0, Id(chroma));

        let range = self.color_range();
        add_if!(properties, VideoColorRange, range != 0, Id(range));

        let matrix = self.color_matrix();
        add_if!(properties, VideoColorMatrix, matrix != 0, Id(matrix));

        let transfer = self.transfer_function();
        add_if!(properties, VideoTransferFunction, transfer != 0, Id(transfer));

        let primaries = self.color_primaries();
        add_if!(properties, VideoColorPrimaries, primaries != 0, Id(primaries));

        properties
    }
}

impl ToSpaProperties for pipewire::spa::param::audio::AudioInfoRaw {
    fn to_spa_properties(&self) -> HashMap<u32, pipewire::spa::pod::Property> {

        use pipewire::spa::{
            sys::SPA_AUDIO_MAX_CHANNELS,
            utils::Id,
            param::{
                audio::{AudioFormat, AudioInfoRawFlags}, 
                format::FormatProperties
            }
        };
        use crate::pw_handlers::pod::KeyValue;
        
        let mut properties = HashMap::new();

        let fmt = self.format();
        add_if!(properties, AudioFormat, fmt != AudioFormat::Unknown, Id(fmt.as_raw()));

        let flags = self.flags();
        add_if!(properties, AudioFlags, flags != AudioInfoRawFlags::empty(), flags.bits());

        let rate = self.rate();
        add_if!(properties, AudioRate, rate != 0, rate);

        let channels = self.channels();
        add_if!(properties, AudioChannels, channels != 0, channels);

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
