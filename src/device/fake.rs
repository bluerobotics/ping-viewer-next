use std::{
    pin::Pin,
    task::{ready, Poll},
    time::Duration,
};

use actix_web::web::BytesMut;
use bluerobotics_ping::message::{DeserializePayload, ProtocolMessage};
use tokio::{
    io::{AsyncRead, AsyncWrite},
    sync::mpsc::{self, Receiver, Sender},
    task::JoinHandle,
    time::{interval, Instant},
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

    fn fake_distance_ping1d(duration: Duration) -> u32 {
        let duration = duration.as_secs_f32();
        (27_000.0
            + ((duration / 5.0).sin() * 5_000.0)
            + ((duration / 10.0).sin() * 2_500.0)
            + (((duration - 1.4) / 15.0).sin() * 2_000.0)) as u32
    }

    fn fake_angle_data_ping360(duration: Duration) -> (u16, Vec<u8>) {
        let duration = duration.as_secs_f32();
        (
            (duration / 0.200).floor() as u16,
            vec![
                0,
                0,
                0,
                match duration % 6.0 {
                    phase @ 4.0..6.0 => {
                        // duration is 4.0 or 6.0 => cosine phase is ±pi/2 => value is 0
                        // duration is 5.0 => cosine phase is 0 => value is max
                        (((phase - 5.0) * std::f32::consts::FRAC_PI_2).cos() * 255.0).floor() as u8
                    }
                    _ => 0,
                },
                0,
                0,
            ],
        )
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
                                        let start = Instant::now();
                                        let mut interval = interval(Duration::from_millis(500));
                                        interval.tick().await;

                                        for i in 0.. {
                                            // https://docs.bluerobotics.com/ping-protocol/pingmessage-ping1d/#1300-profile
                                            let reply =
                                                bluerobotics_ping::ping1d::Messages::Profile(
                                                    bluerobotics_ping::ping1d::ProfileStruct {
                                                        distance: FakeStream::fake_distance_ping1d(
                                                            Instant::now().duration_since(start),
                                                        ),
                                                        confidence: 100,
                                                        transmit_duration: 10_000,
                                                        ping_number: i,
                                                        scan_start: 0,
                                                        scan_length: 100_000,
                                                        gain_setting: 0,
                                                        profile_data_length: 0,
                                                        profile_data: vec![],
                                                    },
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
                            let start = Instant::now();
                            let mut interval = interval(Duration::from_millis(200));
                            interval.tick().await;

                            loop {
                                let (angle, data) = FakeStream::fake_angle_data_ping360(
                                    Instant::now().duration_since(start),
                                );
                                // https://docs.bluerobotics.com/ping-protocol/pingmessage-ping360/#2301-auto_device_data
                                let reply = bluerobotics_ping::ping360::Messages::AutoDeviceData(
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
                                        number_of_samples: payload.number_of_samples,
                                        data_length: data.len() as u16,
                                        data,
                                    },
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
