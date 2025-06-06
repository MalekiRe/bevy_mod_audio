pub mod audio_output;
pub mod microphone;
pub mod spatial_audio;

use crate::audio_output::AudioOutputPlugin;
use crate::microphone::MicrophonePlugin;
use bevy::app::PluginGroupBuilder;

pub struct ModAudioPlugins;

impl bevy::prelude::PluginGroup for ModAudioPlugins {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>()
            .add(spatial_audio::SpatialAudioPlugin)
            .add(AudioOutputPlugin)
            .add(MicrophonePlugin)
    }
}
