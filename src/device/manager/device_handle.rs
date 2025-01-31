use tracing::{error, trace, warn};
use uuid::Uuid;

use crate::device::{
    devices::{self, DeviceActorHandler},
    manager::{Answer, Device, DeviceManager, DeviceSelection, DeviceStatus, ManagerError},
};

impl DeviceManager {
    pub fn check_device_uuid(&self, device_id: Uuid) -> Result<(), ManagerError> {
        if self.device.contains_key(&device_id) {
            return Ok(());
        }
        error!("Getting device handler for device: {device_id:?} : Error, device doesn't exist");
        Err(ManagerError::DeviceNotExist(device_id))
    }

    pub fn get_device(&self, device_id: Uuid) -> Result<&Device, ManagerError> {
        let device = self
            .device
            .get(&device_id)
            .ok_or(ManagerError::DeviceNotExist(device_id))?;
        Ok(device)
    }

    pub async fn get_device_handler(&self, device_id: Uuid) -> Result<Answer, ManagerError> {
        self.check_device_uuid(device_id)?;

        trace!("Getting device handler for device: {device_id:?} : Success");

        // Fail-fast if device is stopped
        self.check_device_status(
            device_id,
            &[DeviceStatus::ContinuousMode, DeviceStatus::Running],
        )?;

        let handler: DeviceActorHandler = self
            .get_device(device_id)?
            .handler
            .clone()
            .ok_or(ManagerError::Other("Unexpected".to_string()))?;

        Ok(Answer::InnerDeviceHandler(handler))
    }

    pub fn check_device_status(
        &self,
        device_id: Uuid,
        valid_statuses: &[DeviceStatus],
    ) -> Result<(), ManagerError> {
        let status = &self.get_device(device_id)?.status;
        if !valid_statuses.contains(status) {
            return Err(ManagerError::DeviceStatus(status.clone(), device_id));
        }
        Ok(())
    }

    pub fn get_mut_device(&mut self, device_id: Uuid) -> Result<&mut Device, ManagerError> {
        let device = self
            .device
            .get_mut(&device_id)
            .ok_or(ManagerError::DeviceNotExist(device_id))?;
        Ok(device)
    }

    pub fn get_device_type(&self, device_id: Uuid) -> Result<DeviceSelection, ManagerError> {
        let device_type = self.device.get(&device_id).unwrap().device_type.clone();
        Ok(device_type)
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
        device_id: Uuid,
    ) -> Result<
        tokio::sync::broadcast::Receiver<bluerobotics_ping::message::ProtocolMessage>,
        ManagerError,
    > {
        let handler_request = self.get_device_handler(device_id).await?;
        let handler = self.extract_handler(handler_request)?;

        let subscriber = handler
            .send(devices::PingRequest::GetSubscriber)
            .await
            .map_err(|err| {
                warn!("Something went wrong while executing get_subscriber, details: {err:?}");
                ManagerError::DeviceError(err)
            })?;

        match subscriber {
            devices::PingAnswer::Subscriber(subscriber) => Ok(subscriber),
            _ => Err(ManagerError::Other(
                "Unreachable: get_subscriber helper".to_string(),
            )),
        }
    }
}
