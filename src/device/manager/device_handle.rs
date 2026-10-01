use tracing::{trace, warn};

use crate::device::{
    devices::{self, DeviceActorHandler},
    manager::{Answer, Device, DeviceManager, DeviceSelection, DeviceStatus, ManagerError},
};

impl DeviceManager {
    pub fn get_device(
        &self,
        device_type: DeviceSelection,
        slot: u8,
    ) -> Result<&Device, ManagerError> {
        self.device
            .values()
            .find(|device| device.device_type == device_type && device.slot == slot)
            .ok_or(ManagerError::DeviceNotExist(device_type, slot))
    }

    pub async fn get_device_handler(
        &self,
        device_type: DeviceSelection,
        slot: u8,
    ) -> Result<Answer, ManagerError> {
        let device = self.get_device(device_type, slot)?;

        trace!(%device_type, slot, "Getting device handler for device: Success");

        // Fail-fast if device is stopped
        self.check_device_status(
            device,
            &[DeviceStatus::ContinuousMode, DeviceStatus::Running],
        )?;

        let handler: DeviceActorHandler = device
            .handler
            .clone()
            .ok_or(ManagerError::Other("Unexpected".to_string()))?;

        Ok(Answer::InnerDeviceHandler(handler))
    }

    pub fn check_device_status(
        &self,
        device: &Device,
        valid_statuses: &[DeviceStatus],
    ) -> Result<(), ManagerError> {
        let status = &device.status;
        if !valid_statuses.contains(status) {
            return Err(ManagerError::DeviceStatus(
                status.clone(),
                device.device_type,
                device.slot,
            ));
        }
        Ok(())
    }

    pub fn get_mut_device(
        &mut self,
        device_type: DeviceSelection,
        slot: u8,
    ) -> Result<&mut Device, ManagerError> {
        self.device
            .values_mut()
            .find(|device| device.device_type == device_type && device.slot == slot)
            .ok_or(ManagerError::DeviceNotExist(device_type, slot))
    }

    pub fn remove_device(
        &mut self,
        device_type: DeviceSelection,
        slot: u8,
    ) -> Result<Device, ManagerError> {
        self.device
            .remove(&self.get_device(device_type, slot)?.source.clone())
            .ok_or(ManagerError::DeviceNotExist(device_type, slot))
    }

    pub fn extract_handler(
        &self,
        device_handler: Answer,
    ) -> Result<DeviceActorHandler, ManagerError> {
        match device_handler {
            Answer::InnerDeviceHandler(handler) => Ok(handler),
            answer => Err(ManagerError::Other(format!(
                "Unreachable: extract_handler helper, detail: {answer:?}"
            ))),
        }
    }

    pub async fn get_subscriber(
        &self,
        device_type: DeviceSelection,
        slot: u8,
    ) -> Result<
        tokio::sync::broadcast::Receiver<bluerobotics_ping::message::ProtocolMessage>,
        ManagerError,
    > {
        let handler_request = self.get_device_handler(device_type, slot).await?;
        let handler = self.extract_handler(handler_request)?;

        let subscriber = handler
            .send(devices::PingRequest::GetSubscriber)
            .await
            .map_err(|error| {
                warn!(
                    ?error,
                    "Something went wrong while executing get_subscriber"
                );
                ManagerError::DeviceError(error)
            })?;

        match subscriber {
            devices::PingAnswer::Subscriber(subscriber) => Ok(subscriber),
            _ => Err(ManagerError::Other(
                "Unreachable: get_subscriber helper".to_string(),
            )),
        }
    }
}
