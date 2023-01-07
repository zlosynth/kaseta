use core::mem;

/// State machine representing 8 display LEDs of the module.
///
/// This structure handles the prioritization of display modes, their
/// changing from one to another, or animations.
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Display {
    pub prioritized: [Option<Screen>; 5],
}

#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Screen {
    Dialog(DialogScreen),
    Failure(u32),
    AltAttribute(u32, AltAttributeScreen),
    Attribute(u32, AttributeScreen),
    Clipping(u32),
}

#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum DialogScreen {
    Calibration(CalibrationScreen),
    Mapping(usize, u32),
    Configuration(ConfigurationScreen),
}

#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum CalibrationScreen {
    SelectOctave1(usize, u32),
    SelectOctave2(usize, u32),
}

#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum ConfigurationScreen {
    Idle(u32),
    Rewind((usize, usize)),
}

#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum AltAttributeScreen {
    PreAmpMode(PreAmpMode),
    SpeedRange(SpeedRange),
    TonePosition(TonePosition),
    HysteresisRange(HysteresisRange),
}

#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum PreAmpMode {
    PreAmp,
    Oscillator,
}

#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum SpeedRange {
    Short,
    Long,
}

#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum TonePosition {
    Volume,
    Feedback,
}

#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum HysteresisRange {
    Unlimited,
    Limited,
}

#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum AttributeScreen {
    Positions(Positions),
    OctaveOffset(usize),
    OscillatorTone(f32),
    PreAmp(f32),
}

pub type Positions = ([bool; 4], [bool; 4]);

impl Default for Display {
    fn default() -> Self {
        Self {
            prioritized: [
                None,
                None,
                None,
                None,
                Some(Screen::Attribute(
                    0,
                    AttributeScreen::Positions(([false; 4], [false; 4])),
                )),
            ],
        }
    }
}

impl Display {
    pub fn tick(&mut self) {
        for screen in self.prioritized.iter_mut().filter(|p| p.is_some()) {
            *screen = screen.unwrap().ticked();
        }
    }

    pub fn active_screen(&self) -> &Screen {
        self.prioritized
            .iter()
            .find_map(Option::as_ref)
            .expect("The always is at least one active page.")
    }

    pub fn set_failure(&mut self) {
        self.set_screen(0, Screen::Failure(0));
    }

    pub fn set_dialog(&mut self, dialog: DialogScreen) {
        self.set_screen(1, Screen::Dialog(dialog));
    }

    pub fn reset_dialog(&mut self) {
        self.reset_screen(1);
    }

    pub fn set_alt_menu(&mut self, alt_menu: AltAttributeScreen) {
        self.set_screen(2, Screen::AltAttribute(0, alt_menu));
    }

    pub fn set_clipping(&mut self) {
        match self.prioritized[3] {
            Some(Screen::Clipping(_)) => (),
            _ => self.set_screen(3, Screen::Clipping(0)),
        }
    }

    pub fn force_attribute(&mut self, attribute: AttributeScreen) {
        self.set_screen(4, Screen::Attribute(0, attribute));
    }

    pub fn set_attribute(&mut self, attribute: AttributeScreen) {
        let same_type_or_empty = 'block: {
            if let Some(Screen::Attribute(_, current_attribute)) = self.prioritized[4] {
                if mem::discriminant(&current_attribute) == mem::discriminant(&attribute) {
                    break 'block true;
                }
            } else {
                break 'block true;
            }
            false
        };
        if same_type_or_empty {
            self.set_screen(4, Screen::Attribute(0, attribute));
        }
    }

    pub fn update_attribute(&mut self, attribute: AttributeScreen) {
        let (same_type, age) = 'block: {
            if let Some(Screen::Attribute(age, current_attribute)) = self.prioritized[4] {
                if mem::discriminant(&current_attribute) == mem::discriminant(&attribute) {
                    break 'block (true, age);
                }
            }
            (false, 0)
        };
        if same_type {
            self.set_screen(4, Screen::Attribute(age, attribute));
        }
    }

    fn set_screen(&mut self, priority: usize, screen: Screen) {
        self.prioritized[priority] = Some(screen);
    }

    fn reset_screen(&mut self, priority: usize) {
        self.prioritized[priority] = None;
    }
}

impl Screen {
    pub fn leds(&self) -> [bool; 8] {
        match self {
            Self::Failure(cycles) => leds_for_failure(*cycles),
            Self::Dialog(dialog) => leds_for_dialog(dialog),
            Self::AltAttribute(_, alt_attribute) => leds_for_alt_attribute(*alt_attribute),
            Self::Attribute(_, attribute) => leds_for_attribute(*attribute),
            Self::Clipping(cycles) => leds_for_clipping(*cycles),
        }
    }

