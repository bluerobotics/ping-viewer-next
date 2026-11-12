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

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, Apiv2Schema)]
#[serde(rename_all = "lowercase")]
pub enum UnitSystem {
    #[default]
    Metric,
    Imperial,
}

/// Plot appearance shared by Ping1D and Ping360. The theme stays in the browser.
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

/// Field-wise overlay for `DisplayProfile`. Omitted fields keep their stored
/// value, so a partial body cannot reset the rest of the profile to defaults.
#[derive(Debug, Clone, Default, Deserialize, Apiv2Schema)]
pub struct DisplayProfileUpdate {
    #[serde(default)]
    pub a_scan: Option<bool>,
    #[serde(default)]
    pub color_palette: Option<String>,
    #[serde(default)]
    pub debug_mode: Option<bool>,
    #[serde(default)]
    pub background_mode: Option<String>,
    #[serde(default)]
    pub background_color: Option<String>,
}

impl DisplayProfile {
    fn patched(&self, update: &DisplayProfileUpdate) -> Self {
        let mut next = self.clone();
        if let Some(a_scan) = update.a_scan {
            next.a_scan = a_scan;
        }
        if let Some(color_palette) = &update.color_palette {
            next.color_palette = color_palette.clone();
        }
        if let Some(debug_mode) = update.debug_mode {
            next.debug_mode = debug_mode;
        }
        if let Some(background_mode) = &update.background_mode {
            next.background_mode = background_mode.clone();
        }
        if let Some(background_color) = &update.background_color {
            next.background_color = background_color.clone();
        }
        next
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Apiv2Schema)]
pub struct ViewerSettings {
    /// Starts at 0 and increases with every update
    pub revision: u64,
    pub head_down: bool,
    pub unit_system: UnitSystem,
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
    pub unit_system: Option<UnitSystem>,
    #[serde(default)]
    pub display: Option<DisplayProfileUpdate>,
}

/// Acoustic settings for one Ping360, keyed by `DeviceConnection` and slot.
///
/// Startup loads these slots into the registry and leaves them vacant until
/// that device connects. Serial and MAC-less UDP devices share one connection
/// key, so only one saved slot can be restored for each of those.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct StoredPing360 {
    pub(crate) connection: DeviceConnection,
    /// Settings files written before profiles included a slot load as slot 0.
    #[serde(default)]
    pub(crate) slot: u8,
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
    #[serde(default)]
    pub(crate) unit_system: UnitSystem,
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
            unit_system: UnitSystem::default(),
            display: DisplayProfile::default(),
        }
    }
}

pub(crate) fn stored_ping360_config(
    connection: &DeviceConnection,
    slot: u8,
) -> Option<Ping360Config> {
    let settings = SETTINGS.lock().expect("not poisoned");
    settings
        .ping360
        .iter()
        .find(|entry| &entry.connection == connection && entry.slot == slot)
        .map(|entry| entry.config)
}

pub(crate) fn remember_ping360_config(
    connection: DeviceConnection,
    slot: u8,
    config: Ping360Config,
) {
    let mut settings = SETTINGS.lock().expect("not poisoned");
    if let Some(entry) = settings
        .ping360
        .iter_mut()
        .find(|entry| entry.connection == connection && entry.slot == slot)
    {
        entry.config = config;
    } else {
        settings.ping360.push(StoredPing360 {
            connection,
            slot,
            config,
        });
    }
}

