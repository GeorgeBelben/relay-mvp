use crate::system::bluetooth::{BluetoothDevice, BluetoothPairError};
use crate::system::provider::{ActiveSystemProvider, SystemProvider};

#[tauri::command]
pub async fn scan_for_bluetooth_devices() -> Result<Vec<BluetoothDevice>, String> {
    ActiveSystemProvider::default().scan_for_bluetooth_devices().await
}

#[tauri::command]
pub async fn list_paired_bluetooth_devices() -> Result<Vec<BluetoothDevice>, String> {
    ActiveSystemProvider::default().list_paired_bluetooth_devices().await
}

#[tauri::command]
pub async fn pair_bluetooth_device(address: String) -> Result<(), BluetoothPairError> {
    ActiveSystemProvider::default().pair_bluetooth_device(&address).await
}

#[tauri::command]
pub async fn remove_bluetooth_device(address: String) -> Result<(), String> {
    ActiveSystemProvider::default().remove_bluetooth_device(&address).await
}
