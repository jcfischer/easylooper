use std::collections::VecDeque;

// handling of audio
type SamplePair = (f32, f32);

pub struct RecordingBuffer {
    pub buffer: VecDeque<SamplePair>
}

impl RecordingBuffer {
    pub fn new() -> RecordingBuffer {
        RecordingBuffer::with_size(102400)
    }

    // create a new (empty) buffer with *size* samples
    pub fn with_size(size: usize) -> RecordingBuffer {
        let mut buffer = VecDeque::with_capacity(size);
        for _ in 0..size {
            buffer.push_back( (0.0, 0.0));
        }
        RecordingBuffer { buffer: buffer }
    }

    // return the length of the recording buffer
    pub fn length(&self) -> usize {
        self.buffer.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_new_buffer() {
        let b = RecordingBuffer::new();

        assert_eq!(b.buffer.len(), 102400);
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