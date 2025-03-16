use bevy::app::App;
use bevy::math::Vec3;
#[allow(deprecated)]
use bevy::prelude::{Changed, Component, GlobalTransform, Query, Update, With};
use bevy::prelude::{Single, Transform};
use cpal::FromSample;
use rodio::{Sample, Source};

pub struct SpatialAudioPlugin;

impl bevy::prelude::Plugin for SpatialAudioPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, set_spatial_audio_sink_pos);
    }
}

#[derive(Component)]
pub struct Ears {
    pub left: Vec3,
    pub right: Vec3,
}

impl Default for Ears {
    fn default() -> Self {
        Self {
            left: Vec3::X * 4.0 / -2.0,
            right: Vec3::X * 4.0 / 2.0,
        }
    }
}

#[derive(Component)]
#[require(Transform)]
pub struct SpatialAudioSink {
    pub(crate) sink: rodio::SpatialSink,
}

impl SpatialAudioSink {
    #[inline]
    pub fn append<S>(&self, source: S)
    where
        S: Source + Send + 'static,
        f32: FromSample<S::Item>,
        S::Item: Sample + Send,
    {
        self.sink.append(source);
    }
}

/// Only a single one of these at a time in an app
/// This is the audio listener, where the imaginary microphone is that receives all sound
#[derive(Component)]
#[require(Transform)]
#[require(Ears)]
pub struct SpatialAudioListener;

fn set_spatial_audio_sink_pos(
    mut spatial_audio_sinks: Query<
        (&mut SpatialAudioSink, &GlobalTransform),
        Changed<GlobalTransform>,
    >,
    listener: Option<Single<(&GlobalTransform, &Ears), With<SpatialAudioListener>>>,
) {
    let Some(listener) = listener else { return };
    let (listener_position, ears) = (listener.0, listener.1);

    for (spatial_audio_sink, global_transform) in spatial_audio_sinks.iter_mut() {
        spatial_audio_sink.sink.set_emitter_position(
            (listener_position.translation() - global_transform.translation()).to_array(),
        );
        spatial_audio_sink
            .sink
            .set_left_ear_position(ears.left.to_array());
        spatial_audio_sink
            .sink
            .set_right_ear_position(ears.right.to_array());
    }
}
