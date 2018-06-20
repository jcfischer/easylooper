use std::fmt;
use ELState;
use recording_buffer::RecordingBuffer;
use ELPlugin;

// State machine of the looper
// based on https://www.youtube.com/watch?v=b8slVcXtg3k
#[derive(Clone, Copy, PartialEq)]
pub enum LooperState {
    Stopped,
    Recording,
    Clearing,
    Overdubbing,
    Multiplying,
    Playing,
    Replacing,
    Inserting,
    Muted,
}


impl Default for LooperState {
    fn default() -> LooperState { LooperState::Stopped }
}

impl fmt::Display for LooperState {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let printable = match *self {
            LooperState::Stopped => "Stopped",
            LooperState::Recording => "Recording",
            LooperState::Playing => "Playing",
            LooperState::Overdubbing => "Overdubbing",
            LooperState::Multiplying => "Multiplying",
            LooperState::Clearing => "Clearing",
            LooperState::Replacing => "Replacing",
            LooperState::Inserting => "Inserting",
            LooperState::Muted => "Muted",
        };
        write!(f, "{}", printable)
    }
}

pub enum Commands {
    Stop,
    Play,
    Record,
    Overdub,
    Multiply,  // normal, cycle based
    MultiplyStart,  // unrounded multiply
    MultiplyStop,
    ReplaceStart,
    ReplaceStop,
    InsertStart,
    InsertStop,
    Mute,
}



pub fn looper_cycle(plugin_state: &mut ELState, command: Commands) -> LooperState {
    use LooperState::*;
    use Commands::*;

    let state = plugin_state.state;
    let prev_state = plugin_state.prev_state;

    match(state, command) {
        (Stopped, Play) => Playing,
        (Stopped, Record) => start_clearing(plugin_state),
        (Stopped, Overdub) => overdub_start(plugin_state),
        (Stopped, _) => Stopped,

        // We need to take care that the buffers are cleared before recording again
        (Clearing, Record) => Recording,
        (Clearing, _) => Recording,

        (Playing, Stop) => Stopped,
        (Playing, Record) => start_clearing(plugin_state),
        (Playing, Overdub) => overdub_start(plugin_state),
        (Playing, ReplaceStart) => Replacing,
        (Playing, InsertStart) => Inserting,
        (Playing, MultiplyStart) => multiply_start(plugin_state),
        (Playing, Mute) => Muted,
        (Playing, _) => Playing,

        (Recording, Stop) => recording_stop(plugin_state, Stopped),
        (Recording, Record) => recording_stop(plugin_state, Playing),
        (Recording, Overdub) => overdub_start(plugin_state),
        (Recording, MultiplyStart) => multiply_start(plugin_state),
        (Recording, Play) => recording_stop( plugin_state, Playing),
        (Recording, _) => Recording,

        (Overdubbing, Play) => Playing,
        (Overdubbing, Stop) => Stopped,
        (Overdubbing, Record) => start_clearing(plugin_state),
        (Overdubbing, Overdub) => Playing,
        (Overdubbing, MultiplyStart) => multiply_start(plugin_state),
        (Overdubbing, _) => Overdubbing,

        (Multiplying, MultiplyStop) => multiply_end(plugin_state),
        (Multiplying, _) => Multiplying,

        (Replacing, ReplaceStop) => prev_state,
        (Replacing, _) => Replacing,

        (Inserting, InsertStop) => prev_state,
        (Inserting, _) => Inserting,

        (Muted, Mute) => Playing,
        (Muted, _) => Muted,
        (_, Mute) => Muted,
    }
}

fn start_clearing(plugin_state: &mut ELState) -> LooperState {
    info!("clearing");
    plugin_state.buffers = ELPlugin::clear_buffers();
    plugin_state.play_position = 0;
    plugin_state.loop_length = 0;
    plugin_state.cycle_len = 0;
    LooperState::Recording

}
fn recording_stop(plugin_state: &mut ELState, next_state: LooperState) -> LooperState {
    plugin_state.cycle_len = plugin_state.loop_length;
    info!("Stopping -> {}: cycle_len: {}", next_state, plugin_state.cycle_len);
    next_state
}

fn overdub_start(_plugin_state: &mut ELState) -> LooperState {
    // plugin_state.loop_index += 1;
    LooperState::Overdubbing
}

fn multiply_start(plugin_state: &mut ELState) -> LooperState {
    let new_buffer = RecordingBuffer::with_size(plugin_state.cycle_len);
    plugin_state.buffers.push(new_buffer);
    LooperState::Multiplying
}

fn multiply_end(plugin_state: &mut ELState) -> LooperState {
    plugin_state.prev_state
}
