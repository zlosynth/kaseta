//! Manage schedule of multiple control actions.
//!
//! This covers mapping and calibration of control inputs. While only one can
//! happen at the time, multiple types for multiple control inputs can be
//! waiting in line.

use heapless::Vec;

/// The queue of control inputs waiting for mapping or calibration.
///
/// This queue is used to sequentially process these operations without loosing
/// new requests that may be added meanwhile.
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Queue {
    queue: Vec<ControlAction, 8>,
}

#[derive(Debug, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum ControlAction {
    Calibrate(usize),
    Map(usize),
}

impl Default for Queue {
    fn default() -> Self {
        Queue { queue: Vec::new() }
    }
}

impl Queue {
    pub fn push(&mut self, action: ControlAction) {
        // NOTE: The capacity is set to accomodate for all possible actions.
        let _ = self.queue.push(action);
    }

    pub fn pop(&mut self) -> Option<ControlAction> {
        if self.queue.is_empty() {
            None
        } else {
            Some(self.queue.remove(0))
        }
    }

    pub fn remove_control(&mut self, control_id: usize) {
        self.queue.retain(|a| match a {
            ControlAction::Map(id) | ControlAction::Calibrate(id) => *id != control_id,
        });
    }

    #[cfg(test)]
    pub fn len(&self) -> usize {
        self.queue.len()
    }

    #[cfg(test)]
    pub fn contains(&self, action: &ControlAction) -> bool {
        self.queue.contains(action)
    }
}
