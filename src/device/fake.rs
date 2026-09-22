use std::{
    pin::Pin,
    task::{ready, Poll},
    time::Duration,
};

use actix_web::web::BytesMut;
use bluerobotics_ping::{
    message::{DeserializePayload, ProtocolMessage},
    ping360::{AutoDeviceDataStruct, AutoTransmitStruct},
};
use rand::{rngs::StdRng, Rng, SeedableRng};
use tokio::{
    io::{AsyncRead, AsyncWrite},
    sync::mpsc::{self, Receiver, Sender},
    task::JoinHandle,
    time::interval,
};
use tokio_util::codec::Decoder;
use tracing::{debug, warn};

use crate::device::manager::DeviceSelection;

pub struct FakeStream {
    writer: Sender<ProtocolMessage>,
    writer_buf: BytesMut,
    /// Whether the poll_write data has been consumed on the current
    /// polling attempts.
    writer_buf_consumed: bool,
    /// Message that couldn't be sent in the latest poll_write/poll_flush attempt.
    writer_pending: Option<ProtocolMessage>,
    reader: Receiver<ProtocolMessage>,
    /// Data that couldn't be sent in the latest poll_read attempt.
    reader_pending: Option<Vec<u8>>,
}

impl FakeStream {
    pub fn new(device_selection: DeviceSelection) -> Self {
        let (writer, rx) = mpsc::channel(64);
        let (tx, reader) = mpsc::channel(64);
        tokio::spawn(FakeStream::run_loop(device_selection, tx, rx));

        FakeStream {
            writer,
            writer_buf: BytesMut::with_capacity(4096),
            writer_buf_consumed: false,
            writer_pending: None,
            reader,
            reader_pending: None,
        }
    }

    /// Generate a parabolic echo bump profile surrounded by noise
    fn fake_echo_profile(
        points: usize,
        bump_start: f32,
        bump_stop: f32,
        rng: &mut StdRng,
    ) -> Vec<u8> {
        let bump_width = bump_stop - bump_start;

        (0..points)
            .map(|index| {
                let index = index as f32;
                let point = if index < bump_start {
                    0.1 * ((rng.next_u32() >> 24) as f32)
                } else if index < bump_stop {
                    255.0
                        * ((-4.0 / bump_width.powi(2))
                            * (index - bump_start - bump_width / 2.0).powi(2)
                            + 1.0)
                } else {
                    0.45 * ((rng.next_u32() >> 24) as f32)
                };
                point as u8
            })
            .collect()
    }

    /// Simulates a Ping1D parabolic echo bump with oscillating bump and scan range
    fn fake_profile_ping1d(
        ping_number: u32,
        rng: &mut StdRng,
    ) -> bluerobotics_ping::ping1d::ProfileStruct {
        const POINTS: usize = 200;
        const MAX_SCAN_LENGTH: f32 = 120_000.0;

        let counter = ping_number as f32;
        let bump_start = POINTS as f32 / 2.0 - 10.0 * (counter / 10.0).sin();
        let bump_stop = 3.0 * POINTS as f32 / 5.0 + 6.0 * (counter / 5.5).cos();
        let bump_width = bump_stop - bump_start;
        let scan_length = MAX_SCAN_LENGTH * (1.3 + (counter / 40.0).cos()) / 2.3;
        let profile_data = FakeStream::fake_echo_profile(POINTS, bump_start, bump_stop, rng);

        bluerobotics_ping::ping1d::ProfileStruct {
            distance: (scan_length * (bump_stop + bump_start) / (POINTS as f32 * 2.0)) as u32,
            confidence: (400.0 / bump_width) as u16,
            transmit_duration: 200,
            ping_number,
            scan_start: 0,
            scan_length: scan_length as u32,
            gain_setting: 4,
            profile_data_length: profile_data.len() as u16,
            profile_data,
        }
    }

