use bluerobotics_ping::{ping1d::ProfileStruct, ping360::AutoDeviceDataStruct};
use foxglove::Context;
use foxglove::McapWriterHandle;
use paperclip::actix::Apiv2Schema;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::BufWriter;
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Arc,
};
use tokio::sync::oneshot;
use tokio::sync::{
    broadcast::{self, Receiver},
    mpsc, RwLock,
};
use tracing::{error, info, trace, warn};
use uuid::Uuid;

use crate::device::{
    devices::DeviceActorHandler,
    manager::{DeviceSelection, ManagerError},
};
use crate::vehicle::VehicleData;

use super::manager::{ManagerActorHandler, UuidWrapper};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordingSession {
    pub device_id: Uuid,
    pub file_path: PathBuf,
    pub is_active: bool,
    pub start_time: chrono::DateTime<chrono::Utc>,
    pub device_type: DeviceSelection,
}

pub struct SessionGuard {
    pub session: RecordingSession,
    pub writer: Option<McapWriterHandle<BufWriter<File>>>,
}

pub struct RecordingManager {
    receiver: mpsc::Receiver<ManagerActorRequest>,
    sessions: Arc<RwLock<HashMap<Uuid, SessionGuard>>>,
    base_path: PathBuf,
    status_broadcast: broadcast::Sender<RecordingSession>,
    devices_manager_handler: ManagerActorHandler,
    vehicle_data: Arc<RwLock<Option<VehicleData>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Apiv2Schema)]
#[serde(tag = "command", content = "payload")]
pub enum RecordingManagerCommand {
    StartRecording(UuidWrapper),
    StopRecording(UuidWrapper),
    GetRecordingStatus(UuidWrapper),
    GetAllRecordingStatus,
    GetSubscriber,
}

#[derive(Clone)]
pub struct RecordingsManagerHandler {
    sender: mpsc::Sender<ManagerActorRequest>,
}

#[derive(Debug)]
pub struct ManagerActorRequest {
    pub request: RecordingManagerCommand,
    pub respond_to: oneshot::Sender<Result<Answer, ManagerError>>,
}

#[derive(Debug, Serialize, Deserialize, Apiv2Schema)]
pub enum Answer {
    RecordingSession(RecordingSession),
    RecordingStatus(Option<RecordingSession>),
    AllRecordingStatus(Vec<RecordingSession>),
    #[serde(skip)]
    RecordingManager(Receiver<RecordingSession>),
}

impl RecordingManager {
    pub fn new_with_pose(
        size: usize,
        base_path: impl AsRef<Path>,
        device_manager: ManagerActorHandler,
        vehicle_data: Arc<RwLock<Option<VehicleData>>>,
    ) -> (Self, RecordingsManagerHandler) {
        let (sender, receiver) = mpsc::channel(size);
        let actor_handler: RecordingsManagerHandler = RecordingsManagerHandler { sender };
        let (status_broadcast, _) = broadcast::channel(100);
        let actor = RecordingManager {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            base_path: base_path.as_ref().to_path_buf(),
            status_broadcast,
            receiver,
            devices_manager_handler: device_manager,
            vehicle_data,
        };
        (actor, actor_handler)
    }

    pub fn new(
        size: usize,
        base_path: impl AsRef<Path>,
        device_manager: ManagerActorHandler,
    ) -> (Self, RecordingsManagerHandler) {
        Self::new_with_pose(size, base_path, device_manager, Arc::new(RwLock::new(None)))
    }

    pub async fn run(mut self) {
        info!("RecordingsManager is running");

        loop {
            tokio::select! {
                Some(msg) = self.receiver.recv() => {
                    self.handle_message(msg).await;
                }
                else => break,
            }
        }

        error!("RecordingsManager has stopped please check your application");
    }

