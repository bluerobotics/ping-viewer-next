pub mod cli;
pub mod device;
pub mod logger;
pub mod server;
pub(crate) mod settings;
pub mod vehicle;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "module")]
pub enum ModuleType {
    DeviceManager(device::manager::Request),
}
