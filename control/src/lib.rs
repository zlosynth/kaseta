//! Components of user inteface, passing user input to DSP and reactions back.
//!
//! It is mainly targetted to run in a firmware with multiple loops running in
//! different frequencies, passing messages from one to another. However, parts
//! of it may be useful in software as well.

#![no_std]
#![allow(clippy::items_after_statements)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_lossless)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::cast_possible_wrap)]
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::explicit_iter_loop)]

#[cfg(test)]
#[macro_use]
extern crate approx;

mod action;
mod calibration;
mod display;
mod input;
mod led;
mod mapping;
mod quantization;
mod reconcile;
mod taper;
mod trigger;

use heapless::Vec;
use kaseta_dsp::processor::{
    Attributes as DSPAttributes, AttributesHead as DSPAttributesHead, Reaction as DSPReaction,
};

use crate::action::{ControlAction, Queue};
use crate::calibration::Calibration;
use crate::display::{ConfigurationScreen, Display, Screen};
use crate::input::snapshot::Snapshot as InputSnapshot;
use crate::input::store::Store as Inputs;
use crate::led::Led;
use crate::mapping::{AttributeIdentifier, Mapping};
use crate::trigger::Trigger;

/// The main store of peripheral abstraction and module configuration.
///
/// This struct is the central piece of the control module. It takes
/// `InputSnapshot` on its inputs, passes it to peripheral abstractions,
/// interprets the current input into module configuration and manages
/// the whole state machine of that.
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Cache {
    inputs: Inputs,
    // TODO: cache
    // TODO: results
    state: State,
    queue: Queue,
    mapping: Mapping,
    calibrations: Calibrations,
    options: Options,
    configuration: Configuration,
    tapped_tempo: TappedTempo,
    attributes: Attributes,
    outputs: Outputs,
}

/// The current state of the control state machine.
///
/// The module can be in one of multiple states. The behavior of input and
/// output peripherals will differ based on the current state.
#[derive(Debug, PartialEq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub(crate) enum State {
    Calibrating(usize, CalibrationPhase),
    Mapping(usize),
    Configuring(Configuration),
    Normal,
}

#[derive(Debug, PartialEq, Clone, Copy)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub(crate) enum CalibrationPhase {
    Octave1,
    Octave2(f32),
}

/// TODO Docs
// TODO: One per head, offset and amplification
type Calibrations = [Calibration; 4];

/// Easy to access modifications of the default module behavior.
///
/// Options are configured using the DIP switch on the front of the module.
/// They are meant to provide a quick access to some common extended features
/// such as quantization, rewind, etc.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Default)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
struct Options {
    quantize_8: bool,
    quantize_6: bool,
    short_delay_range: bool,
    rewind: bool,
}

/// Tweaking of the default module configuration.
///
/// This is mean to allow tweaking of some more niche configuration of the
/// module. Unlike with `Options`, the parameters here may be continuous
/// (float) or offer enumeration of variants. An examle of a configuration
/// may be tweaking of head's rewind speed.
#[derive(Debug, Default, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub(crate) struct Configuration {
    rewind_speed: [(f32, f32); 4],
}

/// TODO: doc
type TappedTempo = Option<f32>;

/// Interpreted attributes for the DSP.
///
/// This structure can be directly translated to DSP configuration, used
/// to build the audio processor model.
#[derive(Debug, Default)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
struct Attributes {
    pre_amp: f32,
    drive: f32,
    saturation: f32,
    bias: f32,
    dry_wet: f32,
    wow: f32,
    flutter: f32,
    speed: f32,
    tone: f32,
    head: [AttributesHead; 4],
}

#[derive(Debug, Default)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
struct AttributesHead {
    position: f32,
    volume: f32,
    feedback: f32,
    pan: f32,
}

/// TODO docs
#[derive(Debug, Default)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
struct Outputs {
    impulse_trigger: Trigger,
    impulse_led: Led,
    display: Display,
}

/// Desired state of output peripherals with the exception of audio.
///
/// This structure transfers request to the module, asking to lit LEDs or
/// set control output.
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct DesiredOutput {
    pub display: [bool; 8],
    pub impulse_led: bool,
    pub impulse_trigger: bool,
}

/// TODO: Docs
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Save {
    mapping: Mapping,
    calibrations: Calibrations,
    configuration: Configuration,
    tapped_tempo: TappedTempo,
}

// NOTE: Inputs and outputs will be passed through queues
#[allow(clippy::new_without_default)]
impl Cache {
    #[must_use]
    pub fn new() -> Self {
        Self {
            inputs: Inputs::default(),
            state: State::default(),
            queue: Queue::default(),
            mapping: Mapping::default(),
            calibrations: Calibrations::default(),
            options: Options::default(),
            configuration: Configuration::default(),
            tapped_tempo: TappedTempo::default(),
            attributes: Attributes::default(),
            outputs: Outputs::default(),
        }
    }

    pub fn warm_up(&mut self, snapshot: InputSnapshot) {
        self.inputs.update(snapshot);
    }

    pub fn apply_input_snapshot(
        &mut self,
        snapshot: InputSnapshot,
    ) -> (DSPAttributes, Option<Save>) {
        self.inputs.update(snapshot);
        let save = self.converge_internal_state();
        self.reconcile_attributes();
        let dsp_attributes = self.build_dsp_attributes();
        (dsp_attributes, save)
    }

