// TODO: Try to mark as many as possible to pub(crate)
use nb::block;

use crate::system::hal::adc::{Adc, Enabled};
use crate::system::hal::gpio;
use crate::system::hal::pac::{ADC1, ADC2};

use super::probe::Detector as ProbeDetector;

pub struct CVs {
    pub cv: [CV; 4],
    pins: Pins,
}

#[derive(Default)]
pub struct CV {
    pub value: Option<f32>,
    probe: ProbeDetector,
}

pub struct Pins {
    pub cv_1: CV1Pin,
    pub cv_2: CV2Pin,
    pub cv_3: CV3Pin,
    pub cv_4: CV4Pin,
}

pub type CV1Pin = gpio::gpioc::PC1<gpio::Analog>;
pub type CV2Pin = gpio::gpiob::PB1<gpio::Analog>;
pub type CV3Pin = gpio::gpioc::PC0<gpio::Analog>;
pub type CV4Pin = gpio::gpioc::PC4<gpio::Analog>;

impl CVs {
    pub fn new(pins: Pins) -> Self {
        Self {
            cv: [CV::default(), CV::default(), CV::default(), CV::default()],
            pins,
        }
    }

    pub fn sample(&mut self, adc_1: &mut Adc<ADC1, Enabled>, adc_2: &mut Adc<ADC2, Enabled>) {
        adc_1.start_conversion(&mut self.pins.cv_1);
        adc_2.start_conversion(&mut self.pins.cv_2);
        let sample_1: u32 = block!(adc_1.read_sample()).unwrap();
        let sample_2: u32 = block!(adc_2.read_sample()).unwrap();

        adc_1.start_conversion(&mut self.pins.cv_3);
        adc_2.start_conversion(&mut self.pins.cv_4);
        let sample_3: u32 = block!(adc_1.read_sample()).unwrap();
        let sample_4: u32 = block!(adc_2.read_sample()).unwrap();

        self.cv[0].set(sample_1, adc_1.slope());
        self.cv[1].set(sample_2, adc_2.slope());
        self.cv[2].set(sample_3, adc_1.slope());
        self.cv[3].set(sample_4, adc_2.slope());
    }
}

impl CV {
    fn set(&mut self, sample: u32, slope: u32) {
        let value = transpose_adc(sample, slope);
        self.probe.write(value > 2.0);
        self.value = if self.probe.detected() {
            None
        } else {
            Some(value)
        };
    }
}

#[allow(clippy::cast_precision_loss)]
fn transpose_adc(sample: u32, slope: u32) -> f32 {
    let min = -5.0;
    let span = 10.0;
    let phase = (slope as f32 - sample as f32) / slope as f32;
    min + phase * span
}
