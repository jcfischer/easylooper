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

use std::sync::RwLock;
use std::ops::{Deref, DerefMut};

use vst::plugin::{Info, Category, HostCallback, CanDo};
use vst::buffer::{AudioBuffer, SendEventBuffer};
use vst::host::Host;
use vst::editor::Editor;
use vst::api::{self};
use vst::event::MidiEvent;

mod version;

use version::*;

use easyvst::*;

use std::path::PathBuf;

mod recording_buffer;

use recording_buffer::*;

mod looper_fsm;

use looper_fsm::*;

use tinyui::*;

easyvst!(ParamId, ELState, ELPlugin);


#[repr(usize)]
#[derive(Debug, Copy, Clone)]
pub enum ParamId {
    Feedback,
    Division,
}


struct Command {
    note: u8,
    // the midi note this command is bound to
    command: Commands,
}


#[derive(Default)]
pub struct ELState {
    my_folder: PathBuf,
    // Parameters
    feedback: f32,
    division: usize, // how many 8ths are we dividing into (1 - 16)

    // buffers

    send_buffer: SendEventBuffer,
    buffers: Vec<RecordingBuffer>,
    buffer: RecordingBuffer,
    write_idx: usize,
    // index of Vec<rRecordingBuffer>
    cycle_len: usize,
    division_len: usize,
    subdivision: usize,
    in_sync: bool,
    sync_waiting: bool,
    sync_point: usize,

    cycles: usize,
    total_cycles: usize,
    play_position: usize,
    // index into current recording buffer
    write_position: usize,
    loop_length: usize,
    // length of current loop in samples
    // the playback position
    seconds: String,
    // display current position in seconds
    sample_rate: RwLock<f64>,

    state: LooperState,
    prev_state: LooperState,
    next_state: Option<LooperState>,
    // In case we need to return to the previous state after
    // a state change
    events: Vec<MidiEvent>,
}

impl UserState<ParamId> for ELState {
    fn param_changed(&mut self, _host: &mut HostCallback, param_id: ParamId, val: f32) {
        info!("param_changed {:?} {:2}", param_id, val);
        use ParamId::*;
        match param_id {
            Feedback => self.feedback = val,
            Division => self.division = val as usize,
        }
    }

    fn format_param(&self, param_id: ParamId, val: f32) -> String {
        info!("format_param {:?} {:.2}", param_id, val);
        use ParamId::*;
        match param_id {
            Feedback => format!("{:.2} ", val),
            Division => format!("{}", val),
        }
    }
}

type ELPluginState = PluginState<ParamId, ELState>;

#[derive(Default)]
struct ELPlugin {
    state: ELPluginState,
    window: Option<ui::PluginWindow>,
}

impl ELPlugin {}

impl EasyVst<ParamId, ELState> for ELPlugin {
    fn params() -> Vec<ParamDef> {
        vec![
            ParamDef::new("Feedback", 0.0, 1.0, 1.0),
            ParamDef::new("Division", 1.0, 16.0, 8.0),
        ]
    }

    fn state(&self) -> &ELPluginState { &self.state }

    fn state_mut(&mut self) -> &mut ELPluginState { &mut self.state }

