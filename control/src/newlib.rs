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
    state: State,
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
    // TODO: Read from it, providing variable smoothening (depending on mode and attribute)
}

#[derive(Debug, Default)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
struct CV {
    // TODO: Indicate plug-in
    // TODO: Read from it, providing variable smoothening (depending on mode and attribute)
}

type Switch = bool;

#[derive(Debug, Default)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
struct Button {
    // TODO: Indicate whether it was just clicked
}

/// The current state of the control state machine.
///
/// The module can be in one of multiple states. The behavior of input and
/// output peripherals will differ based on the current state.
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
enum State {
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
#[derive(Debug)]
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

#[derive(Debug)]
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

    pub fn apply_input_snapshot(&mut self, input: InputSnapshot) -> () {
        // TODO: Update self.inputs
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_should_be_possible_to_initialize_cache() {
        let _cache = Cache::new();
    }
}
