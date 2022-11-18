// TODO: This is a temporary draft of the new control architecture.
//
// - [X] Define stubs of all needed structs
// - [X] Write documentation of these structs
// - [X] Implement defmt display for all
// - [X] Implement tests stub
// - [X] Write initializer of cache
// - [ ] Implement raw input passing
// - [ ] Implement pot processing with smoothening
// - [ ] Implement CV processing with plug-in
// - [ ] Implement translation to attributes
// - [ ] Implement passing of attributes to pseudo-DSP
// - [ ] Implement passing of config and options to pseudo-DSP
// - [ ] Implement impulse output
// - [ ] Implement display output for basic attributes
// - [ ] Implement CV select
// - [ ] Implement calibration
// - [ ] Implement backup snapshoting (all data needed for restore)
// - [ ] Implement reset, connected CVs must be reassigned

#[allow(unused_imports)]
use micromath::F32Ext;

use heapless::Vec;

use crate::quantization::{quantize, Quantization};
use crate::taper;

/// The main store of peripheral abstraction and module configuration.
///
/// This struct is the cenral piece of the control module. It takes
/// `InputSnapshot` on its inputs, passes it to peripheral abstractions,
/// interprets the current input into module configuration and manages
/// the whole state machine of that.
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Cache {
    inputs: Inputs,
    pub(crate) state: State,
    queue: Queue,
    mapping: Mapping,
    calibrations: Calibrations,
    options: Options,
    configurations: Configuration,
    attributes: Attributes,
    display: Display,
}

/// Stateful store of raw inputs.
///
/// This struct turns the raw snapshot into a set of abstracted peripherals.
/// These peripherals provide features such as smoothening or click detection.
#[derive(Debug, Default)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
struct Inputs {
    pre_amp: Pot,
    drive: Pot,
    bias: Pot,
    dry_wet: Pot,
    wow_flut: Pot,
    speed: Pot,
    tone: Pot,
    head: [InputsHead; 4],
    control: [CV; 4],
    switch: [Switch; 10],
    button: Button,
}

#[derive(Debug, Default)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
struct InputsHead {
    position: Pot,
    volume: Pot,
    feedback: Pot,
    pan: Pot,
}

#[derive(Debug, Default)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
struct Pot {
    buffer: SmoothBuffer<4>,
}

#[derive(Debug, Default)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
struct CV {
    pub is_plugged: bool,
    pub was_plugged: bool,
    pub was_unplugged: bool,
    buffer: SmoothBuffer<4>,
}

// TODO: Try to optimize this with memory manager and a slice reference
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct SmoothBuffer<const N: usize> {
    buffer: [f32; N],
    pointer: usize,
}

type Switch = bool;

#[derive(Debug, Default)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
struct Button {
    pub pressed: bool,
    pub clicked: bool,
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
    Normal,
}

#[derive(Debug, PartialEq, Clone, Copy)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub(crate) enum CalibrationPhase {
    Octave1,
    Octave2(f32),
}

/// The queue of control inputs waiting for mapping or calibration.
///
/// This queue is used to sequentially process these operations without loosing
/// new requests that may be added meanwhile.
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
struct Queue {
    queue: Vec<ControlAction, 8>,
}

#[derive(Debug, PartialEq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
enum ControlAction {
    Calibrate(usize),
    Map(usize),
}

/// Linking between universal CV input and attributes controlled through pots.
//
/// This mapping is used to store mapping between control inputs and
/// attributes. It also represents the state machine ordering controls that
/// are yet to be mapped.
type Mapping = [AttributeIdentifier; 4];

#[derive(Debug, PartialEq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
enum AttributeIdentifier {
    PreAmp,
    Drive,
    Bias,
    DryWet,
    WowFlut,
    Speed,
    Tone,
    Position(usize),
    Volume(usize),
    Feedback(usize),
    Pan(usize),
    None,
}

/// TODO Docs
// TODO: One per head, offset and amplification
type Calibrations = [Calibration; 4];

/// TODO Docs
// TODO: One per head, offset and amplification
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
struct Calibration {
    offset: f32,
    scaling: f32,
}

