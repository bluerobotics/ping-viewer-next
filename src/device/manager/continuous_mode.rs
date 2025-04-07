use serde_json::json;
use tracing::{debug, error, trace};
use uuid::Uuid;

use crate::device::{
    devices::DeviceActorHandler,
    manager::{Answer, DeviceAnswer, DeviceManager, DeviceSelection, ManagerError},
};

use super::{DeviceProperties, Ping360Properties};

impl DeviceManager {
    // Call the helpers specifically for each device type
    pub async fn continuous_mode_start(
        &mut self,
        mut subscriber: tokio::sync::broadcast::Receiver<
            bluerobotics_ping::message::ProtocolMessage,
        >,
        device_id: Uuid,
        device_type: DeviceSelection,
    ) -> Option<tokio::task::JoinHandle<()>> {
        let raw_handler = match self.get_device_handler(device_id).await {
            Ok(handler) => handler,
            Err(err) => {
                trace!("Error during start_continuous_mode: Failed to get device handler: {err:?}");
                return None;
            }
        };

        let handler = match self.extract_handler(raw_handler) {
            Ok(handler) => handler,
            Err(err) => {
                trace!("Error during start_continuous_mode: Failed to extract handler: {err:?}");
                return None;
            }
        };

        match device_type {
            DeviceSelection::Ping1D => Some(tokio::spawn(async move {
                loop {
                    match subscriber.recv().await {
                        Ok(msg) => {
                            Self::ping1d_continuous_mode_helper(msg, device_id);
                        }
                        Err(err @ tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                            error!("Device subscriber channel issue {err:?}, device: {device_id}");
                            Self::handle_error_continuous_mode(err, device_id);
                        }
                        Err(err) => {
                            Self::handle_error_continuous_mode(err, device_id);
                            break;
                        }
                    }
                }
            })),
            DeviceSelection::Ping360 => {
                let device_properties = self.get_device_properties(device_id).await.ok()?;
                let Some(DeviceProperties::Ping360(properties)) = device_properties else {
                    error!("No properties available for Ping360 device, device: {device_id}");
                    return None;
                };

                // Check if firmware supports auto-transmit mode
                if properties.common.device_information.firmware_version_major >= 3
                    && properties.common.device_information.firmware_version_minor >= 3
                {
                    Some(Self::start_ping360_firmware_mode(
                        handler,
                        device_id,
                        properties.clone(),
                        subscriber,
                    ))
                } else {
                    Some(Self::start_ping360_software_mode(
                        handler,
                        device_id,
                        properties.clone(),
                    ))
                }
            }
            DeviceSelection::Common | DeviceSelection::Auto => None,
        }
    }

    // Execute some especial commands required for device enter in auto_send mode
    pub async fn continuous_mode_startup_routine(
        &self,
        device_id: Uuid,
        device_type: DeviceSelection,
    ) -> Result<(), ManagerError> {
        if device_type == DeviceSelection::Ping1D {
            let handler_request = self.get_device_handler(device_id).await?;
            let handler = self.extract_handler(handler_request)?;

            let id = <bluerobotics_ping::ping1d::ProfileStruct as bluerobotics_ping::message::MessageInfo>::id();
            let _ = handler
                .send(crate::device::devices::PingRequest::Ping1D(
                    crate::device::devices::Ping1DRequest::ContinuousStart(
                        bluerobotics_ping::ping1d::ContinuousStartStruct { id },
                    ),
                ))
                .await
                .map_err(|err| {trace!("Something went wrong while executing continuous_mode_startup, details: {err:?}"); ManagerError::DeviceError(err)})?;
        }
        Ok(())
    }

    // Execute some especial commands required for device stop auto_send mode
    pub async fn continuous_mode_shutdown_routine(
        &self,
        device_id: Uuid,
        device_type: DeviceSelection,
    ) -> Result<(), ManagerError> {
        let handler_request = self.get_device_handler(device_id).await?;
        let handler = self.extract_handler(handler_request)?;

        match device_type {
            DeviceSelection::Ping1D => {
                let id = <bluerobotics_ping::ping1d::ProfileStruct as bluerobotics_ping::message::MessageInfo>::id();
                if let Err(err) = handler
                    .send(crate::device::devices::PingRequest::Ping1D(
                        crate::device::devices::Ping1DRequest::ContinuousStop(
                            bluerobotics_ping::ping1d::ContinuousStopStruct { id },
                        ),
                    ))
                    .await
                {
                    error!("Something went wrong while executing continuous_mode_shutdown_routine, details: {err:?}, device: {device_id}");
                }
            }
            DeviceSelection::Ping360 => {
                if let Err(err) = handler
                    .send(crate::device::devices::PingRequest::Ping360(
                        crate::device::devices::Ping360Request::MotorOff,
                    ))
                    .await
                {
                    error!("Something went wrong while executing continuous_mode_shutdown_routine, details: {err:?}, device: {device_id}");
                }
            }
            _ => {}
        }

        Ok(())
    }