    async fn handle_message(&mut self, actor_request: ManagerActorRequest) {
        trace!("RecordingsManager: Received a request, details: {actor_request:?}");

        let result = match actor_request.request {
            RecordingManagerCommand::StartRecording(uuid_wrapper) => self
                .start_recording(*uuid_wrapper)
                .await
                .map(Answer::RecordingSession),
            RecordingManagerCommand::StopRecording(uuid_wrapper) => self
                .stop_recording(*uuid_wrapper)
                .await
                .map(Answer::RecordingSession),
            RecordingManagerCommand::GetRecordingStatus(uuid_wrapper) => self
                .get_recording_status(*uuid_wrapper)
                .await
                .map(Answer::RecordingStatus),
            RecordingManagerCommand::GetAllRecordingStatus => self
                .get_all_recording_status()
                .await
                .map(Answer::AllRecordingStatus),
            RecordingManagerCommand::GetSubscriber => {
                Ok(Answer::RecordingManager(self.subscribe()))
            }
        };

        if let Err(e) = actor_request.respond_to.send(result) {
            error!("RecordingsManager: Failed to return response: {e:?}");
        }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<RecordingSession> {
        self.status_broadcast.subscribe()
    }

    async fn broadcast_status(&self, session: &RecordingSession) {
        let _ = self.status_broadcast.send(session.clone());
    }

    pub async fn start_recording(&self, device_id: Uuid) -> Result<RecordingSession, ManagerError> {
        if self.sessions.read().await.contains_key(&device_id) {
            return Err(ManagerError::Other(format!(
                "Device {} is already recording",
                device_id
            )));
        }

        tokio::fs::create_dir_all(&self.base_path)
            .await
            .map_err(|e| {
                ManagerError::Other(format!("Failed to create recording directory: {}", e))
            })?;

        let timestamp = chrono::Utc::now();
        let filename = format!(
            "device_{}_{}.mcap",
            device_id,
            timestamp.format("%Y%m%d_%H%M%S")
        );
        let file_path = self.base_path.join(filename);

        let request = self
            .devices_manager_handler
            .send(crate::device::manager::Request::Info(
                crate::device::manager::UuidWrapper { uuid: device_id },
            ))
            .await?;

        let device_info = match request {
            crate::device::manager::Answer::DeviceInfo(h) => h.first().unwrap().clone(),
            _ => return Err(ManagerError::Other("Invalid device handler".to_string())),
        };

        let ctx = Context::new();
        let mcap_writer: McapWriterHandle<BufWriter<File>> = ctx
            .mcap_writer()
            .create_new_buffered_file(&file_path)
            .map_err(|e| ManagerError::Other(format!("Failed to create MCAP file: {}", e)))?;

        let session = RecordingSession {
            device_id,
            file_path: file_path.clone(),
            is_active: true,
            start_time: timestamp,
            device_type: device_info.device_type.clone(),
        };

        let session_guard = SessionGuard {
            session: session.clone(),
            writer: Some(mcap_writer),
        };

        self.sessions.write().await.insert(device_id, session_guard);
        self.broadcast_status(&session).await;

        let sessions = self.sessions.clone();
        let devices_manager_handler = self.devices_manager_handler.clone();
        let vehicle_data = self.vehicle_data.clone();

        let device_handler = devices_manager_handler
            .send(crate::device::manager::Request::GetDeviceHandler(
                crate::device::manager::UuidWrapper { uuid: device_id },
            ))
            .await?;

        let handler = match device_handler {
            crate::device::manager::Answer::InnerDeviceHandler(h) => h,
            _ => return Err(ManagerError::Other("Invalid device handler".to_string())),
        };

        tokio::spawn(async move {
            if let Err(e) =
                Self::recording_task(handler, file_path, sessions, device_id, ctx, vehicle_data)
                    .await
            {
                error!("Recording task failed for device {}: {:?}", device_id, e);
            }
        });

        Ok(session)
    }

    pub async fn stop_recording(&self, device_id: Uuid) -> Result<RecordingSession, ManagerError> {
        let mut sessions = self.sessions.write().await;
        let session_guard = sessions.get_mut(&device_id).ok_or_else(|| {
            ManagerError::Other(format!("No recording session for device {}", device_id))
        })?;

        session_guard.session.is_active = false;
        if let Some(writer) = session_guard.writer.take() {
            writer
                .close()
                .map_err(|e| ManagerError::Other(format!("Failed to close MCAP writer: {}", e)))?;
        }
        let session = session_guard.session.clone();
        self.broadcast_status(&session).await;
        Ok(session)
    }

    pub async fn get_recording_status(
        &self,
        device_id: Uuid,
    ) -> Result<Option<RecordingSession>, ManagerError> {
        Ok(self
            .sessions
            .read()
            .await
            .get(&device_id)
            .map(|g| g.session.clone()))
    }

    pub async fn get_all_recording_status(&self) -> Result<Vec<RecordingSession>, ManagerError> {
        Ok(self
            .sessions
            .read()
            .await
            .values()
            .map(|g| g.session.clone())
            .collect())
    }

    async fn recording_task(
        handler: DeviceActorHandler,
        _file_path: PathBuf,
        sessions: Arc<RwLock<HashMap<Uuid, SessionGuard>>>,
        device_id: Uuid,
        ctx: Arc<Context>,
        vehicle_data: Arc<RwLock<Option<VehicleData>>>,
    ) -> Result<(), ManagerError> {
        let subscriber = handler
            .send(super::devices::PingRequest::GetSubscriber)
            .await
            .map_err(|err| {
                warn!("Something went wrong while executing get_subscriber, details: {err:?}");
                ManagerError::DeviceError(err)
            })?;

        let mut receiver = match subscriber {
            super::devices::PingAnswer::Subscriber(subscriber) => subscriber,
            msg => {
                error!("Failed to receive broadcasted message: {:?}", msg);
                return Err(ManagerError::NoDevices);
            }
        };

        // Define topic strings
        let ping1d_topic = format!("/device_{}/Ping1D", device_id);
        let ping360_topic = format!("/device_{}/Ping360", device_id);
        let vehicle_topic = format!("/device_{}/VehicleData", device_id);

        // Create device-specific channels with proper schema
        let ping1d_channel = ctx.channel_builder(&ping1d_topic).build::<ProfileStruct>();
        let ping360_channel = ctx
            .channel_builder(&ping360_topic)
            .build::<AutoDeviceDataStruct>();
        let vehicle_channel = ctx.channel_builder(&vehicle_topic).build::<VehicleData>();

        while {
            let sessions_guard = sessions.read().await;
            sessions_guard
                .get(&device_id)
                .map(|s| s.session.is_active)
                .unwrap_or(false)
        } {
            match receiver.recv().await {
                Ok(msg) => {
                    let timestamp = foxglove::schemas::Timestamp::now();
                    // Handle Ping360
                    if let Ok(bluerobotics_ping::Messages::Ping360(
                        bluerobotics_ping::ping360::Messages::AutoDeviceData(answer),
                    )) = bluerobotics_ping::Messages::try_from(&msg)
                    {
                        ping360_channel.log_with_time(&answer, timestamp);
                    } else if let Ok(bluerobotics_ping::Messages::Ping360(
                        bluerobotics_ping::ping360::Messages::DeviceData(answer),
                    )) = bluerobotics_ping::Messages::try_from(&msg)
                    {
                        let autotransducer = AutoDeviceDataStruct {
                            mode: answer.mode,
                            gain_setting: answer.gain_setting,
                            angle: answer.angle,
                            transmit_duration: answer.transmit_duration,
                            sample_period: answer.sample_period,
                            transmit_frequency: answer.transmit_frequency,
                            start_angle: 0,
                            stop_angle: 399,
                            num_steps: 1,
                            delay: 0,
                            number_of_samples: answer.number_of_samples,
                            data_length: answer.number_of_samples,
                            data: answer.data,
                        };
                        ping360_channel.log_with_time(&autotransducer, timestamp);
                    } else if let Ok(bluerobotics_ping::Messages::Ping1D(
                        bluerobotics_ping::ping1d::Messages::Profile(answer),
                    )) = bluerobotics_ping::Messages::try_from(&msg)
                    {
                        ping1d_channel.log_with_time(&answer, timestamp);
                    }
                    if let Some(vehicle) = vehicle_data.read().await.as_ref() {
                        vehicle_channel.log_with_time(vehicle, timestamp);
                    }
                }
                Err(e) => {
                    error!("Failed to receive broadcasted message: {:?}", e);
                    break;
                }
            }
        }

        sessions.write().await.remove(&device_id);
        Ok(())
    }
}

impl RecordingsManagerHandler {
    pub async fn send(&self, request: RecordingManagerCommand) -> Result<Answer, ManagerError> {
        let (result_sender, result_receiver) = oneshot::channel();

        trace!("Handling RecordingManager request: {request:?}: Forwarding request.");
        let device_request = ManagerActorRequest {
            request,
            respond_to: result_sender,
        };

        self.sender
            .send(device_request)
            .await
            .map_err(|err| ManagerError::TokioMpsc(err.to_string()))?;

        match result_receiver
            .await
            .map_err(|err| ManagerError::TokioMpsc(err.to_string()))?
        {
            Ok(ans) => {
                trace!("Handling RecordingManager request: Success");
                Ok(ans)
            }
            Err(err) => {
                error!("Handling RecordingManager request: Error occurred on manager: {err:?}",);
                Err(err)
            }
        }
    }
}
