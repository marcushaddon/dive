use nih_plug::prelude::*;
use std::sync::Arc;

// This is a shortened version of the gain example with most comments removed, check out
// https://github.com/robbert-vdh/nih-plug/blob/master/plugins/examples/gain/src/lib.rs to get
// started

struct Dive {
	params: Arc<WhammyParams>,
	buffer: Vec<f32>,
	write_pos: usize,
  read_pos: usize,
}

#[derive(Params)]
struct WhammyParams {
	/// The parameter's ID is used to identify the parameter in the wrappred plugin API. As long as
	/// these IDs remain constant, you can rename and reorder these fields as you wish. The
	/// parameters are exposed to the host in the same order they were defined. In this case, this
	/// gain parameter is stored as linear gain while the values are displayed in decibels.
	#[id = "dive"]
	pub dive: FloatParam,

  #[id = "depth"]
  pub depth: FloatParam
}

impl Default for Dive {
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
      read_pos: 0
		}
	}
}

impl Default for WhammyParams {
	fn default() -> Self {
		Self {
			dive: FloatParam::new(
				"Dive",
				0.,
				FloatRange::Linear {
					min: 0.,
					max: 0.5,
				},
			)
			.with_smoother(SmoothingStyle::Linear(250.)),

      depth: FloatParam::new(
        "Depth",
        1.,
        FloatRange::Linear {
          min: 0.,
          max: 3.
        }
      ).with_smoother(SmoothingStyle::Linear(250.))
		}
	}
}

impl Plugin for Dive {
	const NAME: &'static str = "Whammy";
	const VENDOR: &'static str = "Marcus Haddon";
	const URL: &'static str = env!("CARGO_PKG_HOMEPAGE");
	const EMAIL: &'static str = "haddon.marcus@gmail.com";

	const VERSION: &'static str = env!("CARGO_PKG_VERSION");

	const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[AudioIOLayout {
		main_input_channels: NonZeroU32::new(2),
		main_output_channels: NonZeroU32::new(2),

		aux_input_ports: &[],
		aux_output_ports: &[],

		names: PortNames::const_default(),
	}];

	const MIDI_INPUT: MidiConfig = MidiConfig::MidiCCs;
	const MIDI_OUTPUT: MidiConfig = MidiConfig::None;

	const SAMPLE_ACCURATE_AUTOMATION: bool = true;

	type SysExMessage = ();
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
    // TODO: allocate here
		true
	}



	fn process(
		&mut self,
		buffer: &mut Buffer,
		_aux: &mut AuxiliaryBuffers,
		_context: &mut impl ProcessContext<Self>,
	) -> ProcessStatus {
    self.read(buffer);

    self.write(buffer);

		ProcessStatus::Normal
	}
}

impl Dive {
    fn read(&mut self, buffer: &mut Buffer) {
      for channel_samples in buffer.iter_samples() {
        // TODO: support > 1 channel
        for sample in channel_samples {
          self.buffer[self.write_pos] = *sample;
          break;
        }

        self.write_pos = (self.write_pos + 1) % self.buffer.len();
      }
    }

    fn write(&mut self, buffer: &mut Buffer) {
      for channel_samples in buffer.iter_samples() {
        let delayed_read = self.read_pos as f32 - 1. -
          self.params.dive.smoothed.next() * self.params.depth.smoothed.next() * 1000.;
        let wrapped = if delayed_read < 0. {
          self.buffer.len() as f32 + delayed_read // TODO: store buffer len as float in struct
        } else {
          delayed_read % self.buffer.len() as f32
        };

        for sample in channel_samples {
          *sample = interpolate_2(&wrapped, &self.buffer)
        }
        self.read_pos = (self.read_pos + 1) % self.buffer.len();
      }
    }
}

fn interpolate_2(f_idx: &f32, buffer: &Vec<f32>) -> f32 {
  // y1 + (x - x1) * (y2 - y1) / (x2 - x1)
  // low_sample + (f_idx - low_idx) * (high_sample - low_sample) / (high_idx - low_idx) = 1\
  // low_sample + (f_idx - low_idx) * (high_sample - low_sample)
  let clamped = f_idx.clamp(0., buffer.len() as f32);


  let low_idx = clamped as usize;
  let high_idx = low_idx + 1;
  let high_idx_wrapped = if high_idx >= buffer.len() {
    high_idx - buffer.len()
  } else {
    high_idx
  };
  let low_sample = buffer[low_idx];
  let high_sample = buffer[high_idx_wrapped];

  low_sample + (f_idx - (low_idx as f32)) * (high_sample - low_sample)
}

impl ClapPlugin for Dive {
	const CLAP_ID: &'static str = "com.your-domain.whammy";
	const CLAP_DESCRIPTION: Option<&'static str> = Some("Creates 'whammy bar' or 'tremolo arm' style bends, dips, or divebombs, as well as MBV style 'glid guitar' pitch bending, but on any incoming audio stream.");
	const CLAP_MANUAL_URL: Option<&'static str> = Some(Self::URL);
	const CLAP_SUPPORT_URL: Option<&'static str> = None;

	// Don't forget to change these features
	const CLAP_FEATURES: &'static [ClapFeature] = &[ClapFeature::AudioEffect, ClapFeature::Stereo];
}

impl Vst3Plugin for Dive {
	const VST3_CLASS_ID: [u8; 16] = *b"WhammyAudioBuffr";

	// And also don't forget to change these categories
	const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] =
		&[Vst3SubCategory::Fx, Vst3SubCategory::Dynamics];
}

nih_export_clap!(Dive);
nih_export_vst3!(Dive);