    fn get_info(&self) -> Info {
        Info {
            name: "Plex Looper".to_string(),
            vendor: "SunMachines".to_string(),
            unique_id: 0x87a93b3,
            category: Category::Effect,
            version: Version::get_version(),
            inputs: 2,
            outputs: 2,
            parameters: 2,

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
        let log_file = File::create(my_folder.join("plexlooper.log")).unwrap();
        use std::fs::File;

        let _ = CombinedLogger::init(vec![WriteLogger::new(LevelFilter::Info,
                                                           Config::default(), log_file)]);
        info!("init in host {:?}", self.state.host.get_info());
        info!("my folder {:?}", my_folder);

        log_panics::init();

        let state = &mut self.state.user_state;


        // generate the buffers
        state.buffers = ELPlugin::clear_buffers();
        state.buffer = RecordingBuffer::new();

        state.write_idx = 0;  // which loop buffer are we recording to?
        state.state = LooperState::Stopped;
        state.play_position = 0;
        state.write_position = 0;

        state.division_len = 0;
        state.subdivision = 0;

        state.sync_waiting = false;
        state.in_sync = false;


        state.total_cycles = 1;
        state.my_folder = my_folder;
        state.events = Vec::with_capacity(1024);
        info!("Init Done");
    }

    fn get_editor(&mut self) -> Option<&mut Editor> {
        Some(self)
    }

    fn set_sample_rate(&mut self, fs: f32) {
        info!("set_sample_rate: {}", fs);
        let fs = fs as f64;
        let state = &mut self.state.user_state;
        *state.sample_rate.write().unwrap().deref_mut() = fs;
    }

    fn process<T: Float + AsPrim>(&mut self, events: &api::Events, buffer: &mut AudioBuffer<T>) {
        const WET_MULT: f32 = 0.98;
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


        let stereo_in_len = stereo_in.len();
        let stereo_out_len = stereo_out.len();
        let play_position = state.play_position;
        let write_position = state.write_position;

        // info!("write pos/reading pos {}/{}", write_position, play_position);

        // record new material into separate rec_buffer
        state.sync_point = (state.subdivision + 0) * state.division_len;

        for (index, (left_in, right_in)) in stereo_in.enumerate() {

            // select the buffer we are recording into
            let mut record_buffer = &mut state.buffer;
            // let play_buffer = &state.buffers[state.read_idx];

            let in_sync = write_position + index == state.sync_point;

            match state.state {
                LooperState::Recording => {
                    // Push the new samples into the loop buffers.
                    if index == 0 {
                        // info!("record[{}]: {} / {}", state.loop_length, left_in.as_f32(), right_in.as_f32());
                    }
                    if (state.write_position + index) < record_buffer.buffer.len() {
                        if let Some((left_old, right_old)) = record_buffer.buffer.get_mut(write_position + index) {
                            *left_old = left_in.as_f32();
                            *right_old = right_in.as_f32();
                        }
                    } else {
                        record_buffer.buffer.push_back((left_in.as_f32(), right_in.as_f32()));
                    }

                    state.loop_length += 1;
                }
                LooperState::Inserting => {
                    record_buffer.buffer.insert(write_position + index, (left_in.as_f32(), right_in.as_f32()));
                    state.loop_length += 1;
                }
                LooperState::Overdubbing => {
                    if let Some((left_old, right_old)) = record_buffer.buffer.get_mut(write_position + index) {
                        if index == 0 {
                            // info!("overdub[{}]: {} / {}", write_position, left_old, right_old);
                        }
                        *left_old = (*left_old * WET_MULT) * state.feedback + left_in.as_f32();
                        *right_old = (*right_old * WET_MULT) * state.feedback + right_in.as_f32();
                    }
                }
                LooperState::Replacing => {
                    // check to see if we need to sync up before starting to replace
                    match (state.sync_waiting, state.in_sync, in_sync) {
                        // we are waiting for a sync, not yet replacing and sync point has arrived
                        (true, false, true) => {  // start replacing
                            info!("sync point reached: {} - replacing", state.sync_point);
                            state.in_sync = true;
                            state.sync_waiting = false;
                        }
                        // we are waiting for a sync, not yet replacing and not on sync point
                        (true, false, false) => {} // keep waiting
                        // waiting for a sync, already replacing and sync point has arrived
                        (true, true, true) => { // stop replacing
                            let next_state = state.next_state.unwrap();
                            info!("sync point reached: {} - stopping replace -> {}", state.sync_point, next_state);
                            state.in_sync = false;
                            state.sync_waiting = false;

                            state.state = next_state;
                            state.next_state = None;
                        }
                        // we are waiting for a sync, already replacing and not on sync point
                        (true, true, false) => {
                            // info!("idx: {} / {}, sync_point: {}", write_position + index, play_position + index, state.sync_point);
                        } // keep replacing
                        // not waiting for a sync point
                        (false, _, _) => {} // do nothing
                    }

                    if state.in_sync {
                        if let Some((left_old, right_old)) = record_buffer.buffer.get_mut(write_position + index) {
                            if index == 0 {
                                // info!("replace[{}]: {} / {}", write_position, left_in.as_f32(), right_in.as_f32());
                            }
                            *left_old = left_in.as_f32();
                            *right_old = right_in.as_f32();
                        }
                    }
                }
                LooperState::Multiplying => {
                    // copy from the currently playing buffer to the (new) recording buffer

//                    if let Some((left_old, right_old)) = play_buffer.buffer.get(play_position + index) {
//                        let left_new = (*left_old * WET_MULT) * state.feedback + left_in.as_f32();
//                        let right_new = (*right_old * WET_MULT) * state.feedback + right_in.as_f32();
//                        record_buffer.buffer.push_back((left_new.as_f32(), right_new.as_f32()));
//                    }
                }

                _ => {}
            }
        }


        // play back from the play buffer
        for (index, (left_out, right_out)) in stereo_out.enumerate() {
            let play_buffer = &state.buffer;

            let mut left_processed: f32 = 0.0;
            let mut right_processed: f32 = 0.0;

            match state.state {
                LooperState::Muted | LooperState::Stopped => {
                    left_processed = 0. as f32;
                    right_processed = 0. as f32;
                }
                _ => {
                    if let Some((left_old, right_old)) = play_buffer.buffer.get(play_position + index) {
                        const WET_MULT: f32 = 0.98;
                        if index == 0 {
                            // info!("play[{}]: {}/{}", play_position + index, left_old, right_old );
                        }

                        left_processed = *left_old * WET_MULT;
                        right_processed = *right_old * WET_MULT;
                    }
                }
            }

            *left_out = left_processed.as_();
            *right_out = right_processed.as_();
        }

        match state.state {
            // update the write position
            LooperState::Recording | LooperState::Inserting | LooperState::Overdubbing | LooperState::Replacing => {
                state.write_position += stereo_in_len;
                state.write_position = state.write_position % state.loop_length;
            }
            _ => {}
        }

        if state.state != LooperState::Stopped {
            state.play_position += stereo_out_len;
            state.play_position = if state.loop_length > 0 {
                state.play_position % state.loop_length
            } else { 0 };
            state.division_len = (state.cycle_len / state.division) as usize;
            state.subdivision = if state.division_len > 0 {
                (state.play_position / state.division_len) as usize
            } else { 0 }
        }

        // info!("loop_len / buf_len / write_pos / play_pos {} / {} / {} / {}", state.loop_length, rec_buffer_len, state.write_position, state.play_position);


        use vst::event::Event;

        for e in events.events() {
            const A3_PITCH: u8 = 69;  // Record
            const G3_PITCH: u8 = 67;  // Stop
            const F3_PITCH: u8 = 65;  // Play
            const E3_PITCH: u8 = 64;  // Overdub
            const D3_PITCH: u8 = 62;  // Replace
            const C3_PITCH: u8 = 60;  // Mute
            const B2_PITCH: u8 = 59;  // Insert
            const A2_PITCH: u8 = 57;  // Multiply
            match e {
                Event::Midi(mut ev) => {
                    let midi_event = status(ev.data[0]);
                    info!("Midi Event: {:?}", midi_event);

                    match midi_event {
                        Status::NoteOn => {
                            let pitch = ev.data[1];
                            info!("Pitch: {}", pitch);
                            state.prev_state = state.state;
                            match pitch {
                                A3_PITCH => {
                                    state.state = looper_cycle(state, Commands::Record);
                                }
                                G3_PITCH => {
                                    state.state = looper_cycle(state, Commands::Stop);
                                }
                                F3_PITCH => {
                                    state.state = looper_cycle(state, Commands::Play);
                                }
                                E3_PITCH => {
                                    state.state = looper_cycle(state, Commands::Overdub);
                                }
                                D3_PITCH => {
                                    state.state = looper_cycle(state, Commands::ReplaceStart);
                                }
                                C3_PITCH => {
                                    state.state = looper_cycle(state, Commands::Mute);
                                }
                                B2_PITCH => {
                                    state.state = looper_cycle(state, Commands::InsertStart);
                                }
                                A2_PITCH => {
                                    state.state = looper_cycle(state, Commands::MultiplyStart);
                                }
                                _ => {}
                            }
                        }
                        Status::NoteOff => {
                            let pitch = ev.data[1];
                            info!("Pitch: {}", pitch);
                            match pitch {
                                D3_PITCH => {
                                    state.state = looper_cycle(state, Commands::ReplaceStop);
                                }
                                B2_PITCH => {
                                    state.state = looper_cycle(state, Commands::InsertStop);
                                }
                                A2_PITCH => {
                                    state.state = looper_cycle(state, Commands::MultiplyStop);
                                }
                                _ => {}
                            }
                        }
                        _ => {}
                    }
                    info!("new state: {}", state.state);
                    // info!("Size Buffer {}: {}", state.loop_index, buffer.buffer.len());
                    match self.window {
                        Some(window) => {
                            match state.state {
                                LooperState::Recording | LooperState::Overdubbing | LooperState::Replacing => {
                                    window.state_label.set_text_color(Color::red());
                                    window.counter.set_text_color(Color::red());
                                }
                                LooperState::Playing => {
                                    window.state_label.set_text_color(Color::green());
                                    window.counter.set_text_color(Color::green());
                                }
                                LooperState::Stopped | LooperState::Muted => {
                                    window.state_label.set_text_color(Color::black());
                                    window.counter.set_text_color(Color::black());
                                }
                                _ => {}
                            }
                            window.state_label.set_text(&state.state.to_string());
                        }
                        _ => {}
                    };
                    state.events.push(ev);
                }
                _ => ()
            }
        }

        // state.send_buffer.send_events(events, &mut self.state.host)

        match self.window {
            Some(window) => {
                let sample_rate = *state.sample_rate.read().unwrap().deref();
                let seconds = state.play_position as f64 / sample_rate;
                let seconds = format!("{:.*}", 2, seconds);
                if seconds != state.seconds {
                    window.counter.set_text(&seconds.to_string());
                    state.seconds = seconds;
                }

                let cycles = format!("{} | {}", state.cycles, state.total_cycles);
                let division = format!("{}", state.division);
                let subdiv = format!("{} - {} - {}:{}", state.subdivision, state.sync_point, state.in_sync, state.sync_waiting);
                window.cycle_label.set_text(&cycles.to_string());
                window.division_label.set_text(&division.to_string());
                window.subdiv_label.set_text(&subdiv.to_string());
            }
            _ => {}
        };
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
    TuneRequest = 0xF6,
    // F4 anf 5 are reserved and unused
    SysExEnd = 0xF7,
    TimingClock = 0xF8,
    Start = 0xFA,
    Continue = 0xFB,
    Stop = 0xFC,
    ActiveSensing = 0xFE,
    // FD also res/unused
    SystemReset = 0xFF,
}

extern crate tinyui;


mod ui;

use std::os::raw::c_void;


const WINDOW_WIDTH: u32 = 480;
const WINDOW_HEIGHT: u32 = 160;

impl Editor for ELPlugin {
    fn size(&self) -> (i32, i32) { (WINDOW_WIDTH as i32, WINDOW_HEIGHT as i32) }

    fn position(&self) -> (i32, i32) { (0, 0) }

    fn close(&mut self) { self.window = None; }

    fn idle(&mut self) {}

    fn is_open(&mut self) -> bool { self.window.is_some() }

    fn open(&mut self, parent: *mut c_void) {
        info!("open {}", parent as usize);
        self.window = Some(ui::PluginWindow::new(Window::new_with_parent(parent).unwrap()));
    }
}

impl ELPlugin {
    fn clear_buffers() -> Vec<RecordingBuffer> {
        const NUM_BUFFERS: usize = 4;
        let mut buffers = Vec::new();
        for _i in 0..NUM_BUFFERS {
            let buffer = RecordingBuffer::new();

            buffers.push(buffer);
        }
        buffers
    }
}
