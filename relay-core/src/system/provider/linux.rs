// The real implementation -- a straight extraction of the pre-existing nmcli/bluetoothctl/
// timedatectl call sites. No logic changed, only moved: the parsing/classification code these
// wrap stays in system::{network,bluetooth,datetime} untouched.

use super::SystemProvider;
use crate::system::bluetooth::{self, BluetoothDevice, BluetoothPairError};
use crate::system::datetime::{self, DateTimeStatus};
use crate::system::network::{self, WifiConnectError, WifiNetwork};
use crate::system::power;

const NMCLI_BIN: &str = "nmcli";
const BLUETOOTHCTL_BIN: &str = "bluetoothctl";
const TIMEDATECTL_BIN: &str = "timedatectl";
const SYSTEMCTL_BIN: &str = "systemctl";

#[derive(Default)]
pub struct LinuxSystemProvider;

impl SystemProvider for LinuxSystemProvider {
    async fn list_wifi_networks(&self) -> Result<Vec<WifiNetwork>, String> {
        network::list_wifi_networks(NMCLI_BIN).await
    }

    async fn connect_to_wifi_network(&self, ssid: &str, password: Option<&str>) -> Result<(), WifiConnectError> {
        network::connect_to_wifi_network(NMCLI_BIN, ssid, password).await
    }

    async fn scan_for_bluetooth_devices(&self) -> Result<Vec<BluetoothDevice>, String> {
        bluetooth::scan_for_devices(BLUETOOTHCTL_BIN).await
    }

    async fn list_paired_bluetooth_devices(&self) -> Result<Vec<BluetoothDevice>, String> {
        bluetooth::list_paired_devices(BLUETOOTHCTL_BIN).await
    }

    async fn pair_bluetooth_device(&self, address: &str) -> Result<(), BluetoothPairError> {
        bluetooth::pair_device(BLUETOOTHCTL_BIN, address).await
    }

    async fn remove_bluetooth_device(&self, address: &str) -> Result<(), String> {
        bluetooth::remove_device(BLUETOOTHCTL_BIN, address).await
    }

    async fn get_datetime_status(&self) -> Result<DateTimeStatus, String> {
        datetime::get_status(TIMEDATECTL_BIN).await
    }

    async fn list_timezones(&self) -> Result<Vec<String>, String> {
        datetime::list_timezones(TIMEDATECTL_BIN).await
    }

    async fn set_timezone(&self, timezone: &str) -> Result<(), String> {
        datetime::set_timezone(TIMEDATECTL_BIN, timezone).await
    }

    async fn set_ntp_enabled(&self, enabled: bool) -> Result<(), String> {
        datetime::set_ntp_enabled(TIMEDATECTL_BIN, enabled).await
    }

    async fn set_time(&self, date_time: &str) -> Result<(), String> {
        datetime::set_time(TIMEDATECTL_BIN, date_time).await
    }

    async fn shutdown_device(&self) -> Result<(), String> {
        power::shutdown(SYSTEMCTL_BIN).await
    }

    async fn reboot_device(&self) -> Result<(), String> {
        power::reboot(SYSTEMCTL_BIN).await
    }
}