    fn converge_internal_state(&mut self) -> Option<Save> {
        let mut needs_save = false;

        let (plugged_controls, unplugged_controls) = self.plugged_and_unplugged_controls();

        for i in &unplugged_controls {
            self.queue.remove_control(*i);
            self.mapping[*i] = AttributeIdentifier::None;
        }

        for i in &plugged_controls {
            self.queue.remove_control(*i);
            if self.inputs.button.pressed {
                self.queue.push(ControlAction::Calibrate(*i));
            }
            if self.mapping[*i].is_none() {
                self.queue.push(ControlAction::Map(*i));
            }
        }

        match self.state {
            State::Normal => {
                if let Some(detected_tempo) = self.inputs.button.detected_tempo() {
                    needs_save = true;
                    self.tapped_tempo = Some(detected_tempo as f32 / 1000.0);
                }

                if self.inputs.button.held > 5_000 {
                    self.state = State::Configuring(self.configuration);
                    self.outputs.display.prioritized[1] = Some(Screen::configuration());
                } else if let Some(action) = self.queue.pop() {
                    match action {
                        ControlAction::Calibrate(i) => {
                            self.state = State::Calibrating(i, CalibrationPhase::Octave1);
                            self.outputs.display.prioritized[1] = Some(Screen::calibration_1(i));
                        }
                        ControlAction::Map(i) => {
                            self.state = State::Mapping(i);
                            self.outputs.display.prioritized[1] = Some(Screen::mapping(i));
                        }
                    }
                } else {
                    self.outputs.display.prioritized[1] = None;
                }
            }
            State::Configuring(draft) => {
                if self.inputs.button.clicked {
                    needs_save = true;
                    self.configuration = draft;
                    self.state = State::Normal;
                } else {
                    let active_configuration_screen = self.active_configuration_screen();
                    if active_configuration_screen.is_some() {
                        self.outputs.display.prioritized[1] = active_configuration_screen;
                    }
                    self.state = State::Configuring(self.snapshot_configuration_from_pots(draft));
                }
            }
            State::Calibrating(i, phase) => {
                if !self.inputs.control[i].is_plugged {
                    self.outputs.display.prioritized[0] = Some(Screen::failure());
                    self.state = State::Normal;
                } else if self.inputs.button.clicked {
                    match phase {
                        CalibrationPhase::Octave1 => {
                            let octave_1 = self.inputs.control[i].value();
                            self.state = State::Calibrating(i, CalibrationPhase::Octave2(octave_1));
                            self.outputs.display.prioritized[1] = Some(Screen::calibration_2(i));
                        }
                        CalibrationPhase::Octave2(octave_1) => {
                            let octave_2 = self.inputs.control[i].value();
                            if let Some(calibration) = Calibration::try_new(octave_1, octave_2) {
                                needs_save = true;
                                self.calibrations[i] = calibration;
                            } else {
                                self.outputs.display.prioritized[0] = Some(Screen::failure());
                            }
                            self.state = State::Normal;
                        }
                    }
                }
            }
            State::Mapping(i) => {
                let destination = self.active_attribute();
                if !destination.is_none() && !self.mapping.contains(&destination) {
                    needs_save = true;
                    self.mapping[i] = destination;
                    self.state = State::Normal;
                }
            }
        };

        if needs_save {
            Some(self.save())
        } else {
            None
        }
    }

    fn active_configuration_screen(&mut self) -> Option<Screen> {
        let mut active_configuration_screen = None;
        for head in self.inputs.head.iter() {
            if head.volume.active() || head.feedback.active() {
                let volume = head.volume.value();
                let rewind_index = if volume < 0.25 {
                    0
                } else if volume < 0.5 {
                    1
                } else if volume < 0.75 {
                    2
                } else {
                    3
                };
                let feedback = head.feedback.value();
                let fast_forward_index = if feedback < 0.25 {
                    0
                } else if feedback < 0.5 {
                    1
                } else if feedback < 0.75 {
                    2
                } else {
                    3
                };
                active_configuration_screen = Some(Screen::Configuration(
                    ConfigurationScreen::Rewind((rewind_index, fast_forward_index)),
                ));
                break;
            }
        }
        active_configuration_screen
    }

    // TODO: Display should be handled elsewhere
    fn reconcile_attributes(&mut self) {
        self.reconcile_pre_amp();
        self.reconcile_hysteresis();
        self.reconcile_wow_flutter();
        self.reconcile_tone();
        self.reconcile_speed();
        self.reconcile_heads();

        self.outputs.display.prioritized[2] = Some(self.screen_for_heads());
    }

    fn screen_for_heads(&self) -> Screen {
        let ordered_heads = self.heads_ordered_by_position();

        Screen::Heads(
            [
                ordered_heads[0].volume > 0.05,
                ordered_heads[1].volume > 0.05,
                ordered_heads[2].volume > 0.05,
                ordered_heads[3].volume > 0.05,
            ],
            [
                ordered_heads[0].feedback > 0.05,
                ordered_heads[1].feedback > 0.05,
                ordered_heads[2].feedback > 0.05,
                ordered_heads[3].feedback > 0.05,
            ],
        )
    }

    fn heads_ordered_by_position(&self) -> [&AttributesHead; 4] {
        let mut ordered_heads = [
            &self.attributes.head[0],
            &self.attributes.head[1],
            &self.attributes.head[2],
            &self.attributes.head[3],
        ];
        for i in 0..ordered_heads.len() {
            for j in 0..ordered_heads.len() - 1 - i {
                if ordered_heads[j].position > ordered_heads[j + 1].position {
                    ordered_heads.swap(j, j + 1);
                }
            }
        }
        ordered_heads
    }

    fn plugged_and_unplugged_controls(&self) -> (Vec<usize, 4>, Vec<usize, 4>) {
        let mut plugged = Vec::new();
        let mut unplugged = Vec::new();
        for (i, cv) in self.inputs.control.iter().enumerate() {
            if cv.was_plugged {
                // NOTE: This is safe since the number of controls is equal to the
                // size of the Vec.
                let _ = plugged.push(i);
            }
            if cv.was_unplugged {
                // NOTE: Ditto.
                let _ = unplugged.push(i);
            }
        }
        (plugged, unplugged)
    }

    fn active_attribute(&self) -> AttributeIdentifier {
        for (pot, identifier) in [
            (&self.inputs.pre_amp, AttributeIdentifier::PreAmp),
            (&self.inputs.drive, AttributeIdentifier::Drive),
            (&self.inputs.bias, AttributeIdentifier::Bias),
            (&self.inputs.dry_wet, AttributeIdentifier::DryWet),
            (&self.inputs.wow_flut, AttributeIdentifier::WowFlut),
            (&self.inputs.speed, AttributeIdentifier::Speed),
            (&self.inputs.tone, AttributeIdentifier::Tone),
        ] {
            if pot.active() {
                return identifier;
            }
        }

        for (i, head) in self.inputs.head.iter().enumerate() {
            for (pot, identifier) in [
                (&head.position, AttributeIdentifier::Position(i)),
                (&head.volume, AttributeIdentifier::Volume(i)),
                (&head.feedback, AttributeIdentifier::Feedback(i)),
                (&head.pan, AttributeIdentifier::Pan(i)),
            ] {
                if pot.active() {
                    return identifier;
                }
            }
        }

        AttributeIdentifier::None
    }

