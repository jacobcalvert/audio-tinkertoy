use rodio::Source;
use std::num::NonZero;
use std::sync::{Arc, Mutex};

/// Thread-safe shared waveform data for audio synthesis.
/// Frequency and volume are stored here so that live slider changes
/// are immediately visible to the audio source without recreating it.
#[derive(Clone)]
pub struct SharedWaveform {
    pub data: Arc<Mutex<Vec<f32>>>,
    pub frequency: Arc<Mutex<f32>>,
    pub volume: Arc<Mutex<f32>>,
}

impl SharedWaveform {
    pub fn new() -> Self {
        Self {
            data: Arc::new(Mutex::new(Vec::new())),
            frequency: Arc::new(Mutex::new(440.0)),
            volume: Arc::new(Mutex::new(0.5)),
        }
    }

    pub fn update(&self, samples: Vec<f32>, frequency: f32, volume: f32) {
        let mut data = self.data.lock().unwrap();
        *data = samples;
        {
            let mut freq = self.frequency.lock().unwrap();
            *freq = frequency;
        }
        {
            let mut vol = self.volume.lock().unwrap();
            *vol = volume;
        }
    }

    pub fn get_samples(&self) -> Vec<f32> {
        self.data.lock().unwrap().clone()
    }

    pub fn get_frequency(&self) -> f32 {
        *self.frequency.lock().unwrap()
    }

    pub fn get_volume(&self) -> f32 {
        *self.volume.lock().unwrap()
    }
}

/// A custom audio source that generates a looping waveform at a given frequency.
pub struct WaveformSource {
    waveform: SharedWaveform,
    sample_rate: u32,
    phase: f32,
}

impl WaveformSource {
    pub fn new(waveform: SharedWaveform, sample_rate: u32) -> Self {
        Self {
            waveform,
            sample_rate,
            phase: 0.0,
        }
    }
}

impl Iterator for WaveformSource {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        let samples = self.waveform.get_samples();
        if samples.is_empty() {
            return Some(0.0);
        }

        // Read frequency and volume from the shared state so live
        // slider changes are reflected in real time.
        let frequency = self.waveform.get_frequency();
        let volume = self.waveform.get_volume();

        let waveform_len = samples.len() as f32;
        let phase_pos = self.phase % waveform_len;
        let idx = phase_pos.floor() as usize;
        let next_idx = (idx + 1) % samples.len();
        let frac = phase_pos - phase_pos.floor();

        // Linear interpolation between adjacent samples
        let sample = samples[idx] * (1.0 - frac) + samples[next_idx] * frac;

        // Apply volume
        let output = sample * volume;

        // Advance phase based on frequency
        let phase_step = frequency / self.sample_rate as f32 * waveform_len;
        self.phase += phase_step;

        Some(output)
    }
}

impl Source for WaveformSource {
    fn current_span_len(&self) -> Option<usize> {
        None // Indefinite span — we loop forever
    }

    fn channels(&self) -> NonZero<u16> {
        NonZero::new(1).unwrap() // Mono
    }

    fn sample_rate(&self) -> NonZero<u32> {
        NonZero::new(self.sample_rate).unwrap()
    }

    fn total_duration(&self) -> Option<std::time::Duration> {
        None // Infinite source
    }
}

/// The audio engine that manages playback using rodio.
pub struct AudioEngine {
    device_sink: Option<rodio::MixerDeviceSink>,
    shared_waveform: Option<SharedWaveform>,
}

impl AudioEngine {
    /// Create a new AudioEngine.
    pub fn new() -> Self {
        Self {
            device_sink: None,
            shared_waveform: None,
        }
    }

    /// Start playing audio with the given waveform editor settings.
    pub fn start(&mut self, waveform: &crate::editor::WaveformData, frequency: f32, volume: f32) {
        // Open the default audio device and create a mixer
        let device_sink = rodio::DeviceSinkBuilder::open_default_sink()
            .expect("Failed to open audio output device");

        // Generate a full-resolution waveform buffer for interpolation
        let waveform_samples = waveform.interpolate_to(1024);

        // Create shared waveform data for thread-safe access
        let shared_waveform = SharedWaveform::new();
        shared_waveform.update(waveform_samples, frequency, volume);

        // Clone for the audio source (Arc is cheap to clone)
        let source_waveform = shared_waveform.clone();

        // Create and add the audio source to the mixer
        let source = WaveformSource::new(source_waveform, 44100);
        device_sink.mixer().add(source);

        self.shared_waveform = Some(shared_waveform);
        self.device_sink = Some(device_sink);
    }

    /// Stop audio playback by dropping the device sink.
    pub fn stop(&mut self) {
        self.device_sink.take();
    }

    /// Update the shared waveform data for real-time audio feedback.
    pub fn update_waveform(&self, waveform: &crate::editor::WaveformData, frequency: f32, volume: f32) {
        if let Some(ref shared) = self.shared_waveform {
            let waveform_samples = waveform.interpolate_to(1024);
            shared.update(waveform_samples, frequency, volume);
        }
    }
}
