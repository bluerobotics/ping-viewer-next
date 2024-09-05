use bluerobotics_ping::device::PingDevice;
use paperclip::actix::Apiv2Schema;
use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, oneshot};
use tracing::{error, trace, warn};

pub struct DeviceActor {
    pub receiver: mpsc::Receiver<DeviceActorRequest>,
    pub device_type: DeviceType,
}

#[derive(Debug)]
pub struct DeviceActorRequest {
    pub request: PingRequest,
    pub respond_to: oneshot::Sender<Result<PingAnswer, DeviceError>>,
}

impl DeviceActor {
    async fn handle_message(&mut self, request: DeviceActorRequest) {
        match request.request.clone() {
            PingRequest::Ping1D(device_request) => match &self.device_type {
                DeviceType::Ping1D(device) => {
                    trace!("Handling Ping1D request: {device_request:?}");
                    let answer = device.handle(device_request).await;
                    let _ = request.respond_to.send(answer);
                }
                _ => {
                    warn!(
                        "Unsupported request for device type: {:?}",
                        &self.device_type
                    );
                    let ping_request = request.request;
                    let _ = request
                        .respond_to
                        .send(Ok(PingAnswer::NotSupported(ping_request)));
                }
            },
            PingRequest::Ping360(device_request) => match &self.device_type {
                DeviceType::Ping360(device) => {
                    trace!("Handling Ping360 request: {device_request:?}");
                    let answer = device.handle(device_request).await;
                    let _ = request.respond_to.send(answer);
                }
                _ => {
                    warn!(
                        "Unsupported request for device type: {:?}",
                        &self.device_type
                    );
                    let ping_request = request.request;
                    let _ = request
                        .respond_to
                        .send(Ok(PingAnswer::NotSupported(ping_request)));
                }
            },
            PingRequest::Common(device_request) => match &self.device_type {
                DeviceType::Common(device) => {
                    trace!("Handling Common request: {device_request:?}");
                    let answer = device.handle(device_request).await;
                    let _ = request.respond_to.send(answer);
                }
                DeviceType::Ping1D(device) => {
                    trace!("Handling Common request: {device_request:?}");
                    let answer = device.handle(device_request).await;
                    let _ = request.respond_to.send(answer);
                }
                DeviceType::Ping360(device) => {
                    trace!("Handling Common request: {device_request:?}");
                    let answer = device.handle(device_request).await;
                    let _ = request.respond_to.send(answer);
                }
                _ => {
                    warn!(
                        "Unsupported request for device type: {:?}",
                        &self.device_type
                    );
                    let ping_request = request.request;
                    let _ = request
                        .respond_to
                        .send(Ok(PingAnswer::NotSupported(ping_request)));
                }
            },
            PingRequest::GetSubscriber => {
                let answer = self.handle(request.request).await;
                let _ = request.respond_to.send(Ok(answer));
            }
            PingRequest::Upgrade => {
                let answer = self.try_upgrade().await;
                let _ = request.respond_to.send(answer);
            }
            _ => todo!(),
        }
    }

    pub async fn run(mut self) -> Self {
        while let Some(msg) = self.receiver.recv().await {
            match &msg.request {
                PingRequest::Stop => {
                    trace! {"Device received stop request, returning structure."}
                    return self;
                }
                _ => self.handle_message(msg).await,
            }
        }
        error! {"Device closed it's channel, returning structure."}
        self
    }