    /// Simulates a Ping360 full-length echo profile at a heading that wraps every 400 gradians,
    /// as if the transducer were slowly drifting around the center of a round pool
    fn fake_device_data_ping360(
        payload: &AutoTransmitStruct,
        counter: u32,
        number_of_samples: u16,
        rng: &mut StdRng,
    ) -> AutoDeviceDataStruct {
        const ANGULAR_RESOLUTION: u32 = 400;
        // Pool wall distance from its center, as a fraction of the scanned range
        const POOL_RADIUS: f32 = 0.7;
        // Echo thickness of the pool wall, as a fraction of the scanned range
        const WALL_THICKNESS: f32 = 0.05;
        // Transducer distance from the pool center, as a fraction of the pool radius
        const DRIFT: f32 = 0.08;

        let points = number_of_samples.max(1) as usize;
        let angle = (counter % ANGULAR_RESOLUTION) as u16;

        let radius = POOL_RADIUS * points as f32;
        let heading = f32::from(angle) * std::f32::consts::TAU / ANGULAR_RESOLUTION as f32;
        let revolutions = counter as f32 / ANGULAR_RESOLUTION as f32;

        // x and y oscillate at different periods, tracing a curve that does not close
        // on each spin the way a circular orbit would
        // (<https://en.wikipedia.org/wiki/Lissajous_curve>)
        let drift_x = DRIFT * radius * (revolutions * std::f32::consts::TAU / 5.0).sin();
        let drift_y = DRIFT * radius * (revolutions * std::f32::consts::TAU / 7.0).sin();
        let drift = drift_x.hypot(drift_y);
        let bearing = heading - drift_y.atan2(drift_x);

        // Ray-circle intersection from the drifted transducer towards the pool wall
        let wall = -drift * bearing.cos()
            + (radius.powi(2) - (drift * bearing.sin()).powi(2))
                .max(0.0)
                .sqrt();

        let half_thickness = WALL_THICKNESS * points as f32 / 2.0;
        let data = FakeStream::fake_echo_profile(
            points,
            wall - half_thickness,
            wall + half_thickness,
            rng,
        );
        bluerobotics_ping::ping360::AutoDeviceDataStruct {
            mode: payload.mode,
            gain_setting: payload.gain_setting,
            angle,
            transmit_duration: payload.transmit_duration,
            sample_period: payload.sample_period,
            transmit_frequency: payload.transmit_frequency,
            start_angle: payload.start_angle,
            stop_angle: payload.stop_angle,
            num_steps: payload.num_steps,
            delay: payload.delay,
            number_of_samples,
            data_length: data.len() as u16,
            data,
        }
    }

