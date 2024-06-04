use paperclip::actix::Apiv2Schema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Apiv2Schema)]
pub struct AnsPackage {
    pub device: String,
    #[serde(flatten)]
    pub operation: Operation,
}

impl AnsPackage {
    pub fn new(operation: Operation) -> AnsPackage {
        // All the AnsPackage's requests can be broadcasted to websocket clients
        // This helps all clients to be in sync.
        // websocket::send_to_websockets(json!(package));

        AnsPackage {
            device: "NI".to_string(),
            operation,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Apiv2Schema)]
#[serde(untagged)]
pub enum Operation {
    Get,
    Set,
    Settings,
}

#[derive(Debug, Serialize, Deserialize, Apiv2Schema)]
pub struct DevicesList {
    pub list: Vec<String>,
}
