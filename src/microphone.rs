use bevy::app::App;
use bevy::log::{debug, error};
#[allow(deprecated)]
use bevy::prelude::{Commands, Resource, Startup, warn};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use flume::Receiver;
use std::ops::{Deref, DerefMut};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::{Closure, wasm_bindgen};
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::{JsCast, JsValue};
#[cfg(target_arch = "wasm32")]
use web_sys::{
    AudioContext, HtmlMediaElement, MediaStream, MediaStreamAudioSourceNode,
    MediaStreamConstraints, window,
};

pub struct MicrophonePlugin;

impl bevy::prelude::Plugin for MicrophonePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, create_microphone);
    }
}

#[derive(Resource)]
pub struct MicrophoneAudio {
    audio_data: Receiver<Vec<f32>>,
    pub config: MicrophoneConfig,
}

impl Deref for MicrophoneAudio {
    type Target = Receiver<Vec<f32>>;

    fn deref(&self) -> &Self::Target {
        &self.audio_data
    }
}

impl DerefMut for MicrophoneAudio {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.audio_data
    }
}

#[derive(Debug)]
pub struct MicrophoneConfig {
    pub channels: u16,
    pub sample_rate: u32,
}

impl Default for MicrophoneConfig {
    fn default() -> Self {
        Self {
            channels: 1,
            sample_rate: 48_000,
        }
    }
}

pub fn create_microphone(mut commands: Commands) {
    let (tx, rx) = flume::unbounded();
    #[cfg(target_arch = "wasm32")]
    {
        let window = window().unwrap();
        let navigator = window.navigator();
        let media_devices = navigator.media_devices().unwrap();

        // Create constraints to request microphone input
        let mut constraints = MediaStreamConstraints::new();
        constraints.get_audio();
        constraints.audio(&JsValue::from_bool(true));

        // Request the microphone stream
        let promise = media_devices
            .get_user_media_with_constraints(&constraints)
            .unwrap();

        let tx = tx.clone();
        let closure = Closure::wrap(Box::new(move |stream: JsValue| {
            let media_stream = MediaStream::from(stream);

            // Create an AudioContext
            let audio_context = AudioContext::new().unwrap();

            // Create a MediaStreamAudioSourceNode from the MediaStream
            let media_stream_audio_source = audio_context
                .create_media_stream_source(&media_stream)
                .unwrap();

            // Create a ScriptProcessorNode with a buffer size of 4096 samples
            let script_processor_node = audio_context
                .create_script_processor_with_buffer_size(4096)
                .unwrap();

            // Connect the MediaStreamAudioSourceNode to the ScriptProcessorNode
            media_stream_audio_source
                .connect_with_audio_node(&script_processor_node)
                .unwrap();

            // Connect the ScriptProcessorNode to the AudioContext destination (speakers, etc.)
            script_processor_node
                .connect_with_audio_node(&audio_context.destination())
                .unwrap();

            let tx = tx.clone();
            // Set up an audio processing callback
            let onaudioprocess_closure = Closure::wrap(Box::new(move |event: JsValue| {
                let audio_event = event.dyn_into::<web_sys::AudioProcessingEvent>().unwrap();
                let input_buffer = audio_event.input_buffer().unwrap();

                // Get the first channel (mono audio)
                let channel_data = input_buffer.get_channel_data(0).unwrap(); // Float32Array

                // Here you can process the raw PCM data
                // For example, logging the first 10 samples
                tx.send(channel_data).unwrap();
            }) as Box<dyn FnMut(JsValue)>);

            script_processor_node
                .set_onaudioprocess(Some(onaudioprocess_closure.as_ref().unchecked_ref()));
            onaudioprocess_closure.forget();
        }) as Box<dyn FnMut(JsValue)>);

        promise.then(&closure);
        closure.forget();
        commands.insert_resource(MicrophoneAudio(Mutex::new(rx)));
        return;
    }

    #[allow(unused_mut)]
    let mut microphone_config = MicrophoneConfig::default();

    // Setup microphone device
    let device = match cpal::default_host().default_input_device() {
        None => {
            return warn!("no audio input device found, microphone functionality will be disabled");
        }
        Some(device) => device,
    };
    let configs = match device.supported_input_configs() {
        Ok(configs) => configs,
        Err(err) => {
            return warn!(
                "supported stream config error, microphone functionality will be disabled, error: {}",
                err
            );
        }
    };
    for config in configs {
        debug!("supported microphone config: {:#?}", config);
    }
    let mut configs = match device.supported_input_configs() {
        Ok(configs) => configs,
        Err(err) => {
            return warn!(
                "supported stream config error, microphone functionality will be disabled, error: {}",
                err
            );
        }
    };

    #[cfg(target_os = "android")]
    {
        microphone_config.channels = 2;
    }

    let config = match configs.find(|c| {
        c.sample_format() == cpal::SampleFormat::F32
            && c.channels() == microphone_config.channels
            && c.min_sample_rate().0 <= microphone_config.sample_rate
            && c.max_sample_rate().0 >= microphone_config.sample_rate
    }) {
        None => {
            return warn!(
            "microphone config of {:?} not supported, microphone functionality will be disabled",
            microphone_config
        )
        }
        Some(config) => config,
    }
        .with_sample_rate(cpal::SampleRate(microphone_config.sample_rate));

    // Run microphone audio through our channel
    let err_fn = |err| error!("an error occurred on the output audio stream: {}", err);
    let stream = device
        .build_input_stream(
            &config.into(),
            move |d: &[f32], _| {
                // sending errors imply the receiver is dropped.
                tx.send(d.to_vec()).ok();
            },
            err_fn,
            None,
        )
        .expect("failed to build audio input stream");

    // we wanna share the output from our thread loop thing in here continuously with the rest of bevy.
    commands.insert_resource(MicrophoneAudio {
        audio_data: rx,
        config: microphone_config,
    });

    // we play the stream, and then in order to not drop the stuff we have here, to continue to play it continously
    // we have to loop, we sleep for 100 seconds so this thread rarely ever does anything.
    stream.play().expect("failed to play audio stream");
    Box::leak(Box::new(stream));
}
