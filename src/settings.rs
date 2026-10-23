use std::{
    collections::HashMap,
    fs::File,
    io::{BufReader, BufWriter, Write},
    ops::{Deref, DerefMut},
    path::Path,
    sync::{LazyLock, Mutex, MutexGuard},
};

use paperclip::actix::Apiv2Schema;
use serde::{Deserialize, Serialize};
use tokio::sync::watch::{channel, Sender};
use tracing::error;

use crate::{
    cli,
    device::manager::{slots::DeviceConnection, DeviceSelection, Ping360Config},
};

fn default_fake_id_counter() -> u16 {
    1
}

fn default_mount_offset() -> u16 {
    180
}

fn default_a_scan() -> bool {
    true
}

fn default_color_palette() -> String {
    "Thermal Blue".to_string()
}

fn default_background_mode() -> String {
    "gradient".to_string()
}

fn default_background_color() -> String {
    "#001a2e".to_string()
}

/// Plot appearance shared by Ping1D and Ping360. Units and theme stay in the browser.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Apiv2Schema)]
pub struct DisplayProfile {
    #[serde(default = "default_a_scan")]
    pub a_scan: bool,
    #[serde(default = "default_color_palette")]
    pub color_palette: String,
    #[serde(default)]
    pub debug_mode: bool,
    #[serde(default = "default_background_mode")]
    pub background_mode: String,
    #[serde(default = "default_background_color")]
    pub background_color: String,
}

impl Default for DisplayProfile {
    fn default() -> Self {
        Self {
            a_scan: default_a_scan(),
            color_palette: default_color_palette(),
            debug_mode: false,
            background_mode: default_background_mode(),
            background_color: default_background_color(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Apiv2Schema)]
pub struct ViewerSettings {
    /// Starts at 0 and increases with every update
    pub revision: u64,
    pub head_down: bool,
    /// Degrees of display rotation for a Ping360 mounted away from its default heading
    pub mount_offset: u16,
    pub display: DisplayProfile,
}

#[derive(Debug, Clone, Default, Deserialize, Apiv2Schema)]
pub struct ViewerSettingsUpdate {
    /// Reject the update unless this is still the current revision
    #[serde(default)]
    pub expected_revision: Option<u64>,
    #[serde(default)]
    pub head_down: Option<bool>,
    #[serde(default)]
    pub mount_offset: Option<u16>,
    #[serde(default)]
    pub display: Option<DisplayProfile>,
}

/// Acoustic settings for a Ping360 sonar, keyed by slot
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct StoredPing360 {
    pub(crate) connection: DeviceConnection,
    pub(crate) config: Ping360Config,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct PingViewerNextV1Settings {
    #[serde(default)]
    pub(crate) fake_devices: HashMap<u16, DeviceSelection>,
    #[serde(default = "default_fake_id_counter")]
    pub(crate) fake_id_counter: u16,
    #[serde(default)]
    pub(crate) ping360: Vec<StoredPing360>,
    #[serde(default)]
    pub(crate) revision: u64,
    #[serde(default)]
    pub(crate) head_down: bool,
    #[serde(default = "default_mount_offset")]
    pub(crate) mount_offset: u16,
    #[serde(default)]
    pub(crate) display: DisplayProfile,
}

impl Default for PingViewerNextV1Settings {
    fn default() -> Self {
        Self {
            fake_devices: HashMap::new(),
            fake_id_counter: default_fake_id_counter(),
            ping360: Vec::new(),
            revision: 0,
            head_down: false,
            mount_offset: default_mount_offset(),
            display: DisplayProfile::default(),
        }
    }
}

pub(crate) fn stored_ping360_config(connection: &DeviceConnection) -> Option<Ping360Config> {
    let settings = SETTINGS.lock().expect("not poisoned");
    settings
        .ping360
        .iter()
        .find(|entry| &entry.connection == connection)
        .map(|entry| entry.config)
}

pub(crate) fn remember_ping360_config(connection: DeviceConnection, config: Ping360Config) {
    let mut settings = SETTINGS.lock().expect("not poisoned");
    if let Some(entry) = settings
        .ping360
        .iter_mut()
        .find(|entry| entry.connection == connection)
    {
        entry.config = config;
    } else {
        settings.ping360.push(StoredPing360 { connection, config });
    }
}

fn viewer_snapshot(settings: &PingViewerNextV1Settings) -> ViewerSettings {
    ViewerSettings {
        revision: settings.revision,
        head_down: settings.head_down,
        mount_offset: settings.mount_offset,
        display: settings.display.clone(),
    }
}

pub(crate) fn viewer_settings() -> ViewerSettings {
    viewer_snapshot(&SETTINGS.lock().expect("not poisoned"))
}

/// Fails with the current settings when `expected_revision` is stale
pub(crate) fn update_viewer_settings(
    update: ViewerSettingsUpdate,
) -> Result<ViewerSettings, ViewerSettings> {
    SETTINGS.update_viewer(update)
}

fn apply_viewer_update(settings: &mut PingViewerNextV1Settings, update: &ViewerSettingsUpdate) {
    settings.revision += 1;
    if let Some(head_down) = update.head_down {
        settings.head_down = head_down;
    }
    if let Some(mount_offset) = update.mount_offset {
        settings.mount_offset = mount_offset;
    }
    if let Some(display) = update.display.clone() {
        settings.display = display;
    }
}

/// Keep the next fake id strictly ahead of every id still stored
pub(crate) fn reconcile_fake_ids(settings: &mut PingViewerNextV1Settings) {
    let next_from_devices = settings
        .fake_devices
        .keys()
        .copied()
        .max()
        .and_then(|id| id.checked_add(1))
        .unwrap_or(1);
    settings.fake_id_counter = settings.fake_id_counter.max(next_from_devices).max(1);
}

pub(crate) struct PingViewerNextSettingsGuard<'a> {
    guard: MutexGuard<'a, PingViewerNextV1Settings>,
    writer_sender: Sender<PingViewerNextV1Settings>,
    dirty: bool,
}

impl Deref for PingViewerNextSettingsGuard<'_> {
    type Target = PingViewerNextV1Settings;

    fn deref(&self) -> &Self::Target {
        self.guard.deref()
    }
}

impl DerefMut for PingViewerNextSettingsGuard<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.dirty = true;
        self.guard.deref_mut()
    }
}

