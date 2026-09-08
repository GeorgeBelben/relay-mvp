use std::path::Path;

use walkdir::WalkDir;

#[derive(Debug)]
pub struct ScanReport {
    pub roms_found: u32,
}

pub fn scan_library(library_root: &Path) -> std::io::Result<ScanReport> {
    let mut roms_found = 0;

    for system in crate::systems::ALL {
        let system_dir = library_root.join("roms").join(system.id);

        for entry in WalkDir::new(&system_dir) {
            let entry = entry?;
            if entry.file_type().is_file() {
                roms_found += 1;
            }
        }
    }

    Ok(ScanReport { roms_found })
}
