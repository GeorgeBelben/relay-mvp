use std::path::{Path, PathBuf};

use walkdir::WalkDir;

#[derive(Debug)]
pub struct ScannedRom {
    pub system_id: &'static str,
    pub path: PathBuf,
}

#[derive(Debug)]
pub struct ScanReport {
    pub roms: Vec<ScannedRom>,
}

pub fn scan_library(library_root: &Path) -> std::io::Result<ScanReport> {
    let mut roms = Vec::new();

    for system in crate::systems::ALL {
        let system_dir = library_root.join("roms").join(system.id);

        for entry in WalkDir::new(&system_dir) {
            let entry = entry?;
            if !entry.file_type().is_file() {
                continue;
            }

            let matches_system = entry
                .path()
                .extension()
                .and_then(|ext| ext.to_str())
                .is_some_and(|ext| {
                    system
                        .extensions
                        .iter()
                        .any(|known| known.eq_ignore_ascii_case(ext))
                });

            if matches_system {
                roms.push(ScannedRom {
                    system_id: system.id,
                    path: entry.path().to_path_buf(),
                });
            }
        }
    }

    Ok(ScanReport { roms })
}
