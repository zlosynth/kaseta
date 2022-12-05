use core::mem;

use crc::{Crc, CRC_16_USB};

use crate::cache::mapping::Mapping;
use crate::cache::{Calibrations, Configuration, TappedTempo};

/// TODO: Docs
#[derive(Debug, Default, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Save {
    pub mapping: Mapping,
    pub calibrations: Calibrations,
    pub configuration: Configuration,
    pub tapped_tempo: TappedTempo,
}

impl Save {
    const SIZE: usize = mem::size_of::<Self>();

    fn from_bytes(bytes: [u8; Self::SIZE]) -> Self {
        unsafe { mem::transmute(bytes) }
    }

    fn to_bytes(self) -> [u8; Self::SIZE] {
        unsafe { mem::transmute(self) }
    }
}

// This constant is used to invalidate data when needed
const TOKEN: u16 = 1;
const CRC: Crc<u16> = Crc::<u16>::new(&CRC_16_USB);
pub struct InvalidData;

#[derive(Clone, Copy)]
pub struct Store {
    version: u32,
    token: u16,
    save_raw: [u8; Save::SIZE],
    crc: u16,
}

impl Store {
    pub const SIZE: usize = mem::size_of::<Self>();

    #[must_use]
    pub fn new(save: Save, version: u32) -> Self {
        let save_raw = save.to_bytes();
        let crc = CRC.checksum(&save_raw);
        Self {
            version,
            save_raw,
            crc,
            token: TOKEN,
        }
    }

    /// # Errors
    ///
    /// This fails with `InvalidData` when recovered save does not pass CRC
    /// check.
    pub fn from_bytes(bytes: [u8; Self::SIZE]) -> Result<Self, InvalidData> {
        let store: Self = unsafe { mem::transmute(bytes) };

        if store.token != TOKEN {
            return Err(InvalidData);
        }

        let crc = CRC.checksum(&store.save_raw);
        if crc == store.crc {
            Ok(store)
        } else {
            Err(InvalidData)
        }
    }

    #[must_use]
    pub fn to_bytes(self) -> [u8; Self::SIZE] {
        unsafe { mem::transmute(self) }
    }

    #[must_use]
    pub fn save(&self) -> Save {
        Save::from_bytes(self.save_raw)
    }

    #[must_use]
    pub fn version(&self) -> u32 {
        self.version
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initialize_store() {
        let _store = Store::new(Save::default(), 0);
    }

    #[test]
    fn get_save_from_store() {
        let save = Save::default();
        let store = Store::new(save, 0);
        assert!(store.save() == save);
    }

    #[test]
    fn get_version_from_store() {
        let store = Store::new(Save::default(), 10);
        assert_eq!(store.version(), 10);
    }

    #[test]
    fn initialize_store_from_bytes() {
        let store_a = Store::new(Save::default(), 0);
        let bytes = store_a.to_bytes();
        let store_b = Store::from_bytes(bytes).ok().unwrap();
        assert!(store_a.save() == store_b.save());
    }

    #[test]
    fn detect_invalid_crc_while_initializing_from_bytes() {
        let store = Store::new(Save::default(), 0);
        let mut bytes = store.to_bytes();
        bytes[5] = 0x13;
        assert!(Store::from_bytes(bytes).is_err());
    }

    #[test]
    fn dump_store_as_bytes() {
        let save_a = Save {
            tapped_tempo: Some(1.0),
            ..Save::default()
        };
        let store_a = Store::new(save_a, 0);
        let bytes_a = store_a.to_bytes();

        let save_b = Save {
            tapped_tempo: Some(2.0),
            ..Save::default()
        };
        let store_b = Store::new(save_b, 0);
        let bytes_b = store_b.to_bytes();

        assert!(bytes_a != bytes_b);
    }

    #[test]
    fn store_fits_into_one_page() {
        let page_size = 256;
        let store_size = mem::size_of::<Store>();
        assert!(store_size < page_size);
    }
}
