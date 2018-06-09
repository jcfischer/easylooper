#[macro_use]
extern crate vst;
#[macro_use]
extern crate easyvst;
#[macro_use]
extern crate log;
extern crate log_panics;
extern crate simplelog;
extern crate num_traits;
extern crate asprim;


use simplelog::*;

use num_traits::Float;
use asprim::AsPrim;

use vst::plugin::{Info, Category, HostCallback, CanDo};
use vst::buffer::{AudioBuffer, SendEventBuffer};
use vst::host::Host;
use vst::api::Events;


use easyvst::*;

use std::path::PathBuf;

easyvst!(ParamId, ELState, ELPlugin);

#[repr(usize)]
#[derive(Debug, Copy, Clone)]
pub enum ParamId {
    GainDb,
}

/// A left channel and right channel sample.
type SamplePair = (f32, f32);

#[derive(Default)]
struct ELState {
    my_folder: PathBuf,
    gain_amp: f32,
    send_buffer: SendEventBuffer,
    buffer_l: Vec<f32>,
    buffer_r: Vec<f32>,
    index: usize,
    // current index in loop
    recording: bool,
}

impl UserState<ParamId> for ELState {
    fn param_changed(&mut self, _host: &mut HostCallback, param_id: ParamId, val: f32) {
        info!("param_changed {:?} {:2}", param_id, val);
        use ParamId::*;
        match param_id {
            GainDb => self.gain_amp = db_to_amp(val),
        }
    }

    fn format_param(&self, param_id: ParamId, val: f32) -> String {
        // info!("format_param {:?} {:.2}", param_id, val);
        use ParamId::*;
        match param_id {
            GainDb => format!("{:.2} dB", val),
        }
    }
}

type ELPluginState = PluginState<ParamId, ELState>;

#[derive(Default)]
struct ELPlugin {
    state: ELPluginState,

    // ui: Option<UiState>
}

impl ELPlugin {
    fn process_one_channel<F: Float + AsPrim>(&mut self, input: &[F], output: &mut [F],
                                              channel: usize) {

        let state = &mut self.state.user_state;

        let buffer = [&mut state.buffer_l, &mut state.buffer_r];
        if state.recording {

            for input in input.iter() {
                buffer[channel][state.index] = input.as_f32();
                state.index += 1;
                state.index = state.index % 102400;
            }
            info!("index: {:}", state.index);

        }

        for (input_sample, output_sample) in input.iter().zip(output) {
            *output_sample = *input_sample * state.gain_amp.as_();
        }
    }
}

impl EasyVst<ParamId, ELState> for ELPlugin {
    fn params() -> Vec<ParamDef> {
        vec![
            ParamDef::new("Gain2", -48.0, 12.0, 0.0),
        ]
    }

    fn state(&self) -> &ELPluginState { &self.state }

    fn state_mut(&mut self) -> &mut ELPluginState { &mut self.state }

    fn get_info(&self) -> Info {
        Info {
            name: "Easy Looper".to_string(),
            vendor: "SunMachines".to_string(),
            unique_id: 0x87a93b3,
            category: Category::Effect,

            inputs: 2,
            outputs: 2,
            parameters: 1,

            ..Info::default()
        }
    }

    fn new(state: ELPluginState) -> Self {
        let mut p: ELPlugin = Default::default();
        p.state = state;
        p
    }

    fn init(&mut self) {
        #[cfg(windows)] let my_folder = fs::get_folder_path().unwrap();
        // #[cfg(not(windows))] let my_folder = ::std::env::home_dir().unwrap();
        #[cfg(not(windows))] let my_folder = ::std::path::PathBuf::from("/Users/fischer/Desktop");
        ;
        let log_file = File::create(my_folder.join("plexlooper1.log")).unwrap();
        use std::fs::File;

        let _ = CombinedLogger::init(vec![WriteLogger::new(LogLevelFilter::Info,
                                                           Config::default(), log_file)]);
        info!("init in host {:?}", self.state.host.get_info());
        info!("my folder {:?}", my_folder);

        let state = &mut self.state.user_state;
        state.buffer_l = Vec::with_capacity(102400);
        state.buffer_r = Vec::with_capacity(102400);
        state.index = 0;
        state.recording = true;
        state.my_folder = my_folder;
        info!("Init Done");
    }

    fn process<T: Float + AsPrim>(&mut self, events: &Events, buffer: &mut AudioBuffer<T>) {

        for (i, (input_buffer, output_buffer)) in buffer.zip().enumerate() {
            self.process_one_channel(input_buffer, output_buffer, i);
        }


        use vst::event::Event;

        let events = events.events().filter_map(|e| {
            match e {
                Event::Midi(e) => Some(e),
                _ => None
            }
        });
        self.state.user_state.send_buffer.send_events(events, &mut self.state.host)
    }

    fn can_do(&self, can_do: CanDo) -> vst::api::Supported {
        use vst::api::Supported::*;
        use vst::plugin::CanDo::*;

        match can_do {
            SendEvents | SendMidiEvent | ReceiveEvents | ReceiveMidiEvent => Yes,
            _ => No,
        }
    }
}

#[inline]
pub fn amp_to_db<F: Float + AsPrim>(x: F) -> F {
    20.0.as_::<F>() * x.log10()
}

#[inline]
pub fn db_to_amp<F: Float + AsPrim>(x: F) -> F {
    10.0.as_::<F>().powf(x / 20.0.as_())
}