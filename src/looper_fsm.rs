use std::fmt;

// State machine of the looper
// based on https://www.youtube.com/watch?v=b8slVcXtg3k
#[derive(Clone, Copy)]
pub enum LooperState {
    Stopped,
    Recording,
    Clearing,
    Overdubbing,
    Playing,
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
            LooperState::Clearing => "Clearing",
        };
        write!(f, "{}", printable)
    }
}

pub enum Commands {
    Stop,
    Play,
    Record,
    Overdub,
}



pub fn looper_cycle(state: LooperState, command: Commands) -> LooperState {
    use self::LooperState::*;
    use self::Commands::*;

    match(state, command) {
        (Stopped, Play) => Playing,
        (Stopped, Record) => Clearing,
        (Stopped, Overdub) => Overdubbing,
        (Stopped, _) => Stopped,

        // We need to take care that the buffers are cleared before recording again
        (Clearing, Record) => Recording,
        (Clearing, _) => Recording,

        (Playing, Stop) => Stopped,
        (Playing, Record) => Clearing,
        (Playing, Overdub) => Overdubbing,
        (Playing, _) => Playing,

        (Recording, Stop) => Stopped,
        (Recording, Record) => Clearing,
        (Recording, Overdub) => Overdubbing,
        (Recording, Play) => Playing,
        (Recording, _) => Recording,

        (Overdubbing, Play) => Playing,
        (Overdubbing, Stop) => Stopped,
        (Overdubbing, Record) => Clearing,
        (Overdubbing, Overdub) => Playing,
        (Overdubbing, _) => Overdubbing,
    }
}
