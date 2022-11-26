//! Components of user inteface, passing user input to DSP and reactions back.
//!
//! It is mainly targetted to run in a firmware with multiple loops running in
//! different frequencies, passing messages from one to another. However, parts
//! of it may be useful in software as well.
//!
//! Following is an example of communication that may be used in an eurorack
//! module:
//! TODO: Fix this based on the new architecture.
//!
//! ```text
//!                    [ DSPLoop ]
//!                       A   |
//!           (DSPAction) |   | (DSPReaction)
//!                       |   V
//!                 [ Reducer {Cache} ] <--------> {Store}
//!                     A        |
//!     (ControlAction) |        | (DisplayReaction)
//!             +-------+        +--+
//!             |                   |
//!             |                   V
//!    [ ControlLoop ]         [ DisplayLoop ]
//!     |     |     |                 |
//!   [CV] [Pots] [Buttons]         [LEDs]
//! ```

#![no_std]
#![allow(clippy::items_after_statements)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_lossless)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::cast_possible_wrap)]
#![allow(clippy::cast_precision_loss)]

#[cfg(test)]
#[macro_use]
extern crate approx;

mod quantization;
mod taper;

use heapless::Vec;
// use kaseta_dsp::processor::{Attributes, Reaction as DSPReaction};
#[allow(unused_imports)]
use micromath::F32Ext;

use crate::quantization::{quantize, Quantization};

// This is a temporary draft of the new control architecture.
//
// - [X] Define stubs of all needed structs
// - [X] Write documentation of these structs
// - [X] Implement defmt display for all
// - [X] Implement tests stub
// - [X] Write initializer of cache
// - [X] Implement raw input passing
// - [X] Implement pot processing with smoothening
// - [X] Implement control processing with plug-in
// - [X] Implement translation to attributes
// - [X] Implement passing of attributes to pseudo-DSP
// - [X] Implement passing of config and options to pseudo-DSP
// - [X] Implement impulse output
// - [X] Implement display for impulse output
// - [X] Implement control select
// - [X] Implement calibration
// - [X] Unify cv naming to Control
// - [X] Unify control select naming to mapping
// - [X] Implement display output for basic attributes
// - [X] Implement display for calibration and mapping
// - [X] Implement backup snapshoting (all data needed for restore)
// - [X] Implement reset, connected controls must be reassigned
// - [X] Implement saving, returned from Cache when a change to it was made
// - [X] Implement Configuration passing
// - [X] Detect 4 consecutive clicks setting a tempo and set it in attributes
// - [X] Detect that control input is getting clock triggers and reflect that
// - [ ] Use this instead of the current lib binding, update automation
// - [ ] Divide this into submodules
// - [ ] List all the improvements introduced here in the changelog
// - [ ] Remove this TODO list and release a new version

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
    control: [Control; 4],
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
struct Control {
    pub is_plugged: bool,
    pub was_plugged: bool,
    pub was_unplugged: bool,
    buffer: SmoothBuffer<4>,
    trigger_age: [u32; 3],
    pub detected_tempo: Option<u32>,
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
    pub held: u32,
    clicks_age: [u32; 3],
    pub detected_tempo: Option<u32>,
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

/// Linking between universal control input and attributes controlled through pots.
//
/// This mapping is used to store mapping between control inputs and
/// attributes. It also represents the state machine ordering controls that
/// are yet to be mapped.
type Mapping = [AttributeIdentifier; 4];

#[derive(Debug, PartialEq, Clone, Copy)]
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
#[derive(Debug, Clone, Copy, PartialEq)]
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
    impulse_trigger: TriggerOutput,
    impulse_led: Led,
    display: Display,
}

/// TODO docs
#[derive(Debug, Default)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
struct TriggerOutput {
    since: u32,
}

/// TODO docs
#[derive(Debug, Default)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
struct Led {
    since: u32,
}

/// State machine representing 8 display LEDs of the module.
///
/// This structure handles the prioritization of display modes, their
/// changing from one to another, or animations.
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
struct Display {
    prioritized: [Option<Screen>; 3],
}

#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
enum Screen {
    Calibration(CalibrationScreen),
    Mapping(usize, u32),
    Configuration(ConfigurationScreen),
    Failure(u32),
    Heads([bool; 4], [bool; 4]),
}

#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
enum CalibrationScreen {
    SelectOctave1(usize, u32),
    SelectOctave2(usize, u32),
}

