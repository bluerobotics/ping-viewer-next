use serde_json::json;
use tracing::{error, trace, warn};
use uuid::Uuid;

use crate::device::{
    manager::{Answer, DeviceAnswer, ManagerError},
    manager::{DeviceManager, DeviceSelection},
};

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
            Ok(handler) => handler.clone(),
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
                        Err(err) => {
                            Self::handle_error_continuous_mode(err, device_id);
                            break;
                        }
                    }
                }
            })),
            DeviceSelection::Ping360 => {
                Some(tokio::spawn(async move {
                    let handler = handler.clone();

                    // Attempt to send the Ping360 request and handle the result
                    let device_data = match handler
                        .send(crate::device::devices::PingRequest::Ping360(
                            crate::device::devices::Ping360Request::DeviceData,
                        ))
                        .await
                    {
                        Ok(response) => match response {
                            crate::device::devices::PingAnswer::PingMessage(
                                bluerobotics_ping::Messages::Ping360(
                                    bluerobotics_ping::ping360::Messages::DeviceData(msg),
                                ),
                            ) => msg,
                            msg => {
                                error!("Error during start_continuous_mode: unexpected message: {msg:?}");
                                return;
                            }
                        },
                        Err(err) => {
                            error!("Error during start_continuous_mode: Device Error: {err:?}");
                            return;
                        }
                    };

                    loop {
                        for n in 0..=399 {
                            // Handle timeout and errors
                            let result = tokio::time::timeout(
                                std::time::Duration::from_millis(1000),
                                handler.send(crate::device::devices::PingRequest::Ping360(
                                    crate::device::devices::Ping360Request::Transducer(
                                        bluerobotics_ping::ping360::TransducerStruct {
                                            mode: device_data.mode,
                                            gain_setting: device_data.gain_setting,
                                            transmit_duration: device_data.transmit_duration,
                                            sample_period: device_data.sample_period,
                                            transmit_frequency: device_data.transmit_frequency,
                                            number_of_samples: device_data.number_of_samples,
                                            angle: n,
                                            transmit: 1,
                                            reserved: 0,
                                        },
                                    ),
                                )),
                            )
                            .await;

                            match result {
                                Ok(Ok(answer)) => match answer {
                                    crate::device::devices::PingAnswer::PingMessage(msg) => {
                                        Self::ping360_continuous_mode_helper(msg, device_id)
                                    }
                                    msg => {
                                        error!("Error during continuous_mode: Unexpected Message: {msg:?}");
                                        return;
                                    }
                                },
                                Ok(Err(err)) => {
                                    error!("Error during continuous_mode: Device Error: {err:?}");
                                    return;
                                }
                                Err(_err) => {
                                    warn!("Error during continuous_mode: Answer delayed more than 1 s");
                                }
                            }
                        }
                    }
                }))
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

        if device_type == DeviceSelection::Ping1D {
            let id = <bluerobotics_ping::ping1d::ProfileStruct as bluerobotics_ping::message::MessageInfo>::id();
            let _ = handler
                .send(crate::device::devices::PingRequest::Ping1D(
                    crate::device::devices::Ping1DRequest::ContinuousStop(
                        bluerobotics_ping::ping1d::ContinuousStopStruct { id },
                    ),
                ))
                .await
                .map_err(|err| {trace!("Something went wrong while executing broadcast_startup_routine, details: {err:?}"); ManagerError::DeviceError(err)})?;
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
                        bluerobotics_ping::Messages::try_from(&msg).unwrap(),
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
}
