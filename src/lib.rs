#[macro_use]
extern crate vst;
extern crate easyvst;
extern crate time;
extern crate log;
extern crate log_panics;
extern crate simplelog;


use simplelog::*;

use vst::plugin::{Info, Plugin, Category};
use vst::buffer::AudioBuffer;

use easyvst::*;

use std::path::{Path, PathBuf};

use std::mem;
use std::collections::VecDeque;
use std::f64::consts::PI;

fn delay(index: usize, mut size: f32) -> isize {
    const SIZE_OFFSET: f32 = 0.06;
    const SIZE_MULT: f32 = 1_000.0;

    size += SIZE_OFFSET;

    const SPREAD: f32 = 0.3;

    let base = size * SIZE_MULT;
    let mult = (index as f32 * SPREAD) + 1.0;
    let offset = if index > 2 { base * SPREAD / 2.0 } else { 0.0 };

    (base * mult + offset) as isize

}


type SamplePair = (f32, f32);

struct EchoLooper {
    buffers: Vec<VecDeque<SamplePair>>,
    dry_wet: f32,
    size: f32,

}

impl Default for EchoLooper {
    fn default() -> EchoLooper {
        EchoLooper::new(0.12, 0.66)
    }
}

impl EchoLooper {
    fn new(size: f32, dry_wet: f32) -> EchoLooper {
        const NUM_DELAYS: usize = 4;

        let mut buffers = Vec::new();

        // generate Delay buffers
        for i in 0..NUM_DELAYS {
            let samples = delay(i, size);
            let mut buffer = VecDeque::with_capacity(samples as usize);

            for _ in 0..samples {
                buffer.push_back((0.0, 0.0));
            }

            buffers.push(buffer);
        }

        EchoLooper {
            buffers: buffers,
            dry_wet: dry_wet,
            size: size,
        }

    }

    fn resize(&mut self, n: f32) {
        let old_size = mem::replace(&mut self.size, n);

        for (i, buffer) in self.buffers.iter_mut().enumerate() {
            let old_delay = delay(i, old_size);
            let new_delay = delay(i, n);

            let diff = new_delay - old_delay;

            if diff > 0 {
                for _ in 0..diff {
                    buffer.push_back((0.0, 0.0));
                }
            } else if diff < 0 {
                for _ in 0..-diff {
                    let _ = buffer.pop_front();
                }
            }
        }
    }
}
impl Plugin for EchoLooper {
    fn get_info(&self) -> Info {
        Info {
            name: "EchoLooper".to_string(),
            vendor: "SunMachines".to_string(),

            inputs: 2,
            outputs: 2,
            category: Category::Effect,
            version: 0002,
            parameters: 2,

            // random
            unique_id: 1359,
            ..Default::default()
        }
    }

    fn get_parameter(&self, index: i32) -> f32 {
        match index {
            0 => self.size,
            1 => self.dry_wet,
            _ => 0.0,
        }
    }

    fn get_parameter_text(&self, index: i32) -> String {
        match index {
            0 => "Length",
            1 => "Dry/Wet",
            _ => "",
        }.to_string()
    }

    fn set_parameter(&mut self, index: i32, val: f32) {
        match index {
            0 => self.resize(val),
            1 => self.dry_wet = val,
            _ => (),
        }
    }


    fn process(&mut self, buffer: &mut AudioBuffer<f32>) {

        let (inputs, mut outputs) = buffer.split();

        if inputs.len() < 2 || outputs.len() < 2 {
            return;
        }

        // iterate over inputs as (&f32, &f32)
        let (l, r) = inputs.split_at(1);
        let stereo_in = l[0].iter().zip(r[0].iter());

        // iterate over outputs
        let (mut l, mut r) = outputs.split_at_mut(1);
        let stereo_out = l[0].iter_mut().zip(r[0].iter_mut());


        for ((left_in, right_in), (left_out, right_out)) in stereo_in.zip(stereo_out) {
            for buffer in &mut self.buffers {
                buffer.push_back((*left_in, *right_in));
            }

            let mut left_processed = 0.0;
            let mut right_processed = 0.0;

            let time_s = time::precise_time_ns() as f64 / 1_000_000_000.0;

            for (n, buffer) in self.buffers.iter_mut().enumerate() {
                if let Some((left_old, right_old)) = buffer.pop_front() {
                    const LFO_FREQ: f64 = 0.5;
                    const WET_MULT: f32 = 0.66;

                    let offset = 0.25 * (n % 4) as f64;

                    let lfo = ((time_s * LFO_FREQ + offset) * PI * 2.0).sin() as f32;

                    let wet = self.dry_wet * WET_MULT;
                    let mono = (left_old + right_old) / 2.0;

                    left_processed  += mono * wet * lfo;
                    right_processed += -mono * wet * lfo;
                }
            }

            *left_out = *left_in + left_processed;
            *right_out = * right_in + right_processed;
        }

    }

}

plugin_main!(EchoLooper);

