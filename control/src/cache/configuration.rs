/// Tweaking of the default module configuration.
///
/// This is mean to allow tweaking of some more niche configuration of the
/// module. Unlike with `Options`, the parameters here may be continuous
/// (float) or offer enumeration of variants. An examle of a configuration
/// may be tweaking of head's rewind speed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Configuration {
    pub rewind_speed: [(usize, usize); 4],
    pub default_display_page: DisplayPage,
    pub position_reset_mapping: PositionResetMapping,
    pub pause_resume_mapping: PauseResumeMapping,
    pub tap_interval_denominator: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum DisplayPage {
    Heads,
    Position,
}

pub type PositionResetMapping = Option<usize>;

pub type PauseResumeMapping = Option<usize>;

impl Configuration {
    pub(crate) fn rewind_speed(&self) -> [(f32, f32); 4] {
        rewind_indices_to_speeds(self.rewind_speed)
    }
}

impl Default for Configuration {
    fn default() -> Self {
        Self {
            rewind_speed: [(0, 0), (1, 1), (2, 2), (3, 3)],
            // TODO: Change this to position. Make sure there is fallback set from the get go
            default_display_page: DisplayPage::Heads,
            position_reset_mapping: None,
            pause_resume_mapping: None,
            tap_interval_denominator: 1,
        }
    }
}

impl DisplayPage {
    pub fn is_heads(&self) -> bool {
        matches!(self, Self::Heads)
    }

    pub fn is_position(&self) -> bool {
        matches!(self, Self::Position)
    }
}

fn rewind_indices_to_speeds(x: [(usize, usize); 4]) -> [(f32, f32); 4] {
    let mut speeds = [(0.0, 0.0); 4];
    for (i, indices) in x.iter().enumerate() {
        speeds[i].0 = rewind_index_to_speed(indices.0);
        speeds[i].1 = fast_forward_index_to_speed(indices.1);
    }
    speeds
}

fn fast_forward_index_to_speed(i: usize) -> f32 {
    [
        -0.25,   // Fifth up
        -0.5,    // Octave up
        -1.4999, // Two octaves up
        -1.9999, // Just fast as hell
    ][i]
}

fn rewind_index_to_speed(i: usize) -> f32 {
    [
        0.125,  // One fifth slowed down
        0.25,   // One octave slowed down
        0.9999, // Same speed backwards. NOTE: Slightly less than 1 to avoid bumps while crossing samples
        1.4999, // One octave up backwards. NOTE: Slightly less than 1.5 to avoid bumps while crossing samples
    ][i]
}