    fn ticked(self) -> Option<Self> {
        match self {
            Screen::Failure(cycles) => ticked_failure(cycles),
            Screen::Dialog(menu) => Some(Screen::Dialog(ticked_dialog(menu))),
            Screen::AltAttribute(age, alt_attribute) => ticked_alt_attribute(age, alt_attribute),
            Screen::Attribute(age, attribute) => ticked_attribute(age, attribute),
            Screen::Clipping(age) => ticked_clipping(age),
        }
    }
}

fn ticked_dialog(menu: DialogScreen) -> DialogScreen {
    match menu {
        DialogScreen::Configuration(configuration) => match configuration {
            ConfigurationScreen::Idle(cycles) => ticked_configuration_idle(cycles),
            ConfigurationScreen::Rewind(rewind) => ticked_configuration_rewind(rewind),
        },
        DialogScreen::Calibration(calibration) => match calibration {
            CalibrationScreen::SelectOctave1(i, cycles) => ticked_calibration_1(i, cycles),
            CalibrationScreen::SelectOctave2(i, cycles) => ticked_calibration_2(i, cycles),
        },
        DialogScreen::Mapping(i, cycles) => ticked_mapping(i, cycles),
    }
}

fn ticked_configuration_idle(mut cycles: u32) -> DialogScreen {
    cycles = if cycles > 1000 { 0 } else { cycles + 1 };
    DialogScreen::Configuration(ConfigurationScreen::Idle(cycles))
}

fn ticked_configuration_rewind(rewind: (usize, usize)) -> DialogScreen {
    DialogScreen::Configuration(ConfigurationScreen::Rewind(rewind))
}

fn ticked_calibration_1(i: usize, mut cycles: u32) -> DialogScreen {
    cycles = if cycles > 240 * 6 { 0 } else { cycles + 1 };
    DialogScreen::Calibration(CalibrationScreen::SelectOctave1(i, cycles))
}

fn ticked_calibration_2(i: usize, mut cycles: u32) -> DialogScreen {
    cycles = if cycles > 240 * 6 { 0 } else { cycles + 1 };
    DialogScreen::Calibration(CalibrationScreen::SelectOctave2(i, cycles))
}

fn ticked_mapping(i: usize, cycles: u32) -> DialogScreen {
    DialogScreen::Mapping(i, if cycles > 240 * 4 { 0 } else { cycles + 1 })
}

fn ticked_failure(cycles: u32) -> Option<Screen> {
    if cycles > 480 {
        None
    } else {
        Some(Screen::Failure(cycles + 1))
    }
}

fn ticked_alt_attribute(age: u32, menu: AltAttributeScreen) -> Option<Screen> {
    if age > 1000 {
        None
    } else {
        Some(Screen::AltAttribute(age + 1, menu))
    }
}

fn ticked_attribute(age: u32, menu: AttributeScreen) -> Option<Screen> {
    if age > 2000 {
        None
    } else {
        Some(Screen::Attribute(age + 1, menu))
    }
}

fn ticked_clipping(age: u32) -> Option<Screen> {
    if age > 120 {
        None
    } else {
        Some(Screen::Clipping(age + 1))
    }
}

fn leds_for_failure(cycles: u32) -> [bool; 8] {
    const INTERVAL_ON: u32 = 80;
    const INTERVAL_OFF: u32 = INTERVAL_ON * 2;
    if cycles < INTERVAL_ON {
        [true; 8]
    } else if cycles < INTERVAL_ON + INTERVAL_OFF {
        [false; 8]
    } else if cycles < INTERVAL_ON + INTERVAL_OFF + INTERVAL_ON {
        [true; 8]
    } else {
        [false; 8]
    }
}

fn leds_for_dialog(menu: &DialogScreen) -> [bool; 8] {
    match menu {
        DialogScreen::Calibration(calibration) => match calibration {
            CalibrationScreen::SelectOctave1(i, cycles) => leds_for_calibration(*i, *cycles, 0),
            CalibrationScreen::SelectOctave2(i, cycles) => leds_for_calibration(*i, *cycles, 1),
        },
        DialogScreen::Mapping(i, cycles) => leds_for_mapping(*i, *cycles),
        DialogScreen::Configuration(configuration) => leds_for_configuration(configuration),
    }
}