/// Easy to access modifications of the default module behavior.
///
/// Options are configured using the DIP switch on the front of the module.
/// They are meant to provide a quick access to some common extended features
/// such as quantization, rewind, etc.
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
/// This is mean to allow tweaking of some more nieche configuration of the
/// module. Unlike with `Options`, the parameters here may be continuous
/// (float) or offer enumeration of variants. An examle of a configuration
/// may be tweaking of head's rewind speed.
#[derive(Debug, Default)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
struct Configuration {
    rewind_speed: (),
}

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

/// State machine representing 8 display LEDs of the module.
///
/// This structure handles the prioritization of display modes, their
/// changing from one to another, or animations.
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
struct Display {
    forced: Option<Screen>,
    prioritized: Option<(Screen, u32)>,
    backup: Screen,
}

#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
enum Screen {
    Calibration,
    ControlSelect,
    Placeholder,
}

/// Desired state of output peripherals with the exception of audio.
///
/// This structure transfers request to the module, asking to lit LEDs or
/// set CV output.
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct DesiredOutput {
    pub display: [bool; 8],
    pub implulse_led: bool,
    pub impulse_trigger: bool,
}

/// The current state of all peripherals.
///
/// InputSnapshot is meant to be passed from the hardware binding to the
/// control package. It should pass pretty raw data, with two exceptions:
///
/// 1. Detection of plugged CV input is done by the caller.
/// 2. Button debouncing is done by the caller.
#[derive(Debug, Default, Clone, Copy)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct InputSnapshot {
    pub pre_amp: f32,
    pub drive: f32,
    pub bias: f32,
    pub dry_wet: f32,
    pub wow_flut: f32,
    pub speed: f32,
    pub tone: f32,
    pub head: [InputSnapshotHead; 4],
    pub control: [Option<f32>; 4],
    pub switch: [bool; 10],
    pub button: bool,
}

#[derive(Debug, Default, Clone, Copy)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct InputSnapshotHead {
    pub position: f32,
    pub volume: f32,
    pub feedback: f32,
    pub pan: f32,
}

impl Default for State {
    fn default() -> Self {
        Self::Normal
    }
}

impl Display {
    pub fn new_with_screen(screen: Screen) -> Self {
        Self {
            forced: None,
            prioritized: None,
            backup: screen,
        }
    }
}

// NOTE: Inputs and outputs will be passed through queues
impl Cache {
    pub fn new() -> Self {
        Self {
            inputs: Inputs::default(),
            state: State::default(),
            queue: Queue::default(),
            mapping: Mapping::default(),
            calibrations: Calibrations::default(),
            options: Options::default(),
            configurations: Configuration::default(),
            attributes: Attributes::default(),
            display: Display::new_with_screen(Screen::Placeholder),
        }
    }

    pub fn apply_input_snapshot(&mut self, snapshot: InputSnapshot) -> () {
        self.inputs.update(snapshot);
        self.reconcile_changed_inputs();
        // TODO: Return config for DSP
    }

    fn reconcile_changed_inputs(&mut self) {
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
            self.queue.push(ControlAction::Map(*i));
        }

        match self.state {
            State::Normal => {
                if let Some(action) = self.queue.pop() {
                    match action {
                        ControlAction::Calibrate(i) => {
                            self.state = State::Calibrating(i, CalibrationPhase::Octave1)
                        }
                        ControlAction::Map(i) => self.state = State::Mapping(i),
                    }
                }
            }
            State::Calibrating(i, phase) => {
                if self.inputs.button.clicked {
                    match phase {
                        CalibrationPhase::Octave1 => {
                            let octave_1 = self.inputs.control[i].value();
                            self.state = State::Calibrating(i, CalibrationPhase::Octave2(octave_1));
                        }
                        CalibrationPhase::Octave2(octave_1) => {
                            let octave_2 = self.inputs.control[i].value();
                            if let Some(calibration) = Calibration::try_new(octave_1, octave_2) {
                                self.calibrations[i] = calibration;
                            }
                            self.state = State::Normal;
                        }
                    }
                }
            }
            State::Mapping(i) => {
                let destination = self.active_attribute();
                if !destination.is_none() && !self.mapping.contains(&destination) {
                    self.mapping[i] = destination;
                    self.state = State::Normal;
                }
            }
        };