    pub async fn try_upgrade(&mut self) -> Result<PingAnswer, DeviceError> {
        let device_type_check = match &self.device_type {
            DeviceType::Common(device) => {
                let device_type_check = match device.device_information().await {
                    Ok(result) => result.device_type,
                    Err(e) => {
                        return Err(DeviceError::PingError(e));
                    }
                };
                if device_type_check == 0 {
                    return Ok(PingAnswer::UpgradeResult(UpgradeResult::Unknown));
                };
                device_type_check
            }
            DeviceType::Ping1D(device) => {
                let device_type_check = match device.device_information().await {
                    Ok(result) => result.device_type,
                    Err(e) => {
                        return Err(DeviceError::PingError(e));
                    }
                };
                if device_type_check == 1 {
                    return Ok(PingAnswer::UpgradeResult(UpgradeResult::Ping1D));
                };
                device_type_check
            }
            DeviceType::Ping360(device) => {
                let device_type_check = match device.device_information().await {
                    Ok(result) => result.device_type,
                    Err(e) => {
                        return Err(DeviceError::PingError(e));
                    }
                };
                if device_type_check == 2 {
                    return Ok(PingAnswer::UpgradeResult(UpgradeResult::Ping360));
                };
                device_type_check
            }
            _ => {
                todo!()
            }
        };

        // Previous strategy tested and return fast if error/or already have the device upgraded,
        // Otherwise it will be upgraded or return current structure.

        // Helper function to create a new device type based on the type check
        fn create_device_type(
            common: bluerobotics_ping::device::Common,
            device_type_check: u8,
        ) -> DeviceType {
            match device_type_check {
                1 => DeviceType::Ping1D(bluerobotics_ping::ping1d::Device { common }),
                2 => DeviceType::Ping360(bluerobotics_ping::ping360::Device { common }),
                _ => DeviceType::Common(bluerobotics_ping::common::Device { common }),
            }
        }

        // Strategy to manipulate self.device_type while matches the current value.
        let placeholder = DeviceType::Null;
        let device_type_tmp = std::mem::replace(&mut self.device_type, placeholder);

        let upgrade_result = match device_type_check {
            1 => UpgradeResult::Ping1D,
            2 => UpgradeResult::Ping360,
            _ => UpgradeResult::Unknown,
        };

        self.device_type = match device_type_tmp {
            DeviceType::Common(device) => create_device_type(device.common, device_type_check),
            DeviceType::Ping1D(device) => create_device_type(device.common, device_type_check),
            DeviceType::Ping360(device) => create_device_type(device.common, device_type_check),
            _ => {
                // Unreachable.
                self.device_type = device_type_tmp;
                return Ok(PingAnswer::UpgradeResult(UpgradeResult::Unknown));
            }
        };

        Ok(PingAnswer::UpgradeResult(upgrade_result))
    }

    pub fn new(device: DeviceType, size: usize) -> (Self, DeviceActorHandler) {
        let (sender, receiver) = mpsc::channel(size);
        let actor = DeviceActor {
            receiver,
            device_type: device,
        };
        let actor_handler = DeviceActorHandler { sender };

        trace!("Device and handler successfully created: Success");
        (actor, actor_handler)
    }
}