    /// Runs a simulated device loop for the given device selection (Ping1D or Ping360).
    async fn run_loop(
        device_selection: DeviceSelection,
        tx: Sender<ProtocolMessage>,
        mut rx: Receiver<ProtocolMessage>,
    ) {
        let mut ping1d_profile_task: Option<JoinHandle<()>> = None;
        let mut ping360_auto_device_data_task: Option<JoinHandle<()>> = None;

        while let Some(message) = rx.recv().await {
            if !message.has_valid_crc() {
                continue;
            }
            match message.message_id {
                // general_request (must reply immediately)
                6 => {
                    if let Some(chunk) = message.payload().first_chunk::<2>() {
                        match u16::from_le_bytes(*chunk) {
                            // device_information
                            4 => {
                                // https://docs.bluerobotics.com/ping-protocol/pingmessage-common/#get
                                let reply = bluerobotics_ping::common::Messages::DeviceInformation(
                                    bluerobotics_ping::common::DeviceInformationStruct {
                                        device_type: match device_selection {
                                            // ping1d simulation
                                            DeviceSelection::Common
                                            | DeviceSelection::Ping1D
                                            | DeviceSelection::Auto => 1,
                                            // ping360 simulation
                                            DeviceSelection::Ping360 => 2,
                                        },
                                        device_revision: 1,
                                        firmware_version_major: 3,
                                        firmware_version_minor: 3,
                                        firmware_version_patch: 0,
                                        reserved: 0,
                                    },
                                );

                                let mut msg = ProtocolMessage::new();
                                msg.set_message(&reply);
                                let _ = tx.send(msg).await;
                            }

                            // protocol_version
                            5 => {
                                // https://docs.bluerobotics.com/ping-protocol/pingmessage-common/#5-protocol_version
                                let reply = bluerobotics_ping::common::Messages::ProtocolVersion(
                                    bluerobotics_ping::common::ProtocolVersionStruct {
                                        version_major: 1,
                                        version_minor: 1,
                                        version_patch: 0,
                                        reserved: 0,
                                    },
                                );

                                let mut msg = ProtocolMessage::new();
                                msg.set_message(&reply);
                                let _ = tx.send(msg).await;
                            }

                            // ping360 device_data
                            2300 => {
                                // https://docs.bluerobotics.com/ping-protocol/pingmessage-ping360/#2300-device_data
                                let reply = bluerobotics_ping::ping360::Messages::DeviceData(
                                    bluerobotics_ping::ping360::DeviceDataStruct {
                                        mode: 1,
                                        gain_setting: 1,
                                        angle: 0,
                                        transmit_duration: 1_000,
                                        sample_period: 80,
                                        transmit_frequency: 650,
                                        number_of_samples: 1_200,
                                        data_length: 0,
                                        data: vec![],
                                    },
                                );

                                let mut msg = ProtocolMessage::new();
                                msg.set_message(&reply);
                                let _ = tx.send(msg).await;
                            }

                            _ => debug!(?message, "FakeStream: Unhandled general_request message"),
                        }
                    } else {
                        warn!("FakeStream: Invalid payload for general_request message");
                    }
                }

                // ping1d continuous_start (starts an event stream)
                1400 if matches!(
                    device_selection,
                    DeviceSelection::Common | DeviceSelection::Ping1D | DeviceSelection::Auto
                ) =>
                {
                    if let Some(chunk) = message.payload().first_chunk::<2>() {
                        match u16::from_le_bytes(*chunk) {
                            // profile
                            1300 => {
                                let tx = tx.clone();
                                if let Some(handle) =
                                    ping1d_profile_task.replace(tokio::spawn(async move {
                                        let mut rng = StdRng::seed_from_u64(0x426c7565);
                                        let mut interval = interval(Duration::from_millis(50));
                                        interval.tick().await;

                                        for i in 0.. {
                                            // https://docs.bluerobotics.com/ping-protocol/pingmessage-ping1d/#1300-profile
                                            let reply =
                                                bluerobotics_ping::ping1d::Messages::Profile(
                                                    FakeStream::fake_profile_ping1d(i, &mut rng),
                                                );

                                            let mut msg = ProtocolMessage::new();
                                            msg.set_message(&reply);
                                            let _ = tx.send(msg).await;

                                            interval.tick().await;
                                        }

                                        std::future::pending::<()>().await
                                    }))
                                {
                                    handle.abort();
                                }
                            }

                            _ => debug!(
                                ?message,
                                "FakeStream: Unhandled ping1d continuous_start message"
                            ),
                        }
                    } else {
                        warn!("FakeStream: Invalid payload for ping1d continuous_start message");
                    }
                }

                // ping1d continuous_stop (stops an event stream)
                1401 if matches!(
                    device_selection,
                    DeviceSelection::Common | DeviceSelection::Ping1D | DeviceSelection::Auto
                ) =>
                {
                    if let Some(chunk) = message.payload().first_chunk::<2>() {
                        match u16::from_le_bytes(*chunk) {
                            // profile
                            1300 => {
                                if let Some(handle) = ping1d_profile_task.take() {
                                    handle.abort();
                                }
                            }

                            _ => debug!(
                                ?message,
                                "FakeStream: Unhandled ping1d continuous_stop message"
                            ),
                        }
                    } else {
                        warn!("FakeStream: Invalid payload for ping1d continuous_stop message");
                    }
                }

                // ping360 auto_transmit (starts an auto_device_data stream)
                2602 if matches!(device_selection, DeviceSelection::Ping360) => {
                    let payload = bluerobotics_ping::ping360::AutoTransmitStruct::deserialize(
                        message.payload(),
                    );

                    let tx = tx.clone();

                    if let Some(handle) =
                        ping360_auto_device_data_task.replace(tokio::spawn(async move {
                            let mut rng = StdRng::seed_from_u64(0x426c7565);
                            let mut interval = interval(Duration::from_millis(7));
                            interval.tick().await;

                            let number_of_samples = payload.number_of_samples.max(1);
                            for i in 0.. {
                                // https://docs.bluerobotics.com/ping-protocol/pingmessage-ping360/#2301-auto_device_data
                                let reply = bluerobotics_ping::ping360::Messages::AutoDeviceData(
                                    FakeStream::fake_device_data_ping360(
                                        &payload,
                                        i,
                                        number_of_samples,
                                        &mut rng,
                                    ),
                                );

                                let mut msg = ProtocolMessage::new();
                                msg.set_message(&reply);
                                let _ = tx.send(msg).await;

                                interval.tick().await;
                            }
                        }))
                    {
                        handle.abort();
                    }
                }

                // ping360 motor_off (replies with ack and cancels the auto_device_data stream)
                2903 if matches!(device_selection, DeviceSelection::Ping360) => {
                    if let Some(handle) = ping360_auto_device_data_task.take() {
                        handle.abort();
                    }

                    // https://docs.bluerobotics.com/ping-protocol/pingmessage-common/#1-ack
                    let reply = bluerobotics_ping::common::Messages::Ack(
                        bluerobotics_ping::common::AckStruct {
                            acked_id: message.message_id,
                        },
                    );

                    let mut msg = ProtocolMessage::new();
                    msg.set_message(&reply);
                    let _ = tx.send(msg).await;
                }

                _ => debug!(?message, "FakeStream: Unhandled message"),
            }
        }

        if let Some(handle) = ping1d_profile_task.take() {
            handle.abort();
        }
        if let Some(handle) = ping360_auto_device_data_task.take() {
            handle.abort();
        }
    }

