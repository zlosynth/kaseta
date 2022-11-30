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
