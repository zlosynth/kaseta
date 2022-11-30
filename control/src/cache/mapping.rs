//! Manage mapping between control input to its designated attribute.

/// Linking between universal control input and attributes controlled through pots.
///
/// This mapping is used to store mapping between control inputs and
/// attributes. It also represents the state machine ordering controls that
/// are yet to be mapped.
pub type Mapping = [AttributeIdentifier; 4];

/// Unique identifier of instrument's attribute.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum AttributeIdentifier {
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

impl Default for AttributeIdentifier {
    fn default() -> Self {
        Self::None
    }
}

impl AttributeIdentifier {
    pub fn is_none(&self) -> bool {
        matches!(self, Self::None)
    }
}