        self.reconcile_pre_amp();
        self.reconcile_hysteresis();
        self.reconcile_wow_flut();
        self.reconcile_tone();
        self.reconcile_speed();
        self.reconcile_heads();
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

    fn reconcile_pre_amp(&mut self) {
        // Pre-amp scales between -20 to +28 dB.
        const PRE_AMP_RANGE: (f32, f32) = (0.0, 25.0);

        self.attributes.pre_amp = calculate(
            self.inputs.pre_amp.value(),
            self.control_for_attribute(AttributeIdentifier::PreAmp),
            PRE_AMP_RANGE,
            Some(taper::log),
        );
    }

    fn reconcile_hysteresis(&mut self) {
        const DRY_WET_RANGE: (f32, f32) = (0.0, 1.0);
        const DRIVE_RANGE: (f32, f32) = (0.1, 1.1);
        const SATURATION_RANGE: (f32, f32) = (0.0, 1.0);
        const BIAS_RANGE: (f32, f32) = (0.01, 1.0);

        // Maximum limit of how much place on the slider is occupied by drive. This
        // gets scaled down based on bias.
        const DRIVE_PORTION: f32 = 1.0 / 2.0;

        // Using <https://mycurvefit.com/> quadratic interpolation over data gathered
        // while testing the instability peak. The lower bias end was flattened not
        // to overdrive too much. Input data (maximum stable bias per drive):
        // 1.0 0.9 0.8 0.7 0.6 0.5 0.4 0.3 0.2 0.1
        // 0.1 0.992 1.091 1.240 1.240 1.388 1.580 1.580 1.780 1.780
        fn max_bias_for_drive(drive: f32) -> f32 {
            const D4B_B: f32 = 1.0;
            const D4B_A1: f32 = 0.0;
            const D4B_A2: f32 = -0.2;

            let drive2 = drive * drive;
            D4B_B + D4B_A1 * drive + D4B_A2 * drive2
        }

        self.attributes.dry_wet = calculate(
            self.inputs.dry_wet.value(),
            self.control_for_attribute(AttributeIdentifier::DryWet),
            DRY_WET_RANGE,
            None,
        );

        let drive_input = calculate(
            self.inputs.drive.value(),
            self.control_for_attribute(AttributeIdentifier::Drive),
            (0.0, 1.0),
            None,
        );

        let drive = calculate(
            (drive_input / DRIVE_PORTION).min(1.0),
            None,
            DRIVE_RANGE,
            None,
        );
        self.attributes.drive = drive;

        self.attributes.saturation = calculate(
            ((drive_input - DRIVE_PORTION) / (1.0 - DRIVE_PORTION)).clamp(0.0, 1.0),
            None,
            SATURATION_RANGE,
            None,
        );

        let max_bias = max_bias_for_drive(drive).clamp(BIAS_RANGE.0, BIAS_RANGE.1);
        self.attributes.bias = calculate(
            self.inputs.bias.value(),
            self.control_for_attribute(AttributeIdentifier::Bias),
            (0.01, max_bias),
            Some(taper::log),
        );
    }

    fn reconcile_wow_flut(&mut self) {
        const WOW_DEPTH_RANGE: (f32, f32) = (0.0, 0.2);
        const FLUTTER_DEPTH_RANGE: (f32, f32) = (0.0, 0.0015);

        let depth = calculate(
            self.inputs.wow_flut.value(),
            self.control_for_attribute(AttributeIdentifier::WowFlut),
            (-1.0, 1.0),
            None,
        );

        (self.attributes.wow, self.attributes.flutter) = if depth.is_sign_negative() {
            (calculate(-depth, None, WOW_DEPTH_RANGE, None), 0.0)
        } else {
            (0.0, calculate(depth, None, FLUTTER_DEPTH_RANGE, None))
        };
    }

    fn reconcile_tone(&mut self) {
        self.attributes.tone = calculate(
            self.inputs.tone.value(),
            self.control_for_attribute(AttributeIdentifier::Tone),
            (0.0, 1.0),
            None,
        );
    }

