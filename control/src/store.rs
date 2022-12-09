//! Components of user inteface, passing user input to DSP and reactions back.
//!
//! It is mainly targetted to run in a firmware with multiple loops running in
//! different frequencies, passing messages from one to another. However, parts
//! of it may be useful in software as well.

use heapless::Vec;
use kaseta_dsp::processor::{Attributes as DSPAttributes, Reaction as DSPReaction};

use crate::action::{ControlAction, Queue};
use crate::cache::calibration::Calibration;
use crate::cache::display::{ConfigurationScreen, Screen};
use crate::cache::mapping::AttributeIdentifier;
use crate::cache::{Cache, Configuration};
use crate::input::snapshot::Snapshot as InputSnapshot;
use crate::input::store::{Store as Input, StoreHead as InputHead};
use crate::output::DesiredOutput;
use crate::save::Save;

/// The main store of peripheral abstraction and module configuration.
///
/// This struct is the central piece of the control module. It takes
/// `InputSnapshot` on its inputs, passes it to peripheral abstractions,
/// interprets the current input into module configuration and manages
/// the whole state machine of that.
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Store {
    pub input: Input,
    state: State,
    pub queue: Queue,
    pub cache: Cache,
}

/// The current state of the control state machine.
///
/// The module can be in one of multiple states. The behavior of input and
/// output peripherals will differ based on the current state.
#[derive(Debug, PartialEq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub(crate) enum State {
    Calibrating(StateCalibrating),
    Mapping(StateMapping),
    Configuring(StateConfiguring),
    Normal,
}

#[derive(Debug, PartialEq, Clone, Copy)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub(crate) struct StateCalibrating {
    input: usize,
    phase: CalibrationPhase,
}

#[derive(Debug, PartialEq, Clone, Copy)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub(crate) enum CalibrationPhase {
    Octave1,
    Octave2(f32),
}

#[derive(Debug, PartialEq, Clone, Copy)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub(crate) struct StateMapping {
    input: usize,
}

#[derive(Debug, PartialEq, Clone, Copy)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub(crate) struct StateConfiguring {
    draft: Configuration,
}

/// Response of control store after processing new input snapshot.
///
/// This response should be evaluated by the caller and passed further to save
/// and DSP processors.
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct ApplyInputSnapshotResult {
    pub dsp_attributes: DSPAttributes,
    pub save: Option<Save>,
}

// TODO: Try to mark as many control methods as private as possible
#[allow(clippy::new_without_default)]
impl Store {
    #[must_use]
    pub fn new() -> Self {
        Self {
            input: Input::default(),
            state: State::default(),
            queue: Queue::default(),
            cache: Cache::default(),
        }
    }

    pub fn warm_up(&mut self, snapshot: InputSnapshot) {
        self.input.update(snapshot);
    }

    pub fn apply_input_snapshot(&mut self, snapshot: InputSnapshot) -> ApplyInputSnapshotResult {
        self.input.update(snapshot);
        let save = self.converge_internal_state();
        let dsp_attributes = self.cache.build_dsp_attributes();
        ApplyInputSnapshotResult {
            dsp_attributes,
            save,
        }
    }

    pub fn apply_dsp_reaction(&mut self, dsp_reaction: DSPReaction) {
        self.cache.apply_dsp_reaction(dsp_reaction);
    }

    pub fn tick(&mut self) -> DesiredOutput {
        self.cache.tick()
    }

    fn converge_internal_state(&mut self) -> Option<Save> {
        let (plugged_controls, unplugged_controls) = self.plugged_and_unplugged_controls();
        self.cache.unmap_controls(&unplugged_controls);
        self.dequeue_controls(&unplugged_controls);
        self.enqueue_controls(&plugged_controls);

        let mut needs_save = false;

        match self.state {
            State::Normal => {
                self.converge_from_normal_state(&mut needs_save);
            }
            State::Configuring(configuring) => {
                self.converge_from_configuring_state(configuring, &mut needs_save);
            }
            State::Calibrating(calibrating) => {
                self.converge_from_calibrating_state(calibrating, &mut needs_save);
            }
            State::Mapping(mapping) => {
                self.converge_from_mapping_state(mapping, &mut needs_save);
            }
        };

        self.reconcile_detectors();
        self.reconcile_attributes();

        self.cache
            .display
            .set_screen(2, self.cache.screen_for_heads());

        if needs_save {
            Some(self.cache.save())
        } else {
            None
        }
    }

