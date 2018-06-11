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
use vst::api::{self};
use vst::event::MidiEvent;

use easyvst::*;

use std::path::PathBuf;

mod recording_buffer;
use recording_buffer::*;

mod looper_fsm;
use looper_fsm::*;


easyvst!(ParamId, ELState, ELPlugin);

#[repr(usize)]
#[derive(Debug, Copy, Clone)]
pub enum ParamId {
    GainDb,
}




struct Command {
    note: u8, // the midi note this command is bound to
    command: Commands,
}





#[derive(Default)]
struct ELState {
    my_folder: PathBuf,
    gain_amp: f32,
    send_buffer: SendEventBuffer,
    buffers: Vec<RecordingBuffer>,
    loop_index: usize,
    loop_len: usize,
    // current index in loop
    state: LooperState,
    events: Vec<MidiEvent>,
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
        let log_file = File::create(my_folder.join("plexlooper7.log")).unwrap();
        use std::fs::File;

        let _ = CombinedLogger::init(vec![WriteLogger::new(LogLevelFilter::Info,
                                                           Config::default(), log_file)]);
        info!("init in host {:?}", self.state.host.get_info());
        info!("my folder {:?}", my_folder);

        const NUM_BUFFERS: usize = 2;
        // genearate the buffers

        let mut buffers = Vec::new();

        for _i in 0..NUM_BUFFERS {
            let buffer = RecordingBuffer::new();

            buffers.push(buffer);
        }

        let state = &mut self.state.user_state;
        state.buffers = buffers;

        state.loop_index = 0;  // which loop buffer are we recording to?
        state.state = LooperState::Stopped;

        state.my_folder = my_folder;
        state.events = Vec::with_capacity(1024);
        info!("Init Done");
    }

    fn process<T: Float + AsPrim>(&mut self, events: &api::Events, buffer: &mut AudioBuffer<T>) {

        let state = &mut self.state.user_state;

        let (inputs, mut outputs) = buffer.split();

        // Assume 2 channels
        if inputs.len() < 2 || outputs.len() < 2 {
            return;
        }

        // Iterate over inputs as (&f32, &f32)
        let (l, r) = inputs.split_at(1);
        let stereo_in = l[0].iter().zip(r[0].iter());

        // Iterate over outputs as (&mut f32, &mut f32)
        let (mut l, mut r) = outputs.split_at_mut(1);
        let stereo_out = l[0].iter_mut().zip(r[0].iter_mut());

        // select the buffer we are recording into
        let buffer = &mut state.buffers[state.loop_index];

        for ((left_in, right_in), (left_out, right_out)) in stereo_in.zip(stereo_out) {


            let mut left_processed: f32 = 0.0;
            let mut right_processed: f32 = 0.0;
            // Push the new samples into the loop buffers.
            match state.state {
                LooperState::Recording => {
                    buffer.buffer.push_back((left_in.as_f32(), right_in.as_f32()));
                }
                LooperState::Playing => {
                    if let Some((left_old, right_old)) = buffer.buffer.pop_front() {
                        buffer.buffer.push_back((left_old, right_old));
                        const WET_MULT: f32 = 0.66;

                        left_processed = left_old * WET_MULT;
                        right_processed = right_old * WET_MULT;
                    }
                }
                _ => { }
            }


            *left_out = *left_in + left_processed.as_();
            *right_out = *right_in + right_processed.as_();

        }


        use vst::event::Event;

        for e in events.events() {
            const A3_PITCH: u8 = 69;
            const G3_PITCH: u8 = 67;
            const F3_PITCH: u8 = 65;
            match e {
                Event::Midi(mut ev) => {
                    info!("Midi Event: {:?}", status(ev.data[0]));
                    if status(ev.data[0]) == Status::NoteOn {
                        let pitch = ev.data[1];
                        info!("Pitch: {}", pitch);
                        match pitch {
                            A3_PITCH => {
                                state.state = looper_cycle(state.state, Commands::Record);
                            }
                            G3_PITCH => {
                                state.state = looper_cycle(state.state, Commands::Stop);
                            }
                            F3_PITCH => {
                                state.state = looper_cycle(state.state, Commands::Play);
                            }
                            _ => { }
                        }
                        info!("new state: {}", state.state);
                        info!("Size Buffer {}: {}", state.loop_index, buffer.buffer.len());
                    }
                    state.events.push(ev);
                }
                _ => ()
            }
        }

        // state.send_buffer.send_events(events, &mut self.state.host)
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

fn status(b: u8) -> Status { b.into() }

impl_clike!(Status);

#[repr(usize)]
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Status {
    // voice
    NoteOff = 0x80,
    NoteOn = 0x90,
    PolyphonicAftertouch = 0xA0,
    ControlChange = 0xB0,
    ProgramChange = 0xC0,
    ChannelAftertouch = 0xD0,
    PitchBend = 0xE0,

    // sysex
    SysExStart = 0xF0,
    MIDITimeCodeQtrFrame = 0xF1,
    SongPositionPointer = 0xF2,
    SongSelect = 0xF3,
    TuneRequest = 0xF6, // F4 anf 5 are reserved and unused
    SysExEnd = 0xF7,
    TimingClock = 0xF8,
    Start = 0xFA,
    Continue = 0xFB,
    Stop = 0xFC,
    ActiveSensing = 0xFE, // FD also res/unused
    SystemReset = 0xFF,
}