    fn reconcile_speed(&mut self) {
        // TODO: Rename to speed and revert it 1/X
        const LENGTH_LONG_RANGE: (f32, f32) = (0.02, 2.0 * 60.0);
        const LENGTH_SHORT_RANGE: (f32, f32) = (1.0 / 400.0, 1.0);

        self.attributes.speed = calculate(
            self.inputs.speed.value(),
            self.control_for_attribute(AttributeIdentifier::Speed),
            if self.options.short_delay_range {
                LENGTH_SHORT_RANGE
            } else {
                LENGTH_LONG_RANGE
            },
            Some(taper::reverse_log),
        );
    }

    fn reconcile_heads(&mut self) {
        for i in 0..4 {
            self.reconcile_head(i);
        }
    }

    fn reconcile_head(&mut self, i: usize) {
        self.attributes.head[i].position = quantize(
            calculate(
                self.inputs.head[i].position.value(),
                self.control_for_attribute(AttributeIdentifier::Position(i)),
                (0.0, 1.0),
                None,
            ),
            Quantization::from((self.options.quantize_6, self.options.quantize_8)),
        );
        self.attributes.head[i].volume = calculate(
            self.inputs.head[i].volume.value(),
            self.control_for_attribute(AttributeIdentifier::Volume(i)),
            (0.0, 1.0),
            None,
        );
        self.attributes.head[i].feedback = calculate(
            self.inputs.head[i].feedback.value(),
            self.control_for_attribute(AttributeIdentifier::Feedback(i)),
            (0.0, 1.0),
            None,
        );
        self.attributes.head[i].pan = calculate(
            self.inputs.head[i].pan.value(),
            self.control_for_attribute(AttributeIdentifier::Pan(i)),
            (0.0, 1.0),
            None,
        );
    }

    fn control_for_attribute(&mut self, attribute: AttributeIdentifier) -> Option<f32> {
        let pre_amp_control_index = self.mapping.iter().position(|a| *a == attribute);
        if let Some(pre_amp_control_index) = pre_amp_control_index {
            let control = &self.inputs.control[pre_amp_control_index];
            if control.is_plugged {
                Some(control.value())
            } else {
                None
            }
        } else {
            None
        }
    }

    pub fn apply_dsp_reaction(&mut self, dsp_reaction: ()) {
        // TODO: If DSP detected clipping, or impulse, apply it in cache
        todo!();
    }

    pub fn tick(&mut self) -> DesiredOutput {
        // TODO: Mark that time has passed
        // TODO: This will update timers of display and impulse output
        // self.display.tick();
        todo!()
    }
}

#[allow(clippy::let_and_return)]
fn calculate(
    pot: f32,
    cv: Option<f32>,
    range: (f32, f32),
    taper_function: Option<fn(f32) -> f32>,
) -> f32 {
    let sum = (pot + cv.unwrap_or(0.0)).clamp(0.0, 1.0);
    let curved = if let Some(taper_function) = taper_function {
        taper_function(sum)
    } else {
        sum
    };
    let scaled = curved * (range.1 - range.0) + range.0;
    scaled
}

impl Inputs {
    fn update(&mut self, snapshot: InputSnapshot) {
        self.pre_amp.update(snapshot.pre_amp);
        self.drive.update(snapshot.drive);
        self.bias.update(snapshot.bias);
        self.dry_wet.update(snapshot.dry_wet);
        self.wow_flut.update(snapshot.wow_flut);
        self.speed.update(snapshot.speed);
        self.tone.update(snapshot.tone);
        for (i, head) in self.head.iter_mut().enumerate() {
            head.position.update(snapshot.head[i].position);
            head.volume.update(snapshot.head[i].volume);
            head.feedback.update(snapshot.head[i].feedback);
            head.pan.update(snapshot.head[i].pan);
        }
        for (i, control) in self.control.iter_mut().enumerate() {
            control.update(snapshot.control[i]);
        }
        self.switch = snapshot.switch;
        self.button.update(snapshot.button);
    }
}

impl Pot {
    pub fn update(&mut self, value: f32) {
        self.buffer.write(value);
    }

    // TODO: Implement window support
    pub fn value(&self) -> f32 {
        self.buffer.read()
    }

