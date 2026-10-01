use bluerobotics_ping::{ping1d::ProfileStruct, ping360::AutoDeviceDataStruct};
use foxglove::Context;
use foxglove::McapWriterHandle;
use futures_util::stream::FuturesUnordered;
use futures_util::StreamExt;
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

use crate::device::{
    devices::DeviceActorHandler,
    manager::{DeviceSelection, ManagerError},
};
use crate::vehicle::VehicleData;

use super::manager::{DeviceSlotWrapper, ManagerActorHandler};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordingSession {
    pub device_type: DeviceSelection,
    pub slot: u8,
    pub file_path: PathBuf,
    pub is_active: bool,
    pub start_time: chrono::DateTime<chrono::Utc>,
}

pub struct SessionGuard {
    pub session: RecordingSession,
    pub writer: Option<McapWriterHandle<BufWriter<File>>>,
}

pub struct RecordingManager {
    receiver: mpsc::Receiver<ManagerActorRequest>,
    sessions: Arc<RwLock<HashMap<DeviceSlotWrapper, SessionGuard>>>,
    base_path: PathBuf,
    status_broadcast: broadcast::Sender<RecordingSession>,
    devices_manager_handler: ManagerActorHandler,
    vehicle_data: Arc<RwLock<Option<VehicleData>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Apiv2Schema)]
#[serde(tag = "command", content = "payload")]
pub enum RecordingManagerCommand {
    StartRecording(DeviceSlotWrapper),
    StopRecording(DeviceSlotWrapper),
    GetRecordingStatus(DeviceSlotWrapper),
    GetAllRecordingStatus,
    GetSubscriber,
    #[serde(skip)]
    Shutdown,
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
    #[serde(skip)]
    Shutdown,
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
        trace!(?actor_request, "RecordingsManager: Received a request");

        let result = match actor_request.request {
            RecordingManagerCommand::StartRecording(DeviceSlotWrapper { device_type, slot }) => {
                self.start_recording(device_type, slot)
                    .await
                    .map(Answer::RecordingSession)
            }
            RecordingManagerCommand::StopRecording(DeviceSlotWrapper { device_type, slot }) => self
                .stop_recording(device_type, slot)
                .await
                .map(Answer::RecordingSession),
            RecordingManagerCommand::GetRecordingStatus(DeviceSlotWrapper {
                device_type,
                slot,
            }) => self
                .get_recording_status(device_type, slot)
                .await
                .map(Answer::RecordingStatus),
            RecordingManagerCommand::GetAllRecordingStatus => self
                .get_all_recording_status()
                .await
                .map(Answer::AllRecordingStatus),
            RecordingManagerCommand::GetSubscriber => {
                Ok(Answer::RecordingManager(self.subscribe()))
            }
            RecordingManagerCommand::Shutdown => {
                self.shutdown().await;
                Ok(Answer::Shutdown)
            }
        };

        if let Err(error) = actor_request.respond_to.send(result) {
            error!(?error, "RecordingsManager: Failed to return response");
        }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<RecordingSession> {
        self.status_broadcast.subscribe()
    }

    async fn broadcast_status(&self, session: &RecordingSession) {
        let _ = self.status_broadcast.send(session.clone());
    }

    pub async fn start_recording(
        &self,
        device_type: DeviceSelection,
        slot: u8,
    ) -> Result<RecordingSession, ManagerError> {
        let key = DeviceSlotWrapper { device_type, slot };
        if self.sessions.read().await.contains_key(&key) {
            return Err(ManagerError::Other(format!(
                "Device {}/{} is already recording",
                key.device_type, key.slot,
            )));
        }

        tokio::fs::create_dir_all(&self.base_path)
            .await
            .map_err(|e| {
                ManagerError::Other(format!("Failed to create recording directory: {}", e))
            })?;

        let timestamp = chrono::Utc::now();
        let filename = format!(
            "device_{}_{}_{}.mcap",
            key.device_type,
            key.slot,
            timestamp.format("%Y%m%d_%H%M%S")
        );
        let file_path = self.base_path.join(filename);

        let ctx = Context::new();
        let mcap_writer: McapWriterHandle<BufWriter<File>> = ctx
            .mcap_writer()
            .create_new_buffered_file(&file_path)
            .map_err(|e| ManagerError::Other(format!("Failed to create MCAP file: {}", e)))?;

        let session = RecordingSession {
            device_type,
            slot,
            file_path: file_path.clone(),
            is_active: true,
            start_time: timestamp,
        };

        let session_guard = SessionGuard {
            session: session.clone(),
            writer: Some(mcap_writer),
        };

        self.sessions.write().await.insert(key, session_guard);
        self.broadcast_status(&session).await;

        let sessions = self.sessions.clone();
        let devices_manager_handler = self.devices_manager_handler.clone();
        let vehicle_data = self.vehicle_data.clone();

        let device_handler = devices_manager_handler
            .send(crate::device::manager::Request::GetDeviceHandler(
                crate::device::manager::DeviceSlotWrapper { device_type, slot },
            ))
            .await?;

        let handler = match device_handler {
            crate::device::manager::Answer::InnerDeviceHandler(h) => h,
            _ => return Err(ManagerError::Other("Invalid device handler".to_string())),
        };

        tokio::spawn(async move {
            if let Err(error) = Self::recording_task(
                handler,
                file_path,
                sessions,
                device_type,
                slot,
                ctx,
                vehicle_data,
            )
            .await
            {
                error!(%device_type, slot, ?error, "Recording task failed for device");
            }
        });

        Ok(session)
    }

