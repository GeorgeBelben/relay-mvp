// A seam over the handful of system calls that only exist on the real target (an HP EliteDesk
// running Ubuntu Server + cage, with NetworkManager/BlueZ/systemd installed) so this crate (and
// anything built on it, e.g. relay-cli/relay-ui) can also run directly on macOS for iteration --
// see dev-fixtures/ and the RELAY_MOCK_FIXTURE workflow. Selection happens once at compile time
// via #[cfg(target_os)], not as a runtime branch: every call site just goes through
// `ActiveSystemProvider`, which is a type alias picking `linux::LinuxSystemProvider` or
// `mock::MockSystemProvider`.
//
// Disk usage (crate::storage, via nix::statvfs) isn't behind this trait -- it already works
// unmodified on macOS, so mocking it would be pure overhead.

#[cfg(target_os = "linux")]
mod linux;
#[cfg(not(target_os = "linux"))]
mod mock;

#[cfg(target_os = "linux")]
pub use linux::LinuxSystemProvider as ActiveSystemProvider;
#[cfg(not(target_os = "linux"))]
pub use mock::MockSystemProvider as ActiveSystemProvider;

use std::future::Future;

use crate::system::bluetooth::{BluetoothDevice, BluetoothPairError};
use crate::system::datetime::DateTimeStatus;
use crate::system::network::{WifiConnectError, WifiNetwork};

// Desugared (`-> impl Future<..> + Send` rather than plain `async fn`) so every implementation's
// future is guaranteed Send -- required for callers awaiting these from a multi-threaded runtime,
// and plain `async fn` in a trait can't express that bound.
pub trait SystemProvider: Default {
    fn list_wifi_networks(&self) -> impl Future<Output = Result<Vec<WifiNetwork>, String>> + Send;
    fn connect_to_wifi_network(&self, ssid: &str, password: Option<&str>) -> impl Future<Output = Result<(), WifiConnectError>> + Send;

    fn scan_for_bluetooth_devices(&self) -> impl Future<Output = Result<Vec<BluetoothDevice>, String>> + Send;
    fn list_paired_bluetooth_devices(&self) -> impl Future<Output = Result<Vec<BluetoothDevice>, String>> + Send;
    fn pair_bluetooth_device(&self, address: &str) -> impl Future<Output = Result<(), BluetoothPairError>> + Send;
    fn remove_bluetooth_device(&self, address: &str) -> impl Future<Output = Result<(), String>> + Send;

    fn get_datetime_status(&self) -> impl Future<Output = Result<DateTimeStatus, String>> + Send;
    fn list_timezones(&self) -> impl Future<Output = Result<Vec<String>, String>> + Send;
    fn set_timezone(&self, timezone: &str) -> impl Future<Output = Result<(), String>> + Send;
    fn set_ntp_enabled(&self, enabled: bool) -> impl Future<Output = Result<(), String>> + Send;
    fn set_time(&self, date_time: &str) -> impl Future<Output = Result<(), String>> + Send;

    // Power Menu's Shut Down/Reboot. The mock never actually touches the dev machine's power
    // state -- see mock.rs.
    fn shutdown_device(&self) -> impl Future<Output = Result<(), String>> + Send;
    fn reboot_device(&self) -> impl Future<Output = Result<(), String>> + Send;
}
