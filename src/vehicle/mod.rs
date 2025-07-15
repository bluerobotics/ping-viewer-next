use std::sync::Arc;

use mavlink::ardupilotmega::ATTITUDE_DATA;
use mavlink::ardupilotmega::GLOBAL_POSITION_INT_DATA;

use serde::Deserialize;
use serde::Serialize;
use tokio::sync::RwLock;
use tracing::{error, info};

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct VehicleData {
    #[schemars(description = "Roll angle in radians")]
    pub roll: f32,
    #[schemars(description = "Pitch angle in radians")]
    pub pitch: f32,
    #[schemars(description = "Yaw angle in radians")]
    pub yaw: f32,
    #[schemars(description = "Altitude in meters above sea level")]
    pub alt: f64,
    #[schemars(description = "Latitude in decimal degrees")]
    pub lat: f64,
    #[schemars(description = "Longitude in decimal degrees")]
    pub lon: f64,
}

#[derive(Deserialize)]
struct Envelope<T> {
    message: T,
}

fn make_default_config(node_name: &str) -> zenoh::Config {
    let mut config = zenoh::Config::default();

    // Set client mode (common to both)
    config
        .insert_json5("mode", r#""client""#)
        .expect("Failed to insert client mode");
    config
        .insert_json5("metadata", &format!(r#"{{"name": "{}"}}"#, node_name))
        .expect("Failed to insert metadata");
    config
        .insert_json5("adminspace/enabled", r#"true"#)
        .expect("Failed to insert adminspace/enabled");

    #[cfg(feature = "blueos-extension")]
    {
        config
            .insert_json5("connect/endpoints", r#"["tcp/127.0.0.1:7447"]"#)
            .expect("Failed to insert endpoints");
    }
    info!("Generated zenoh config with default settings");
    config
}

pub async fn zenoh_client_bridge(latest_pose: Arc<RwLock<Option<VehicleData>>>) {
    let node_name = env!("CARGO_PKG_NAME");

    let config = make_default_config(node_name);

    let session = match zenoh::open(config).await {
        Ok(s) => s,
        Err(e) => {
            error!("Zenoh session error: {e}");
            return;
        }
    };
    let attitude_sub = match session.declare_subscriber("mavlink/**/1/ATTITUDE").await {
        Ok(s) => s,
        Err(e) => {
            error!("Zenoh subscribe error for ATTITUDE: {e}");
            return;
        }
    };
    let position_sub = match session
        .declare_subscriber("mavlink/**/1/GLOBAL_POSITION_INT")
        .await
    {
        Ok(s) => s,
        Err(e) => {
            error!("Zenoh subscribe error for GLOBAL_POSITION_INT: {e}");
            return;
        }
    };
    info!("Subscribed to mavlink/**/1/ATTITUDE and mavlink/**/1/GLOBAL_POSITION_INT");

    let mut latest_attitude: Option<ATTITUDE_DATA> = None;
    let mut latest_position: Option<GLOBAL_POSITION_INT_DATA> = None;

    loop {
        tokio::select! {
            Ok(sample) = attitude_sub.recv_async() => {
                if let Ok(env) = serde_json5::from_slice::<Envelope<ATTITUDE_DATA>>(&sample.payload().to_bytes()) {
                    latest_attitude = Some(env.message);
                }
            }
            Ok(sample) = position_sub.recv_async() => {
                if let Ok(env) = serde_json5::from_slice::<Envelope<GLOBAL_POSITION_INT_DATA>>(&sample.payload().to_bytes()) {
                    latest_position = Some(env.message);
                }
            }
        }

            if let (Some(att), Some(pos)) = (&latest_attitude, &latest_position) {
                let pose = VehicleData {
                    roll: att.roll,
                    pitch: att.pitch,
                    yaw: att.yaw,
                    alt: pos.alt as f64 / 1000.0,
                    lat: pos.lat as f64 / 1e7,
                    lon: pos.lon as f64 / 1e7,
                };
                let mut pose_guard = latest_pose.write().await;
                *pose_guard = Some(pose);
            }
        }

        error!("Zenoh client bridge disconnected, retrying in {reconnect_delay_secs}s");
    }
}
