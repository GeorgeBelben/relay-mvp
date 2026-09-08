use relay_core::library;
use relay_core::storage::{self, StorageUsage};

#[tauri::command]
pub async fn get_storage_usage() -> Result<StorageUsage, String> {
    storage::get_storage_usage(&library::library_root()).await.map_err(crate::logging::err_to_string)
}
