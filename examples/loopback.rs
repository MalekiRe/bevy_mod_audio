use bevy::prelude::*;
use bevy_mod_audio::ModAudioPlugins;
use bevy_mod_audio::audio_output::AudioOutput;
use bevy_mod_audio::microphone::MicrophoneAudio;

pub fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(ModAudioPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, update)
        .run();
}

pub fn setup(mut commands: Commands, audio_output: ResMut<AudioOutput>) {
    commands.spawn(audio_output.new_sink().expect("Unable to spawn audio sink"));
}
pub fn update(
    sink: Single<&mut bevy_mod_audio::spatial_audio::SpatialAudioSink>,
    mic: ResMut<MicrophoneAudio>,
) {
    for owo in mic.try_iter() {
        sink.append(rodio::buffer::SamplesBuffer::new(
            mic.config.channels,
            mic.config.sample_rate,
            owo,
        ));
    }
}
