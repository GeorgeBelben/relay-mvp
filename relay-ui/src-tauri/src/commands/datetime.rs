use relay_core::system::datetime::DateTimeStatus;
use relay_core::system::provider::{ActiveSystemProvider, SystemProvider};

#[tauri::command]
pub async fn get_datetime_status() -> Result<DateTimeStatus, String> {
    ActiveSystemProvider::default().get_datetime_status().await
}

#[tauri::command]
pub async fn list_timezones() -> Result<Vec<String>, String> {
    ActiveSystemProvider::default().list_timezones().await
}

#[tauri::command]
pub async fn set_timezone(timezone: String) -> Result<(), String> {
    ActiveSystemProvider::default().set_timezone(&timezone).await
}

#[tauri::command]
pub async fn set_ntp_enabled(enabled: bool) -> Result<(), String> {
    ActiveSystemProvider::default().set_ntp_enabled(enabled).await
}

#[tauri::command]
pub async fn set_time(date_time: String) -> Result<(), String> {
    ActiveSystemProvider::default().set_time(&date_time).await
}
