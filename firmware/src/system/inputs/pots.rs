use nb::block;

use crate::system::hal::adc::{Adc, Enabled};
use crate::system::hal::gpio;
use crate::system::hal::pac::{ADC1, ADC2};

#[derive(defmt::Format)]
pub struct Pots {
    pub pre_amp: f32,
    pub drive: f32,
    pub bias: f32,
    pub dry_wet: f32,
    pub wow_flut: f32,
    pub speed: f32,
    pub tone: f32,
    pub head: [Head; 4],
    pub pins: Pins,
}

#[derive(Default, defmt::Format)]
pub struct Head {
    pub position: f32,
    pub volume: f32,
    pub feedback: f32,
    pub pan: f32,
}

#[derive(defmt::Format)]
pub struct Pins {
    pub multiplexer_1: Multiplexer1Pin,
    pub multiplexer_2: Multiplexer2Pin,
    pub multiplexer_3: Multiplexer3Pin,
}

pub type Multiplexer1Pin = gpio::gpioa::PA7<gpio::Analog>;
pub type Multiplexer2Pin = gpio::gpioa::PA6<gpio::Analog>;
pub type Multiplexer3Pin = gpio::gpioa::PA2<gpio::Analog>;

impl Pots {
    pub(crate) fn new(pins: Pins) -> Self {
        Self {
            pre_amp: 0.0,
            drive: 0.0,
            bias: 0.0,
            dry_wet: 0.0,
            wow_flut: 0.0,
            speed: 0.0,
            tone: 0.0,
            head: [
                Head::default(),
                Head::default(),
                Head::default(),
                Head::default(),
            ],
            pins,
        }
    }

    pub fn sample(
        &mut self,
        cycle: u8,
        adc_1: &mut Adc<ADC1, Enabled>,
        adc_2: &mut Adc<ADC2, Enabled>,
    ) {
        let (a, b, c) = self.read_values(adc_1, adc_2);
        match cycle {
            0 => {
                self.speed = a;
                self.head[3].volume = b;
                self.head[1].volume = c;
            }
            1 => {
                self.head[3].feedback = b;
                self.head[1].feedback = c;
            }
            2 => {
                self.tone = a;
                self.head[3].pan = b;
                self.head[1].pan = c;
            }
            3 => {
                self.head[0].volume = a;
                self.head[2].position = b;
                self.head[2].pan = c;
            }
            4 => {
                self.head[0].feedback = a;
                self.wow_flut = b;
                self.head[2].feedback = c;
            }
            5 => {
                self.pre_amp = a;
                self.bias = b;
                self.head[1].position = c;
            }
            6 => {
                self.head[0].pan = a;
                self.dry_wet = b;
                self.head[2].volume = c;
            }
            7 => {
                self.head[0].position = a;
                self.head[3].position = b;
                self.drive = c;
            }
            _ => unreachable!(),
        }
    }

    fn read_values(
        &mut self,
        adc_1: &mut Adc<ADC1, Enabled>,
        adc_2: &mut Adc<ADC2, Enabled>,
    ) -> (f32, f32, f32) {
        adc_1.start_conversion(&mut self.pins.multiplexer_1);
        adc_2.start_conversion(&mut self.pins.multiplexer_2);
        let sample_1: u32 = block!(adc_1.read_sample()).unwrap_or_default();
        let sample_2: u32 = block!(adc_2.read_sample()).unwrap_or_default();

        adc_1.start_conversion(&mut self.pins.multiplexer_3);
        let sample_3: u32 = block!(adc_1.read_sample()).unwrap_or_default();

        (
            transpose_adc(sample_1, adc_1.slope()),
            transpose_adc(sample_2, adc_2.slope()),
            transpose_adc(sample_3, adc_1.slope()),
        )
    }
}

fn transpose_adc(sample: u32, slope: u32) -> f32 {
    let float = (slope as f32 - sample as f32) / slope as f32;
    // NOTE: Pots are connected to -5 to +5 V ADC while they span only
    // from 0 to +5 V.
    let half_range = float * 2.0 - 1.0;
    // NOTE: The pot never reaches all the way up to max voltage.
    let scaled_up = half_range * (1.0 / 0.988);
    scaled_up.clamp(0.0, 1.0)
}
