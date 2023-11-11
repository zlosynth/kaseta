use core::mem;

/// State machine representing 8 display LEDs of the module.
///
/// This structure handles the prioritization of display modes, their
/// changing from one to another, or animations.
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Display {
    pub prioritized: [Option<Screen>; 8],
}

#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Screen {
    Dialog(DialogScreen),
    Failure(u32),
    AltAttribute(u32, AltAttributeScreen),
    Attribute(u32, AttributeScreen),
    Clipping(u32),
    Paused(u32),
    BufferReset(u32),
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
    DefaultScreen(usize),
    ControlMapping(Option<usize>),
    TapIntervalDenominator(usize),
}

#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum AltAttributeScreen {
    PreAmpMode(PreAmpMode),
    SpeedRange(SpeedRange),
    FilterPlacement(FilterPlacement),
    HysteresisRange(HysteresisRange),
    WowFlutterPlacement(WowFlutterPlacement),
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
    Long,
    Short,
    Audio,
}

#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum FilterPlacement {
    Input,
    Feedback,
    Both,
}

#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum HysteresisRange {
    Unlimited,
    Limited,
}

#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum WowFlutterPlacement {
    Input,
    Read,
    Both,
}

#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum AttributeScreen {
    HeadsOverview(HeadsOverview),
    Position(usize),
    OctaveOffset(usize),
    OscillatorTone(f32),
    PreAmp(f32),
    Drive(f32),
    Bias(f32),
    DryWet(f32),
    Wow(f32),
    Flutter(f32),
    Speed(f32),
    Tone(f32),
    Volume(usize, f32),
    Feedback(usize, f32),
    Pan(usize, f32),
}

pub type HeadsOverview = ([bool; 4], [bool; 4]);

