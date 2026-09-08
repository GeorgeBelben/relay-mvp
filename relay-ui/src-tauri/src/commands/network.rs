use crate::system::network::{WifiConnectError, WifiNetwork};
use crate::system::provider::{ActiveSystemProvider, SystemProvider};

#[tauri::command]
pub async fn list_wifi_networks() -> Result<Vec<WifiNetwork>, String> {
    ActiveSystemProvider::default().list_wifi_networks().await
}

#[tauri::command]
pub async fn connect_to_wifi_network(ssid: String, password: Option<String>) -> Result<(), WifiConnectError> {
    ActiveSystemProvider::default().connect_to_wifi_network(&ssid, password.as_deref()).await
}
