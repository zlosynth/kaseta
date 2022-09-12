//! Useful to increase nyquist frequency before non-linear manipulation.
//!
//! # Example
//!
//! ```
//! use sirena::signal::{self, Signal};
//! use kaseta_dsp::oversampling::{SignalDownsample, SignalUpsample, Downsampler4, Upsampler4};
//! use sirena::memory_manager::MemoryManager;
//! use core::mem::MaybeUninit;
//!
//! static mut MEMORY: [MaybeUninit<u32>; 512] = unsafe { MaybeUninit::uninit().assume_init() };
//! let mut memory_manager = MemoryManager::from(unsafe { &mut MEMORY[..] });
//! let mut upsampler = Upsampler4::new_4(&mut memory_manager);
//! let mut downsampler = Downsampler4::new_4(&mut memory_manager);
//!
//! let processed_signal = signal::sine(48000.0, 200.0)
//!      .upsample(&mut upsampler)
//!      // .nonlinear_processing()
//!      .downsample(&mut downsampler);
//! ```

mod coefficients;
pub mod downsampling;
pub mod upsampling;

pub use downsampling::{Downsampler4, SignalDownsample};
pub use upsampling::{SignalUpsample, Upsampler4};

#[cfg(test)]
mod tests {
    use super::*;
    use core::mem::MaybeUninit;
    use heapless::Vec;
    use sirena::memory_manager::MemoryManager;

    #[test]
    fn given_oversampled_signal_with_tone_above_original_nyquist_when_downsampling_it_removes_the_tone(
    ) {
        use sirena::signal::{self, Signal, SignalTake};
        use sirena::spectral_analysis::SpectralAnalysis;

        static mut MEMORY: [MaybeUninit<u32>; 512] = unsafe { MaybeUninit::uninit().assume_init() };
        let mut memory_manager = MemoryManager::from(unsafe { &mut MEMORY[..] });

        const FS: f32 = 1024.0;
        const NYQUIST: f32 = FS / 2.0 - 1.0;
        const SAMPLES: usize = 1024;
        const OVERSAMPLING: usize = 4;

        let mut downsampler = Downsampler4::new_4(&mut memory_manager);

        // Downsample oversampled signal with sine over original nyquist rate
        // and store it in a buffer.
        let buffer: [f32; SAMPLES] = signal::sine(OVERSAMPLING as f32 * FS, NYQUIST * 2.0)
            .downsample(&mut downsampler)
            .by_ref()
            .take(SAMPLES)
            .collect::<Vec<_, SAMPLES>>()
            .as_slice()
            .try_into()
            .unwrap();

        let analysis = SpectralAnalysis::analyze(&buffer, FS as u32);
        assert!(analysis.mean_magnitude(0.0, NYQUIST) < 1.0);
    }

    #[test]
    fn given_signal_when_upsample_and_downsample_it_retains_original_signal_and_amplitude() {
        use sirena::signal::{self, Signal, SignalTake};
        use sirena::spectral_analysis::SpectralAnalysis;

        static mut MEMORY: [MaybeUninit<u32>; 512] = unsafe { MaybeUninit::uninit().assume_init() };
        let mut memory_manager = MemoryManager::from(unsafe { &mut MEMORY[..] });

        const FS: f32 = 1024.0;
        const NYQUIST: f32 = FS / 2.0 - 1.0;
        const SAMPLES: usize = 1024;

        let mut upsampler = Upsampler4::new_4(&mut memory_manager);
        let mut downsampler = Downsampler4::new_4(&mut memory_manager);

        let signal = signal::sine(FS, NYQUIST / 2.0);

        let original_buffer: [f32; SAMPLES] = signal
            .clone()
            .by_ref()
            .take(SAMPLES)
            .collect::<Vec<_, SAMPLES>>()
            .as_slice()
            .try_into()
            .unwrap();
        let processed_buffer: [f32; SAMPLES] = signal
            .upsample(&mut upsampler)
            .downsample(&mut downsampler)
            .by_ref()
            .take(SAMPLES)
            .collect::<Vec<_, SAMPLES>>()
            .as_slice()
            .try_into()
            .unwrap();

        let original_amplitude = original_buffer
            .iter()
            .fold(0.0, |a, b| f32::max(a, f32::abs(*b)));
        let processed_amplitude = processed_buffer
            .iter()
            .fold(0.0, |a, b| f32::max(a, f32::abs(*b)));
        assert_relative_eq!(original_amplitude, processed_amplitude, epsilon = 0.05);

        let original_analysis = SpectralAnalysis::analyze(&original_buffer, FS as u32);
        let processed_analysis = SpectralAnalysis::analyze(&processed_buffer, FS as u32);
        assert_relative_eq!(
            original_analysis.strongest_peak(),
            processed_analysis.strongest_peak(),
            epsilon = 1.0
        );
        assert_relative_eq!(
            original_analysis.mean_magnitude(0.0, NYQUIST),
            processed_analysis.mean_magnitude(0.0, NYQUIST),
            max_relative = 0.1
        );
    }
}