#[derive(Clone, Debug)]
pub struct DeviceActorHandler {
    pub sender: mpsc::Sender<DeviceActorRequest>,
}
impl DeviceActorHandler {
    pub async fn send(&self, device_request: PingRequest) -> Result<PingAnswer, DeviceError> {
        let (result_sender, result_receiver) = oneshot::channel();

        let device_request = DeviceActorRequest {
            request: device_request,
            respond_to: result_sender,
        };

        if let Err(err) = self.sender.send(device_request).await {
            error!("DeviceManagerHandler: Failed to reach Device, details: {err:?}");
            return Err(DeviceError::TokioError(err.to_string()));
        }

        match result_receiver
            .await
            .map_err(|err| DeviceError::TokioError(err.to_string()))
        {
            Ok(ans) => ans,
            Err(err) => {
                error!(
                    "DeviceManagerHandler: Failed to receive message from Device, details: {err:?}"
                );
                Err(err)
            }
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub enum PingAnswer {
    PingMessage(bluerobotics_ping::Messages),
    NotSupported(PingRequest),
    PingAcknowledge(PingRequest),
    NotImplemented(PingRequest),
    #[serde(skip)]
    Subscriber(tokio::sync::broadcast::Receiver<bluerobotics_ping::message::ProtocolMessage>),
    UpgradeResult(UpgradeResult),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum DeviceError {
    PingError(bluerobotics_ping::error::PingError),
    TokioError(String),
}

impl Clone for PingAnswer {
    fn clone(&self) -> Self {
        match self {
            PingAnswer::PingMessage(msg) => PingAnswer::PingMessage(msg.clone()),
            PingAnswer::NotSupported(req) => PingAnswer::NotSupported(req.clone()),
            PingAnswer::PingAcknowledge(req) => PingAnswer::PingAcknowledge(req.clone()),
            PingAnswer::NotImplemented(req) => PingAnswer::NotImplemented(req.clone()),
            PingAnswer::Subscriber(receiver) => PingAnswer::Subscriber(receiver.resubscribe()),
            PingAnswer::UpgradeResult(result) => PingAnswer::UpgradeResult(result.clone()),
        }
    }
}

#[derive(Debug)]
pub enum DeviceType {
    Common(bluerobotics_ping::common::Device),
    Ping1D(bluerobotics_ping::device::Ping1D),
    Ping360(bluerobotics_ping::device::Ping360),
    Null,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum UpgradeResult {
    Unknown,
    Ping1D,
    Ping360,
}

#[derive(Debug, Clone, Serialize, Deserialize, Apiv2Schema)]
pub enum PingRequest {
    Ping1D(Ping1DRequest),
    Ping360(Ping360Request),
    Common(PingCommonRequest),
    GetSubscriber,
    Upgrade,
    Stop,
}

#[derive(Debug, Clone, Serialize, Deserialize, Apiv2Schema)]
pub enum Ping1DRequest {
    DeviceID,
    ModeAuto,
    Distance,
    Profile,
    SpeedOfSound,
    Voltage5,
    DeviceId,
    FirmwareVersion,
    Range,
    TransmitDuration,
    PingInterval,
    ProcessorTemperature,
    PcbTemperature,
    GeneralInfo,
    GainSetting,
    PingEnable,
    DistanceSimple,
    SetDeviceId(bluerobotics_ping::ping1d::SetDeviceIdStruct),
    SetModeAuto(bluerobotics_ping::ping1d::SetModeAutoStruct),
    SetPingInterval(bluerobotics_ping::ping1d::SetPingIntervalStruct),
    SetPingEnable(bluerobotics_ping::ping1d::SetPingEnableStruct),
    SetSpeedOfSound(bluerobotics_ping::ping1d::SetSpeedOfSoundStruct),
    SetRange(bluerobotics_ping::ping1d::SetRangeStruct),
    SetGainSetting(bluerobotics_ping::ping1d::SetGainSettingStruct),
    ContinuousStart(bluerobotics_ping::ping1d::ContinuousStartStruct),
    ContinuousStop(bluerobotics_ping::ping1d::ContinuousStopStruct),
    GotoBootloader,
}

#[derive(Debug, Clone, Serialize, Deserialize, Apiv2Schema)]
pub enum Ping360Request {
    MotorOff,
    DeviceData,
    AutoDeviceData,
    SetDeviceId(bluerobotics_ping::ping360::SetDeviceIdStruct),
    Transducer(bluerobotics_ping::ping360::TransducerStruct),
    Reset(bluerobotics_ping::ping360::ResetStruct),
    AutoTransmit(bluerobotics_ping::ping360::AutoTransmitStruct),
}

#[derive(Debug, Clone, Serialize, Deserialize, Apiv2Schema)]
pub enum PingCommonRequest {
    DeviceInformation,
    ProtocolVersion,
    SetDeviceId(bluerobotics_ping::common::SetDeviceIdStruct),
}

// All available requests are defined here for each
trait Requests<T> {
    type Reply;
    async fn handle(&self, msg: T) -> Self::Reply;
}

impl Requests<Ping1DRequest> for bluerobotics_ping::device::Ping1D {
    type Reply = Result<PingAnswer, DeviceError>;

    async fn handle(&self, msg: Ping1DRequest) -> Self::Reply {
        match msg.clone() {
            Ping1DRequest::DeviceID => match self.device_id().await {
                Ok(result) => Ok(PingAnswer::PingMessage(
                    bluerobotics_ping::Messages::Ping1D(
                        bluerobotics_ping::ping1d::Messages::DeviceId(result),
                    ),
                )),
                Err(e) => Err(DeviceError::PingError(e)),
            },
            Ping1DRequest::Distance => match self.distance().await {
                Ok(result) => Ok(PingAnswer::PingMessage(
                    bluerobotics_ping::Messages::Ping1D(
                        bluerobotics_ping::ping1d::Messages::Distance(result),
                    ),
                )),
                Err(e) => Err(DeviceError::PingError(e)),
            },
            Ping1DRequest::ModeAuto => match self.mode_auto().await {
                Ok(result) => Ok(PingAnswer::PingMessage(
                    bluerobotics_ping::Messages::Ping1D(
                        bluerobotics_ping::ping1d::Messages::ModeAuto(result),
                    ),
                )),
                Err(e) => Err(DeviceError::PingError(e)),
            },
            Ping1DRequest::Profile => match self.profile().await {
                Ok(result) => Ok(PingAnswer::PingMessage(
                    bluerobotics_ping::Messages::Ping1D(
                        bluerobotics_ping::ping1d::Messages::Profile(result),
                    ),
                )),
                Err(e) => Err(DeviceError::PingError(e)),
            },
            Ping1DRequest::SpeedOfSound => match self.speed_of_sound().await {
                Ok(result) => Ok(PingAnswer::PingMessage(
                    bluerobotics_ping::Messages::Ping1D(
                        bluerobotics_ping::ping1d::Messages::SpeedOfSound(result),
                    ),
                )),
                Err(e) => Err(DeviceError::PingError(e)),
            },
            Ping1DRequest::Voltage5 => match self.voltage_5().await {
                Ok(result) => Ok(PingAnswer::PingMessage(
                    bluerobotics_ping::Messages::Ping1D(
                        bluerobotics_ping::ping1d::Messages::Voltage5(result),
                    ),
                )),
                Err(e) => Err(DeviceError::PingError(e)),
            },
            Ping1DRequest::DeviceId => match self.device_id().await {
                Ok(result) => Ok(PingAnswer::PingMessage(
                    bluerobotics_ping::Messages::Ping1D(
                        bluerobotics_ping::ping1d::Messages::DeviceId(result),
                    ),
                )),
                Err(e) => Err(DeviceError::PingError(e)),
            },
            Ping1DRequest::FirmwareVersion => match self.firmware_version().await {
                Ok(result) => Ok(PingAnswer::PingMessage(
                    bluerobotics_ping::Messages::Ping1D(
                        bluerobotics_ping::ping1d::Messages::FirmwareVersion(result),
                    ),
                )),
                Err(e) => Err(DeviceError::PingError(e)),
            },
            Ping1DRequest::Range => match self.range().await {
                Ok(result) => Ok(PingAnswer::PingMessage(
                    bluerobotics_ping::Messages::Ping1D(
                        bluerobotics_ping::ping1d::Messages::Range(result),
                    ),
                )),
                Err(e) => Err(DeviceError::PingError(e)),
            },
            Ping1DRequest::TransmitDuration => match self.transmit_duration().await {
                Ok(result) => Ok(PingAnswer::PingMessage(
                    bluerobotics_ping::Messages::Ping1D(
                        bluerobotics_ping::ping1d::Messages::TransmitDuration(result),
                    ),
                )),
                Err(e) => Err(DeviceError::PingError(e)),
            },
            Ping1DRequest::PingInterval => match self.ping_interval().await {
                Ok(result) => Ok(PingAnswer::PingMessage(
                    bluerobotics_ping::Messages::Ping1D(
                        bluerobotics_ping::ping1d::Messages::PingInterval(result),
                    ),
                )),
                Err(e) => Err(DeviceError::PingError(e)),
            },
            Ping1DRequest::ProcessorTemperature => match self.processor_temperature().await {
                Ok(result) => Ok(PingAnswer::PingMessage(
                    bluerobotics_ping::Messages::Ping1D(
                        bluerobotics_ping::ping1d::Messages::ProcessorTemperature(result),
                    ),
                )),
                Err(e) => Err(DeviceError::PingError(e)),
            },
            Ping1DRequest::PcbTemperature => match self.pcb_temperature().await {
                Ok(result) => Ok(PingAnswer::PingMessage(
                    bluerobotics_ping::Messages::Ping1D(
                        bluerobotics_ping::ping1d::Messages::PcbTemperature(result),
                    ),
                )),
                Err(e) => Err(DeviceError::PingError(e)),
            },
            Ping1DRequest::GeneralInfo => match self.general_info().await {
                Ok(result) => Ok(PingAnswer::PingMessage(
                    bluerobotics_ping::Messages::Ping1D(
                        bluerobotics_ping::ping1d::Messages::GeneralInfo(result),
                    ),
                )),
                Err(e) => Err(DeviceError::PingError(e)),
            },
            Ping1DRequest::GainSetting => match self.gain_setting().await {
                Ok(result) => Ok(PingAnswer::PingMessage(
                    bluerobotics_ping::Messages::Ping1D(
                        bluerobotics_ping::ping1d::Messages::GainSetting(result),
                    ),
                )),
                Err(e) => Err(DeviceError::PingError(e)),
            },
            Ping1DRequest::PingEnable => match self.ping_enable().await {
                Ok(result) => Ok(PingAnswer::PingMessage(
                    bluerobotics_ping::Messages::Ping1D(
                        bluerobotics_ping::ping1d::Messages::PingEnable(result),
                    ),
                )),
                Err(e) => Err(DeviceError::PingError(e)),
            },
            Ping1DRequest::DistanceSimple => match self.distance_simple().await {
                Ok(result) => Ok(PingAnswer::PingMessage(
                    bluerobotics_ping::Messages::Ping1D(
                        bluerobotics_ping::ping1d::Messages::DistanceSimple(result),
                    ),
                )),
                Err(e) => Err(DeviceError::PingError(e)),
            },
            Ping1DRequest::SetDeviceId(req_body) => {
                match self.set_device_id(req_body.device_id).await {
                    Ok(_) => Ok(PingAnswer::PingAcknowledge(PingRequest::Ping1D(msg))),
                    Err(e) => Err(DeviceError::PingError(e)),
                }
            }
            Ping1DRequest::ContinuousStart(req_body) => {
                match self.continuous_start(req_body.id).await {
                    Ok(_) => Ok(PingAnswer::PingAcknowledge(PingRequest::Ping1D(msg))),
                    Err(e) => Err(DeviceError::PingError(e)),
                }
            }
            Ping1DRequest::ContinuousStop(req_body) => {
                match self.continuous_stop(req_body.id).await {
                    Ok(_) => Ok(PingAnswer::PingAcknowledge(PingRequest::Ping1D(msg))),
                    Err(e) => Err(DeviceError::PingError(e)),
                }
            }
            Ping1DRequest::SetModeAuto(req_body) => {
                match self.set_mode_auto(req_body.mode_auto).await {
                    Ok(_) => Ok(PingAnswer::PingAcknowledge(PingRequest::Ping1D(msg))),
                    Err(e) => Err(DeviceError::PingError(e)),
                }
            }
            Ping1DRequest::SetPingInterval(req_body) => {
                match self.set_ping_interval(req_body.ping_interval).await {
                    Ok(_) => Ok(PingAnswer::PingAcknowledge(PingRequest::Ping1D(msg))),
                    Err(e) => Err(DeviceError::PingError(e)),
                }
            }
            Ping1DRequest::SetPingEnable(req_body) => {
                match self.set_ping_enable(req_body.ping_enabled).await {
                    Ok(_) => Ok(PingAnswer::PingAcknowledge(PingRequest::Ping1D(msg))),
                    Err(e) => Err(DeviceError::PingError(e)),
                }
            }
            Ping1DRequest::SetSpeedOfSound(req_body) => {
                match self.set_speed_of_sound(req_body.speed_of_sound).await {
                    Ok(_) => Ok(PingAnswer::PingAcknowledge(PingRequest::Ping1D(msg))),
                    Err(e) => Err(DeviceError::PingError(e)),
                }
            }
            Ping1DRequest::SetRange(req_body) => {
                match self
                    .set_range(req_body.scan_start, req_body.scan_length)
                    .await
                {
                    Ok(_) => Ok(PingAnswer::PingAcknowledge(PingRequest::Ping1D(msg))),
                    Err(e) => Err(DeviceError::PingError(e)),
                }
            }
            Ping1DRequest::SetGainSetting(req_body) => {
                match self.set_gain_setting(req_body.gain_setting).await {
                    Ok(_) => Ok(PingAnswer::PingAcknowledge(PingRequest::Ping1D(msg))),
                    Err(e) => Err(DeviceError::PingError(e)),
                }
            }
            Ping1DRequest::GotoBootloader => match self.goto_bootloader().await {
                Ok(_) => Ok(PingAnswer::PingAcknowledge(PingRequest::Ping1D(msg))),
                Err(e) => Err(DeviceError::PingError(e)),
            },
        }
    }
}

impl Requests<Ping360Request> for bluerobotics_ping::device::Ping360 {
    type Reply = Result<PingAnswer, DeviceError>;

    async fn handle(&self, msg: Ping360Request) -> Self::Reply {
        match msg.clone() {
            Ping360Request::MotorOff => match self.motor_off().await {
                Ok(_) => Ok(PingAnswer::PingAcknowledge(PingRequest::Ping360(msg))),
                Err(e) => Err(DeviceError::PingError(e)),
            },
            Ping360Request::AutoDeviceData => match self.auto_device_data().await {
                Ok(answer) => Ok(PingAnswer::PingMessage(
                    bluerobotics_ping::Messages::Ping360(
                        bluerobotics_ping::ping360::Messages::AutoDeviceData(answer),
                    ),
                )),
                Err(e) => Err(DeviceError::PingError(e)),
            },
            Ping360Request::DeviceData => match self.device_data().await {
                Ok(answer) => Ok(PingAnswer::PingMessage(
                    bluerobotics_ping::Messages::Ping360(
                        bluerobotics_ping::ping360::Messages::DeviceData(answer),
                    ),
                )),
                Err(e) => Err(DeviceError::PingError(e)),
            },
            Ping360Request::SetDeviceId(req_body) => {
                match self.set_device_id(req_body.id, req_body.reserved).await {
                    Ok(_) => Ok(PingAnswer::PingAcknowledge(PingRequest::Ping360(msg))),
                    Err(e) => Err(DeviceError::PingError(e)),
                }
            }
            Ping360Request::Reset(req_body) => {
                match self.reset(req_body.bootloader, req_body.reserved).await {
                    Ok(_) => Ok(PingAnswer::PingAcknowledge(PingRequest::Ping360(msg))),
                    Err(e) => Err(DeviceError::PingError(e)),
                }
            }
            Ping360Request::AutoTransmit(req_body) => {
                match self
                    .auto_transmit(
                        req_body.mode,
                        req_body.gain_setting,
                        req_body.transmit_duration,
                        req_body.sample_period,
                        req_body.transmit_frequency,
                        req_body.number_of_samples,
                        req_body.start_angle,
                        req_body.stop_angle,
                        req_body.num_steps,
                        req_body.delay,
                    )
                    .await
                {
                    Ok(_) => Ok(PingAnswer::PingAcknowledge(PingRequest::Ping360(msg))),
                    Err(e) => Err(DeviceError::PingError(e)),
                }
            }
            Ping360Request::Transducer(req_body) => {
                match self
                    .transducer(
                        req_body.mode,
                        req_body.gain_setting,
                        req_body.angle,
                        req_body.transmit_duration,
                        req_body.sample_period,
                        req_body.transmit_frequency,
                        req_body.number_of_samples,
                        req_body.transmit,
                        req_body.reserved,
                    )
                    .await
                {
                    Ok(answer) => Ok(PingAnswer::PingMessage(
                        bluerobotics_ping::Messages::Ping360(
                            bluerobotics_ping::ping360::Messages::DeviceData(answer),
                        ),
                    )),
                    Err(e) => Err(DeviceError::PingError(e)),
                }
            }
        }
    }
}

impl Requests<PingCommonRequest> for bluerobotics_ping::common::Device {
    type Reply = Result<PingAnswer, DeviceError>;

    async fn handle(&self, msg: PingCommonRequest) -> Self::Reply {
        match msg.clone() {
            PingCommonRequest::ProtocolVersion => match self.protocol_version().await {
                Ok(result) => Ok(PingAnswer::PingMessage(
                    bluerobotics_ping::Messages::Common(
                        bluerobotics_ping::common::Messages::ProtocolVersion(result),
                    ),
                )),
                Err(e) => Err(DeviceError::PingError(e)),
            },
            PingCommonRequest::DeviceInformation => match self.device_information().await {
                Ok(result) => Ok(PingAnswer::PingMessage(
                    bluerobotics_ping::Messages::Common(
                        bluerobotics_ping::common::Messages::DeviceInformation(result),
                    ),
                )),
                Err(e) => Err(DeviceError::PingError(e)),
            },
            _ => Ok(PingAnswer::NotImplemented(PingRequest::Common(msg))),
        }
    }
}

impl Requests<PingCommonRequest> for bluerobotics_ping::ping1d::Device {
    type Reply = Result<PingAnswer, DeviceError>;

    async fn handle(&self, msg: PingCommonRequest) -> Self::Reply {
        match msg.clone() {
            PingCommonRequest::ProtocolVersion => match self.protocol_version().await {
                Ok(result) => Ok(PingAnswer::PingMessage(
                    bluerobotics_ping::Messages::Common(
                        bluerobotics_ping::common::Messages::ProtocolVersion(result),
                    ),
                )),
                Err(e) => Err(DeviceError::PingError(e)),
            },
            PingCommonRequest::DeviceInformation => match self.device_information().await {
                Ok(result) => Ok(PingAnswer::PingMessage(
                    bluerobotics_ping::Messages::Common(
                        bluerobotics_ping::common::Messages::DeviceInformation(result),
                    ),
                )),
                Err(e) => Err(DeviceError::PingError(e)),
            },
            _ => Ok(PingAnswer::NotImplemented(PingRequest::Common(msg))),
        }
    }
}

impl Requests<PingCommonRequest> for bluerobotics_ping::ping360::Device {
    type Reply = Result<PingAnswer, DeviceError>;

    async fn handle(&self, msg: PingCommonRequest) -> Self::Reply {
        match msg.clone() {
            PingCommonRequest::ProtocolVersion => match self.protocol_version().await {
                Ok(result) => Ok(PingAnswer::PingMessage(
                    bluerobotics_ping::Messages::Common(
                        bluerobotics_ping::common::Messages::ProtocolVersion(result),
                    ),
                )),
                Err(e) => Err(DeviceError::PingError(e)),
            },
            PingCommonRequest::DeviceInformation => match self.device_information().await {
                Ok(result) => Ok(PingAnswer::PingMessage(
                    bluerobotics_ping::Messages::Common(
                        bluerobotics_ping::common::Messages::DeviceInformation(result),
                    ),
                )),
                Err(e) => Err(DeviceError::PingError(e)),
            },
            _ => Ok(PingAnswer::NotImplemented(PingRequest::Common(msg))),
        }
    }
}

impl Requests<PingRequest> for DeviceActor {
    type Reply = PingAnswer;
    async fn handle(&self, msg: PingRequest) -> Self::Reply {
        match msg {
            PingRequest::GetSubscriber => match &self.device_type {
                DeviceType::Common(_) => PingAnswer::NotSupported(msg),
                DeviceType::Ping1D(device) => PingAnswer::Subscriber(device.subscribe()),
                DeviceType::Ping360(device) => PingAnswer::Subscriber(device.subscribe()),
                _ => todo!(),
            },
            PingRequest::Upgrade => todo!(),
            PingRequest::Stop => todo!(),

            _ => PingAnswer::NotSupported(msg),
        }
    }
}
