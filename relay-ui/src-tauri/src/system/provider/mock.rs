// Mock implementation used on every non-Linux target so `cargo tauri dev` runs standalone on
// macOS. Reads fixture data from a TOML file (RELAY_MOCK_FIXTURE env var, or dev-fixtures/
// default.toml alongside this crate if unset) instead of hardcoding it, so switching between UI
// states (empty/degraded/happy-path) doesn't need a recompile -- see dev-fixtures/ in the repo
// root of this crate.
//
// Mutating calls (connect/pair/remove/set-timezone/etc.) just succeed -- there's no real
// NetworkManager/BlueZ/systemd underneath to mutate, and the fixtures are about rendering
// realistic *read* states, not simulating write failures.

use std::path::PathBuf;
use std::sync::OnceLock;

use serde::Deserialize;

use super::SystemProvider;
use crate::system::bluetooth::{BluetoothDevice, BluetoothPairError};
use crate::system::datetime::DateTimeStatus;
use crate::system::network::{WifiConnectError, WifiNetwork};

const DEFAULT_FIXTURE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/dev-fixtures/default.toml");

#[derive(Debug, Deserialize)]
struct Fixture {
    #[serde(default)]
    wifi_networks: Vec<WifiNetwork>,
    #[serde(default)]
    bluetooth_paired: Vec<BluetoothDevice>,
    #[serde(default)]
    bluetooth_scan: Vec<BluetoothDevice>,
    datetime: DateTimeStatus,
}

fn fixture_path() -> PathBuf {
    std::env::var_os("RELAY_MOCK_FIXTURE").map(PathBuf::from).unwrap_or_else(|| PathBuf::from(DEFAULT_FIXTURE))
}

// Panics on a missing/malformed fixture rather than falling back silently -- this only ever runs
// in a local dev build, so a loud failure at first use beats quietly rendering the wrong UI state.
fn fixture() -> &'static Fixture {
    static FIXTURE: OnceLock<Fixture> = OnceLock::new();
    FIXTURE.get_or_init(|| {
        let path = fixture_path();
        let contents = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("RELAY_MOCK_FIXTURE: failed to read {}: {e}", path.display()));
        toml::from_str(&contents).unwrap_or_else(|e| panic!("RELAY_MOCK_FIXTURE: failed to parse {}: {e}", path.display()))
    })
}

// A representative slice of IANA zones, enough to exercise the timezone picker's search/filter UI
// without embedding the OS's full tzdata list -- the real list_timezones pulls that from
// timedatectl, a Linux-only source in the first place.
const MOCK_TIMEZONES: &[&str] = &[
    "UTC",
    "Europe/London",
    "Europe/Paris",
    "Europe/Berlin",
    "America/New_York",
    "America/Los_Angeles",
    "America/Chicago",
    "Asia/Tokyo",
    "Asia/Shanghai",
    "Australia/Sydney",
];

#[derive(Default)]
pub struct MockSystemProvider;

impl SystemProvider for MockSystemProvider {
    async fn list_wifi_networks(&self) -> Result<Vec<WifiNetwork>, String> {
        Ok(fixture().wifi_networks.clone())
    }

    async fn connect_to_wifi_network(&self, _ssid: &str, _password: Option<&str>) -> Result<(), WifiConnectError> {
        Ok(())
    }

    async fn scan_for_bluetooth_devices(&self) -> Result<Vec<BluetoothDevice>, String> {
        Ok(fixture().bluetooth_scan.clone())
    }

    async fn list_paired_bluetooth_devices(&self) -> Result<Vec<BluetoothDevice>, String> {
        Ok(fixture().bluetooth_paired.clone())
    }

    async fn pair_bluetooth_device(&self, _address: &str) -> Result<(), BluetoothPairError> {
        Ok(())
    }

    async fn remove_bluetooth_device(&self, _address: &str) -> Result<(), String> {
        Ok(())
    }

    async fn get_datetime_status(&self) -> Result<DateTimeStatus, String> {
        Ok(fixture().datetime.clone())
    }

    async fn list_timezones(&self) -> Result<Vec<String>, String> {
        Ok(MOCK_TIMEZONES.iter().map(|s| s.to_string()).collect())
    }

    async fn set_timezone(&self, _timezone: &str) -> Result<(), String> {
        Ok(())
    }

    async fn set_ntp_enabled(&self, _enabled: bool) -> Result<(), String> {
        Ok(())
    }

    async fn set_time(&self, _date_time: &str) -> Result<(), String> {
        Ok(())
    }

    // Never actually powers off/reboots the dev machine -- see the trait doc comment.
    async fn shutdown_device(&self) -> Result<(), String> {
        Ok(())
    }

    async fn reboot_device(&self) -> Result<(), String> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Every fixture shipped in dev-fixtures/ must actually deserialize -- catches a typo'd/missing
    // field before it turns into a first-use panic during `cargo tauri dev`.
    #[test]
    fn every_shipped_fixture_parses() {
        let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/dev-fixtures");
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.extension().and_then(|e| e.to_str()) != Some("toml") {
                continue;
            }
            let contents = std::fs::read_to_string(&path).unwrap();
            toml::from_str::<Fixture>(&contents).unwrap_or_else(|e| panic!("{} failed to parse: {e}", path.display()));
        }
    }

    #[tokio::test]
    async fn list_wifi_networks_reads_the_default_fixture() {
        let networks = MockSystemProvider.list_wifi_networks().await.unwrap();
        assert!(!networks.is_empty());
    }

    #[tokio::test]
    async fn list_paired_bluetooth_devices_reads_the_default_fixture() {
        let devices = MockSystemProvider.list_paired_bluetooth_devices().await.unwrap();
        assert!(!devices.is_empty());
    }

    #[tokio::test]
    async fn get_datetime_status_reads_the_default_fixture() {
        let status = MockSystemProvider.get_datetime_status().await.unwrap();
        assert!(!status.timezone.is_empty());
    }

    #[tokio::test]
    async fn shutdown_and_reboot_are_no_ops() {
        assert!(MockSystemProvider.shutdown_device().await.is_ok());
        assert!(MockSystemProvider.reboot_device().await.is_ok());
    }
}
