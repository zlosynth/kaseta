use daisy::hal;
use hal::pac::CorePeripherals;
use systick_monotonic::Systick;

pub struct System {
    pub mono: Systick<1000>,
}

impl System {
    #[must_use]
    pub fn init(mut cp: CorePeripherals) -> Self {
        enable_cache(&mut cp);

        let mono = Systick::new(cp.SYST, 480_000_000);

        Self { mono }
    }
}

/// AN5212: Improve application performance when fetching instruction and
/// data, from both internal andexternal memories.
fn enable_cache(cp: &mut CorePeripherals) {
    cp.SCB.enable_icache();
    // NOTE: This requires cache management around all use of DMA.
    cp.SCB.enable_dcache(&mut cp.CPUID);
}