impl Default for Display {
    fn default() -> Self {
        Self {
            prioritized: [
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                Some(Screen::Attribute(0, AttributeScreen::Position(0))),
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

    pub fn update_attribute(&mut self, attribute: AttributeScreen) {
        let (same_type, age) = 'block: {
            if let Some(Screen::Attribute(age, current_attribute)) = self.prioritized[4] {
                match (attribute, current_attribute) {
                    (AttributeScreen::Volume(i, _), AttributeScreen::Volume(j, _))
                    | (AttributeScreen::Feedback(i, _), AttributeScreen::Feedback(j, _))
                    | (AttributeScreen::Pan(i, _), AttributeScreen::Pan(j, _)) => {
                        if i == j {
                            break 'block (true, age);
                        }
                    }
                    _ => {
                        if mem::discriminant(&current_attribute) == mem::discriminant(&attribute) {
                            break 'block (true, age);
                        }
                    }
                }
            }
            (false, 0)
        };
        if same_type {
            self.set_screen(4, Screen::Attribute(age, attribute));
        }
    }

    pub fn set_buffer_reset(&mut self, progress: u8) {
        self.set_screen(5, Screen::BufferReset(progress as u32));
    }

    pub fn reset_buffer_reset(&mut self) {
        self.reset_screen(5);
    }

    pub fn set_paused(&mut self) {
        match self.prioritized[6] {
            Some(Screen::Paused(_)) => (),
            _ => self.set_screen(6, Screen::Paused(0)),
        }
    }

    pub fn reset_paused(&mut self) {
        self.reset_screen(6);
    }

    pub fn set_fallback_attribute(&mut self, attribute: AttributeScreen) {
        self.set_screen(7, Screen::Attribute(0, attribute));
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
            Self::Paused(cycles) => leds_for_paused(*cycles),
            Self::BufferReset(progress) => leds_for_buffer_reset(*progress),
        }
    }

    fn ticked(self) -> Option<Self> {
        match self {
            Screen::Failure(cycles) => ticked_failure(cycles),
            Screen::Dialog(menu) => Some(Screen::Dialog(ticked_dialog(menu))),
            Screen::AltAttribute(age, alt_attribute) => ticked_alt_attribute(age, alt_attribute),
            Screen::Attribute(age, attribute) => ticked_attribute(age, attribute),
            Screen::Clipping(age) => ticked_clipping(age),
            Screen::Paused(cycles) => ticked_paused(cycles),
            Screen::BufferReset(_) => Some(self),
        }
    }
}

fn ticked_dialog(menu: DialogScreen) -> DialogScreen {
    match menu {
        DialogScreen::Configuration(configuration) => match configuration {
            ConfigurationScreen::Idle(cycles) => ticked_configuration_idle(cycles),
            ConfigurationScreen::Rewind(_) => menu,
            ConfigurationScreen::DefaultScreen(_) => menu,
            ConfigurationScreen::ControlMapping(_) => menu,
            ConfigurationScreen::TapIntervalDenominator(_) => menu,
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

fn ticked_paused(mut cycles: u32) -> Option<Screen> {
    cycles = if cycles > 240 * 8 { 0 } else { cycles + 1 };
    Some(Screen::Paused(cycles))
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
            for i in 0..=*fast_forward {
                leds[i] = true;
            }
            for i in 0..=*rewind {
                leds[leds.len() - 1 - i] = true;
            }
            leds
        }
        ConfigurationScreen::DefaultScreen(selection) => index_to_leds(*selection),
        ConfigurationScreen::ControlMapping(mapping) => {
            let mut leds = [false; 8];
            if let Some(index) = mapping {
                leds[*index] = true;
                leds[*index + 4] = true;
            }
            leds
        }
        ConfigurationScreen::TapIntervalDenominator(denominator) => {
            let index = match *denominator {
                16 => 0,
                8 => 1,
                4 => 2,
                1 => 3,
                _ => unreachable!(),
            };
            index_to_leds(index)
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
            SpeedRange::Long => [true, true, true, true, true, true, true, true],
            SpeedRange::Short => [false, false, true, true, false, false, true, true],
            SpeedRange::Audio => [false, false, false, true, false, false, false, true],
        },
        AltAttributeScreen::FilterPlacement(position) => match position {
            FilterPlacement::Input => [true, false, false, false, true, false, false, false],
            FilterPlacement::Feedback => [false, true, true, true, false, true, true, true],
            FilterPlacement::Both => [true, true, true, true, true, true, true, true],
        },
        AltAttributeScreen::HysteresisRange(range) => match range {
            HysteresisRange::Unlimited => [false, true, false, true, true, false, true, false],
            HysteresisRange::Limited => [false, false, false, false, true, true, true, true],
        },
        AltAttributeScreen::WowFlutterPlacement(placement) => match placement {
            WowFlutterPlacement::Input => [true, false, false, false, true, false, false, false],
            WowFlutterPlacement::Read => [false, true, true, true, false, true, true, true],
            WowFlutterPlacement::Both => [true, true, true, true, true, true, true, true],
        },
    }
}

fn leds_for_attribute(attribute: AttributeScreen) -> [bool; 8] {
    match attribute {
        AttributeScreen::HeadsOverview((top, bottom)) => [
            top[0], top[1], top[2], top[3], bottom[0], bottom[1], bottom[2], bottom[3],
        ],
        AttributeScreen::Position(position) => index_to_leds(position),
        AttributeScreen::OctaveOffset(offset) => {
            let mut leds = [false; 8];
            if offset >= leds.len() {
                unreachable!("The range of octave offset is limited to 4");
            }
            leds[offset] = true;
            leds[offset + 4] = true;
            leds
        }
        AttributeScreen::OscillatorTone(phase)
        | AttributeScreen::PreAmp(phase)
        | AttributeScreen::Drive(phase)
        | AttributeScreen::Bias(phase) => phase_to_leds(phase),
        AttributeScreen::DryWet(phase) => dry_wet_to_leds(phase),
        AttributeScreen::Wow(phase) => wow_to_leds(phase),
        AttributeScreen::Flutter(phase) => flutter_to_leds(phase),
        AttributeScreen::Speed(phase) => speed_to_leds(phase),
        AttributeScreen::Tone(phase) => tone_to_leds(phase),
        AttributeScreen::Volume(position, phase) => volume_to_leds(position, phase),
        AttributeScreen::Feedback(position, phase) => feedback_to_leds(position, phase),
        AttributeScreen::Pan(position, phase) => pan_to_leds(position, phase),
    }
}

fn leds_for_paused(cycles: u32) -> [bool; 8] {
    const INTERVAL: u32 = 240;
    let mut leds = [false; 8];
    let segment = cycles / INTERVAL;
    let on = segment == 0 || segment == 2;
    if on {
        leds[1] = true;
        leds[2] = true;
        leds[5] = true;
        leds[6] = true;
    }
    leds
}

fn leds_for_buffer_reset(progress: u32) -> [bool; 8] {
    let mut leds = [false; 8];
    for i in 0..8 - progress as usize {
        leds[i] = true;
    }
    leds
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

fn index_to_leds(index: usize) -> [bool; 8] {
    let mut leds = [false; 8];
    leds[index] = true;
    leds
}

fn dry_wet_to_leds(phase: f32) -> [bool; 8] {
    let mut leds = [false; 8];

    let wet_len = (phase * 4.9) as usize;

    for led in leds.iter_mut().take(4 - wet_len) {
        *led = true;
    }
    for i in 0..wet_len {
        leds[leds.len() - 1 - i] = true;
    }

    leds
}

fn flutter_to_leds(phase: f32) -> [bool; 8] {
    let mut leds = [false; 8];
    if phase > 0.1 {
        for led in leds.iter_mut().take((phase * 3.9) as usize + 1) {
            *led = true;
        }
    }
    leds
}

fn wow_to_leds(phase: f32) -> [bool; 8] {
    let mut leds = [false; 8];
    if phase > 0.1 {
        for i in 0..=(phase * 3.9) as usize {
            leds[leds.len() - 1 - i] = true;
        }
    }
    leds
}

fn speed_to_leds(phase: f32) -> [bool; 8] {
    let mut leds = [false; 8];
    for i in 0..=(phase * 7.9) as usize {
        leds[leds.len() - 1 - i] = true;
    }
    [
        leds[1], leds[3], leds[5], leds[7], leds[0], leds[2], leds[4], leds[6],
    ]
}

fn tone_to_leds(phase: f32) -> [bool; 8] {
    let mut leds = [true; 8];

    if phase < 0.4 {
        let phase = 1.0 - phase / 0.4;
        for i in 0..=(phase * 7.9) as usize {
            leds[leds.len() - 1 - i] = false;
        }
    } else if phase > 0.6 {
        let phase = (phase - 0.6) / 0.4;
        for i in 0..=(phase * 7.9) as usize {
            leds[i] = false;
        }
    }

    leds
}

fn volume_to_leds(position: usize, phase: f32) -> [bool; 8] {
    let mut leds = [false; 8];
    leds[position] = true;
    if phase < f32::EPSILON {
        return leds;
    }
    for i in 0..=(phase * 3.9) as usize {
        leds[4 + i] = true;
    }
    leds
}

fn feedback_to_leds(position: usize, phase: f32) -> [bool; 8] {
    let mut leds = [false; 8];
    leds[4 + position] = true;
    if phase < f32::EPSILON {
        return leds;
    }
    for i in 0..=(phase * 3.9) as usize {
        leds[i] = true;
    }
    leds
}

fn pan_to_leds(position: usize, phase: f32) -> [bool; 8] {
    let mut leds = [false, false, false, false, true, true, true, true];
    leds[position] = true;
    if phase < 0.4 {
        let phase = 1.0 - phase / 0.4;
        for i in 0..=(phase * 2.9) as usize {
            leds[leds.len() - 1 - i] = false;
        }
    } else if phase > 0.6 {
        let phase = (phase - 0.6) / 0.4;
        for i in 0..=(phase * 2.9) as usize {
            leds[4 + i] = false;
        }
    }
    leds
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
