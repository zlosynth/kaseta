//! Process all inputs peripherals over time.

use super::button::Button;
use super::control::Control;
use super::pot::Pot;
use super::snapshot::Snapshot;
use super::switch::Switch;

/// Stateful store of raw inputs.
///
/// This struct turns the raw snapshot into a set of abstracted peripherals.
/// These peripherals provide features such as smoothening or click detection.
///
/// Note that despite all its attributes are public, they should be only read
/// from.
#[derive(Debug, Default)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Store {
    pub pre_amp: Pot,
    pub drive: Pot,
    pub bias: Pot,
    pub dry_wet: Pot,
    pub wow_flut: Pot,
    pub speed: Pot,
    pub tone: Pot,
    pub head: [Head; 4],
    pub control: [Control; 4],
    pub switch: [Switch; 10],
    pub button: Button,
}

#[derive(Debug, Default)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Head {
    pub position: Pot,
    pub volume: Pot,
    pub feedback: Pot,
    pub pan: Pot,
}

impl Store {
    pub fn update(&mut self, snapshot: Snapshot) {
        self.pre_amp.update(snapshot.pre_amp);
        self.drive.update(snapshot.drive);
        self.bias.update(snapshot.bias);
        self.dry_wet.update(snapshot.dry_wet);
        self.wow_flut.update(snapshot.wow_flut);
        self.speed.update(snapshot.speed);
        self.tone.update(snapshot.tone);
        for (i, head) in self.head.iter_mut().enumerate() {
            head.position.update(snapshot.head[i].position);
            head.volume.update(snapshot.head[i].volume);
            head.feedback.update(snapshot.head[i].feedback);
            head.pan.update(snapshot.head[i].pan);
        }
        for (i, control) in self.control.iter_mut().enumerate() {
            control.update(snapshot.control[i]);
        }
        self.switch = snapshot.switch;
        self.button.update(snapshot.button);
    }

    // TODO: Define function flattening all pots
    pub fn latest_pot_activity(&self) -> u32 {
        [
            self.pre_amp.last_active,
            self.drive.last_active,
            self.bias.last_active,
            self.dry_wet.last_active,
            self.wow_flut.last_active,
            self.speed.last_active,
            self.tone.last_active,
            self.head
                .iter()
                .map(|h| {
                    [
                        h.position.last_active,
                        h.volume.last_active,
                        h.feedback.last_active,
                        h.pan.last_active,
                    ]
                    .into_iter()
                    .min()
                    .unwrap()
                })
                .min()
                .unwrap(),
        ]
        .into_iter()
        .min()
        .unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::snapshot::SnapshotHead;

    #[test]
    fn when_input_snapshot_is_written_its_reflected_in_attributes() {
        let mut inputs = Store::default();
        inputs.update(Snapshot {
            pre_amp: 0.01,
            drive: 0.02,
            bias: 0.03,
            dry_wet: 0.04,
            wow_flut: 0.05,
            speed: 0.06,
            tone: 0.07,
            head: [
                SnapshotHead {
                    position: 0.09,
                    volume: 0.10,
                    feedback: 0.11,
                    pan: 0.12,
                },
                SnapshotHead {
                    position: 0.13,
                    volume: 0.14,
                    feedback: 0.15,
                    pan: 0.16,
                },
                SnapshotHead {
                    position: 0.17,
                    volume: 0.18,
                    feedback: 0.19,
                    pan: 0.20,
                },
                SnapshotHead {
                    position: 0.21,
                    volume: 0.22,
                    feedback: 0.23,
                    pan: 0.24,
                },
            ],
            control: [Some(0.25), Some(0.26), Some(0.27), Some(0.28)],
            switch: [true; 10],
            button: true,
        });

        let mut previous = 0.0;
        for value in [
            inputs.pre_amp.value(),
            inputs.drive.value(),
            inputs.bias.value(),
            inputs.dry_wet.value(),
            inputs.wow_flut.value(),
            inputs.speed.value(),
            inputs.tone.value(),
            inputs.head[0].position.value(),
            inputs.head[0].volume.value(),
            inputs.head[0].feedback.value(),
            inputs.head[0].pan.value(),
            inputs.head[1].position.value(),
            inputs.head[1].volume.value(),
            inputs.head[1].feedback.value(),
            inputs.head[1].pan.value(),
            inputs.head[2].position.value(),
            inputs.head[2].volume.value(),
            inputs.head[2].feedback.value(),
            inputs.head[2].pan.value(),
            inputs.head[3].position.value(),
            inputs.head[3].volume.value(),
            inputs.head[3].feedback.value(),
            inputs.head[3].pan.value(),
            inputs.control[0].value(),
            inputs.control[1].value(),
            inputs.control[2].value(),
            inputs.control[3].value(),
        ] {
            assert!(value > previous, "{value} !> {previous}");
            previous = value;
        }

        for switch in &inputs.switch {
            assert!(switch);
        }

        assert!(inputs.button.clicked);
    }
}
