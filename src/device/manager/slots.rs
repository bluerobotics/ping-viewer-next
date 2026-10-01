use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::device::manager::{DeviceSelection, SourceSelection};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DeviceConnection {
    Serial,
    Ethernet { mac: String },
    Udp,
    Fake { id: u16 },
}

impl From<&SourceSelection> for DeviceConnection {
    fn from(value: &SourceSelection) -> Self {
        match value {
            SourceSelection::UdpStream(udp) => match udp.mac_address.as_ref() {
                Some(mac) => DeviceConnection::Ethernet { mac: mac.clone() },
                None => DeviceConnection::Udp,
            },
            SourceSelection::SerialStream(_) => DeviceConnection::Serial,
            SourceSelection::FakeStream(fake) => DeviceConnection::Fake { id: fake.fake_id },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DeviceIdentity {
    pub device_type: DeviceSelection,
    pub connection: DeviceConnection,
}

#[derive(Default, Serialize, Deserialize)]
pub struct SlotRegistry {
    assignments: HashMap<DeviceSelection, HashMap<DeviceConnection, u8>>,
    #[serde(skip)]
    occupied: HashMap<DeviceSelection, HashSet<u8>>,
}

impl SlotRegistry {
    pub fn new() -> Self {
        Default::default()
    }

    /// Called on successful connect, once device_type is known.
    pub fn resolve(&mut self, identity: &DeviceIdentity) -> u8 {
        let assignments = self.assignments.entry(identity.device_type).or_default();
        let occupied = self.occupied.entry(identity.device_type).or_default();

        if let Some(&slot) = assignments.get(&identity.connection) {
            if occupied.insert(slot) {
                return slot;
            }
        }
        let slot = (0..u8::MAX)
            .find(|slot| !occupied.contains(slot))
            .expect("no free slots for device type");
        assignments.insert(identity.connection.clone(), slot);
        occupied.insert(slot);
        slot
    }

    /// Slot last assigned to this identity, occupied or not.
    pub fn assigned(&self, identity: &DeviceIdentity) -> Option<u8> {
        self.assignments
            .get(&identity.device_type)?
            .get(&identity.connection)
            .copied()
    }

    /// Called on disconnect, letting the slot become reusable.
    pub fn release(&mut self, device_type: DeviceSelection, slot: u8) {
        if let Some(occupied) = self.occupied.get_mut(&device_type) {
            occupied.remove(&slot);
        }
    }
}