fn viewer_snapshot(settings: &PingViewerNextV1Settings) -> ViewerSettings {
    ViewerSettings {
        revision: settings.revision,
        head_down: settings.head_down,
        unit_system: settings.unit_system,
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
    let mut changed = false;
    if let Some(head_down) = update.head_down {
        if head_down != settings.head_down {
            settings.head_down = head_down;
            changed = true;
        }
    }
    if let Some(unit_system) = update.unit_system {
        if unit_system != settings.unit_system {
            settings.unit_system = unit_system;
            changed = true;
        }
    }
    if let Some(display) = &update.display {
        let next = settings.display.patched(display);
        if next != settings.display {
            settings.display = next;
            changed = true;
        }
    }
    if changed {
        settings.revision += 1;
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

/// Drops the saved entries of a fake device. The slot is matched as well
/// because a retired id could otherwise delete a profile recorded under a slot
/// the device no longer occupies.
pub(crate) fn forget_fake_device(settings: &mut PingViewerNextV1Settings, fake_id: u16, slot: u8) {
    settings.fake_devices.remove(&fake_id);
    let connection = DeviceConnection::Fake { id: fake_id };
    settings
        .ping360
        .retain(|entry| entry.connection != connection || entry.slot != slot);
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
            mount_offset: 90,
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
                slot: 2,
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
        assert!(loaded.display.a_scan);
        assert_eq!(loaded.ping360.len(), 1);
        assert_eq!(
            loaded.ping360[0].connection,
            DeviceConnection::Ethernet {
                mac: "aa:bb:cc:dd:ee:ff".to_string(),
            }
        );
        assert_eq!(loaded.ping360[0].slot, 2);
        assert_eq!(loaded.ping360[0].config, config);
        drop(loaded);
        std::fs::remove_dir_all(&directory).unwrap();
    }

    fn ping360_config() -> Ping360Config {
        Ping360Config {
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
            mount_offset: 180,
        }
    }

    #[test]
    fn partial_display_update_keeps_untouched_fields() {
        let mut settings = PingViewerNextV1Settings::default();
        settings.display.color_palette = "Ocean".to_string();
        settings.display.background_color = "#123456".to_string();

        apply_viewer_update(
            &mut settings,
            &ViewerSettingsUpdate {
                display: Some(DisplayProfileUpdate {
                    debug_mode: Some(true),
                    ..Default::default()
                }),
                ..Default::default()
            },
        );

        assert!(settings.display.debug_mode);
        assert_eq!(settings.display.color_palette, "Ocean");
        assert_eq!(settings.display.background_color, "#123456");
        assert_eq!(settings.revision, 1);
    }

    #[test]
    fn display_update_deserializes_without_defaulting_the_rest() {
        let update: ViewerSettingsUpdate =
            serde_json::from_str(r#"{"display":{"a_scan":false}}"#).unwrap();
        let display = update.display.expect("display should be present");

        assert_eq!(display.a_scan, Some(false));
        assert_eq!(display.color_palette, None);
        assert_eq!(display.background_mode, None);
    }

    #[test]
    fn full_display_body_still_replaces_every_field() {
        let mut settings = PingViewerNextV1Settings::default();
        let update: ViewerSettingsUpdate = serde_json::from_str(
            r##"{"display":{"a_scan":false,"color_palette":"Ocean","debug_mode":true,"background_mode":"custom","background_color":"#010203"}}"##,
        )
        .unwrap();

        apply_viewer_update(&mut settings, &update);

        assert!(!settings.display.a_scan);
        assert_eq!(settings.display.color_palette, "Ocean");
        assert!(settings.display.debug_mode);
        assert_eq!(settings.display.background_mode, "custom");
        assert_eq!(settings.display.background_color, "#010203");
    }

    #[test]
    fn empty_update_does_not_consume_a_revision() {
        let mut settings = PingViewerNextV1Settings::default();

        apply_viewer_update(&mut settings, &ViewerSettingsUpdate::default());

        assert_eq!(settings.revision, 0);
    }

    #[test]
    fn redundant_update_does_not_consume_a_revision() {
        let mut settings = PingViewerNextV1Settings {
            revision: 7,
            head_down: true,
            unit_system: UnitSystem::Imperial,
            ..Default::default()
        };

        apply_viewer_update(
            &mut settings,
            &ViewerSettingsUpdate {
                head_down: Some(true),
                unit_system: Some(UnitSystem::Imperial),
                display: Some(DisplayProfileUpdate {
                    color_palette: Some(default_color_palette()),
                    ..Default::default()
                }),
                ..Default::default()
            },
        );

        assert_eq!(settings.revision, 7);
    }

    #[test]
    fn forget_fake_device_drops_only_that_device() {
        let mut settings = PingViewerNextV1Settings {
            fake_devices: HashMap::from([
                (1, DeviceSelection::Ping360),
                (2, DeviceSelection::Ping1D),
            ]),
            ping360: vec![
                StoredPing360 {
                    connection: DeviceConnection::Fake { id: 1 },
                    slot: 0,
                    config: ping360_config(),
                },
                StoredPing360 {
                    connection: DeviceConnection::Fake { id: 2 },
                    slot: 1,
                    config: ping360_config(),
                },
            ],
            ..Default::default()
        };

        forget_fake_device(&mut settings, 1, 0);

        assert!(!settings.fake_devices.contains_key(&1));
        assert_eq!(
            settings.fake_devices.get(&2),
            Some(&DeviceSelection::Ping1D)
        );
        assert_eq!(settings.ping360.len(), 1);
        assert_eq!(
            settings.ping360[0].connection,
            DeviceConnection::Fake { id: 2 }
        );
    }

    #[test]
    fn forget_fake_device_keeps_profiles_of_other_slots() {
        let mut settings = PingViewerNextV1Settings {
            fake_devices: HashMap::from([(1, DeviceSelection::Ping360)]),
            ping360: vec![
                StoredPing360 {
                    connection: DeviceConnection::Fake { id: 1 },
                    slot: 0,
                    config: ping360_config(),
                },
                StoredPing360 {
                    connection: DeviceConnection::Fake { id: 1 },
                    slot: 3,
                    config: ping360_config(),
                },
            ],
            ..Default::default()
        };

        forget_fake_device(&mut settings, 1, 0);

        assert_eq!(settings.ping360.len(), 1);
        assert_eq!(settings.ping360[0].slot, 3);
    }
}