    /// Helper method to send Ping protocol messages to the channel in the FakeStream's loop.
    fn poll_send_message(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        message: ProtocolMessage,
    ) -> Poll<Result<(), std::io::Error>> {
        // Reserve a slot for the writer
        let mut poll_sender = tokio_util::sync::PollSender::new(self.writer.clone());
        match poll_sender.poll_reserve(cx) {
            // Slot has been reserved or channel is closed
            Poll::Ready(result) => result
                .map_err(|error| std::io::Error::new(std::io::ErrorKind::BrokenPipe, error))?,
            // All slots are occupied; save message to retry later
            Poll::Pending => {
                self.writer_pending = Some(message);
                return Poll::Pending;
            }
        }

        // Send decoded message to writer
        poll_sender
            .send_item(message)
            .map_err(|error| std::io::Error::new(std::io::ErrorKind::BrokenPipe, error))?;

        Poll::Ready(Ok(()))
    }
}

impl AsyncRead for FakeStream {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        let mut data = if let Some(data) = self.reader_pending.take() {
            data
        } else if let Some(message) = ready!(self.reader.poll_recv(cx)) {
            message.serialized()
        } else {
            // Channel is closed
            return Poll::Ready(Ok(()));
        };

        if buf.remaining() == 0 {
            // Buffer is full
            self.reader_pending = Some(data);
            return Poll::Ready(Ok(()));
        } else if buf.remaining() < data.len() {
            // Buffer doesn't have enough capacity; write until we fill it
            // and save the rest to reader_pending
            self.reader_pending = Some(data.split_off(buf.remaining()));
        }
        buf.put_slice(&data);
        Poll::Ready(Ok(()))
    }
}

impl AsyncWrite for FakeStream {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, std::io::Error>> {
        if !self.writer_buf_consumed {
            self.writer_buf.extend_from_slice(buf);
            self.writer_buf_consumed = true;
        }

        if let Some(message) = self.writer_pending.take() {
            ready!(self.as_mut().poll_send_message(cx, message))?;
        }

        // Attempt to decode one or more messages
        let mut codec = bluerobotics_ping::codec::PingCodec::new();
        loop {
            match codec.decode(&mut self.writer_buf) {
                Err(error) => {
                    return Poll::Ready(Err(std::io::Error::other(format!("{:?}", error))))
                }
                // Frame isn't finished yet
                Ok(None) => break,
                // Frame is finished
                Ok(Some(message)) => ready!(self.as_mut().poll_send_message(cx, message))?,
            }
        }

        self.writer_buf_consumed = false;
        Poll::Ready(Ok(buf.len()))
    }

    fn poll_flush(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Result<(), std::io::Error>> {
        if let Some(message) = self.writer_pending.take() {
            self.as_mut().poll_send_message(cx, message)
        } else {
            Poll::Ready(Ok(()))
        }
    }

    fn poll_shutdown(
        self: Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> Poll<Result<(), std::io::Error>> {
        Poll::Ready(Ok(()))
    }
}
