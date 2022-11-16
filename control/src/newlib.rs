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

#[allow(unused_imports)]
use micromath::F32Ext;

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
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub(crate) enum State {
    Calibrating,
    SelectingControl,
    Normal,
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
    bias: f32,
    dry_wet: f32,
    wow_flut: f32,
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
#[derive(Debug, Default)]
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

#[derive(Debug, Default)]
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
            options: Options::default(),
            configurations: Configuration::default(),
            attributes: Attributes::default(),
            display: Display::new_with_screen(Screen::Placeholder),
        }
    }

    pub fn apply_input_snapshot(&mut self, snapshot: InputSnapshot) -> () {
        self.inputs.update(snapshot);

        self.state = match self.state {
            State::Normal => {
                // TODO: Consider entering another mode
                State::Normal
            }
            State::Calibrating => State::Normal,
            State::SelectingControl => State::Normal,
        };

        // TODO: Set the state
        // TODO: Based on state update options, configuration and attributes
        // TODO: Return config for DSP
        todo!();
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
        let newest = (self.pointer - 1).rem_euclid(N);
        let oldest = self.pointer;
        (self.buffer[newest] - self.buffer[oldest]).abs()
    }
}

#[cfg(test)]
mod cache_tests {
    use super::*;

    #[test]
    fn it_should_be_possible_to_initialize_cache() {
        let _cache = Cache::new();
    }

    // TODO: pass input snapshot, test that inputs were updated accordingly

    #[test]
    fn given_normal_mode_when_control_is_plugged_in_it_enters_select_mode() {
        let mut cache = Cache::new();

        let mut first_input = InputSnapshot::default();
        first_input.control[0] = None;
        cache.apply_input_snapshot(first_input);

        let mut second_input = InputSnapshot::default();
        second_input.control[0] = Some(0.0);
        cache.apply_input_snapshot(second_input);

        assert!(matches!(cache.state, State::Calibrating));
    }

    #[test]
    fn given_normal_mode_when_control_is_plugged_while_holding_button_it_enters_calibration_mode() {
        let mut cache = Cache::new();

        let mut first_input = InputSnapshot::default();
        first_input.control[0] = None;
        first_input.button = false;
        cache.apply_input_snapshot(first_input);

        let mut second_input = InputSnapshot::default();
        second_input.control[0] = Some(0.0);
        second_input.button = true;
        cache.apply_input_snapshot(second_input);

        assert!(matches!(cache.state, State::SelectingControl));
    }

    #[test]
    fn given_normal_mode_when_regular_parameter_is_changed_it_stays_in_normal_state() {
        let mut cache = Cache::new();

        let mut first_input = InputSnapshot::default();
        first_input.drive = 0.1;
        cache.apply_input_snapshot(first_input);
        assert!(matches!(cache.state, State::Normal));

        let mut second_input = InputSnapshot::default();
        second_input.drive = 0.2;
        cache.apply_input_snapshot(second_input);
        assert!(matches!(cache.state, State::Normal));
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