    // An inner helper focused on Ping1D, which uses Profile message to plot graphs
    pub fn ping1d_continuous_mode_helper(
        msg: bluerobotics_ping::message::ProtocolMessage,
        device_id: Uuid,
    ) {
        if msg.message_id == <bluerobotics_ping::ping1d::ProfileStruct as bluerobotics_ping::message::MessageInfo>::id() {
            if let Ok(bluerobotics_ping::Messages::Ping1D(bluerobotics_ping::ping1d::Messages::Profile(_answer))) = bluerobotics_ping::Messages::try_from(&msg) {
                let answer = Answer::DeviceMessage(DeviceAnswer {
                    answer: crate::device::devices::PingAnswer::PingMessage(
                        match  bluerobotics_ping::Messages::try_from(&msg){
                            Ok(msg) => msg,
                            Err(err) => {
                                error!("Unexpected message during scan: {err:?}");
                                return},
                        }
                    ),
                    device_id,
                });
                crate::server::protocols::v1::websocket::send_to_websockets(json!(answer), Some(device_id));
            }
        }
    }

    // An inner helper focused on Ping360 on AutoTransmit mode, which uses AutoDeviceData.
    pub fn ping360_continuous_mode_helper_auto(
        msg: bluerobotics_ping::message::ProtocolMessage,
        device_id: Uuid,
    ) {
        if msg.message_id == <bluerobotics_ping::ping360::AutoDeviceDataStruct as bluerobotics_ping::message::MessageInfo>::id() {
                if let Ok(bluerobotics_ping::Messages::Ping360(bluerobotics_ping::ping360::Messages::AutoDeviceData(_answer))) = bluerobotics_ping::Messages::try_from(&msg) {
                    let answer = Answer::DeviceMessage(DeviceAnswer {
                        answer: crate::device::devices::PingAnswer::PingMessage(
                            match  bluerobotics_ping::Messages::try_from(&msg){
                                Ok(msg) => msg,
                                Err(err) => {
                                    error!("Unexpected message during scan: {err:?}");
                                    return},
                            }
                        ),
                        device_id,
                    });
                    crate::server::protocols::v1::websocket::send_to_websockets(json!(answer), Some(device_id));
                }
            }
    }

    // An inner helper focused on Ping360, which uses DeviceData message to plot graphs
    pub fn ping360_continuous_mode_helper(msg: bluerobotics_ping::Messages, device_id: Uuid) {
        let answer = Answer::DeviceMessage(DeviceAnswer {
            answer: crate::device::devices::PingAnswer::PingMessage(msg),
            device_id,
        });
        crate::server::protocols::v1::websocket::send_to_websockets(json!(answer), Some(device_id));
    }

    // An inner helper that returns error to requester
    pub fn handle_error_continuous_mode(
        error: tokio::sync::broadcast::error::RecvError,
        device_id: Uuid,
    ) {
        let error = ManagerError::DeviceError(crate::device::devices::DeviceError::PingError(
            bluerobotics_ping::error::PingError::TokioBroadcastError(error.to_string()),
        ));
        crate::server::protocols::v1::websocket::send_to_websockets(json!(error), Some(device_id));
    }

    fn start_ping360_firmware_mode(
        handler: DeviceActorHandler,
        device_id: Uuid,
        properties: Ping360Properties,
        mut subscriber: tokio::sync::broadcast::Receiver<
            bluerobotics_ping::message::ProtocolMessage,
        >,
    ) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            'main: loop {
                let config = properties.continuous_mode_settings.clone();
                let initial_settings = match config.read() {
                    Ok(settings) => settings.clone(),
                    Err(err) => {
                        error!("Failed to read Ping360Config: {err:?}, device: {device_id}");
                        break;
                    }
                };

                // Stop the motor before starting auto-transmit
                if let Err(err) = handler
                    .send(crate::device::devices::PingRequest::Ping360(
                        crate::device::devices::Ping360Request::MotorOff,
                    ))
                    .await
                {
                    error!("Failed to stop motor: {err:?}, device: {device_id}");
                    break;
                }