impl Drop for PingViewerNextSettingsGuard<'_> {
    fn drop(&mut self) {
        if self.dirty {
            if let Err(error) = self.writer_sender.send(self.guard.deref().clone()) {
                error!(?error, "settings writer is closed");
            }
        }
    }
}

pub(crate) struct PingViewerNextSettings {
    writer_sender: Sender<PingViewerNextV1Settings>,
    mutex: Mutex<PingViewerNextV1Settings>,
}

impl PingViewerNextSettings {
    pub(crate) fn lock(&self) -> Result<PingViewerNextSettingsGuard<'_>, &'static str> {
        match self.mutex.lock() {
            Ok(guard) => Ok(PingViewerNextSettingsGuard {
                guard,
                writer_sender: self.writer_sender.clone(),
                dirty: false,
            }),
            Err(_) => Err("mutex is poisoned"),
        }
    }

    fn update_viewer(
        &self,
        update: ViewerSettingsUpdate,
    ) -> Result<ViewerSettings, ViewerSettings> {
        let mut settings = self.lock().expect("not poisoned");
        // Only dereferenced immutably until the check passes, so a rejected update is not saved
        if update
            .expected_revision
            .is_some_and(|revision| revision != settings.revision)
        {
            return Err(viewer_snapshot(&settings));
        }
        apply_viewer_update(&mut settings, &update);
        Ok(viewer_snapshot(&settings))
    }
}

fn persist_settings_to(
    directory: &Path,
    settings: &PingViewerNextV1Settings,
) -> std::io::Result<()> {
    let temporary = directory.join("settings_v1.json.tmp");
    let target = directory.join("settings_v1.json");
    let write_result = (|| {
        let file = File::create(&temporary)?;
        let mut writer = BufWriter::new(file);
        serde_json::to_writer(&mut writer, settings)
            .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))?;
        writer.flush()?;
        writer.get_ref().sync_all()?;
        std::io::Result::Ok(())
    })();
    if let Err(error) = write_result {
        let _ = std::fs::remove_file(&temporary);
        return Err(error);
    }
    std::fs::rename(&temporary, &target)?;
    Ok(())
}