fn leds_for_calibration(i: usize, cycles: u32, octave: usize) -> [bool; 8] {
    const INTERVAL: u32 = 240;
    let mut leds = [false; 8];
    leds[4 + i] = true;
    if cycles < INTERVAL {
        leds[octave * 2] = true;
        leds[octave * 2 + 1] = true;
    } else if cycles < INTERVAL * 2 {
        leds[octave * 2] = false;
        leds[octave * 2 + 1] = false;
    } else if cycles < INTERVAL * 3 {
        leds[octave * 2] = true;
        leds[octave * 2 + 1] = true;
    } else {
        leds[octave * 2] = false;
        leds[octave * 2 + 1] = false;
    }
    leds
}

fn leds_for_mapping(i: usize, cycles: u32) -> [bool; 8] {
    const INTERVAL: u32 = 240;
    let mut leds = [false; 8];
    leds[4 + i] = true;
    if cycles < INTERVAL {
        leds[0] = true;
    } else if cycles < INTERVAL * 2 {
        leds[1] = true;
    } else if cycles < INTERVAL * 3 {
        leds[2] = true;
    } else {
        leds[3] = true;
    }
    leds
}

fn leds_for_configuration(configuration: &ConfigurationScreen) -> [bool; 8] {
    match configuration {
        ConfigurationScreen::Idle(cycles) => {
            if *cycles < 500 {
                [true, false, true, false, false, true, false, true]
            } else {
                [false, true, false, true, true, false, true, false]
            }
        }
        ConfigurationScreen::Rewind((rewind, fast_forward)) => {
            let mut leds = [false; 8];
            #[allow(clippy::needless_range_loop)] // Keep it symmetrical with rewind
            for i in 0..=*fast_forward {
                leds[i] = true;
            }
            for i in 0..=*rewind {
                leds[leds.len() - 1 - i] = true;
            }
            leds
        }
    }
}

fn leds_for_alt_attribute(alt_attribute: AltAttributeScreen) -> [bool; 8] {
    match alt_attribute {
        AltAttributeScreen::PreAmpMode(mode) => match mode {
            PreAmpMode::PreAmp => [true, true, false, false, true, true, false, false],
            PreAmpMode::Oscillator => [false, false, true, true, false, false, true, true],
        },
        AltAttributeScreen::SpeedRange(range) => match range {
            SpeedRange::Short => [true, false, false, false, true, false, false, false],
            SpeedRange::Long => [true, true, true, true, true, true, true, true],
        },
        AltAttributeScreen::TonePosition(position) => match position {
            TonePosition::Volume => [true, true, true, true, false, false, false, false],
            TonePosition::Feedback => [false, false, false, false, true, true, true, true],
        },
        AltAttributeScreen::HysteresisRange(range) => match range {
            HysteresisRange::Unlimited => [false, true, false, true, true, false, true, false],
            HysteresisRange::Limited => [false, false, false, false, true, true, true, true],
        },
    }
}

fn leds_for_attribute(attribute: AttributeScreen) -> [bool; 8] {
    match attribute {
        AttributeScreen::Positions((top, bottom)) => [
            top[0], top[1], top[2], top[3], bottom[0], bottom[1], bottom[2], bottom[3],
        ],
        AttributeScreen::OctaveOffset(offset) => {
            let mut leds = [false; 8];
            if offset >= leds.len() {
                unreachable!("The range of octave offset is limited to 4");
            }
            leds[offset] = true;
            leds[offset + 4] = true;
            leds
        }
        AttributeScreen::OscillatorTone(tone) => phase_to_leds(tone),
        AttributeScreen::PreAmp(phase) => phase_to_leds(phase),
    }
}

fn phase_to_leds(phase: f32) -> [bool; 8] {
    let mut leds = [false; 8];
    for led in leds.iter_mut().take((phase * 7.9) as usize + 1) {
        *led = true;
    }
    [
        leds[1], leds[3], leds[5], leds[7], leds[0], leds[2], leds[4], leds[6],
    ]
}

fn leds_for_clipping(cycles: u32) -> [bool; 8] {
    if cycles < 40 {
        [true, true, false, false, true, true, false, false]
    } else if cycles < 80 {
        [true, true, true, false, true, true, true, false]
    } else {
        [true, true, true, true, true, true, true, true]
    }
}

impl DialogScreen {
    pub fn configuration() -> Self {
        DialogScreen::Configuration(ConfigurationScreen::Idle(0))
    }

    pub fn calibration_1(i: usize) -> Self {
        DialogScreen::Calibration(CalibrationScreen::SelectOctave1(i, 0))
    }

    pub fn calibration_2(i: usize) -> Self {
        DialogScreen::Calibration(CalibrationScreen::SelectOctave2(i, 0))
    }

    pub fn mapping(i: usize) -> Self {
        DialogScreen::Mapping(i, 0)
    }
}