    pub async fn stop_recording(
        &self,
        device_type: DeviceSelection,
        slot: u8,
    ) -> Result<RecordingSession, ManagerError> {
        let key = DeviceSlotWrapper { device_type, slot };
        let mut sessions = self.sessions.write().await;
        let session_guard = sessions.get_mut(&key).ok_or_else(|| {
            ManagerError::Other(format!(
                "No recording session for device {}/{}",
                key.device_type, key.slot
            ))
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
        device_type: DeviceSelection,
        slot: u8,
    ) -> Result<Option<RecordingSession>, ManagerError> {
        Ok(self
            .sessions
            .read()
            .await
            .get(&DeviceSlotWrapper { device_type, slot })
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
        sessions: Arc<RwLock<HashMap<DeviceSlotWrapper, SessionGuard>>>,
        device_type: DeviceSelection,
        slot: u8,
        ctx: Arc<Context>,
        vehicle_data: Arc<RwLock<Option<VehicleData>>>,
    ) -> Result<(), ManagerError> {
        let key = DeviceSlotWrapper { device_type, slot };

        let subscriber = handler
            .send(super::devices::PingRequest::GetSubscriber)
            .await
            .map_err(|error| {
                warn!(
                    ?error,
                    "Something went wrong while executing get_subscriber"
                );
                ManagerError::DeviceError(error)
            })?;

        let mut receiver = match subscriber {
            super::devices::PingAnswer::Subscriber(subscriber) => subscriber,
            message => {
                error!(?message, "Failed to receive broadcasted message");
                return Err(ManagerError::NoDevices);
            }
        };

        // Define topic strings
        let ping1d_topic = format!("device_{}_{}/Ping1D", key.device_type, key.slot);
        let ping360_topic = format!("device_{}_{}/Ping360", key.device_type, key.slot);
        let vehicle_topic = format!("device_{}_{}/VehicleData", key.device_type, key.slot);

        // Create device-specific channels with proper schema
        let ping1d_channel = ctx.channel_builder(&ping1d_topic).build::<ProfileStruct>();
        let ping360_channel = ctx
            .channel_builder(&ping360_topic)
            .build::<AutoDeviceDataStruct>();
        let vehicle_channel = ctx.channel_builder(&vehicle_topic).build::<VehicleData>();

        while {
            let sessions_guard = sessions.read().await;
            sessions_guard
                .get(&key)
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
                Err(error) => {
                    error!(?error, "Failed to receive broadcasted message");
                    break;
                }
            }
        }

        sessions.write().await.remove(&key);
        Ok(())
    }

    pub async fn shutdown(&self) {
        let device_ids: Vec<_> = self.sessions.read().await.keys().cloned().collect();
        let mut futures: FuturesUnordered<_> = device_ids
            .into_iter()
            .map(|DeviceSlotWrapper { device_type, slot }| self.stop_recording(device_type, slot))
            .collect();
        while let Some(result) = futures.next().await {
            if let Err(error) = result {
                warn!("RecordingManager shutdown failed: {error}");
            }
        }
    }
}

impl RecordingsManagerHandler {
    pub async fn send(&self, request: RecordingManagerCommand) -> Result<Answer, ManagerError> {
        let (result_sender, result_receiver) = oneshot::channel();

        trace!(
            ?request,
            "Handling RecordingManager request: Forwarding request."
        );
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
            Ok(answer) => {
                trace!("Handling RecordingManager request: Success");
                Ok(answer)
            }
            Err(error) => {
                error!(
                    ?error,
                    "Handling RecordingManager request: Error occurred on manager",
                );
                Err(error)
            }
        }
    }
}