    pub fn active(&self) -> bool {
        self.buffer.traveled() > 0.001
    }
}

impl CV {
    pub fn update(&mut self, value: Option<f32>) {
        let was_plugged = self.is_plugged;
        if let Some(value) = value {
            self.is_plugged = true;
            self.buffer.write(value);
        } else {
            self.is_plugged = false;
            self.buffer.reset();
        }
        self.was_plugged = !was_plugged && self.is_plugged;
        self.was_unplugged = was_plugged && !self.is_plugged;
    }

    pub fn value(&self) -> f32 {
        self.buffer.read()
    }
}

impl Button {
    pub fn update(&mut self, down: bool) {
        let was_pressed = self.pressed;
        self.pressed = down;
        self.clicked = !was_pressed && self.pressed;
    }
}

impl<const N: usize> Default for SmoothBuffer<N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const N: usize> SmoothBuffer<N> {
    pub fn new() -> Self {
        Self {
            buffer: [0.0; N],
            pointer: 0,
        }
    }

    // TODO: Optimize by wrapping pow 2
    pub fn write(&mut self, value: f32) {
        self.buffer[self.pointer] = value;
        self.pointer = (self.pointer + 1) % N;
    }

    // TODO: Implement read with variable smoothing range
    pub fn read(&self) -> f32 {
        let sum: f32 = self.buffer.iter().sum();
        sum / N as f32
    }

    // TODO: Try optimizing with block of zeros
    pub fn reset(&mut self) {
        self.buffer = [0.0; N];
    }

    pub fn traveled(&self) -> f32 {
        let newest = (self.pointer as i32 - 1).rem_euclid(N as i32) as usize;
        let oldest = self.pointer;
        (self.buffer[newest] - self.buffer[oldest]).abs()
    }
}

impl Default for Queue {
    fn default() -> Self {
        Queue { queue: Vec::new() }
    }
}

impl Queue {
    fn len(&self) -> usize {
        self.queue.len()
    }

    fn push(&mut self, action: ControlAction) {
        self.queue.push(action);
    }

    fn pop(&mut self) -> Option<ControlAction> {
        if self.queue.is_empty() {
            None
        } else {
            Some(self.queue.remove(0))
        }
    }

    fn contains(&self, action: &ControlAction) -> bool {
        self.queue.contains(action)
    }

    fn remove_control(&mut self, control_id: usize) {
        self.queue.retain(|a| match a {
            ControlAction::Calibrate(id) => *id != control_id,
            ControlAction::Map(id) => *id != control_id,
        })
    }
}

impl Default for AttributeIdentifier {
    fn default() -> Self {
        Self::None
    }
}

impl AttributeIdentifier {
    fn is_none(&self) -> bool {
        matches!(self, Self::None)
    }
}

impl Default for Calibration {
    fn default() -> Self {
        Calibration {
            offset: 0.0,
            scaling: 1.0,
        }
    }
}

impl Calibration {
    fn try_new(octave_1: f32, octave_2: f32) -> Option<Self> {
        let (bottom, top) = if octave_1 < octave_2 {
            (octave_1, octave_2)
        } else {
            (octave_2, octave_1)
        };

        let distance = top - bottom;
        if distance < 0.5 {
            return None;
        } else if distance > 1.9 {
            return None;
        }

        let scaling = 1.0 / (top - bottom);

        let scaled_bottom_fract = (bottom * scaling).fract();
        let offset = if scaled_bottom_fract > 0.5 {
            1.0 - scaled_bottom_fract
        } else {
            -1.0 * scaled_bottom_fract
        };

        Some(Self { scaling, offset })
    }

    fn apply(&self, value: f32) -> f32 {
        value * self.scaling + self.offset
    }
}

#[cfg(test)]
mod cache_tests {
    use super::*;

    #[test]
    fn it_should_be_possible_to_initialize_cache() {
        let _cache = Cache::new();
    }

    #[cfg(test)]
    mod given_normal_mode {
        use super::*;

        fn init_cache() -> Cache {
            Cache::new()
        }