fn load_settings_from(directory: &Path, reset: bool) -> PingViewerNextSettings {
    let settings_path = directory.join("settings_v1.json");
    if reset && settings_path.exists() {
        if let Err(error) = std::fs::remove_file(&settings_path) {
            error!(
                ?error,
                path = %settings_path.display(),
                "failed to delete settings file for --reset"
            );
        }
    }

    let directory_path_buf = directory.to_path_buf();

    let (writer_sender, mut writer_receiver) = channel(PingViewerNextV1Settings::default());
    writer_receiver.mark_unchanged();

    tokio::spawn(async move {
        loop {
            if writer_receiver.changed().await.is_err() {
                return;
            };
            let last_value = writer_receiver.borrow_and_update().clone();

            let directory = directory_path_buf.clone();
            match tokio::task::spawn_blocking(move || persist_settings_to(&directory, &last_value))
                .await
            {
                Err(error) => error!(?error, "settings writer thread panicked"),
                Ok(Err(error)) => error!(?error, "failed to persist settings"),
                Ok(Ok(())) => {}
            }
        }
    });

    let mutex = if settings_path.exists() {
        match File::open(&settings_path) {
            Ok(file) => {
                match serde_json::from_reader::<_, PingViewerNextV1Settings>(BufReader::new(file)) {
                    Ok(mut parsed) => {
                        let stored_counter = parsed.fake_id_counter;
                        reconcile_fake_ids(&mut parsed);
                        if parsed.fake_id_counter != stored_counter {
                            writer_sender
                                .send(parsed.clone())
                                .expect("settings channel is empty");
                        }
                        Mutex::new(parsed)
                    }
                    Err(error) => {
                        error!(
                            ?error,
                            path = %settings_path.display(),
                            "settings file is invalid; starting from defaults"
                        );
                        let invalid_path = directory.join("settings_v1.json.invalid");
                        if let Err(rename_error) = std::fs::rename(&settings_path, &invalid_path) {
                            error!(?rename_error, "failed to move invalid settings file aside");
                        }
                        Mutex::default()
                    }
                }
            }
            Err(error) => {
                error!(
                    ?error,
                    "failed to open settings file; starting from defaults"
                );
                Mutex::default()
            }
        }
    } else {
        Mutex::default()
    };

    PingViewerNextSettings {
        writer_sender,
        mutex,
    }
}

pub(crate) static SETTINGS: LazyLock<PingViewerNextSettings> = LazyLock::new(|| {
    let settings_directory =
        cli::manager::settings_directory().expect("Failed to get settings directory");
    load_settings_from(&settings_directory, cli::manager::is_reset())
});

#[cfg(test)]
mod tests {
    use std::{path::PathBuf, time::Duration};

    use super::*;

    fn test_dir(name: &str) -> PathBuf {
        let directory = std::env::temp_dir().join(format!(
            "pvn-settings-{}-{}-{}",
            std::process::id(),
            name,
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system clock should be after the unix epoch")
                .as_nanos()
        ));
        std::fs::create_dir_all(&directory).expect("test directory should be creatable");
        directory
    }

    #[test]
    fn counter_advances_past_saved_ids() {
        let mut settings = PingViewerNextV1Settings {
            fake_devices: HashMap::from([
                (4, DeviceSelection::Ping1D),
                (2, DeviceSelection::Ping360),
            ]),
            fake_id_counter: 1,
            ..Default::default()
        };
        reconcile_fake_ids(&mut settings);
        assert_eq!(settings.fake_id_counter, 5);
    }

    #[test]
    fn counter_is_not_rewound() {
        let mut settings = PingViewerNextV1Settings {
            fake_devices: HashMap::from([(2, DeviceSelection::Ping360)]),
            fake_id_counter: 9,
            ..Default::default()
        };
        reconcile_fake_ids(&mut settings);
        assert_eq!(settings.fake_id_counter, 9);
    }

    #[test]
    fn zero_counter_starts_at_one() {
        let mut settings = PingViewerNextV1Settings {
            fake_devices: HashMap::new(),
            fake_id_counter: 0,
            ..Default::default()
        };
        reconcile_fake_ids(&mut settings);
        assert_eq!(settings.fake_id_counter, 1);
    }

    #[tokio::test]
    async fn round_trip_reconciles_counter() {
        let directory = test_dir("round");
        let settings = PingViewerNextV1Settings {
            fake_devices: HashMap::from([(4, DeviceSelection::Ping1D)]),
            fake_id_counter: 1,
            ..Default::default()
        };
        persist_settings_to(&directory, &settings).unwrap();

        let loaded = load_settings_from(&directory, false);
        {
            let loaded = loaded
                .lock()
                .expect("settings mutex should not be poisoned");
            assert_eq!(loaded.fake_devices.get(&4), Some(&DeviceSelection::Ping1D));
            assert_eq!(loaded.fake_id_counter, 5);
        }

        let path = directory.join("settings_v1.json");
        let deadline = tokio::time::Instant::now() + Duration::from_secs(2);
        // Writing and reading the settings file is a race, so keep checking until it updates
        let stored = loop {
            let stored: PingViewerNextV1Settings = serde_json::from_reader(BufReader::new(
                File::open(&path).expect("settings file should exist"),
            ))
            .expect("settings file should parse");
            if stored.fake_id_counter == 5 {
                break stored;
            }
            assert!(
                tokio::time::Instant::now() < deadline,
                "reconciled counter was not persisted, file still has {}",
                stored.fake_id_counter
            );
            tokio::time::sleep(Duration::from_millis(5)).await;
        };
        assert_eq!(stored.fake_id_counter, 5);
        drop(loaded);
        std::fs::remove_dir_all(&directory).unwrap();
    }

