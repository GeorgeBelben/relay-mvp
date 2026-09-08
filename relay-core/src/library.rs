use std::path::{Path, PathBuf};

pub fn library_root() -> PathBuf {
    dirs::home_dir()
        .expect("could not determine home directory")
        .join("Relay")
}

pub async fn ensure_library_dirs(root: &Path) -> std::io::Result<()> {
    for subdir in [
        "roms",
        "bios",
        "media",
        "saves",
        "savestates",
        "screenshots",
    ] {
        tokio::fs::create_dir_all(root.join(subdir)).await?;
    }

    let roms_dir = root.join("roms");
    for system in crate::systems::ALL {
        tokio::fs::create_dir_all(roms_dir.join(system.id)).await?;
    }

    Ok(())
}