                // Start auto-transmit mode
                if let Err(err) = handler
                    .send(crate::device::devices::PingRequest::Ping360(
                        crate::device::devices::Ping360Request::AutoTransmit(
                            bluerobotics_ping::ping360::AutoTransmitStruct {
                                mode: initial_settings.mode,
                                gain_setting: initial_settings.gain_setting,
                                transmit_duration: initial_settings.transmit_duration,
                                sample_period: initial_settings.sample_period,
                                transmit_frequency: initial_settings.transmit_frequency,
                                number_of_samples: initial_settings.number_of_samples,
                                start_angle: initial_settings.start_angle,
                                stop_angle: initial_settings.stop_angle,
                                num_steps: initial_settings.num_steps,
                                delay: initial_settings.delay,
                            },
                        ),
                    ))
                    .await
                {
                    error!("Failed to start auto transmit: {err:?}, device: {device_id}");
                    break;
                }

                loop {
                    let current_settings = match config.read() {
                        Ok(settings) => settings.clone(),
                        Err(err) => {
                            error!("Failed to read Ping360Config: {err:?}, device: {device_id}");
                            break 'main;
                        }
                    };
                    if initial_settings != current_settings {
                        debug!("Restarting firmware scanning routine for Ping360Config, device: {device_id}");
                        break;
                    }

                    match subscriber.recv().await {
                        Ok(msg) => Self::ping360_continuous_mode_helper_auto(msg, device_id),
                        Err(err @ tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                            error!("Device subscriber channel issue {err:?}, device: {device_id}");
                            Self::handle_error_continuous_mode(err, device_id);
                        }
                        Err(err) => {
                            error!("Failed to receive from subscriber channel: {err:?}, device: {device_id}");
                            Self::handle_error_continuous_mode(err, device_id);
                            break 'main;
                        }
                    }
                }
            }
        })
    }

    fn start_ping360_software_mode(
        handler: DeviceActorHandler,
        device_id: Uuid,
        properties: Ping360Properties,
    ) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            loop {
                let config = properties.continuous_mode_settings.clone();
                let initial_settings = match config.read() {
                    Ok(settings) => settings.clone(),
                    Err(err) => {
                        error!("Failed to read Ping360Config: {err:?}, device: {device_id}");
                        break;
                    }
                };

                let mut angle = initial_settings.start_angle;
                let step_size = initial_settings.num_steps as u16;
                let is_full_circle =
                    initial_settings.start_angle == 0 && initial_settings.stop_angle == 399;
                let mut direction = 1i16;

                loop {
                    let current_settings = match config.read() {
                        Ok(settings) => settings.clone(),
                        Err(err) => {
                            error!("Failed to read Ping360Config: {err:?}, device: {device_id}");
                            break;
                        }
                    };
                    if initial_settings != current_settings {
                        break;
                    }

                    match handler
                        .send(crate::device::devices::PingRequest::Ping360(
                            crate::device::devices::Ping360Request::Transducer(
                                bluerobotics_ping::ping360::TransducerStruct {
                                    mode: initial_settings.mode,
                                    gain_setting: initial_settings.gain_setting,
                                    transmit_duration: initial_settings.transmit_duration,
                                    sample_period: initial_settings.sample_period,
                                    transmit_frequency: initial_settings.transmit_frequency,
                                    number_of_samples: initial_settings.number_of_samples,
                                    angle,
                                    transmit: 1,
                                    reserved: 0,
                                },
                            ),
                        ))
                        .await
                    {
                        Ok(answer) => match answer {
                            crate::device::devices::PingAnswer::PingMessage(msg) => {
                                Self::ping360_continuous_mode_helper(msg, device_id)
                            }
                            msg => {
                                error!("Unexpected message during scan: {msg:?}");
                                return;
                            }
                        },
                        Err(err) => {
                            error!("Failed to send transducer command: {err:?}");
                            return;
                        }
                    }

                    angle = Self::calculate_next_angle(
                        angle,
                        step_size,
                        is_full_circle,
                        &mut direction,
                        initial_settings.start_angle,
                        initial_settings.stop_angle,
                    );
                }
            }
        })
    }

    fn calculate_next_angle(
        current_angle: u16,
        step_size: u16,
        is_full_circle: bool,
        direction: &mut i16,
        start_angle: u16,
        stop_angle: u16,
    ) -> u16 {
        if is_full_circle {
            if current_angle + step_size >= 400 {
                0
            } else {
                current_angle + step_size
            }
        } else {
            if *direction > 0 {
                if current_angle + step_size > stop_angle {
                    *direction = -1;
                    stop_angle
                } else {
                    current_angle + step_size
                }
            } else {
                if (current_angle as i32 - step_size as i32) <= start_angle as i32 {
                    *direction = 1;
                    start_angle
                } else {
                    current_angle.wrapping_sub(step_size)
                }
            }
        }
    }
}
