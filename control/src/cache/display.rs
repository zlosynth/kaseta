/// State machine representing 8 display LEDs of the module.
///
/// This structure handles the prioritization of display modes, their
/// changing from one to another, or animations.
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Display {
    pub prioritized: [Option<Screen>; 3],
}

#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Screen {
    Calibration(CalibrationScreen),
    Mapping(usize, u32),
    Configuration(ConfigurationScreen),
    Failure(u32),
    Heads([bool; 4], [bool; 4]),
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

impl Default for Display {
    fn default() -> Self {
        Self {
            prioritized: [None, None, Some(Screen::Heads([false; 4], [false; 4]))],
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

    pub fn set_screen(&mut self, priority: usize, screen: Screen) {
        self.prioritized[priority] = Some(screen);
    }

    pub fn reset_screen(&mut self, priority: usize) {
        self.prioritized[priority] = None;
    }
}

impl Default for Screen {
    fn default() -> Self {
        Screen::Heads([false; 4], [false; 4])
    }
}

impl Screen {
    pub fn leds(&self) -> [bool; 8] {
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
                    #[allow(clippy::needless_range_loop)] // Keep it symmetrical with rewind
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
            Screen::Heads(_, _) => Some(self),
        }
    }

    pub fn failure() -> Self {
        Self::Failure(0)
    }

    pub fn configuration() -> Self {
        Self::Configuration(ConfigurationScreen::Idle(0))
    }

    pub fn calibration_1(i: usize) -> Self {
        Self::Calibration(CalibrationScreen::SelectOctave1(i, 0))
    }

    pub fn calibration_2(i: usize) -> Self {
        Self::Calibration(CalibrationScreen::SelectOctave2(i, 0))
    }

    pub fn mapping(i: usize) -> Self {
        Self::Mapping(i, 0)
    }
}
