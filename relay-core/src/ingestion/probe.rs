use std::path::Path;

use md5::{Digest, Md5};
use tokio::fs::File;
use tokio::io::{AsyncReadExt, BufReader};

/// Result of probing a single file: a CRC32 checksum (lowercase, zero-padded to 8 hex digits,
/// matching the Electron MVP's `.toString(16).padStart(8, "0")`), an MD5 checksum (uppercase hex,
/// matching RetroAchievements' own hash format -- RA identification matches on MD5, not CRC32),
/// plus its size in bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProbeResult {
    pub crc32: String,
    pub md5: String,
    pub size_bytes: i64,
}

/// Streams `path` to compute its CRC32 + MD5 + size without loading it into memory whole -- ROM/
/// disc images can be multi-gigabyte. CRC32 (ported from the Electron MVP's
/// `scanner/hash.ts#crc32File`) feeds No-Intro title correction; MD5 feeds RetroAchievements hash
/// matching -- computed together in the same pass since both are cheap relative to the I/O.
pub async fn probe_file(path: &Path) -> std::io::Result<ProbeResult> {
    let file = File::open(path).await?;
    let mut reader = BufReader::new(file);
    let mut buf = [0u8; 64 * 1024];
    let mut crc32_hasher = crc32fast::Hasher::new();
    let mut md5_hasher = Md5::new();
    let mut size_bytes: i64 = 0;

    loop {
        let n = reader.read(&mut buf).await?;
        if n == 0 {
            break;
        }
        crc32_hasher.update(&buf[..n]);
        md5_hasher.update(&buf[..n]);
        size_bytes += n as i64;
    }

    Ok(ProbeResult {
        crc32: format!("{:08x}", crc32_hasher.finalize()),
        md5: to_hex_upper(&md5_hasher.finalize()),
        size_bytes,
    })
}

// md-5's Digest::finalize() returns a fixed-size byte array with no UpperHex impl of its own.
fn to_hex_upper(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02X}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[tokio::test]
    async fn matches_known_crc32_test_vector() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("vector.bin");
        fs::write(&file_path, b"123456789").unwrap();

        let result = probe_file(&file_path).await.unwrap();

        // Standard CRC-32 (IEEE 802.3) check value for the ASCII string "123456789".
        assert_eq!(result.crc32, "cbf43926");
        // RFC 1321's own MD5 test suite includes this exact string -> hash pair.
        assert_eq!(result.md5, "25F9E794323B453885F5181F1B624D0B");
        assert_eq!(result.size_bytes, 9);
    }

    #[tokio::test]
    async fn empty_file_hashes_to_zero() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("empty.bin");
        fs::write(&file_path, b"").unwrap();

        let result = probe_file(&file_path).await.unwrap();

        assert_eq!(result.crc32, "00000000");
        // The universally-cited MD5-of-empty-string constant (RFC 1321).
        assert_eq!(result.md5, "D41D8CD98F00B204E9800998ECF8427E");
        assert_eq!(result.size_bytes, 0);
    }

    #[tokio::test]
    async fn large_file_hashes_correctly_across_multiple_chunks() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("large.bin");
        // Bigger than the 64KB read buffer, to exercise the incremental-hashing path.
        let content = vec![0xABu8; 200_000];
        fs::write(&file_path, &content).unwrap();

        let result = probe_file(&file_path).await.unwrap();

        let mut crc32_hasher = crc32fast::Hasher::new();
        crc32_hasher.update(&content);
        assert_eq!(result.crc32, format!("{:08x}", crc32_hasher.finalize()));

        let mut md5_hasher = Md5::new();
        md5_hasher.update(&content);
        assert_eq!(result.md5, to_hex_upper(&md5_hasher.finalize()));

        assert_eq!(result.size_bytes, 200_000);
    }

    #[tokio::test]
    async fn missing_file_returns_error() {
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("does-not-exist.bin");

        assert!(probe_file(&missing).await.is_err());
    }
}
