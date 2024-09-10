pub mod cli;
pub mod device;
pub mod logger;
pub mod server;

use serde::{Deserialize, Serialize};
#[macro_use]
extern crate lazy_static;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "module")]
pub enum ModuleType {
    DeviceManager(device::manager::Request),
}