    fn plugged_and_unplugged_controls(&self) -> (Vec<usize, 4>, Vec<usize, 4>) {
        let mut plugged = Vec::new();
        let mut unplugged = Vec::new();
        for (i, cv) in self.input.control.iter().enumerate() {
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

    fn dequeue_controls(&mut self, unplugged_controls: &Vec<usize, 4>) {
        for i in unplugged_controls {
            self.queue.remove_control(*i);
        }
    }

    fn enqueue_controls(&mut self, plugged_controls: &Vec<usize, 4>) {
        for i in plugged_controls {
            self.queue.remove_control(*i);
            if self.input.button.pressed {
                self.queue.push(ControlAction::Calibrate(*i));
            }
            if self.cache.mapping[*i].is_none() {
                self.queue.push(ControlAction::Map(*i));
            }
        }
    }

    fn converge_from_normal_state(&mut self, needs_save: &mut bool) {
        self.detect_tapped_tempo(needs_save);
        if self.button_is_long_held() {
            self.state = State::configuring_from_draft(self.cache.configuration);
            self.cache.display.set_screen(1, Screen::configuration());
        } else if let Some(action) = self.queue.pop() {
            match action {
                ControlAction::Calibrate(i) => {
                    self.state = State::calibrating_octave_1(i);
                    self.cache.display.set_screen(1, Screen::calibration_1(i));
                }
                ControlAction::Map(i) => {
                    self.state = State::mapping(i);
                    self.cache.display.set_screen(1, Screen::mapping(i));
                }
            }
        } else {
            self.cache.display.reset_screen(1);
        }
    }

    fn detect_tapped_tempo(&mut self, needs_save: &mut bool) {
        if let Some(detected_tempo) = self.cache.tap_detector.detected_tempo() {
            if self.input.speed.active() {
                self.cache.tap_detector.reset();
                self.cache.tapped_tempo = None;
            } else {
                *needs_save = true;
                self.cache.tapped_tempo = Some(detected_tempo as f32 / 1000.0);
            }
        }
    }

    fn button_is_long_held(&mut self) -> bool {
        self.input.button.held > 5_000
    }

    fn converge_from_mapping_state(
        &mut self,
        StateMapping { input }: StateMapping,
        needs_save: &mut bool,
    ) {
        let destination = self.active_attribute();
        if !self.input.control[input].is_plugged {
            self.cache.display.set_screen(0, Screen::failure());
            self.state = State::Normal;
        } else if !destination.is_none() && !self.cache.mapping.contains(&destination) {
            *needs_save = true;
            self.cache.mapping[input] = destination;
            self.state = State::Normal;
        }
    }

    fn active_attribute(&self) -> AttributeIdentifier {
        for (pot, identifier) in [
            (&self.input.pre_amp, AttributeIdentifier::PreAmp),
            (&self.input.drive, AttributeIdentifier::Drive),
            (&self.input.bias, AttributeIdentifier::Bias),
            (&self.input.dry_wet, AttributeIdentifier::DryWet),
            (&self.input.wow_flut, AttributeIdentifier::WowFlut),
            (&self.input.speed, AttributeIdentifier::Speed),
            (&self.input.tone, AttributeIdentifier::Tone),
        ] {
            if pot.active() {
                return identifier;
            }
        }

        for (i, head) in self.input.head.iter().enumerate() {
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

    fn converge_from_calibrating_state(
        &mut self,
        StateCalibrating { input, phase }: StateCalibrating,
        needs_save: &mut bool,
    ) {
        if !self.input.control[input].is_plugged {
            self.cache.display.set_screen(0, Screen::failure());
            self.state = State::Normal;
        } else if self.input.button.clicked {
            match phase {
                CalibrationPhase::Octave1 => {
                    let octave_1 = self.input.control[input].value();
                    self.state = State::calibrating_octave_2(input, octave_1);
                    self.cache
                        .display
                        .set_screen(1, Screen::calibration_2(input));
                }
                CalibrationPhase::Octave2(octave_1) => {
                    let octave_2 = self.input.control[input].value();
                    if let Some(calibration) = Calibration::try_new(octave_1, octave_2) {
                        *needs_save = true;
                        self.cache.calibrations[input] = calibration;
                    } else {
                        self.cache.display.set_screen(0, Screen::failure());
                    }
                    self.state = State::Normal;
                }
            }
        }
    }

    fn converge_from_configuring_state(
        &mut self,
        configuring: StateConfiguring,
        needs_save: &mut bool,
    ) {
        if self.input.button.clicked {
            *needs_save = true;
            self.cache.configuration = configuring.draft;
            self.state = State::Normal;
        } else {
            let (draft, screen) = self.updated_configuration_draft(configuring.draft);
            if let Some(screen) = screen {
                self.cache.display.set_screen(1, screen);
            }
            self.state = State::Configuring(StateConfiguring { draft });
        }
    }

    // TODO: Pass reference instead
    fn updated_configuration_draft(
        &self,
        mut draft: Configuration,
    ) -> (Configuration, Option<Screen>) {
        for (i, head) in self.input.head.iter().enumerate() {
            let maybe_screen = update_rewind_configuration(&mut draft, i, head);
            if let Some(screen) = maybe_screen {
                return (draft, Some(screen));
            }
        }
        (draft, None)
    }

    pub(crate) fn control_value_for_attribute(
        &self,
        attribute: AttributeIdentifier,
    ) -> Option<f32> {
        let i = self.control_index_for_attribute(attribute);
        if let Some(i) = i {
            let control = &self.input.control[i];
            if control.is_plugged {
                let calibration = self.cache.calibrations[i];
                Some(calibration.apply(control.value()))
            } else {
                None
            }
        } else {
            None
        }
    }

    pub(crate) fn control_index_for_attribute(
        &self,
        attribute: AttributeIdentifier,
    ) -> Option<usize> {
        self.cache.mapping.iter().position(|a| *a == attribute)
    }

    fn reconcile_attributes(&mut self) {
        self.reconcile_pre_amp();
        self.reconcile_hysteresis();
        self.reconcile_wow_flutter();
        self.reconcile_tone();
        self.reconcile_speed();
        self.reconcile_heads();
    }

    fn reconcile_detectors(&mut self) {
        if self.input.button.clicked {
            self.cache.tap_detector.trigger();
        }

        for (i, control) in self.input.control.iter().enumerate() {
            if !control.is_plugged || control.value_raw() < 0.45 {
                self.cache.clock_detectors[i].reset();
            }

            let triggered = control.previous_value_raw() <= 0.9
                && control.value_raw() > 0.9
                && control.traveled() > 0.3;
            if triggered {
                self.cache.clock_detectors[i].trigger();
            }
        }
    }
}

fn update_rewind_configuration(
    draft: &mut Configuration,
    i: usize,
    head: &InputHead,
) -> Option<Screen> {
    let screen = None;

    let volume_active = head.volume.active();
    let feedback_active = head.feedback.active();

    if !volume_active && !feedback_active {
        return screen;
    }

    let rewind_speed = if volume_active {
        f32_to_index_of_4(head.volume.value())
    } else {
        draft.rewind_speed[i].0
    };

    let fast_forward_speed = if feedback_active {
        f32_to_index_of_4(head.feedback.value())
    } else {
        draft.rewind_speed[i].1
    };

    let tuple = (rewind_speed, fast_forward_speed);
    draft.rewind_speed[i] = tuple;
    Some(Screen::Configuration(ConfigurationScreen::Rewind(tuple)))
}

fn f32_to_index_of_4(x: f32) -> usize {
    if x < 0.25 {
        0
    } else if x < 0.5 {
        1
    } else if x < 0.75 {
        2
    } else {
        3
    }
}

impl From<Save> for Store {
    fn from(save: Save) -> Self {
        let mut store = Self::new();
        store.cache.mapping = save.mapping;
        store.cache.calibrations = save.calibrations;
        store.cache.configuration = save.configuration;
        store.cache.tapped_tempo = save.tapped_tempo;
        store
    }
}

impl State {
    fn configuring_from_draft(draft: Configuration) -> Self {
        State::Configuring(StateConfiguring { draft })
    }

    fn calibrating_octave_1(input: usize) -> Self {
        State::Calibrating(StateCalibrating {
            input,
            phase: CalibrationPhase::Octave1,
        })
    }

    fn calibrating_octave_2(input: usize, octave_1: f32) -> Self {
        State::Calibrating(StateCalibrating {
            input,
            phase: CalibrationPhase::Octave2(octave_1),
        })
    }

    fn mapping(input: usize) -> Self {
        State::Mapping(StateMapping { input })
    }
}

impl Default for State {
    fn default() -> Self {
        Self::Normal
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::field_reassign_with_default)]
    #![allow(clippy::manual_assert)]
    #![allow(clippy::zero_prefixed_literal)]

    use super::*;

    #[test]
    fn it_should_be_possible_to_initialize_store() {
        let _store = Store::new();
    }

    fn click_button(store: &mut Store, mut input: InputSnapshot) -> Option<Save> {
        input.button = true;
        let save_1 = store.apply_input_snapshot(input).save;
        store.tick();
        input.button = false;
        let save_2 = store.apply_input_snapshot(input).save;
        store.tick();
        save_1.or(save_2)
    }

    fn hold_button(store: &mut Store, mut input: InputSnapshot) {
        input.button = true;
        for _ in 0..6 * 1000 {
            store.apply_input_snapshot(input);
            store.tick();
        }
        input.button = false;
        store.apply_input_snapshot(input);
        store.tick();
    }

    fn clock_trigger(store: &mut Store, i: usize, mut input: InputSnapshot, time: usize) {
        input.control[i] = Some(1.0);
        store.apply_input_snapshot(input);
        store.tick();
        input.control[i] = Some(0.5);
        for _ in 0..time - 1 {
            store.apply_input_snapshot(input);
            store.tick();
        }
    }

    fn tap_button(store: &mut Store, input: InputSnapshot, time: usize) {
        click_button(store, input);
        for _ in 0..time - 2 {
            store.apply_input_snapshot(input);
            store.tick();
        }
    }

    fn assert_animation(store: &mut Store, transitions: &[u32]) {
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
                let display = store.tick().display;
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
        let mut store = Store::new();
        let mut dsp_reaction = DSPReaction::default();

        dsp_reaction.delay_impulse = true;
        store.apply_dsp_reaction(dsp_reaction);

        dsp_reaction.delay_impulse = false;
        store.apply_dsp_reaction(dsp_reaction);

        let output = store.tick();
        assert!(output.impulse_trigger);
        let output = store.tick();
        assert!(output.impulse_trigger);

        for _ in 0..100 {
            let output = store.tick();
            if !output.impulse_trigger {
                return;
            }
        }

        panic!("Trigger was not set down within given timeout");
    }

    #[test]
    fn when_dsp_returns_impulse_it_should_lit_impulse_led_for_multiple_cycles() {
        let mut store = Store::new();
        let mut dsp_reaction = DSPReaction::default();

        dsp_reaction.delay_impulse = true;
        store.apply_dsp_reaction(dsp_reaction);

        dsp_reaction.delay_impulse = false;
        store.apply_dsp_reaction(dsp_reaction);

        let output = store.tick();
        assert!(output.impulse_led);
        let output = store.tick();
        assert!(output.impulse_led);

        for _ in 0..100 {
            let output = store.tick();
            if !output.impulse_led {
                return;
            }
        }

        panic!("Trigger was not set down within given timeout");
    }

    #[test]
    fn given_save_it_recovers_previously_set_tapped_tempo() {
        let mut store = Store::new();
        let input = InputSnapshot::default();

        tap_button(&mut store, input, 2000);
        tap_button(&mut store, input, 2000);
        tap_button(&mut store, input, 2000);
        tap_button(&mut store, input, 2000);

        let save = click_button(&mut store, input);
        assert_relative_eq!(store.cache.attributes.speed, 2.0);

        let mut store = Store::from(save.unwrap());
        let mut input = InputSnapshot::default();
        input.speed = 0.5;
        for _ in 0..10 {
            store.warm_up(input);
        }
        store.apply_input_snapshot(input);
        assert_relative_eq!(store.cache.attributes.speed, 2.0);
    }

    #[test]
    fn given_save_it_recovers_previously_set_mapping() {
        let mut store = Store::new();
        let mut input = InputSnapshot::default();

        input.control[1] = None;
        store.apply_input_snapshot(input);

        input.control[1] = Some(1.0);
        store.apply_input_snapshot(input);

        input.drive = 0.1;
        store.apply_input_snapshot(input);

        let save = store.cache.save();
        let mut store = Store::from(save);
        store.apply_input_snapshot(input);

        assert_eq!(store.cache.mapping[1], AttributeIdentifier::Drive);
        assert_eq!(store.state, State::Normal);
    }

    #[test]
    fn given_save_if_new_control_was_plugged_since_it_gets_to_the_queque() {
        let store = Store::new();

        let save = store.cache.save();

        let mut input = InputSnapshot::default();
        input.control[1] = Some(1.0);

        let mut store = Store::from(save);
        store.apply_input_snapshot(input);

        assert_eq!(store.state, State::Mapping(StateMapping { input: 1 }));
    }

    #[test]
    fn given_save_it_recovers_previously_set_calibration_and_mapping() {
        let mut store = Store::new();
        let mut input = InputSnapshot::default();

        input.control[1] = None;
        store.apply_input_snapshot(input);

        input.button = true;
        store.apply_input_snapshot(input);

        input.control[1] = Some(1.0);
        store.apply_input_snapshot(input);

        input.button = false;
        store.apply_input_snapshot(input);

        input.control[1] = Some(1.2);
        for _ in 0..10 {
            store.apply_input_snapshot(input);
        }
        click_button(&mut store, input);

        input.control[1] = Some(2.3);
        for _ in 0..10 {
            store.apply_input_snapshot(input);
        }
        click_button(&mut store, input);

        input.drive = 0.1;
        store.apply_input_snapshot(input);
        assert_eq!(store.state, State::Normal);

        let calibration = store.cache.calibrations[1];
        let mapping = store.cache.mapping[1];

        let save = store.cache.save();
        let mut store = Store::from(save);
        store.apply_input_snapshot(input);

        assert_eq!(store.cache.calibrations[1], calibration);
        assert_eq!(store.cache.mapping[1], mapping);
        assert_eq!(store.state, State::Normal);
    }

    #[test]
    fn given_save_after_calibration_was_done_but_mapping_not_it_recovers_calibration_and_continues_mapping(
    ) {
        let mut store = Store::new();
        let mut input = InputSnapshot::default();

        input.control[1] = None;
        store.apply_input_snapshot(input);

        input.button = true;
        store.apply_input_snapshot(input);

        input.control[1] = Some(1.0);
        store.apply_input_snapshot(input);

        input.button = false;
        store.apply_input_snapshot(input);

        input.control[1] = Some(1.2);
        for _ in 0..10 {
            store.apply_input_snapshot(input);
        }
        click_button(&mut store, input);

        input.control[1] = Some(2.3);
        for _ in 0..10 {
            store.apply_input_snapshot(input);
        }
        click_button(&mut store, input);

        input.drive = 0.1;
        store.apply_input_snapshot(input);

        assert_eq!(store.state, State::Normal);

        let calibration = store.cache.calibrations[1];
        let mapping = store.cache.mapping[1];

        let save = store.cache.save();
        let mut store = Store::from(save);
        store.apply_input_snapshot(input);

        assert_eq!(store.cache.calibrations[1], calibration);
        assert_eq!(store.cache.mapping[1], mapping);
        assert_eq!(store.state, State::Normal);
    }

    #[test]
    fn given_save_it_recovers_previously_set_configuration() {
        let mut store = Store::new();
        let mut input = InputSnapshot::default();

        hold_button(&mut store, input);

        input.head[0].volume = 1.0;
        input.head[0].feedback = 1.0;
        for _ in 0..4 {
            store.apply_input_snapshot(input);
        }
        input.head[1].volume = 1.0;
        input.head[1].feedback = 1.0;
        for _ in 0..4 {
            store.apply_input_snapshot(input);
        }
        input.head[2].volume = 1.0;
        input.head[2].feedback = 1.0;
        for _ in 0..4 {
            store.apply_input_snapshot(input);
        }
        input.head[3].volume = 1.0;
        input.head[3].feedback = 1.0;
        for _ in 0..4 {
            store.apply_input_snapshot(input);
        }

        let save = click_button(&mut store, input);

        let mut store = Store::from(save.unwrap());
        store.apply_input_snapshot(input);

        assert_eq!(store.cache.configuration.rewind_speed[0], (3, 3));
        assert_eq!(store.cache.configuration.rewind_speed[1], (3, 3));
        assert_eq!(store.cache.configuration.rewind_speed[2], (3, 3));
        assert_eq!(store.cache.configuration.rewind_speed[3], (3, 3));
    }

    #[cfg(test)]
    mod given_normal_mode {
        use super::*;

        fn init_store() -> Store {
            Store::new()
        }

        #[test]
        fn when_button_is_clicked_in_exact_interval_it_detects_tempo() {
            let mut store = Store::new();
            let input = InputSnapshot::default();

            for _ in 0..4 {
                tap_button(&mut store, input, 2000);
            }

            let attributes = store.apply_input_snapshot(input).dsp_attributes;
            assert_relative_eq!(attributes.speed, 2.0);
        }

        #[test]
        fn when_button_is_clicked_in_rough_interval_within_toleration_it_detects_tempo() {
            let mut store = Store::new();
            let input = InputSnapshot::default();

            tap_button(&mut store, input, 1990);
            tap_button(&mut store, input, 2050);
            tap_button(&mut store, input, 2000);
            tap_button(&mut store, input, 2);

            let attributes = store.apply_input_snapshot(input).dsp_attributes;
            assert_relative_eq!(attributes.speed, 2.0);
        }

        #[test]
        fn when_button_is_clicked_too_fast_it_does_not_detect_tempo() {
            let mut store = Store::new();
            let input = InputSnapshot::default();

            tap_button(&mut store, input, 20);
            tap_button(&mut store, input, 20);
            tap_button(&mut store, input, 20);
            tap_button(&mut store, input, 20);

            assert!(store.cache.tapped_tempo.is_none());
        }

        #[test]
        fn when_button_is_clicked_in_unequal_interval_it_does_not_detect_tempo() {
            let mut store = Store::new();
            let input = InputSnapshot::default();

            tap_button(&mut store, input, 2000);
            tap_button(&mut store, input, 1000);
            tap_button(&mut store, input, 2000);
            tap_button(&mut store, input, 2000);

            assert!(store.cache.tapped_tempo.is_none());
        }

        #[test]
        fn when_tempo_is_tapped_in_it_is_overwritten_only_after_speed_pot_turning() {
            let mut store = init_store();
            let mut input = InputSnapshot::default();

            tap_button(&mut store, input, 2000);
            tap_button(&mut store, input, 2000);
            tap_button(&mut store, input, 2000);
            tap_button(&mut store, input, 2000);

            assert_relative_eq!(store.cache.attributes.speed, 2.0);

            input.tone = 1.0;
            store.apply_input_snapshot(input);
            store.tick();
            assert_relative_eq!(store.cache.attributes.speed, 2.0);

            input.speed = 0.5;
            for _ in 0..4 {
                store.apply_input_snapshot(input);
                store.tick();
            }
            assert!(store.cache.attributes.speed > 20.0);
        }

        #[test]
        fn when_control_is_plugged_then_state_changes_to_mapping() {
            let mut store = init_store();
            let mut input = InputSnapshot::default();

            input.control[1] = None;
            store.apply_input_snapshot(input);

            input.control[1] = Some(1.0);
            let save = store.apply_input_snapshot(input).save;

            assert!(save.is_none());
            assert!(matches!(
                store.state,
                State::Mapping(StateMapping { input: 1 })
            ));
            assert_animation(
                &mut store,
                &[9600, 0800, 0690, 0609, 9600, 0800, 0690, 0609],
            );
        }

        #[test]
        fn when_control_is_plugged_while_holding_button_then_state_changes_to_calibration() {
            let mut store = init_store();
            let mut input = InputSnapshot::default();

            input.button = false;
            input.control[1] = None;
            store.apply_input_snapshot(input);

            input.button = true;
            input.control[1] = Some(1.0);
            let save = store.apply_input_snapshot(input).save;

            assert!(save.is_none());
            assert!(matches!(
                store.state,
                State::Calibrating(StateCalibrating {
                    input: 1,
                    phase: CalibrationPhase::Octave1
                })
            ));
            assert_animation(&mut store, &[9800, 0600, 9800, 0600]);
        }

        #[test]
        fn when_multiple_controls_are_plugged_then_are_all_added_to_queue() {
            let mut store = init_store();

            let mut input = InputSnapshot::default();
            input.control[1] = None;
            input.control[2] = None;
            store.apply_input_snapshot(input);

            let mut input = InputSnapshot::default();
            input.control[1] = Some(1.0);
            input.control[2] = Some(1.0);
            store.apply_input_snapshot(input);

            assert!(matches!(
                store.state,
                State::Mapping(StateMapping { input: 1 })
            ));
            assert_eq!(store.queue.len(), 1);
        }

        #[test]
        fn when_control_is_unplugged_it_is_removed_from_queue() {
            let mut store = init_store();
            let mut input = InputSnapshot::default();

            input.control[1] = None;
            input.control[2] = None;
            input.control[3] = None;
            store.apply_input_snapshot(input);

            input.control[1] = Some(1.0);
            input.control[2] = Some(1.0);
            input.control[3] = Some(1.0);
            store.apply_input_snapshot(input);

            assert!(matches!(
                store.state,
                State::Mapping(StateMapping { input: 1 })
            ));
            assert_eq!(store.queue.len(), 2);

            input.control[2] = None;
            store.apply_input_snapshot(input);

            assert!(matches!(
                store.state,
                State::Mapping(StateMapping { input: 1 })
            ));
            assert_eq!(store.queue.len(), 1);
        }

        #[test]
        fn when_button_is_held_for_5_seconds_it_enters_configuration_mode() {
            let mut store = init_store();
            let input = InputSnapshot::default();

            hold_button(&mut store, input);

            assert!(matches!(store.state, State::Configuring(_)));
        }

        #[test]
        fn when_mapped_control_in_unplugged_it_is_unmapped() {
            let mut store = init_store();
            let mut input = InputSnapshot::default();

            input.control[0] = None;
            store.apply_input_snapshot(input);

            input.control[0] = Some(1.0);
            store.apply_input_snapshot(input);

            input.drive = 0.4;
            store.apply_input_snapshot(input);

            assert_eq!(store.cache.mapping[0], AttributeIdentifier::Drive);

            input.control[0] = None;
            store.apply_input_snapshot(input);

            assert_eq!(store.cache.mapping[0], AttributeIdentifier::None);
        }

        #[test]
        fn it_displays_enabled_volume_and_feedback_based_on_head_order() {
            let mut store = init_store();
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
                store.apply_input_snapshot(input);
            }

            assert_animation(&mut store, &[6098]);
        }
    }

    #[cfg(test)]
    mod given_configuration_mode {
        use super::*;

        fn init_store() -> (Store, InputSnapshot) {
            let mut store = Store::new();
            let input = InputSnapshot::default();

            hold_button(&mut store, input);

            (store, input)
        }

        fn apply_input_snapshot(store: &mut Store, input: InputSnapshot) {
            // NOTE: Applying it 4 times makes sure that pot smoothening is
            // not affecting following asserts.
            for _ in 0..4 {
                store.apply_input_snapshot(input);
            }
        }

        #[test]
        fn when_clicks_button_it_saves_configuration_and_enters_normal_mode() {
            let (mut store, mut input) = init_store();

            input.head[0].volume = 0.05;
            apply_input_snapshot(&mut store, input);
            input.head[0].feedback = 0.05;
            apply_input_snapshot(&mut store, input);
            input.head[1].volume = 0.35;
            apply_input_snapshot(&mut store, input);
            input.head[1].feedback = 0.35;
            apply_input_snapshot(&mut store, input);
            input.head[2].volume = 0.7;
            apply_input_snapshot(&mut store, input);
            input.head[2].feedback = 0.7;
            apply_input_snapshot(&mut store, input);
            input.head[3].volume = 1.0;
            apply_input_snapshot(&mut store, input);
            input.head[3].feedback = 1.0;
            apply_input_snapshot(&mut store, input);

            click_button(&mut store, input);

            assert_eq!(store.cache.configuration.rewind_speed[0], (0, 0));
            assert_eq!(store.cache.configuration.rewind_speed[1], (1, 1));
            assert_eq!(store.cache.configuration.rewind_speed[2], (2, 2));
            assert_eq!(store.cache.configuration.rewind_speed[3], (3, 3));
        }

        #[test]
        fn when_turns_volume_and_feedback_the_rewind_speed_is_visualized_on_display() {
            let (mut store, mut input) = init_store();

            input.head[0].volume = 0.05;
            input.head[0].feedback = 0.05;
            apply_input_snapshot(&mut store, input);
            assert_animation(&mut store, &[9006]);

            input.head[1].volume = 0.35;
            input.head[1].feedback = 0.35;
            apply_input_snapshot(&mut store, input);
            assert_animation(&mut store, &[9966]);

            input.head[2].volume = 0.7;
            input.head[2].feedback = 0.7;
            apply_input_snapshot(&mut store, input);
            assert_animation(&mut store, &[9886]);

            input.head[3].volume = 1.0;
            input.head[3].feedback = 1.0;
            apply_input_snapshot(&mut store, input);
            assert_animation(&mut store, &[8888]);
        }

        #[test]
        fn when_does_not_change_attribute_it_keeps_the_previously_set_value() {
            let (mut store, mut input) = init_store();

            input.head[2].volume = 0.7;
            input.head[2].feedback = 0.7;
            apply_input_snapshot(&mut store, input);
            click_button(&mut store, input);

            assert_eq!(store.cache.configuration.rewind_speed[2], (2, 2));

            input.head[2].volume = 0.0;
            input.head[2].feedback = 0.0;
            apply_input_snapshot(&mut store, input);

            hold_button(&mut store, input);

            input.head[1].volume = 0.35;
            input.head[1].feedback = 0.35;
            apply_input_snapshot(&mut store, input);
            click_button(&mut store, input);

            assert_eq!(store.cache.configuration.rewind_speed[2], (2, 2));
            assert_eq!(store.cache.configuration.rewind_speed[1], (1, 1));
        }

        #[test]
        fn when_no_attribute_was_changed_yet_it_shows_animation() {
            let (mut store, _) = init_store();
            assert_animation(&mut store, &[9696, 6969]);
        }
    }

    #[cfg(test)]
    mod given_mapping_mode {
        use super::*;

        fn init_store(pending: usize) -> (Store, InputSnapshot) {
            let mut store = Store::new();
            let mut input = InputSnapshot::default();

            for i in 0..pending {
                input.control[i] = None;
            }
            store.apply_input_snapshot(input);

            for i in 0..pending {
                input.control[i] = Some(1.0);
            }
            store.apply_input_snapshot(input);

            (store, input)
        }

        fn apply_input_snapshot(store: &mut Store, input: InputSnapshot) {
            // NOTE: Applying it 4 times makes sure that pot smoothening is
            // not affecting following asserts.
            for _ in 0..4 {
                store.apply_input_snapshot(input);
            }
        }

        #[test]
        fn when_pot_is_active_it_gets_mapped_to_the_current_control_and_saved() {
            let (mut store, mut input) = init_store(4);

            input.drive = 0.1;
            let save = store.apply_input_snapshot(input).save;
            assert_eq!(save.unwrap().mapping[0], AttributeIdentifier::Drive);
            assert_eq!(store.cache.mapping[0], AttributeIdentifier::Drive);
            apply_input_snapshot(&mut store, input); // Let the pot converge

            input.speed = 0.1;
            let save = store.apply_input_snapshot(input).save;
            assert_eq!(save.unwrap().mapping[0], AttributeIdentifier::Drive);
            assert_eq!(save.unwrap().mapping[1], AttributeIdentifier::Speed);
            assert_eq!(store.cache.mapping[0], AttributeIdentifier::Drive);
            assert_eq!(store.cache.mapping[1], AttributeIdentifier::Speed);
            apply_input_snapshot(&mut store, input); // Let the pot converge

            input.dry_wet = 0.1;
            let save = store.apply_input_snapshot(input).save;
            assert_eq!(save.unwrap().mapping[0], AttributeIdentifier::Drive);
            assert_eq!(save.unwrap().mapping[1], AttributeIdentifier::Speed);
            assert_eq!(save.unwrap().mapping[2], AttributeIdentifier::DryWet);
            assert_eq!(store.cache.mapping[0], AttributeIdentifier::Drive);
            assert_eq!(store.cache.mapping[1], AttributeIdentifier::Speed);
            assert_eq!(store.cache.mapping[2], AttributeIdentifier::DryWet);
            apply_input_snapshot(&mut store, input); // Let the pot converge

            input.bias = 0.1;
            let save = store.apply_input_snapshot(input).save;
            assert_eq!(save.unwrap().mapping[0], AttributeIdentifier::Drive);
            assert_eq!(save.unwrap().mapping[1], AttributeIdentifier::Speed);
            assert_eq!(save.unwrap().mapping[2], AttributeIdentifier::DryWet);
            assert_eq!(save.unwrap().mapping[3], AttributeIdentifier::Bias);
            assert_eq!(store.cache.mapping[0], AttributeIdentifier::Drive);
            assert_eq!(store.cache.mapping[1], AttributeIdentifier::Speed);
            assert_eq!(store.cache.mapping[2], AttributeIdentifier::DryWet);
            assert_eq!(store.cache.mapping[3], AttributeIdentifier::Bias);
        }

        #[test]
        fn when_last_pending_control_is_processed_then_state_changes_to_normal() {
            let (mut store, mut input) = init_store(1);
            assert_eq!(store.state, State::Mapping(StateMapping { input: 0 }));

            input.drive = 0.1;
            apply_input_snapshot(&mut store, input);
            assert_eq!(store.state, State::Normal);
            assert_animation(&mut store, &[0000]);
        }

        #[test]
        fn when_second_last_pending_control_is_processed_it_moves_to_last() {
            let (mut store, mut input) = init_store(2);

            input.drive = 0.1;
            apply_input_snapshot(&mut store, input);
            assert_eq!(store.state, State::Mapping(StateMapping { input: 1 }));
        }

        #[test]
        fn when_multiple_controls_are_plugged_then_they_are_all_added_to_queue() {
            let (mut store, mut input) = init_store(2);
            assert_eq!(store.state, State::Mapping(StateMapping { input: 0 }));
            assert_eq!(store.queue.len(), 1);
            assert!(store.queue.contains(&ControlAction::Map(1)));

            input.control[2] = Some(1.0);
            input.control[3] = Some(1.0);
            store.apply_input_snapshot(input);
            assert_eq!(store.state, State::Mapping(StateMapping { input: 0 }));
            assert_eq!(store.queue.len(), 3);
            assert!(store.queue.contains(&ControlAction::Map(1)));
            assert!(store.queue.contains(&ControlAction::Map(2)));
            assert!(store.queue.contains(&ControlAction::Map(3)));
        }

        #[test]
        fn when_control_is_unplugged_it_is_removed_from_queue() {
            let (mut store, mut input) = init_store(3);
            assert_eq!(store.state, State::Mapping(StateMapping { input: 0 }));
            assert_eq!(store.queue.len(), 2);
            assert!(store.queue.contains(&ControlAction::Map(1)));
            assert!(store.queue.contains(&ControlAction::Map(2)));

            input.control[1] = None;
            store.apply_input_snapshot(input);
            assert_eq!(store.state, State::Mapping(StateMapping { input: 0 }));
            assert_eq!(store.queue.len(), 1);
            assert!(store.queue.contains(&ControlAction::Map(2)));
        }

        #[test]
        fn when_mapped_control_in_unplugged_it_is_unmapped() {
            let (mut store, mut input) = init_store(2);

            input.drive = 0.4;
            apply_input_snapshot(&mut store, input);
            assert_eq!(store.cache.mapping[0], AttributeIdentifier::Drive);
            assert_eq!(store.state, State::Mapping(StateMapping { input: 1 }));

            input.control[0] = None;
            apply_input_snapshot(&mut store, input);
            assert_eq!(store.cache.mapping[0], AttributeIdentifier::None);
            assert_eq!(store.state, State::Mapping(StateMapping { input: 1 }));
        }

        #[test]
        fn when_active_pot_is_assigned_it_is_not_reassigned() {
            let (mut store, mut input) = init_store(2);

            input.drive = 0.1;
            apply_input_snapshot(&mut store, input);
            assert_eq!(store.cache.mapping[0], AttributeIdentifier::Drive);
            assert_eq!(store.cache.mapping[1], AttributeIdentifier::None);

            input.drive = 0.2;
            apply_input_snapshot(&mut store, input);
            assert_eq!(store.cache.mapping[0], AttributeIdentifier::Drive);
            assert_eq!(store.cache.mapping[1], AttributeIdentifier::None);
            assert_eq!(store.state, State::Mapping(StateMapping { input: 1 }));

            assert_animation(&mut store, &[9600, 0800, 0690, 0609]);
        }

        #[test]
        fn when_control_is_unplugged_it_returns_to_normal_mode() {
            let (mut store, mut input) = init_store(1);
            assert_eq!(store.state, State::Mapping(StateMapping { input: 0 }));

            input.control[0] = None;
            apply_input_snapshot(&mut store, input);
            assert_eq!(store.state, State::Normal);
        }
    }

    #[cfg(test)]
    mod given_calibrating_mode {
        use super::*;

        fn init_store(pending: usize) -> (Store, InputSnapshot) {
            let mut store = Store::new();
            let mut input = InputSnapshot::default();

            for i in 0..pending {
                input.control[i] = None;
            }
            store.apply_input_snapshot(input);

            input.button = true;
            store.apply_input_snapshot(input);

            for i in 0..pending {
                input.control[i] = Some(1.0);
            }
            store.apply_input_snapshot(input);

            input.button = false;
            store.apply_input_snapshot(input);

            (store, input)
        }

        fn apply_input_snapshot(store: &mut Store, input: InputSnapshot) {
            // NOTE: Applying it 4 times makes sure that control smoothening is
            // not affecting following asserts.
            for _ in 0..4 {
                store.apply_input_snapshot(input);
            }
        }

        #[test]
        fn when_correct_values_are_given_it_successfully_converges_turns_to_mapping_and_saves() {
            let (mut store, mut input) = init_store(1);
            assert_eq!(
                store.state,
                State::Calibrating(StateCalibrating {
                    input: 0,
                    phase: CalibrationPhase::Octave1
                })
            );
            assert_animation(&mut store, &[8900, 6000, 8900, 6000]);

            input.control[0] = Some(1.3);
            apply_input_snapshot(&mut store, input);
            click_button(&mut store, input);
            assert_eq!(
                store.state,
                State::Calibrating(StateCalibrating {
                    input: 0,
                    phase: CalibrationPhase::Octave2(1.3)
                })
            );
            assert_animation(&mut store, &[6099, 6000, 6099, 6000]);

            input.control[0] = Some(2.4);
            store.apply_input_snapshot(input);
            let save = click_button(&mut store, input);
            let saved_calibration = save.unwrap().calibrations[0];
            assert_relative_ne!(saved_calibration.offset, Calibration::default().offset);
            assert_relative_ne!(saved_calibration.scaling, Calibration::default().scaling);
            let store_calibration = store.cache.calibrations[0];
            assert_relative_ne!(store_calibration.offset, Calibration::default().offset);
            assert_relative_ne!(store_calibration.scaling, Calibration::default().scaling);
            assert_eq!(store.state, State::Mapping(StateMapping { input: 0 }));
            assert_animation(&mut store, &[8000, 6900, 6090, 6009]);
        }

        #[test]
        fn when_incorrect_values_are_given_it_it_cancels_calibration_turns_to_mapping_and_does_not_save(
        ) {
            let (mut store, mut input) = init_store(1);
            assert_eq!(
                store.state,
                State::Calibrating(StateCalibrating {
                    input: 0,
                    phase: CalibrationPhase::Octave1
                })
            );
            assert_animation(&mut store, &[8900, 6000, 8900, 6000]);

            input.control[0] = Some(1.3);
            apply_input_snapshot(&mut store, input);
            click_button(&mut store, input);
            assert_eq!(
                store.state,
                State::Calibrating(StateCalibrating {
                    input: 0,
                    phase: CalibrationPhase::Octave2(1.3)
                })
            );
            assert_animation(&mut store, &[6099, 6000, 6099, 6000]);

            input.control[0] = Some(1.4);
            store.apply_input_snapshot(input);
            let save = click_button(&mut store, input);
            assert!(save.is_none());
            let store_calibration = store.cache.calibrations[0];
            assert_relative_eq!(store_calibration.offset, Calibration::default().offset);
            assert_relative_eq!(store_calibration.scaling, Calibration::default().scaling);
            assert_eq!(store.state, State::Mapping(StateMapping { input: 0 }));
            assert_animation(&mut store, &[8888, 0000, 8888, 0000, 6090, 6009]);
        }

        #[test]
        fn when_multiple_controls_are_plugged_then_they_are_all_added_to_queue() {
            let (mut store, mut input) = init_store(2);
            assert_eq!(
                store.state,
                State::Calibrating(StateCalibrating {
                    input: 0,
                    phase: CalibrationPhase::Octave1
                })
            );
            assert_eq!(store.queue.len(), 3);
            assert!(store.queue.contains(&ControlAction::Map(0)));
            assert!(store.queue.contains(&ControlAction::Calibrate(1)));
            assert!(store.queue.contains(&ControlAction::Map(1)));

            input.control[2] = Some(1.0);
            input.control[3] = Some(1.0);
            store.apply_input_snapshot(input);
            assert_eq!(
                store.state,
                State::Calibrating(StateCalibrating {
                    input: 0,
                    phase: CalibrationPhase::Octave1
                })
            );
            assert_eq!(store.queue.len(), 5);
            assert!(store.queue.contains(&ControlAction::Map(0)));
            assert!(store.queue.contains(&ControlAction::Calibrate(1)));
            assert!(store.queue.contains(&ControlAction::Map(1)));
            assert!(store.queue.contains(&ControlAction::Map(2)));
            assert!(store.queue.contains(&ControlAction::Map(3)));
        }

        #[test]
        fn when_control_is_unplugged_it_is_removed_from_queue() {
            let (mut store, mut input) = init_store(3);
            assert_eq!(
                store.state,
                State::Calibrating(StateCalibrating {
                    input: 0,
                    phase: CalibrationPhase::Octave1
                })
            );
            assert_eq!(store.queue.len(), 5);

            input.control[1] = None;
            store.apply_input_snapshot(input);
            assert_eq!(
                store.state,
                State::Calibrating(StateCalibrating {
                    input: 0,
                    phase: CalibrationPhase::Octave1
                })
            );
            assert_eq!(store.queue.len(), 3);
        }

        #[test]
        fn when_currently_mapping_control_is_unplugged_it_returns_to_normal() {
            let (mut store, mut input) = init_store(1);
            assert_eq!(
                store.state,
                State::Calibrating(StateCalibrating {
                    input: 0,
                    phase: CalibrationPhase::Octave1
                })
            );
            assert_eq!(store.queue.len(), 1);

            input.control[0] = None;
            store.apply_input_snapshot(input);
            assert_eq!(store.state, State::Normal);
            assert_animation(&mut store, &[8888, 0000, 8888, 0000]);
        }

        #[test]
        fn when_calibrated_control_is_unplugged_it_retains_calibration() {
            let (mut store, mut input) = init_store(1);

            input.control[0] = Some(1.3);
            apply_input_snapshot(&mut store, input);
            click_button(&mut store, input);

            input.control[0] = Some(2.4);
            apply_input_snapshot(&mut store, input);
            click_button(&mut store, input);

            let original = store.cache.calibrations[0];

            input.control[0] = None;
            apply_input_snapshot(&mut store, input);

            assert_relative_eq!(store.cache.calibrations[0].offset, original.offset);
            assert_relative_eq!(store.cache.calibrations[0].scaling, original.scaling);
        }
    }

    #[test]
    fn when_steady_clock_passes_in_it_detects_tempo() {
        let mut store = Store::new();
        let input = InputSnapshot::default();

        clock_trigger(&mut store, 1, input, 2000);
        clock_trigger(&mut store, 1, input, 2000);
        clock_trigger(&mut store, 1, input, 2000);
        clock_trigger(&mut store, 1, input, 1);

        assert_eq!(
            store.cache.clock_detectors[1].detected_tempo().unwrap(),
            2000
        );
    }

    #[test]
    fn when_clock_within_toleration_passes_it_detects_tempo() {
        let mut store = Store::new();
        let input = InputSnapshot::default();

        clock_trigger(&mut store, 1, input, 1995);
        clock_trigger(&mut store, 1, input, 2030);
        clock_trigger(&mut store, 1, input, 2000);
        clock_trigger(&mut store, 1, input, 1);

        assert_eq!(
            store.cache.clock_detectors[1].detected_tempo().unwrap(),
            2000
        );
    }

    #[test]
    fn when_unevenly_spaced_triggers_are_given_it_is_not_recognized_as_tempo() {
        let mut store = Store::new();
        let input = InputSnapshot::default();

        clock_trigger(&mut store, 1, input, 2000);
        clock_trigger(&mut store, 1, input, 999);
        clock_trigger(&mut store, 1, input, 2000);
        clock_trigger(&mut store, 1, input, 1);

        assert!(store.cache.clock_detectors[1].detected_tempo().is_none());
    }

    #[test]
    fn when_signal_goes_below_zero_it_is_not_recognized_as_clock() {
        let mut store = Store::new();
        let mut input = InputSnapshot::default();

        clock_trigger(&mut store, 1, input, 2000);
        clock_trigger(&mut store, 1, input, 2000);

        input.control[1] = Some(1.0);
        store.apply_input_snapshot(input);
        store.tick();
        input.control[1] = Some(0.1);
        store.apply_input_snapshot(input);
        for _ in 0..1999 {
            store.tick();
        }

        clock_trigger(&mut store, 1, input, 1);

        assert!(store.cache.clock_detectors[1].detected_tempo().is_none());
    }

    #[test]
    fn when_signal_has_fast_enough_attacks_it_is_recognized_as_clock() {
        let mut store = Store::new();
        let input = InputSnapshot::default();

        fn attack(store: &mut Store, mut input: InputSnapshot, should_detect: bool) {
            let mut detected = false;
            let attack = 3;
            for i in 0..=attack {
                input.control[1] = Some(0.5 + 0.5 * (i as f32 / attack as f32));
                store.apply_input_snapshot(input);
                store.tick();
                detected |= store.cache.clock_detectors[1].detected_tempo().is_some();
            }
            assert_eq!(should_detect, detected);
        }

        fn silence(store: &mut Store, mut input: InputSnapshot) {
            for _ in 0..1999 {
                input.control[1] = Some(0.5);
                store.apply_input_snapshot(input);
                store.tick();
                assert!(store.cache.clock_detectors[1].detected_tempo().is_none());
            }
        }

        attack(&mut store, input, false);
        silence(&mut store, input);
        attack(&mut store, input, false);
        silence(&mut store, input);
        attack(&mut store, input, false);
        silence(&mut store, input);
        attack(&mut store, input, true);
    }

    #[test]
    fn when_signal_does_not_have_fast_attacks_it_is_not_recognized_as_clock() {
        let mut store = Store::new();
        let input = InputSnapshot::default();

        fn attack(store: &mut Store, mut input: InputSnapshot) {
            let attack = 200;
            for i in 0..=attack {
                input.control[1] = Some(0.5 + 0.5 * (i as f32 / attack as f32));
                store.apply_input_snapshot(input);
                store.tick();
                assert!(store.cache.clock_detectors[1].detected_tempo().is_none());
            }
        }

        fn silence(store: &mut Store, mut input: InputSnapshot) {
            for _ in 0..1999 {
                input.control[1] = Some(0.5);
                store.apply_input_snapshot(input);
                store.tick();
                assert!(store.cache.clock_detectors[1].detected_tempo().is_none());
            }
        }

        attack(&mut store, input);
        silence(&mut store, input);
        attack(&mut store, input);
        silence(&mut store, input);
        attack(&mut store, input);
        silence(&mut store, input);
        attack(&mut store, input);
    }

    #[test]
    fn when_control_is_mapped_to_speed_and_clock_passed_it_sets_it_accordingly() {
        let mut store = Store::new();
        let mut input = InputSnapshot::default();

        // Initialize
        input.control[1] = None;
        store.apply_input_snapshot(input);

        // Plug in
        input.control[1] = Some(0.5);
        store.apply_input_snapshot(input);

        // Turn knob to map control
        input.speed = 0.5;
        store.apply_input_snapshot(input);

        // Send clock signal to control
        clock_trigger(&mut store, 1, input, 2000);
        clock_trigger(&mut store, 1, input, 2000);
        clock_trigger(&mut store, 1, input, 2000);
        clock_trigger(&mut store, 1, input, 1);

        fn check(store: &mut Store, mut input: InputSnapshot, pot: f32, speed: f32) {
            input.speed = pot;
            for _ in 0..5 {
                store.apply_input_snapshot(input);
                store.tick();
            }
            let attributes = store.apply_input_snapshot(input).dsp_attributes;
            assert_relative_eq!(attributes.speed, speed);
        }

        // Test all positions of speed
        check(&mut store, input, 0.0 / 4.0, 2.0);
        check(&mut store, input, 1.0 / 4.0, 1.0);
        check(&mut store, input, 2.0 / 4.0, 0.5);
        check(&mut store, input, 3.0 / 4.0, 0.25);
        check(&mut store, input, 4.0 / 4.0, 0.125);
    }

    #[test]
    fn when_clock_signal_changes_tempo_it_gets_reflected_on_speed() {
        let mut store = Store::new();
        let mut input = InputSnapshot::default();

        // Initialize
        input.control[1] = None;
        store.apply_input_snapshot(input);

        // Plug in
        input.control[1] = Some(0.5);
        store.apply_input_snapshot(input);

        // Turn knob to map control
        input.speed = 0.5;
        store.apply_input_snapshot(input);

        // Move it to neutral position
        input.speed = 0.0;
        for _ in 0..5 {
            store.apply_input_snapshot(input);
        }

        fn check(store: &mut Store, input: InputSnapshot, delay: usize, speed: f32) {
            // Send clock signal to control
            clock_trigger(store, 1, input, delay);
            clock_trigger(store, 1, input, delay);
            clock_trigger(store, 1, input, delay);
            clock_trigger(store, 1, input, 1);
            let attributes = store.apply_input_snapshot(input).dsp_attributes;
            assert_relative_eq!(attributes.speed, speed);
        }

        // Send clock signal to control
        check(&mut store, input, 2000, 2.0);
        check(&mut store, input, 1000, 1.0);
        check(&mut store, input, 500, 0.5);
        check(&mut store, input, 4000, 4.0);
    }

    #[test]
    fn when_clock_signal_stops_speed_gets_set_from_its_analog_signal_and_pot() {
        let mut store = Store::new();
        let mut input = InputSnapshot::default();

        // Initialize
        input.control[1] = None;
        store.apply_input_snapshot(input);

        // Plug in
        input.control[1] = Some(0.5);
        store.apply_input_snapshot(input);

        // Turn knob to map control
        input.speed = 0.5;
        store.apply_input_snapshot(input);

        // Move it to neutral position
        input.speed = 0.0;
        for _ in 0..5 {
            store.apply_input_snapshot(input);
        }

        let original_speed = store.apply_input_snapshot(input).dsp_attributes.speed;
        assert_relative_ne!(original_speed, 2.0);

        // Send clock signal to control, with last clock followed by major delay
        clock_trigger(&mut store, 1, input, 2000);
        clock_trigger(&mut store, 1, input, 2000);
        clock_trigger(&mut store, 1, input, 2000);
        clock_trigger(&mut store, 1, input, 20000);

        let attributes = store.apply_input_snapshot(input).dsp_attributes;
        assert_relative_eq!(attributes.speed, original_speed);
    }
}