pub use daisy::flash::Flash;

use kaseta_control::{Save, SaveStore};

const NUM_SECTORS: usize = 2048;

#[allow(clippy::cast_possible_truncation)]
fn sector_address(sector_index: usize) -> u32 {
    (sector_index << 12) as u32
}

pub struct Storage {
    flash: Flash,
    version: u32,
}

impl Storage {
    #[must_use]
    pub fn new(flash: Flash) -> Self {
        Self { flash, version: 0 }
    }

    pub fn save_save(&mut self, save: Save) {
        defmt::info!("Saving version={:?}: {:?}", self.version, save);
        let data = SaveStore::new(save, self.version).to_bytes();
        self.flash
            .write(sector_address(self.version as usize % NUM_SECTORS), &data);
        self.version = self.version.wrapping_add(1);
    }

    #[must_use]
    pub fn load_save(&mut self) -> Save {
        let mut latest_store: Option<SaveStore> = None;

        for i in 0..NUM_SECTORS {
            let mut store_buffer = [0; SaveStore::SIZE];

            self.flash.read(sector_address(i), &mut store_buffer);

            if let Ok(store) = SaveStore::from_bytes(store_buffer) {
                if let Some(latest) = latest_store {
                    if store.version() > latest.version() {
                        latest_store = Some(store);
                    }
                } else {
                    latest_store = Some(store);
                }
            }
        }

        if let Some(latest) = latest_store {
            let save = latest.save();
            defmt::info!("Loaded save version={:?}: {:?}", latest.version(), save);
            self.version = latest.version() + 1;
            save
        } else {
            defmt::info!("No valid save was found");
            Save::default()
        }
    }
}