        #[test]
        fn when_control_is_plugged_then_state_changes_to_mapping() {
            let mut cache = init_cache();
            let mut input = InputSnapshot::default();

            input.control[1] = None;
            cache.apply_input_snapshot(input);

            input.control[1] = Some(1.0);
            cache.apply_input_snapshot(input);

            assert!(matches!(cache.state, State::Mapping(1)));
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
            cache.apply_input_snapshot(input);

            assert!(matches!(
                cache.state,
                State::Calibrating(1, CalibrationPhase::Octave1)
            ));
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
        fn when_pot_is_active_it_gets_mapped_to_the_current_control() {
            let (mut cache, mut input) = init_cache(4);

            input.drive = 0.1;
            apply_input_snapshot(&mut cache, input);
            assert_eq!(cache.mapping[0], AttributeIdentifier::Drive);

            input.speed = 0.1;
            apply_input_snapshot(&mut cache, input);
            assert_eq!(cache.mapping[0], AttributeIdentifier::Drive);
            assert_eq!(cache.mapping[1], AttributeIdentifier::Speed);

            input.dry_wet = 0.1;
            apply_input_snapshot(&mut cache, input);
            assert_eq!(cache.mapping[0], AttributeIdentifier::Drive);
            assert_eq!(cache.mapping[1], AttributeIdentifier::Speed);
            assert_eq!(cache.mapping[2], AttributeIdentifier::DryWet);

            input.bias = 0.1;
            apply_input_snapshot(&mut cache, input);
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

        fn click_button(cache: &mut Cache, mut input: InputSnapshot) {
            input.button = true;
            apply_input_snapshot(cache, input);
            input.button = false;
            apply_input_snapshot(cache, input);
        }

        #[test]
        fn when_correct_values_are_given_it_successfully_converges_and_turns_to_mapping() {
            let (mut cache, mut input) = init_cache(1);
            assert_eq!(
                cache.state,
                State::Calibrating(0, CalibrationPhase::Octave1)
            );

            input.control[0] = Some(1.3);
            apply_input_snapshot(&mut cache, input);
            click_button(&mut cache, input);
            assert_eq!(
                cache.state,
                State::Calibrating(0, CalibrationPhase::Octave2(1.3))
            );

            input.control[0] = Some(2.4);
            apply_input_snapshot(&mut cache, input);
            click_button(&mut cache, input);
            assert_ne!(cache.calibrations[0].offset, Calibration::default().offset);
            assert_ne!(
                cache.calibrations[0].scaling,
                Calibration::default().scaling
            );
            assert_eq!(cache.state, State::Mapping(0));
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

#[cfg(test)]
mod inputs_tests {
    use super::*;

    #[test]
    fn when_input_snapshot_is_written_its_reflected_in_attributes() {
        let mut inputs = Inputs::default();
        inputs.update(InputSnapshot {
            pre_amp: 0.01,
            drive: 0.02,
            bias: 0.03,
            dry_wet: 0.04,
            wow_flut: 0.05,
            speed: 0.06,
            tone: 0.07,
            head: [
                InputSnapshotHead {
                    position: 0.09,
                    volume: 0.10,
                    feedback: 0.11,
                    pan: 0.12,
                },
                InputSnapshotHead {
                    position: 0.13,
                    volume: 0.14,
                    feedback: 0.15,
                    pan: 0.16,
                },
                InputSnapshotHead {
                    position: 0.17,
                    volume: 0.18,
                    feedback: 0.19,
                    pan: 0.20,
                },
                InputSnapshotHead {
                    position: 0.21,
                    volume: 0.22,
                    feedback: 0.23,
                    pan: 0.24,
                },
            ],
            control: [Some(0.25), Some(0.26), Some(0.27), Some(0.28)],
            switch: [true; 10],
            button: true,
        });

        let mut previous = 0.0;
        for value in [
            inputs.pre_amp.value(),
            inputs.drive.value(),
            inputs.bias.value(),
            inputs.dry_wet.value(),
            inputs.wow_flut.value(),
            inputs.speed.value(),
            inputs.tone.value(),
            inputs.head[0].position.value(),
            inputs.head[0].volume.value(),
            inputs.head[0].feedback.value(),
            inputs.head[0].pan.value(),
            inputs.head[1].position.value(),
            inputs.head[1].volume.value(),
            inputs.head[1].feedback.value(),
            inputs.head[1].pan.value(),
            inputs.head[2].position.value(),
            inputs.head[2].volume.value(),
            inputs.head[2].feedback.value(),
            inputs.head[2].pan.value(),
            inputs.head[3].position.value(),
            inputs.head[3].volume.value(),
            inputs.head[3].feedback.value(),
            inputs.head[3].pan.value(),
            inputs.control[0].value(),
            inputs.control[1].value(),
            inputs.control[2].value(),
            inputs.control[3].value(),
        ] {
            assert!(value > previous, "{} !> {}", value, previous);
            previous = value;
        }

        for switch in &inputs.switch {
            assert!(switch);
        }

        assert!(inputs.button.clicked);
    }
}

#[cfg(test)]
mod cv_tests {
    use super::*;

    #[test]
    fn when_none_is_written_its_value_should_be_zero() {
        let mut cv = CV::default();
        cv.update(Some(10.0));
        cv.update(None);
        assert_relative_eq!(cv.value(), 0.0);
    }

    #[test]
    fn when_some_is_being_written_its_value_should_eventually_reach_it() {
        let mut cv = CV::default();

        let mut value = cv.value();
        for _ in 0..20 {
            cv.update(Some(1.0));
            let new_value = cv.value();
            assert!(new_value > value);
            value = new_value;
            if relative_eq!(value, 1.0) {
                return;
            }
        }

        panic!("CV have not reached the target {}", value);
    }

    #[test]
    fn when_some_is_written_after_none_it_reports_as_plugged_for_one_cycle() {
        let mut cv = CV::default();
        cv.update(None);
        cv.update(Some(10.0));
        assert!(cv.was_plugged);
        cv.update(None);
        assert!(!cv.was_plugged);
    }
}

#[cfg(test)]
mod pot_tests {
    use super::*;

    #[test]
    fn when_some_is_being_written_its_value_should_eventually_reach_it() {
        let mut pot = Pot::default();

        let mut value = pot.value();
        for _ in 0..20 {
            pot.update(1.0);
            let new_value = pot.value();
            assert!(new_value > value);
            value = new_value;
            if relative_eq!(value, 1.0) {
                return;
            }
        }

        panic!("CV have not reached the target {}", value);
    }
}

#[cfg(test)]
mod button_test {
    use super::*;

    #[test]
    fn when_was_up_and_now_is_down_it_is_marked_as_clicked() {
        let mut button = Button::default();
        assert!(!button.clicked);
        button.update(true);
        assert!(button.clicked);
        button.update(true);
        assert!(!button.clicked);
        button.update(false);
        assert!(!button.clicked);
    }
}

#[cfg(test)]
mod calibration_test {
    use super::*;

    #[cfg(test)]
    mod with_octave_2_above_octave_1 {
        use super::*;

        #[test]
        fn when_sets_proper_octaves_it_calibrates_properly() {
            let calibration = Calibration::try_new(1.1, 2.3).expect("Calibration failed");
            assert_relative_eq!(calibration.apply(1.1), 1.0);
            assert_relative_eq!(calibration.apply(2.3), 2.0);
        }

        #[test]
        fn when_sets_second_octave_too_close_it_fails() {
            assert!(Calibration::try_new(1.1, 1.3).is_none());
        }

        #[test]
        fn when_sets_second_octave_too_far_it_fails() {
            assert!(Calibration::try_new(1.3, 3.3).is_none());
        }
    }

    #[cfg(test)]
    mod with_octave_2_below_octave_1 {
        use super::*;

        #[test]
        fn when_sets_proper_octaves_it_sets_offset_and_scale_accordingly() {
            let calibration = Calibration::try_new(2.3, 1.1).expect("Calibration failed");
            assert_relative_eq!(calibration.apply(1.1), 1.0);
            assert_relative_eq!(calibration.apply(2.3), 2.0);
        }

        #[test]
        fn when_sets_second_octave_too_close_it_fails() {
            assert!(Calibration::try_new(1.3, 1.1).is_none());
        }

        #[test]
        fn when_sets_second_octave_too_far_it_fails() {
            assert!(Calibration::try_new(3.3, 1.3).is_none());
        }
    }
}
