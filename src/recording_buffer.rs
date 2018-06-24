use std::collections::VecDeque;

// handling of audio
pub type SamplePair = (f32, f32);

pub struct RecordingBuffer {
    pub buffer: Vec<SamplePair>,
    start_position: usize,
}

const INITIAL_SIZE: usize = 102400;

impl RecordingBuffer {
    pub fn new() -> RecordingBuffer {
        RecordingBuffer::with_size(INITIAL_SIZE)
    }

    // create a new (empty) buffer with *size* samples
    pub fn with_size(size: usize) -> RecordingBuffer {
        let mut buffer = Vec::with_capacity(size);
        for _ in 0..size {
            buffer.push( (0.0, 0.0));
        }
        RecordingBuffer { buffer, start_position: 0 }
    }

    // return the length of the recording buffer
    pub fn length(&self) -> usize {
        self.buffer.len()
    }

    pub fn insert_empty(&self, size: usize) -> bool {
        true
    }
}

impl Default for RecordingBuffer {
    fn default() -> RecordingBuffer {
        RecordingBuffer::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_new_buffer() {
        let b = RecordingBuffer::new();

        assert_eq!(b.buffer.len(), 1024);
    }

    #[test]
    fn build_sized_buffer() {
        let b = RecordingBuffer::with_size(1024);

        assert_eq!(b.buffer.len(), 1024);
    }

    #[test]
    fn test_len() {
        let b = RecordingBuffer::with_size(1024);
        assert_eq!(b.length(), 1024);
    }
}