    fn control_for_attribute(&mut self, attribute: AttributeIdentifier) -> Option<f32> {
        let i = self.mapping.iter().position(|a| *a == attribute);
        if let Some(i) = i {
            let control = &self.inputs.control[i];
            if control.is_plugged {
                let calibration = self.calibrations[i];
                Some(calibration.apply(control.value()))
            } else {
                None
            }
        } else {
            None
        }
    }

    fn build_dsp_attributes(&mut self) -> DSPAttributes {
        DSPAttributes {
            pre_amp: self.attributes.pre_amp,
            drive: self.attributes.drive,
            saturation: self.attributes.saturation,
            bias: self.attributes.bias,
            dry_wet: self.attributes.dry_wet,
            wow: self.attributes.wow,
            flutter: self.attributes.flutter,
            speed: self.attributes.speed,
            tone: self.attributes.tone,
            head: [
                DSPAttributesHead {
                    position: self.attributes.head[0].position,
                    volume: self.attributes.head[0].volume,
                    feedback: self.attributes.head[0].feedback,
                    pan: self.attributes.head[0].pan,
                },
                DSPAttributesHead {
                    position: self.attributes.head[1].position,
                    volume: self.attributes.head[1].volume,
                    feedback: self.attributes.head[1].feedback,
                    pan: self.attributes.head[1].pan,
                },
                DSPAttributesHead {
                    position: self.attributes.head[2].position,
                    volume: self.attributes.head[2].volume,
                    feedback: self.attributes.head[2].feedback,
                    pan: self.attributes.head[2].pan,
                },
                DSPAttributesHead {
                    position: self.attributes.head[3].position,
                    volume: self.attributes.head[3].volume,
                    feedback: self.attributes.head[3].feedback,
                    pan: self.attributes.head[3].pan,
                },
            ],
            rewind: self.options.rewind,
            rewind_speed: self.configuration.rewind_speed,
        }
    }

    pub fn apply_dsp_reaction(&mut self, dsp_reaction: DSPReaction) {
        if dsp_reaction.delay_impulse {
            self.outputs.impulse_trigger.trigger();
            self.outputs.impulse_led.trigger();
        }
    }

    pub fn tick(&mut self) -> DesiredOutput {
        let output = DesiredOutput {
            display: self.outputs.display.active_screen().leds(),
            impulse_trigger: self.outputs.impulse_trigger.triggered(),
            impulse_led: self.outputs.impulse_led.triggered(),
        };

        self.outputs.impulse_trigger.tick();
        self.outputs.impulse_led.tick();
        self.outputs.display.tick();

        output
    }

    fn save(&self) -> Save {
        Save {
            mapping: self.mapping,
            calibrations: self.calibrations,
            configuration: self.configuration,
            tapped_tempo: self.tapped_tempo,
        }
    }

    fn snapshot_configuration_from_pots(&self, mut configuration: Configuration) -> Configuration {
        // TODO: Unite this and the code figuring out the current screen
        for (i, rewind_speed) in configuration.rewind_speed.iter_mut().enumerate() {
            let fast_forward = if self.inputs.head[i].volume.active() {
                let volume = self.inputs.head[i].volume.value();
                if volume < 0.25 {
                    1.0
                } else if volume < 0.5 {
                    2.0
                } else if volume < 0.75 {
                    4.0
                } else {
                    8.0
                }
            } else {
                rewind_speed.1
            };

            let rewind = if self.inputs.head[i].feedback.active() {
                let feedback = self.inputs.head[i].feedback.value();
                if feedback < 0.25 {
                    0.75
                } else if feedback < 0.5 {
                    0.5
                } else if feedback < 0.75 {
                    -1.0
                } else {
                    -4.0
                }
            } else {
                rewind_speed.0
            };

            *rewind_speed = (rewind, fast_forward);
        }

        configuration
    }
}

impl From<Save> for Cache {
    fn from(save: Save) -> Self {
        let mut cache = Self::new();
        cache.mapping = save.mapping;
        cache.calibrations = save.calibrations;
        cache.configuration = save.configuration;
        cache.tapped_tempo = save.tapped_tempo;
        cache
    }
}

impl Default for State {
    fn default() -> Self {
        Self::Normal
    }
}

#[cfg(test)]
mod cache_tests {
    #![allow(clippy::field_reassign_with_default)]
    #![allow(clippy::manual_assert)]
    #![allow(clippy::zero_prefixed_literal)]

    use super::*;

    #[test]
    fn it_should_be_possible_to_initialize_cache() {
        let _cache = Cache::new();
    }