    #[tokio::test]
    async fn missing_counter_uses_saved_ids() {
        let directory = test_dir("missing-counter");
        std::fs::write(
            directory.join("settings_v1.json"),
            r#"{"fake_devices":{"2":"Ping360"}}"#,
        )
        .unwrap();

        let loaded = load_settings_from(&directory, false);
        let loaded = loaded
            .lock()
            .expect("settings mutex should not be poisoned");
        assert_eq!(loaded.fake_devices.get(&2), Some(&DeviceSelection::Ping360));
        assert_eq!(loaded.fake_id_counter, 3);
        assert!(!loaded.head_down);
        assert!(loaded.ping360.is_empty());
        assert_eq!(loaded.display, DisplayProfile::default());
        drop(loaded);
        std::fs::remove_dir_all(&directory).unwrap();
    }

    #[tokio::test]
    async fn invalid_settings_fall_back_to_defaults() {
        let directory = test_dir("invalid");
        std::fs::write(directory.join("settings_v1.json"), "{").unwrap();

        let loaded = load_settings_from(&directory, false);
        let loaded = loaded
            .lock()
            .expect("settings mutex should not be poisoned");
        assert!(loaded.fake_devices.is_empty());
        assert_eq!(loaded.fake_id_counter, 1);
        drop(loaded);

        assert!(directory.join("settings_v1.json.invalid").exists());
        assert!(!directory.join("settings_v1.json").exists());
        std::fs::remove_dir_all(&directory).unwrap();
    }

    #[tokio::test]
    async fn reset_discards_saved_settings() {
        let directory = test_dir("reset");
        let settings = PingViewerNextV1Settings {
            fake_devices: HashMap::from([(3, DeviceSelection::Ping360)]),
            fake_id_counter: 4,
            ..Default::default()
        };
        persist_settings_to(&directory, &settings).unwrap();

        let loaded = load_settings_from(&directory, true);
        let loaded = loaded
            .lock()
            .expect("settings mutex should not be poisoned");
        assert!(loaded.fake_devices.is_empty());
        assert_eq!(loaded.fake_id_counter, 1);
        drop(loaded);

        assert!(!directory.join("settings_v1.json").exists());
        std::fs::remove_dir_all(&directory).unwrap();
    }

    #[tokio::test]
    async fn partial_viewer_update_leaves_the_other_field() {
        let mut settings = PingViewerNextV1Settings::default();
        settings.display.color_palette = "Ocean".to_string();
        apply_viewer_update(
            &mut settings,
            &ViewerSettingsUpdate {
                head_down: Some(true),
                ..Default::default()
            },
        );
        assert!(settings.head_down);
        assert_eq!(settings.display.color_palette, "Ocean");
        assert_eq!(settings.revision, 1);
    }

    #[tokio::test]
    async fn stale_expected_revision_is_rejected() {
        let directory = test_dir("stale-revision");
        let settings = load_settings_from(&directory, false);

        let rejected = settings
            .update_viewer(ViewerSettingsUpdate {
                expected_revision: Some(1),
                head_down: Some(true),
                ..Default::default()
            })
            .unwrap_err();

        assert_eq!(rejected.revision, 0);
        assert!(!rejected.head_down);
        assert_eq!(settings.lock().unwrap().revision, 0);
        std::fs::remove_dir_all(&directory).unwrap();
    }

    #[tokio::test]
    async fn ping360_display_and_head_down_round_trip() {
        let directory = test_dir("viewer");
        let config = Ping360Config {
            mode: 1,
            gain_setting: 2,
            transmit_duration: 32,
            sample_period: 80,
            transmit_frequency: 740,
            number_of_samples: 1200,
            start_angle: 10,
            stop_angle: 200,
            num_steps: 1,
            delay: 10,
            speed_of_sound: 1482,
        };
        let settings = PingViewerNextV1Settings {
            head_down: true,
            display: DisplayProfile {
                color_palette: "Ocean".to_string(),
                debug_mode: true,
                ..DisplayProfile::default()
            },
            ping360: vec![StoredPing360 {
                connection: DeviceConnection::Ethernet {
                    mac: "aa:bb:cc:dd:ee:ff".to_string(),
                },
                config,
            }],
            ..Default::default()
        };
        persist_settings_to(&directory, &settings).unwrap();

        let loaded = load_settings_from(&directory, false);
        let loaded = loaded
            .lock()
            .expect("settings mutex should not be poisoned");
        assert!(loaded.head_down);
        assert_eq!(loaded.display.color_palette, "Ocean");
        assert!(loaded.display.debug_mode);
        assert_eq!(loaded.display.a_scan, true);
        assert_eq!(loaded.ping360.len(), 1);
        assert_eq!(
            loaded.ping360[0].connection,
            DeviceConnection::Ethernet {
                mac: "aa:bb:cc:dd:ee:ff".to_string(),
            }
        );
        assert_eq!(loaded.ping360[0].config, config);
        drop(loaded);
        std::fs::remove_dir_all(&directory).unwrap();
    }
}
