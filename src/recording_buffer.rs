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
            buffer.push((0.0, 0.0));
        }
        RecordingBuffer { buffer, start_position: 0 }
    }

    /// return the length of the recording buffer
    ///
    /// ```
    /// let b = recording_buffer::RecordingBuffer::with_size(16);
    /// assert_eq!(b.length(), 16);
    /// ```
    pub fn length(&self) -> usize {
        self.buffer.len()
    }

    /// insert an empty slice of size at position
    ///
    /// ```
    /// let b = recording_buffer::RecordingBuffer::with_size(16);
    /// for _ in 0..16 {
    ///   b.push(1.0, 1.0);
    /// }
    /// b.insert_empty(2, 4);
    /// assert_eq!(b.length(), 20);
    /// assert_eq!(b[2], (1.0, 1.0));
    /// ```
    pub fn insert_empty(&mut self, at: usize, size: usize)  {
        let mut ins_buf: Vec<SamplePair> = Vec::with_capacity(size);
        for _ in 0..size {
            ins_buf.push((0.0, 0.0));
        }
        println!("len before: {}", self.buffer.len());

        self.buffer.splice(at..at, ins_buf.iter().cloned());
        println!("len after: {}", self.buffer.len());
    }

    /// Get the SamplePair at index idx
    pub fn get(&self, idx: usize) -> Option<&SamplePair>{
        self.buffer.get(idx)
    }

    /// Add a sample pair to the end of the buffer
    pub fn push(&mut self, samples: SamplePair) {
        self.buffer.push(samples);
    }

    /// Overwrite a sample pair with a new one
    pub fn overwrite(&mut self, idx: usize, sample : SamplePair) {
        self.buffer[idx] = sample;
    }

    /// Overdub a value at a specific index
    pub fn overdub(&mut self, idx: usize, sample: SamplePair, feedback: f32) {
        const WET_MULT: f32 = 0.98;
        let (left_in, right_in) = sample;
        if let Some((left, right)) = self.buffer.get_mut(idx) {
            *left = (*left * WET_MULT) * feedback + left_in;
            *right = (*right * WET_MULT) * feedback + right_in;

        }
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


    #[test]
    fn test_insert_empty() {
        let mut b = RecordingBuffer::with_size(16);
        // add another 16 samples
        for _ in 0..16 {
            b.push((1.0, 1.0));
        }
        b.insert_empty(20, 4);
        assert_eq!(b.length(), 36);
        assert_eq!(Some(&(1.0, 1.)), b.get(19));
        assert_eq!(Some(&(0., 0.)), b.get(20));
    }

    #[test]
    fn test_overwrite() {
        let mut b = RecordingBuffer::with_size(16);
        b.overwrite(1, (0.5, 0.5));
        assert_eq!(Some(&(0.5, 0.5)), b.get(1));
    }

    #[test]
    fn test_overdub() {
        let mut b = RecordingBuffer::with_size(16);
        b.overwrite(1, (0.5, 0.5));
        b.overdub(1, (0.2, -0.2), 1.0);
        assert_eq!(Some(&(0.69, 0.29000002)), b.get(1));
    }
}