    fn assert_animation(cache: &mut Cache, transitions: &[u32]) {
        fn u32_into_digits(mut x: u32) -> [u32; 4] {
            assert!(x < 10000);

            let digit_1 = x % 10;
            x /= 10;
            let digit_2 = x % 10;
            x /= 10;
            let digit_3 = x % 10;
            x /= 10;
            let digit_4 = x % 10;

            [digit_4, digit_3, digit_2, digit_1]
        }

        fn digits_into_bools(digits: [u32; 4]) -> [bool; 8] {
            let mut bools = [false; 8];

            for (i, digit) in digits.iter().enumerate() {
                match digit {
                    9 => bools[i] = true,
                    6 => bools[i + 4] = true,
                    8 => {
                        bools[i] = true;
                        bools[i + 4] = true;
                    }
                    0 => (),
                    _ => panic!("Led code must consist of 6, 8, 9 and 0"),
                }
            }

            bools
        }

        fn transition_into_bools(x: u32) -> [bool; 8] {
            let digits = u32_into_digits(x);
            digits_into_bools(digits)
        }

        let mut old_display = None;
        'transition: for transition in transitions {
            for _ in 0..10000 {
                let display = cache.tick().display;
                let bools = transition_into_bools(*transition);
                if display == bools {
                    old_display = Some(display);
                    continue 'transition;
                } else if old_display.is_some() && old_display.unwrap() != display {
                    panic!("Reached unexpected transition {:?}", display);
                }
            }
            panic!("Transition was not reached within the given timeout");
        }
    }

    #[test]
    fn when_dsp_returns_impulse_it_should_set_output_trigger_for_multiple_cycles() {
        let mut cache = Cache::new();
        let mut dsp_reaction = DSPReaction::default();

        dsp_reaction.delay_impulse = true;
        cache.apply_dsp_reaction(dsp_reaction);

        dsp_reaction.delay_impulse = false;
        cache.apply_dsp_reaction(dsp_reaction);

        let output = cache.tick();
        assert!(output.impulse_trigger);
        let output = cache.tick();
        assert!(output.impulse_trigger);

        for _ in 0..100 {
            let output = cache.tick();
            if !output.impulse_trigger {
                return;
            }
        }

        panic!("Trigger was not set down within given timeout");
    }

    #[test]
    fn when_dsp_returns_impulse_it_should_lit_impulse_led_for_multiple_cycles() {
        let mut cache = Cache::new();
        let mut dsp_reaction = DSPReaction::default();

        dsp_reaction.delay_impulse = true;
        cache.apply_dsp_reaction(dsp_reaction);

        dsp_reaction.delay_impulse = false;
        cache.apply_dsp_reaction(dsp_reaction);

        let output = cache.tick();
        assert!(output.impulse_led);
        let output = cache.tick();
        assert!(output.impulse_led);

        for _ in 0..100 {
            let output = cache.tick();
            if !output.impulse_led {
                return;
            }
        }

        panic!("Trigger was not set down within given timeout");
    }

    #[test]
    fn given_save_it_recovers_previously_set_tapped_tempo() {
        let mut cache = Cache::new();
        let mut input = InputSnapshot::default();

        input.button = true;
        cache.apply_input_snapshot(input);

        input.button = false;
        for _ in 0..1999 {
            cache.apply_input_snapshot(input);
        }

        input.button = true;
        cache.apply_input_snapshot(input);

        input.button = false;
        for _ in 0..1999 {
            cache.apply_input_snapshot(input);
        }

        input.button = true;
        cache.apply_input_snapshot(input);

        input.button = false;
        for _ in 0..1999 {
            cache.apply_input_snapshot(input);
        }

        input.button = true;
        let (_, save) = cache.apply_input_snapshot(input);
        assert_relative_eq!(cache.attributes.speed, 2.0);

        let mut cache = Cache::from(save.unwrap());
        let mut input = InputSnapshot::default();
        input.speed = 0.5;
        for _ in 0..10 {
            cache.warm_up(input);
        }
        cache.apply_input_snapshot(input);
        assert_relative_eq!(cache.attributes.speed, 2.0);
    }

    #[test]
    fn given_save_it_recovers_previously_set_mapping() {
        let mut cache = Cache::new();
        let mut input = InputSnapshot::default();

        input.control[1] = None;
        cache.apply_input_snapshot(input);

        input.control[1] = Some(1.0);
        cache.apply_input_snapshot(input);

        input.drive = 0.1;
        cache.apply_input_snapshot(input);

        let save = cache.save();
        let mut cache = Cache::from(save);
        cache.apply_input_snapshot(input);

        assert_eq!(cache.mapping[1], AttributeIdentifier::Drive);
        assert_eq!(cache.state, State::Normal);
    }

    #[test]
    fn given_save_if_new_control_was_plugged_since_it_gets_to_the_queque() {
        let cache = Cache::new();

        let save = cache.save();

        let mut input = InputSnapshot::default();
        input.control[1] = Some(1.0);

        let mut cache = Cache::from(save);
        cache.apply_input_snapshot(input);

        assert_eq!(cache.state, State::Mapping(1));
    }

    #[test]
    fn given_save_it_recovers_previously_set_calibration_and_mapping() {
        fn click_button(cache: &mut Cache, mut input: InputSnapshot) {
            input.button = true;
            cache.apply_input_snapshot(input);
            input.button = false;
            cache.apply_input_snapshot(input);
        }

        let mut cache = Cache::new();
        let mut input = InputSnapshot::default();

        input.control[1] = None;
        cache.apply_input_snapshot(input);

        input.button = true;
        cache.apply_input_snapshot(input);

        input.control[1] = Some(1.0);
        cache.apply_input_snapshot(input);

        input.button = false;
        cache.apply_input_snapshot(input);

        input.control[1] = Some(1.2);
        for _ in 0..10 {
            cache.apply_input_snapshot(input);
        }
        click_button(&mut cache, input);

        input.control[1] = Some(2.3);
        for _ in 0..10 {
            cache.apply_input_snapshot(input);
        }
        click_button(&mut cache, input);

        input.drive = 0.1;
        cache.apply_input_snapshot(input);
        assert_eq!(cache.state, State::Normal);

        let calibration = cache.calibrations[1];
        let mapping = cache.mapping[1];

        let save = cache.save();
        let mut cache = Cache::from(save);
        cache.apply_input_snapshot(input);

        assert_eq!(cache.calibrations[1], calibration);
        assert_eq!(cache.mapping[1], mapping);
        assert_eq!(cache.state, State::Normal);
    }

    #[test]
    fn given_save_after_calibration_was_done_but_mapping_not_it_recovers_calibration_and_continues_mapping(
    ) {
        // TODO: Single helper for all cache tests
        fn click_button(cache: &mut Cache, mut input: InputSnapshot) {
            input.button = true;
            cache.apply_input_snapshot(input);
            input.button = false;
            cache.apply_input_snapshot(input);
        }

        let mut cache = Cache::new();
        let mut input = InputSnapshot::default();

        input.control[1] = None;
        cache.apply_input_snapshot(input);

        input.button = true;
        cache.apply_input_snapshot(input);

        input.control[1] = Some(1.0);
        cache.apply_input_snapshot(input);

        input.button = false;
        cache.apply_input_snapshot(input);

        input.control[1] = Some(1.2);
        for _ in 0..10 {
            cache.apply_input_snapshot(input);
        }
        click_button(&mut cache, input);

        input.control[1] = Some(2.3);
        for _ in 0..10 {
            cache.apply_input_snapshot(input);
        }
        click_button(&mut cache, input);

        input.drive = 0.1;
        cache.apply_input_snapshot(input);

        assert_eq!(cache.state, State::Normal);

        let calibration = cache.calibrations[1];
        let mapping = cache.mapping[1];

        let save = cache.save();
        let mut cache = Cache::from(save);
        cache.apply_input_snapshot(input);

        assert_eq!(cache.calibrations[1], calibration);
        assert_eq!(cache.mapping[1], mapping);
        assert_eq!(cache.state, State::Normal);
    }

    #[test]
    fn given_save_it_recovers_previously_set_configuration() {
        let mut cache = Cache::new();
        let mut input = InputSnapshot::default();

        // TODO: Define global helper for this
        input.button = true;
        for _ in 0..6 * 1000 {
            cache.apply_input_snapshot(input);
        }
        input.button = false;
        cache.apply_input_snapshot(input);

        for head in input.head.iter_mut() {
            head.volume = 1.0;
            head.feedback = 1.0;
        }
        for _ in 0..4 {
            cache.apply_input_snapshot(input);
        }

        input.button = true;
        let (_, save) = cache.apply_input_snapshot(input);

        input.button = false;

        let mut cache = Cache::from(save.unwrap());
        cache.apply_input_snapshot(input);

        assert_eq!(cache.configuration.rewind_speed[0], (-4.0, 8.0));
        assert_eq!(cache.configuration.rewind_speed[1], (-4.0, 8.0));
        assert_eq!(cache.configuration.rewind_speed[2], (-4.0, 8.0));
        assert_eq!(cache.configuration.rewind_speed[3], (-4.0, 8.0));
    }

    #[cfg(test)]
    mod given_normal_mode {
        use super::*;

        fn init_cache() -> Cache {
            Cache::new()
        }

        #[test]
        fn when_clicking_button_in_equal_intervals_it_sets_speed() {
            let mut cache = init_cache();
            let mut input = InputSnapshot::default();

            input.button = true;
            cache.apply_input_snapshot(input);

            input.button = false;
            for _ in 0..1999 {
                cache.apply_input_snapshot(input);
            }

            input.button = true;
            cache.apply_input_snapshot(input);

            input.button = false;
            for _ in 0..1999 {
                cache.apply_input_snapshot(input);
            }

            input.button = true;
            cache.apply_input_snapshot(input);

            input.button = false;
            for _ in 0..1999 {
                cache.apply_input_snapshot(input);
            }

            input.button = true;
            let (_, save) = cache.apply_input_snapshot(input);

            assert_relative_eq!(cache.attributes.speed, 2.0);
            assert_relative_eq!(save.unwrap().tapped_tempo.unwrap(), 2.0);
        }

        #[test]
        fn when_tempo_is_tapped_in_it_is_overwritten_only_after_speed_pot_turning() {
            let mut cache = init_cache();
            let mut input = InputSnapshot::default();

            input.button = true;
            cache.apply_input_snapshot(input);

            input.button = false;
            for _ in 0..1999 {
                cache.apply_input_snapshot(input);
            }

            input.button = true;
            cache.apply_input_snapshot(input);

            input.button = false;
            for _ in 0..1999 {
                cache.apply_input_snapshot(input);
            }

            input.button = true;
            cache.apply_input_snapshot(input);

            input.button = false;
            for _ in 0..1999 {
                cache.apply_input_snapshot(input);
            }

            input.button = true;
            cache.apply_input_snapshot(input);

            assert_relative_eq!(cache.attributes.speed, 2.0);

            input.button = false;
            cache.apply_input_snapshot(input);

            input.tone = 1.0;
            cache.apply_input_snapshot(input);
            assert_relative_eq!(cache.attributes.speed, 2.0);

            input.speed = 0.5;
            cache.apply_input_snapshot(input);
            assert!(cache.attributes.speed > 20.0);
        }

        #[test]
        fn when_control_is_plugged_then_state_changes_to_mapping() {
            let mut cache = init_cache();
            let mut input = InputSnapshot::default();

            input.control[1] = None;
            cache.apply_input_snapshot(input);

            input.control[1] = Some(1.0);
            let (_, save) = cache.apply_input_snapshot(input);

            assert!(save.is_none());
            assert!(matches!(cache.state, State::Mapping(1)));
            assert_animation(
                &mut cache,
                &[9600, 0800, 0690, 0609, 9600, 0800, 0690, 0609],
            );
        }

        #[test]
        fn when_control_is_plugged_while_holding_button_then_state_changes_to_calibration() {
            let mut cache = init_cache();
            let mut input = InputSnapshot::default();

            input.button = false;
            input.control[1] = None;
            cache.apply_input_snapshot(input);

            input.button = true;
            input.control[1] = Some(1.0);
            let (_, save) = cache.apply_input_snapshot(input);

            assert!(save.is_none());
            assert!(matches!(
                cache.state,
                State::Calibrating(1, CalibrationPhase::Octave1)
            ));
            assert_animation(&mut cache, &[9800, 0600, 9800, 0600]);
        }

        #[test]
        fn when_multiple_controls_are_plugged_then_are_all_added_to_queue() {
            let mut cache = init_cache();

            let mut input = InputSnapshot::default();
            input.control[1] = None;
            input.control[2] = None;
            cache.apply_input_snapshot(input);

            let mut input = InputSnapshot::default();
            input.control[1] = Some(1.0);
            input.control[2] = Some(1.0);
            cache.apply_input_snapshot(input);

            assert!(matches!(cache.state, State::Mapping(1)));
            assert_eq!(cache.queue.len(), 1);
        }

        #[test]
        fn when_control_is_unplugged_it_is_removed_from_queue() {
            let mut cache = init_cache();
            let mut input = InputSnapshot::default();

            input.control[1] = None;
            input.control[2] = None;
            input.control[3] = None;
            cache.apply_input_snapshot(input);

            input.control[1] = Some(1.0);
            input.control[2] = Some(1.0);
            input.control[3] = Some(1.0);
            cache.apply_input_snapshot(input);

            assert!(matches!(cache.state, State::Mapping(1)));
            assert_eq!(cache.queue.len(), 2);

            input.control[2] = None;
            cache.apply_input_snapshot(input);

            assert!(matches!(cache.state, State::Mapping(1)));
            assert_eq!(cache.queue.len(), 1);
        }

        #[test]
        fn when_button_is_held_for_5_seconds_it_enters_configuration_mode() {
            let mut cache = init_cache();
            let mut input = InputSnapshot::default();

            input.button = true;
            for _ in 0..6 * 1000 {
                cache.apply_input_snapshot(input);
            }
            input.button = false;
            cache.apply_input_snapshot(input);

            assert!(matches!(cache.state, State::Configuring(_)));
        }

        #[test]
        fn when_mapped_control_in_unplugged_it_is_unmapped() {
            let mut cache = init_cache();
            let mut input = InputSnapshot::default();

            input.control[0] = None;
            cache.apply_input_snapshot(input);

            input.control[0] = Some(1.0);
            cache.apply_input_snapshot(input);

            input.drive = 0.4;
            cache.apply_input_snapshot(input);

            assert_eq!(cache.mapping[0], AttributeIdentifier::Drive);

            input.control[0] = None;
            cache.apply_input_snapshot(input);

            assert_eq!(cache.mapping[0], AttributeIdentifier::None);
        }

        #[test]
        fn it_displays_enabled_volume_and_feedback_based_on_head_order() {
            let mut cache = init_cache();
            let mut input = InputSnapshot::default();

            input.head[0].position = 1.0;
            input.head[0].volume = 1.0;
            input.head[0].feedback = 1.0;

            input.head[1].position = 0.4;
            input.head[1].volume = 0.0;
            input.head[1].feedback = 0.2;

            input.head[2].position = 0.8;
            input.head[2].volume = 0.7;
            input.head[2].feedback = 0.0;

            input.head[3].position = 0.7;
            input.head[3].volume = 0.0;
            input.head[3].feedback = 0.0;

            for _ in 0..4 {
                cache.apply_input_snapshot(input);
            }

            assert_animation(&mut cache, &[6098]);
        }
    }

    #[cfg(test)]
    mod given_configuration_mode {
        use super::*;

        fn init_cache() -> (Cache, InputSnapshot) {
            let mut cache = Cache::new();
            let input = InputSnapshot::default();

            hold_button(&mut cache, input);

            (cache, input)
        }

        fn hold_button(cache: &mut Cache, mut input: InputSnapshot) {
            input.button = true;
            for _ in 0..6 * 1000 {
                cache.apply_input_snapshot(input);
            }
            input.button = false;
            cache.apply_input_snapshot(input);
        }

        fn click_button(cache: &mut Cache, mut input: InputSnapshot) {
            input.button = true;
            cache.apply_input_snapshot(input);
            input.button = false;
            cache.apply_input_snapshot(input);
        }

        fn apply_input_snapshot(cache: &mut Cache, input: InputSnapshot) {
            // NOTE: Applying it 4 times makes sure that pot smoothening is
            // not affecting following asserts.
            for _ in 0..4 {
                cache.apply_input_snapshot(input);
            }
        }

        #[test]
        fn when_clicks_button_it_saves_configuration_and_enters_normal_mode() {
            let (mut cache, mut input) = init_cache();

            input.head[0].volume = 0.05;
            input.head[0].feedback = 0.05;
            input.head[1].volume = 0.35;
            input.head[1].feedback = 0.35;
            input.head[2].volume = 0.7;
            input.head[2].feedback = 0.7;
            input.head[3].volume = 1.0;
            input.head[3].feedback = 1.0;
            apply_input_snapshot(&mut cache, input);

            click_button(&mut cache, input);

            assert_eq!(cache.configuration.rewind_speed[0], (0.75, 1.0));
            assert_eq!(cache.configuration.rewind_speed[1], (0.5, 2.0));
            assert_eq!(cache.configuration.rewind_speed[2], (-1.0, 4.0));
            assert_eq!(cache.configuration.rewind_speed[3], (-4.0, 8.0));
        }

        #[test]
        fn when_turns_volume_and_feedback_the_rewind_speed_is_visualized_on_display() {
            let (mut cache, mut input) = init_cache();

            input.head[0].volume = 0.05;
            input.head[0].feedback = 0.05;
            apply_input_snapshot(&mut cache, input);
            assert_animation(&mut cache, &[9006]);

            input.head[1].volume = 0.35;
            input.head[1].feedback = 0.35;
            apply_input_snapshot(&mut cache, input);
            assert_animation(&mut cache, &[9966]);

            input.head[2].volume = 0.7;
            input.head[2].feedback = 0.7;
            apply_input_snapshot(&mut cache, input);
            assert_animation(&mut cache, &[9886]);

            input.head[3].volume = 1.0;
            input.head[3].feedback = 1.0;
            apply_input_snapshot(&mut cache, input);
            assert_animation(&mut cache, &[8888]);
        }

        #[test]
        fn when_does_not_change_attribute_it_keeps_the_previously_set_value() {
            let (mut cache, mut input) = init_cache();

            input.head[2].volume = 0.7;
            input.head[2].feedback = 0.7;
            apply_input_snapshot(&mut cache, input);
            click_button(&mut cache, input);

            assert_eq!(cache.configuration.rewind_speed[2], (-1.0, 4.0));

            input.head[2].volume = 0.0;
            input.head[2].feedback = 0.0;
            apply_input_snapshot(&mut cache, input);

            hold_button(&mut cache, input);

            input.head[1].volume = 0.35;
            input.head[1].feedback = 0.35;
            apply_input_snapshot(&mut cache, input);
            click_button(&mut cache, input);

            assert_eq!(cache.configuration.rewind_speed[2], (-1.0, 4.0));
            assert_eq!(cache.configuration.rewind_speed[1], (0.5, 2.0));
        }

        #[test]
        fn when_no_attribute_was_changed_yet_it_shows_animation() {
            let (mut cache, _) = init_cache();
            assert_animation(&mut cache, &[9696, 6969]);
        }
    }

    #[cfg(test)]
    mod given_mapping_mode {
        use super::*;

        fn init_cache(pending: usize) -> (Cache, InputSnapshot) {
            let mut cache = Cache::new();
            let mut input = InputSnapshot::default();

            for i in 0..pending {
                input.control[i] = None;
            }
            cache.apply_input_snapshot(input);

            for i in 0..pending {
                input.control[i] = Some(1.0);
            }
            cache.apply_input_snapshot(input);

            (cache, input)
        }

        fn apply_input_snapshot(cache: &mut Cache, input: InputSnapshot) {
            // NOTE: Applying it 4 times makes sure that pot smoothening is
            // not affecting following asserts.
            for _ in 0..4 {
                cache.apply_input_snapshot(input);
            }
        }

        #[test]
        fn when_pot_is_active_it_gets_mapped_to_the_current_control_and_saved() {
            let (mut cache, mut input) = init_cache(4);

            input.drive = 0.1;
            let (_, save) = cache.apply_input_snapshot(input);
            assert_eq!(save.unwrap().mapping[0], AttributeIdentifier::Drive);
            assert_eq!(cache.mapping[0], AttributeIdentifier::Drive);
            apply_input_snapshot(&mut cache, input); // Let the pot converge

            input.speed = 0.1;
            let (_, save) = cache.apply_input_snapshot(input);
            assert_eq!(save.unwrap().mapping[0], AttributeIdentifier::Drive);
            assert_eq!(save.unwrap().mapping[1], AttributeIdentifier::Speed);
            assert_eq!(cache.mapping[0], AttributeIdentifier::Drive);
            assert_eq!(cache.mapping[1], AttributeIdentifier::Speed);
            apply_input_snapshot(&mut cache, input); // Let the pot converge

            input.dry_wet = 0.1;
            let (_, save) = cache.apply_input_snapshot(input);
            assert_eq!(save.unwrap().mapping[0], AttributeIdentifier::Drive);
            assert_eq!(save.unwrap().mapping[1], AttributeIdentifier::Speed);
            assert_eq!(save.unwrap().mapping[2], AttributeIdentifier::DryWet);
            assert_eq!(cache.mapping[0], AttributeIdentifier::Drive);
            assert_eq!(cache.mapping[1], AttributeIdentifier::Speed);
            assert_eq!(cache.mapping[2], AttributeIdentifier::DryWet);
            apply_input_snapshot(&mut cache, input); // Let the pot converge

            input.bias = 0.1;
            let (_, save) = cache.apply_input_snapshot(input);
            assert_eq!(save.unwrap().mapping[0], AttributeIdentifier::Drive);
            assert_eq!(save.unwrap().mapping[1], AttributeIdentifier::Speed);
            assert_eq!(save.unwrap().mapping[2], AttributeIdentifier::DryWet);
            assert_eq!(save.unwrap().mapping[3], AttributeIdentifier::Bias);
            assert_eq!(cache.mapping[0], AttributeIdentifier::Drive);
            assert_eq!(cache.mapping[1], AttributeIdentifier::Speed);
            assert_eq!(cache.mapping[2], AttributeIdentifier::DryWet);
            assert_eq!(cache.mapping[3], AttributeIdentifier::Bias);
        }

        #[test]
        fn when_last_pending_control_is_processed_then_state_changes_to_normal() {
            let (mut cache, mut input) = init_cache(1);
            assert_eq!(cache.state, State::Mapping(0));

            input.drive = 0.1;
            apply_input_snapshot(&mut cache, input);
            assert_eq!(cache.state, State::Normal);
            assert_animation(&mut cache, &[0000]);
        }

        #[test]
        fn when_second_last_pending_control_is_processed_it_moves_to_last() {
            let (mut cache, mut input) = init_cache(2);

            input.drive = 0.1;
            apply_input_snapshot(&mut cache, input);
            assert_eq!(cache.state, State::Mapping(1));
        }

        #[test]
        fn when_multiple_controls_are_plugged_then_they_are_all_added_to_queue() {
            let (mut cache, mut input) = init_cache(2);
            assert_eq!(cache.state, State::Mapping(0));
            assert_eq!(cache.queue.len(), 1);
            assert!(cache.queue.contains(&ControlAction::Map(1)));

            input.control[2] = Some(1.0);
            input.control[3] = Some(1.0);
            cache.apply_input_snapshot(input);
            assert_eq!(cache.state, State::Mapping(0));
            assert_eq!(cache.queue.len(), 3);
            assert!(cache.queue.contains(&ControlAction::Map(1)));
            assert!(cache.queue.contains(&ControlAction::Map(2)));
            assert!(cache.queue.contains(&ControlAction::Map(3)));
        }

        #[test]
        fn when_control_is_unplugged_it_is_removed_from_queue() {
            let (mut cache, mut input) = init_cache(3);
            assert_eq!(cache.state, State::Mapping(0));
            assert_eq!(cache.queue.len(), 2);
            assert!(cache.queue.contains(&ControlAction::Map(1)));
            assert!(cache.queue.contains(&ControlAction::Map(2)));

            input.control[1] = None;
            cache.apply_input_snapshot(input);
            assert_eq!(cache.state, State::Mapping(0));
            assert_eq!(cache.queue.len(), 1);
            assert!(cache.queue.contains(&ControlAction::Map(2)));
        }

        #[test]
        fn when_mapped_control_in_unplugged_it_is_unmapped() {
            let (mut cache, mut input) = init_cache(2);

            input.drive = 0.4;
            apply_input_snapshot(&mut cache, input);
            assert_eq!(cache.mapping[0], AttributeIdentifier::Drive);
            assert_eq!(cache.state, State::Mapping(1));

            input.control[0] = None;
            apply_input_snapshot(&mut cache, input);
            assert_eq!(cache.mapping[0], AttributeIdentifier::None);
            assert_eq!(cache.state, State::Mapping(1));
        }

        #[test]
        fn when_active_pot_is_assigned_it_is_not_reassigned() {
            let (mut cache, mut input) = init_cache(2);

            input.drive = 0.1;
            apply_input_snapshot(&mut cache, input);
            assert_eq!(cache.mapping[0], AttributeIdentifier::Drive);
            assert_eq!(cache.mapping[1], AttributeIdentifier::None);

            input.drive = 0.2;
            apply_input_snapshot(&mut cache, input);
            assert_eq!(cache.mapping[0], AttributeIdentifier::Drive);
            assert_eq!(cache.mapping[1], AttributeIdentifier::None);
            assert_eq!(cache.state, State::Mapping(1));

            assert_animation(&mut cache, &[9600, 0800, 0690, 0609]);
        }
    }

    #[cfg(test)]
    mod given_calibrating_mode {
        use super::*;

        fn init_cache(pending: usize) -> (Cache, InputSnapshot) {
            let mut cache = Cache::new();
            let mut input = InputSnapshot::default();

            for i in 0..pending {
                input.control[i] = None;
            }
            cache.apply_input_snapshot(input);

            input.button = true;
            cache.apply_input_snapshot(input);

            for i in 0..pending {
                input.control[i] = Some(1.0);
            }
            cache.apply_input_snapshot(input);

            input.button = false;
            cache.apply_input_snapshot(input);

            (cache, input)
        }

        fn apply_input_snapshot(cache: &mut Cache, input: InputSnapshot) {
            // NOTE: Applying it 4 times makes sure that control smoothening is
            // not affecting following asserts.
            for _ in 0..4 {
                cache.apply_input_snapshot(input);
            }
        }

        fn click_button(cache: &mut Cache, mut input: InputSnapshot) -> Option<Save> {
            input.button = true;
            let (_, save_1) = cache.apply_input_snapshot(input);
            input.button = false;
            let (_, save_2) = cache.apply_input_snapshot(input);
            save_1.or(save_2)
        }

        #[test]
        fn when_correct_values_are_given_it_successfully_converges_turns_to_mapping_and_saves() {
            let (mut cache, mut input) = init_cache(1);
            assert_eq!(
                cache.state,
                State::Calibrating(0, CalibrationPhase::Octave1)
            );
            assert_animation(&mut cache, &[8900, 6000, 8900, 6000]);

            input.control[0] = Some(1.3);
            apply_input_snapshot(&mut cache, input);
            click_button(&mut cache, input);
            assert_eq!(
                cache.state,
                State::Calibrating(0, CalibrationPhase::Octave2(1.3))
            );
            assert_animation(&mut cache, &[6099, 6000, 6099, 6000]);

            input.control[0] = Some(2.4);
            cache.apply_input_snapshot(input);
            let save = click_button(&mut cache, input);
            let saved_calibration = save.unwrap().calibrations[0];
            assert_relative_ne!(saved_calibration.offset, Calibration::default().offset);
            assert_relative_ne!(saved_calibration.scaling, Calibration::default().scaling);
            let cache_calibration = cache.calibrations[0];
            assert_relative_ne!(cache_calibration.offset, Calibration::default().offset);
            assert_relative_ne!(cache_calibration.scaling, Calibration::default().scaling);
            assert_eq!(cache.state, State::Mapping(0));
            assert_animation(&mut cache, &[8000, 6900, 6090, 6009]);
        }

        #[test]
        fn when_incorrect_values_are_given_it_it_cancels_calibration_turns_to_mapping_and_does_not_save(
        ) {
            let (mut cache, mut input) = init_cache(1);
            assert_eq!(
                cache.state,
                State::Calibrating(0, CalibrationPhase::Octave1)
            );
            assert_animation(&mut cache, &[8900, 6000, 8900, 6000]);

            input.control[0] = Some(1.3);
            apply_input_snapshot(&mut cache, input);
            click_button(&mut cache, input);
            assert_eq!(
                cache.state,
                State::Calibrating(0, CalibrationPhase::Octave2(1.3))
            );
            assert_animation(&mut cache, &[6099, 6000, 6099, 6000]);

            input.control[0] = Some(1.4);
            cache.apply_input_snapshot(input);
            let save = click_button(&mut cache, input);
            assert!(save.is_none());
            let cache_calibration = cache.calibrations[0];
            assert_relative_eq!(cache_calibration.offset, Calibration::default().offset);
            assert_relative_eq!(cache_calibration.scaling, Calibration::default().scaling);
            assert_eq!(cache.state, State::Mapping(0));
            assert_animation(&mut cache, &[8888, 0000, 8888, 0000, 6090, 6009]);
        }

        #[test]
        fn when_multiple_controls_are_plugged_then_they_are_all_added_to_queue() {
            let (mut cache, mut input) = init_cache(2);
            assert_eq!(
                cache.state,
                State::Calibrating(0, CalibrationPhase::Octave1)
            );
            assert_eq!(cache.queue.len(), 3);
            assert!(cache.queue.contains(&ControlAction::Map(0)));
            assert!(cache.queue.contains(&ControlAction::Calibrate(1)));
            assert!(cache.queue.contains(&ControlAction::Map(1)));

            input.control[2] = Some(1.0);
            input.control[3] = Some(1.0);
            cache.apply_input_snapshot(input);
            assert_eq!(
                cache.state,
                State::Calibrating(0, CalibrationPhase::Octave1)
            );
            assert_eq!(cache.queue.len(), 5);
            assert!(cache.queue.contains(&ControlAction::Map(0)));
            assert!(cache.queue.contains(&ControlAction::Calibrate(1)));
            assert!(cache.queue.contains(&ControlAction::Map(1)));
            assert!(cache.queue.contains(&ControlAction::Map(2)));
            assert!(cache.queue.contains(&ControlAction::Map(3)));
        }

        #[test]
        fn when_control_is_unplugged_it_is_removed_from_queue() {
            let (mut cache, mut input) = init_cache(3);
            assert_eq!(
                cache.state,
                State::Calibrating(0, CalibrationPhase::Octave1)
            );
            assert_eq!(cache.queue.len(), 5);

            input.control[1] = None;
            cache.apply_input_snapshot(input);
            assert_eq!(
                cache.state,
                State::Calibrating(0, CalibrationPhase::Octave1)
            );
            assert_eq!(cache.queue.len(), 3);
        }

        #[test]
        fn when_currently_mapping_control_is_unplugged_it_returns_to_normal() {
            let (mut cache, mut input) = init_cache(1);
            assert_eq!(
                cache.state,
                State::Calibrating(0, CalibrationPhase::Octave1)
            );
            assert_eq!(cache.queue.len(), 1);

            input.control[0] = None;
            cache.apply_input_snapshot(input);
            assert_eq!(cache.state, State::Normal);
            assert_animation(&mut cache, &[8888, 0000, 8888, 0000]);
        }

        #[test]
        fn when_calibrated_control_is_unplugged_it_retains_calibration() {
            let (mut cache, mut input) = init_cache(1);

            input.control[0] = Some(1.3);
            apply_input_snapshot(&mut cache, input);
            click_button(&mut cache, input);

            input.control[0] = Some(2.4);
            apply_input_snapshot(&mut cache, input);
            click_button(&mut cache, input);

            let original = cache.calibrations[0];

            input.control[0] = None;
            apply_input_snapshot(&mut cache, input);

            assert_relative_eq!(cache.calibrations[0].offset, original.offset);
            assert_relative_eq!(cache.calibrations[0].scaling, original.scaling);
        }
    }
}

// TODO: Test that control tempo gets combined correctly with speed knob
