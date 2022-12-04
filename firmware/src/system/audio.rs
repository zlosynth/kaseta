use daisy::audio::{self, Interface};

pub const SAMPLE_RATE: u32 = audio::FS.to_Hz();
pub const BLOCK_LENGTH: usize = audio::BLOCK_LENGTH;

pub struct Audio {
    interface: Option<Interface>,
}

impl Audio {
    #[must_use]
    pub fn init(interface: daisy::audio::Interface) -> Self {
        Self {
            interface: Some(interface),
        }
    }

    /// Spawn audio processing.
    ///
    /// # Panics
    ///
    /// Audio processing can be spawned only once. It panics otherwise.
    pub fn spawn(&mut self) {
        self.interface = Some(self.interface.take().unwrap().spawn().unwrap());
    }

    /// Process audio buffer.
    ///
    /// # Panics
    ///
    /// This can panic if executed outside of `DMA1_STR1` interrupt or if it
    /// took too long to process.
    pub fn update_buffer(&mut self, callback: impl FnMut(&mut [(f32, f32); BLOCK_LENGTH])) {
        self.interface
            .as_mut()
            .unwrap()
            .handle_interrupt_dma1_str1(callback)
            .unwrap();
    }
}