#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
enum ConfigurationScreen {
    Idle(u32),
    Rewind((usize, usize)),
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

/// The current state of all peripherals.
///
/// InputSnapshot is meant to be passed from the hardware binding to the
/// control package. It should pass pretty raw data, with two exceptions:
///
/// 1. Detection of plugged control input is done by the caller.
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

/// TODO: Docs
// TODO: Move this under DSP module
#[derive(Debug, Default)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct DSPAttributes {
    pub pre_amp: f32,
    pub drive: f32,
    pub saturation: f32,
    pub bias: f32,
    pub dry_wet: f32,
    pub wow: f32,
    pub flutter: f32,
    pub speed: f32,
    pub tone: f32,
    pub head: [DSPAttributesHead; 4],
    pub rewind: bool,
    // TODO
    #[allow(dead_code)]
    rewind_speed: [(f32, f32); 4],
}

#[derive(Debug, Default)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct DSPAttributesHead {
    pub position: f32,
    pub volume: f32,
    pub feedback: f32,
    pub pan: f32,
}

/// TODO: Docs
// TODO: Move this under DSP module
#[derive(Debug, Default, Clone, Copy)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct DSPReaction {
    pub impulse: bool,
    pub clipping: bool,
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
impl Cache {
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
        let save = self.reconcile_changed_inputs();
        let dsp_attributes = self.build_dsp_attributes();
        (dsp_attributes, save)
    }

    fn reconcile_changed_inputs(&mut self) -> Option<Save> {
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
                if let Some(detected_tempo) = self.inputs.button.detected_tempo {
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
                    let mut active_configuration_screen = None;
                    for head in self.inputs.head.iter() {
                        if head.position.active() || head.feedback.active() {
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

        self.reconcile_pre_amp();
        self.reconcile_hysteresis();
        self.reconcile_wow_flut();
        self.reconcile_tone();
        self.reconcile_speed();
        self.reconcile_heads();

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

        self.outputs.display.prioritized[2] = Some(Screen::Heads(
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
        ));

        if needs_save {
            Some(self.save())
        } else {
            None
        }
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
        if self.inputs.speed.active() {
            self.tapped_tempo = None;

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
        } else if let Some(tapped_tempo) = self.tapped_tempo {
            self.attributes.speed = tapped_tempo;
        }
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
        if dsp_reaction.impulse {
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
        self.buffer.traveled().abs() > 0.001
    }
}

impl Control {
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

        for x in self.trigger_age.iter_mut() {
            *x = x.saturating_add(1);
        }

        if self.buffer.read_raw() < 0.45 {
            self.trigger_age = [0, 0, 0];
        }

        let triggered = self.buffer.read_raw() > 0.9 && self.buffer.traveled() > 0.3;
        if triggered {
            let minus_1 = self.trigger_age[2];
            let minus_2 = self.trigger_age[1];
            let minus_3 = self.trigger_age[0];

            let distance = minus_1;
            let tolerance = distance / 20;
            if distance > 100
                && (minus_2 - minus_1) > (distance - tolerance)
                && (minus_2 - minus_1) < (distance + tolerance)
                && (minus_3 - minus_2) > (distance - tolerance)
                && (minus_3 - minus_2) < (distance + tolerance)
            {
                self.detected_tempo = Some(distance);
            } else {
                self.detected_tempo = None;
            }

            self.trigger_age[0] = self.trigger_age[1];
            self.trigger_age[1] = self.trigger_age[2];
            self.trigger_age[2] = 0;
        }
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
        self.held = if self.pressed {
            self.held.saturating_add(1)
        } else {
            0
        };

        for x in self.clicks_age.iter_mut() {
            *x = x.saturating_add(1);
        }

        if self.clicked {
            let minus_1 = self.clicks_age[2];
            let minus_2 = self.clicks_age[1];
            let minus_3 = self.clicks_age[0];

            let distance = minus_1;
            let tolerance = distance / 20;
            if distance > 100
                && (minus_2 - minus_1) > (distance - tolerance)
                && (minus_2 - minus_1) < (distance + tolerance)
                && (minus_3 - minus_2) > (distance - tolerance)
                && (minus_3 - minus_2) < (distance + tolerance)
            {
                self.detected_tempo = Some(distance);
            } else {
                self.detected_tempo = None;
            }

            self.clicks_age[0] = self.clicks_age[1];
            self.clicks_age[1] = self.clicks_age[2];
            self.clicks_age[2] = 0;
        }
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

    pub fn read_raw(&self) -> f32 {
        let newest = (self.pointer as i32 - 1).rem_euclid(N as i32) as usize;
        self.buffer[newest]
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
    fn push(&mut self, action: ControlAction) {
        // NOTE: The capacity is set to accomodate for all possible actions.
        let _ = self.queue.push(action);
    }

    fn pop(&mut self) -> Option<ControlAction> {
        if self.queue.is_empty() {
            None
        } else {
            Some(self.queue.remove(0))
        }
    }

    fn remove_control(&mut self, control_id: usize) {
        self.queue.retain(|a| match a {
            ControlAction::Calibrate(id) => *id != control_id,
            ControlAction::Map(id) => *id != control_id,
        })
    }

    #[cfg(test)]
    fn len(&self) -> usize {
        self.queue.len()
    }

    #[cfg(test)]
    fn contains(&self, action: &ControlAction) -> bool {
        self.queue.contains(action)
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

impl Default for State {
    fn default() -> Self {
        Self::Normal
    }
}

impl Default for Screen {
    fn default() -> Self {
        Screen::Heads([false; 4], [false; 4])
    }
}

impl TriggerOutput {
    fn trigger(&mut self) {
        self.since = 0;
    }

    fn tick(&mut self) {
        self.since = self.since.saturating_add(1);
    }

    fn triggered(&self) -> bool {
        self.since < 10
    }
}

impl Led {
    fn trigger(&mut self) {
        self.since = 0;
    }

    fn tick(&mut self) {
        self.since = self.since.saturating_add(1);
    }

    fn triggered(&self) -> bool {
        self.since < 100
    }
}

impl Default for Display {
    fn default() -> Self {
        Self {
            prioritized: [None, None, Some(Screen::Heads([false; 4], [false; 4]))],
        }
    }
}

impl Display {
    fn tick(&mut self) {
        for screen in self.prioritized.iter_mut().filter(|p| p.is_some()) {
            *screen = screen.unwrap().ticked();
        }
    }

    fn active_screen(&self) -> &Screen {
        self.prioritized
            .iter()
            .find(|p| p.is_some())
            .map(|p| p.as_ref().unwrap())
            .expect("The always is at least one active page.")
    }
}

impl Screen {
    fn leds(&self) -> [bool; 8] {
        match self {
            Self::Calibration(calibration) => match calibration {
                CalibrationScreen::SelectOctave1(i, cycles) => {
                    let mut leds = [false; 8];
                    leds[4 + i] = true;
                    if *cycles < 240 {
                        leds[0] = true;
                        leds[1] = true;
                    } else if *cycles < 240 * 2 {
                        leds[0] = false;
                        leds[1] = false;
                    } else if *cycles < 240 * 3 {
                        leds[0] = true;
                        leds[1] = true;
                    } else {
                        leds[0] = false;
                        leds[1] = false;
                    }
                    leds
                }
                CalibrationScreen::SelectOctave2(i, cycles) => {
                    let mut leds = [false; 8];
                    leds[4 + i] = true;
                    if *cycles < 240 {
                        leds[2] = true;
                        leds[3] = true;
                    } else if *cycles < 240 * 2 {
                        leds[2] = false;
                        leds[3] = false;
                    } else if *cycles < 240 * 3 {
                        leds[2] = true;
                        leds[3] = true;
                    } else {
                        leds[2] = false;
                        leds[3] = false;
                    }
                    leds
                }
            },
            Self::Mapping(i, cycles) => {
                let mut leds = [false; 8];
                leds[4 + i] = true;
                if *cycles < 240 {
                    leds[0] = true;
                } else if *cycles < 240 * 2 {
                    leds[1] = true;
                } else if *cycles < 240 * 3 {
                    leds[2] = true;
                } else {
                    leds[3] = true;
                }
                leds
            }
            Self::Configuration(configuration) => match configuration {
                ConfigurationScreen::Idle(cycles) => {
                    if *cycles < 500 {
                        [true, false, true, false, false, true, false, true]
                    } else {
                        [false, true, false, true, true, false, true, false]
                    }
                }
                ConfigurationScreen::Rewind((rewind, fast_forward)) => {
                    let mut leds = [false; 8];
                    for i in 0..=*fast_forward {
                        leds[i] = true;
                    }
                    for i in 0..=*rewind {
                        leds[leds.len() - 1 - i] = true;
                    }
                    leds
                }
            },
            Self::Heads(top, bottom) => [
                top[0], top[1], top[2], top[3], bottom[0], bottom[1], bottom[2], bottom[3],
            ],
            Self::Failure(cycles) => {
                if *cycles < 80 {
                    [true; 8]
                } else if *cycles < 240 {
                    [false; 8]
                } else if *cycles < 320 {
                    [true; 8]
                } else {
                    [false; 8]
                }
            }
        }
    }

    fn ticked(self) -> Option<Self> {
        match self {
            Screen::Configuration(configuration) => match configuration {
                ConfigurationScreen::Idle(cycles) => Some(Screen::Configuration(
                    ConfigurationScreen::Idle(if cycles > 1000 { 0 } else { cycles + 1 }),
                )),
                ConfigurationScreen::Rewind(_) => Some(self),
            },
            Screen::Calibration(calibration) => match calibration {
                CalibrationScreen::SelectOctave1(i, cycles) => {
                    Some(Screen::Calibration(CalibrationScreen::SelectOctave1(
                        i,
                        if cycles > 240 * 6 { 0 } else { cycles + 1 },
                    )))
                }
                CalibrationScreen::SelectOctave2(i, cycles) => {
                    Some(Screen::Calibration(CalibrationScreen::SelectOctave2(
                        i,
                        if cycles > 240 * 6 { 0 } else { cycles + 1 },
                    )))
                }
            },
            Screen::Mapping(i, cycles) => Some(Screen::Mapping(
                i,
                if cycles > 240 * 4 { 0 } else { cycles + 1 },
            )),
            Screen::Failure(cycles) => {
                if cycles > 480 {
                    None
                } else {
                    Some(Screen::Failure(cycles + 1))
                }
            }
            _ => Some(self),
        }
    }

    fn failure() -> Self {
        Self::Failure(0)
    }

    fn configuration() -> Self {
        Self::Configuration(ConfigurationScreen::Idle(0))
    }

    fn calibration_1(i: usize) -> Self {
        Self::Calibration(CalibrationScreen::SelectOctave1(i, 0))
    }

    fn calibration_2(i: usize) -> Self {
        Self::Calibration(CalibrationScreen::SelectOctave2(i, 0))
    }

    fn mapping(i: usize) -> Self {
        Self::Mapping(i, 0)
    }
}

#[cfg(test)]
mod cache_tests {
    use super::*;

    #[test]
    fn it_should_be_possible_to_initialize_cache() {
        let _cache = Cache::new();
    }

    fn assert_animation(cache: &mut Cache, transitions: &[u32]) {
        fn u32_into_digits(mut x: u32) -> [u32; 4] {
            assert!(x < 10000);

            let a = x % 10;
            x /= 10;
            let b = x % 10;
            x /= 10;
            let c = x % 10;
            x /= 10;
            let d = x % 10;

            [d, c, b, a]
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

        dsp_reaction.impulse = true;
        cache.apply_dsp_reaction(dsp_reaction);

        dsp_reaction.impulse = false;
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

        dsp_reaction.impulse = true;
        cache.apply_dsp_reaction(dsp_reaction);

        dsp_reaction.impulse = false;
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
mod control_tests {
    use super::*;

    #[test]
    fn when_none_is_written_its_value_should_be_zero() {
        let mut cv = Control::default();
        cv.update(Some(10.0));
        cv.update(None);
        assert_relative_eq!(cv.value(), 0.0);
    }

    #[test]
    fn when_some_is_being_written_its_value_should_eventually_reach_it() {
        let mut cv = Control::default();

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

        panic!("Control have not reached the target {}", value);
    }

    #[test]
    fn when_some_is_written_after_none_it_reports_as_plugged_for_one_cycle() {
        let mut control = Control::default();
        control.update(None);
        control.update(Some(10.0));
        assert!(control.was_plugged);
        control.update(None);
        assert!(!control.was_plugged);
    }

    #[test]
    fn when_steady_clock_passes_in_it_detects_tempo() {
        let mut control = Control::default();
        control.update(Some(1.0));
        for _ in 0..1999 {
            control.update(Some(0.5));
        }
        control.update(Some(1.0));
        for _ in 0..1999 {
            control.update(Some(0.5));
        }
        control.update(Some(1.0));
        for _ in 0..1999 {
            control.update(Some(0.5));
        }
        control.update(Some(1.0));
        assert_eq!(control.detected_tempo.unwrap(), 2000);
    }

    #[test]
    fn when_clock_within_toleration_passes_it_detects_tempo() {
        let mut control = Control::default();
        control.update(Some(1.0));
        for _ in 0..1990 {
            control.update(Some(0.5));
        }
        control.update(Some(1.0));
        for _ in 0..2030 {
            control.update(Some(0.5));
        }
        control.update(Some(1.0));
        for _ in 0..1999 {
            control.update(Some(0.5));
        }
        control.update(Some(1.0));
        assert_eq!(control.detected_tempo.unwrap(), 2000);
    }

    #[test]
    fn when_unevenly_spaced_triggers_are_given_it_is_not_recognized_as_tempo() {
        let mut control = Control::default();
        control.update(Some(1.0));
        for _ in 0..1999 {
            control.update(Some(0.5));
        }
        control.update(Some(1.0));
        for _ in 0..930 {
            control.update(Some(0.5));
        }
        control.update(Some(1.0));
        for _ in 0..1999 {
            control.update(Some(0.5));
        }
        control.update(Some(1.0));
        assert!(control.detected_tempo.is_none());
    }

    #[test]
    fn when_signal_goes_below_zero_it_is_not_recognized_as_clock() {
        let mut control = Control::default();
        control.update(Some(1.0));
        for _ in 0..1999 {
            control.update(Some(0.1));
        }
        control.update(Some(1.0));
        for _ in 0..1999 {
            control.update(Some(0.1));
        }
        control.update(Some(1.0));
        for _ in 0..1999 {
            control.update(Some(0.1));
        }
        control.update(Some(1.0));
        assert!(control.detected_tempo.is_none());
    }

    #[test]
    fn when_signal_has_fast_enough_attacks_it_is_recognized_as_clock() {
        let mut control = Control::default();

        fn attack(control: &mut Control, should_detect: bool) {
            let mut detected = false;
            let attack = 3;
            for i in 0..=attack {
                control.update(Some(0.5 + 0.5 * (i as f32 / attack as f32)));
                detected |= control.detected_tempo.is_some();
            }
            assert_eq!(should_detect, detected, "{:?}", control);
        }

        fn silence(control: &mut Control) {
            for _ in 0..1999 {
                control.update(Some(0.5));
                assert!(control.detected_tempo.is_none());
            }
        }

        attack(&mut control, false);
        silence(&mut control);
        attack(&mut control, false);
        silence(&mut control);
        attack(&mut control, false);
        silence(&mut control);
        attack(&mut control, true);
    }

    #[test]
    fn when_signal_does_not_have_fast_attacks_it_is_not_recognized_as_clock() {
        let mut control = Control::default();

        fn attack(control: &mut Control) {
            for i in 0..200 {
                control.update(Some(0.5 + 0.5 * (i as f32 / 200.0)));
                assert!(control.detected_tempo.is_none());
            }
        }

        fn silence(control: &mut Control) {
            for _ in 0..1999 {
                control.update(Some(0.1));
                assert!(control.detected_tempo.is_none());
            }
        }

        attack(&mut control);
        silence(&mut control);
        attack(&mut control);
        silence(&mut control);
        attack(&mut control);
        silence(&mut control);
        attack(&mut control);
        silence(&mut control);
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

        panic!("Control have not reached the target {}", value);
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

    #[test]
    fn when_is_down_it_reports_how_many_cycles() {
        let mut button = Button::default();
        assert_eq!(button.held, 0);
        button.update(false);
        assert_eq!(button.held, 0);
        button.update(true);
        assert_eq!(button.held, 1);
        button.update(true);
        assert_eq!(button.held, 2);
        button.update(true);
        assert_eq!(button.held, 3);
        button.update(false);
        assert_eq!(button.held, 0);
    }

    #[test]
    fn when_clicked_in_exact_interval_it_detects_tempo() {
        let mut button = Button::default();
        for _ in 0..4 {
            for _ in 0..1999 {
                button.update(false);
            }
            button.update(true);
        }
        assert_eq!(button.detected_tempo, Some(2000));
    }

    #[test]
    fn when_clicked_in_rough_interval_within_toleration_it_detects_tempo() {
        let mut button = Button::default();
        button.update(true);
        for _ in 0..1990 {
            button.update(false);
        }
        button.update(true);
        for _ in 0..2059 {
            button.update(false);
        }
        button.update(true);
        for _ in 0..1999 {
            button.update(false);
        }
        button.update(true);
        assert_eq!(button.detected_tempo, Some(2000));
    }

    #[test]
    fn when_clicked_too_fast_it_does_not_detect_tempo() {
        let mut button = Button::default();
        for _ in 0..4 {
            for _ in 0..19 {
                button.update(false);
            }
            button.update(true);
        }
        assert_eq!(button.detected_tempo, None);
    }

    #[test]
    fn when_clicked_in_unequal_interval_it_does_not_detect_tempo() {
        let mut button = Button::default();
        button.update(true);
        for _ in 0..1999 {
            button.update(false);
        }
        button.update(true);
        for _ in 0..1999 {
            button.update(false);
        }
        button.update(true);
        for _ in 0..1083 {
            button.update(false);
        }
        button.update(true);
        assert_eq!(button.detected_tempo, None);
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

// TODO: Test that control tempo gets combined correctly with speed knob
