use nih_plug::prelude::*;
use std::sync::Arc;

// This is a shortened version of the gain example with most comments removed, check out
// https://github.com/robbert-vdh/nih-plug/blob/master/plugins/examples/gain/src/lib.rs to get
// started

struct Whammy {
	params: Arc<WhammyParams>,
	buffer: Vec<f32>,
	write_pos: usize,
  envelope: Vec<f32>,
  envelope_pos: usize,
}

#[derive(Params)]
struct WhammyParams {
	/// The parameter's ID is used to identify the parameter in the wrappred plugin API. As long as
	/// these IDs remain constant, you can rename and reorder these fields as you wish. The
	/// parameters are exposed to the host in the same order they were defined. In this case, this
	/// gain parameter is stored as linear gain while the values are displayed in decibels.
	#[id = "dive"]
	pub dive: FloatParam,
}

impl Default for Whammy {
	fn default() -> Self {
		let mut buffer: Vec<f32> = Vec::new();
    // Zero out ring buffer
		for _ in 0..(44100 * 3) {
			buffer.push(0.);
		}

    let mut envelope: Vec<f32> = Vec::new();
    let inc: f32 = 1. / 22050.;

    // Manually create envelop
    let mut level: f32 = 0.;
    for i in 0..22050 {
      envelope.push(level);
      level += inc;
    }
    for i in 0..22050 {
      level -= inc;
      envelope.push(level);
    }

		Self {
			params: Arc::new(WhammyParams::default()),
			buffer,
			write_pos: 0,
			envelope,
      envelope_pos: 0
		}
	}
}

impl Default for WhammyParams {
	fn default() -> Self {
		Self {
			// This gain is stored as linear gain. NIH-plug comes with useful conversion functions
			// to treat these kinds of parameters as if we were dealing with decibels. Storing this
			// as decibels is easier to work with, but requires a conversion for every sample.
			dive: FloatParam::new(
				"Div",
				0.,
				FloatRange::Skewed {
					min: -1.,
					max: 0.,
					// This makes the range appear as if it was linear when displaying the values as
					// decibels
					factor: 1.
				},
			)
			// Because the gain parameter is stored as linear gain instead of storing the value as
			// decibels, we need logarithmic smoothing
			.with_smoother(SmoothingStyle::Logarithmic(50.0))
			.with_unit(" dB")
			// There are many predefined formatters we can use here. If the gain was stored as
			// decibels instead of as a linear gain value, we could have also used the
			// `.with_step_size(0.1)` function to get internal rounding.
			.with_value_to_string(formatters::v2s_f32_gain_to_db(2))
			.with_string_to_value(formatters::s2v_f32_gain_to_db()),
		}
	}
}

impl Plugin for Whammy {
	const NAME: &'static str = "Whammy";
	const VENDOR: &'static str = "Marcus Haddon";
	const URL: &'static str = env!("CARGO_PKG_HOMEPAGE");
	const EMAIL: &'static str = "haddon.marcus@gmail.com";

	const VERSION: &'static str = env!("CARGO_PKG_VERSION");

	// The first audio IO layout is used as the default. The other layouts may be selected either
	// explicitly or automatically by the host or the user depending on the plugin API/backend.
	const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[AudioIOLayout {
		main_input_channels: NonZeroU32::new(2),
		main_output_channels: NonZeroU32::new(2),

		aux_input_ports: &[],
		aux_output_ports: &[],

		// Individual ports and the layout as a whole can be named here. By default these names
		// are generated as needed. This layout will be called 'Stereo', while a layout with
		// only one input and output channel would be called 'Mono'.
		names: PortNames::const_default(),
	}];

	const MIDI_INPUT: MidiConfig = MidiConfig::MidiCCs;
	const MIDI_OUTPUT: MidiConfig = MidiConfig::None;

	const SAMPLE_ACCURATE_AUTOMATION: bool = true;

	// If the plugin can send or receive SysEx messages, it can define a type to wrap around those
	// messages here. The type implements the `SysExMessage` trait, which allows conversion to and
	// from plain byte buffers.
	type SysExMessage = ();
	// More advanced plugins can use this to run expensive background tasks. See the field's
	// documentation for more information. `()` means that the plugin does not have any background
	// tasks.
	type BackgroundTask = ();

	fn params(&self) -> Arc<dyn Params> {
		self.params.clone()
	}

	fn initialize(
		&mut self,
		_audio_io_layout: &AudioIOLayout,
		_buffer_config: &BufferConfig,
		context: &mut impl InitContext<Self>,
	) -> bool {
		// Resize buffers and perform other potentially expensive initialization operations here.
		// The `reset()` function is always called right after this function. You can remove this
		// function if you do not need it.
		true
	}

	fn reset(&mut self) {
		// Reset buffers and envelopes here. This can be called from the audio thread and may not
		// allocate. You can remove this function if you do not need it.
	}

	fn process(
		&mut self,
		buffer: &mut Buffer,
		_aux: &mut AuxiliaryBuffers,
		_context: &mut impl ProcessContext<Self>,
	) -> ProcessStatus {
		// WRITE OUT
		for channel_samples in buffer.iter_samples() {
			// Smoothing is optionally built into the parameters themselves
			// let gain = self.params.gain.smoothed.next();

			for sample in channel_samples {
        self.buffer[self.write_pos] = sample.clone();
        
        let delayed_read = self.write_pos as f32 - self.params.dive.smoothed.next() * 1000.;
        let wrapped = if delayed_read < 0. {
          self.buffer.len() as f32 + delayed_read // TODO: store buffer len as float in struct
        } else {
          delayed_read % self.buffer.len() as f32
        };

        let interpolated = self.interpolate(&wrapped, &self.buffer);
        *sample = interpolated;
        
        self.write_pos = (self.write_pos + 1) % self.buffer.len();
        self.envelope_pos = (self.envelope_pos + 1) % self.envelope.len();
			}
		}

		ProcessStatus::Normal
	}
}

impl Whammy {
  fn interpolate(&self, f_idx: &f32, buffer: &Vec<f32>) -> f32 {
    let low_idx = *f_idx as usize;
    let high_idx = (low_idx + 1) % buffer.len();
    let low_sample = buffer[low_idx];
    let high_sample = buffer[high_idx];
    
    (low_sample + high_sample) * 0.5
  }
}

impl ClapPlugin for Whammy {
	const CLAP_ID: &'static str = "com.your-domain.whammy";
	const CLAP_DESCRIPTION: Option<&'static str> = Some("Creates 'whammy bar' or 'tremolo arm' style bends, dips, or divebombs, as well as MBV style 'glid guitar' pitch bending, but on any incoming audio stream.");
	const CLAP_MANUAL_URL: Option<&'static str> = Some(Self::URL);
	const CLAP_SUPPORT_URL: Option<&'static str> = None;

	// Don't forget to change these features
	const CLAP_FEATURES: &'static [ClapFeature] = &[ClapFeature::AudioEffect, ClapFeature::Stereo];
}

impl Vst3Plugin for Whammy {
	const VST3_CLASS_ID: [u8; 16] = *b"WhammyAudioBuffr";

	// And also don't forget to change these categories
	const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] =
		&[Vst3SubCategory::Fx, Vst3SubCategory::Dynamics];
}

nih_export_clap!(Whammy);
nih_export_vst3!(Whammy);
