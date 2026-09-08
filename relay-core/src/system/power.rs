// Device power actions (Power Menu's Shut Down/Reboot), via systemctl. Same shape as datetime.rs's
// timedatectl wrapper: the binary name is threaded through as a parameter so tests can point it at
// `sh` or a nonexistent binary to fabricate scenarios -- calling the real thing with the literal
// "systemctl" name would actually power off or reboot whatever machine runs the test, so unlike
// network.rs/bluetooth.rs's own command wrappers, shutdown()/reboot() below are deliberately never
// exercised end-to-end against a real `systemctl` from this crate's test suite, only via
// run_systemctl/require_success directly (see the tests module) and a missing-binary case (safe --
// nothing runs).
//
// Needs a polkit rule granting the kiosk service account passwordless
// `org.freedesktop.login1.power-off`/`...reboot` (or an equivalent sudoers entry), provisioned on
// the device itself during setup -- this module doesn't set that up, same prerequisite shape as
// bluetooth.rs's bt-agent systemd service.

use std::process::Stdio;
use std::time::Duration;

use tokio::process::Command;

const COMMAND_TIMEOUT: Duration = Duration::from_secs(10);

async fn run_systemctl(systemctl_bin: &str, args: &[&str], timeout: Duration) -> Result<std::process::Output, String> {
    let output_fut = Command::new(systemctl_bin).args(args).stdin(Stdio::null()).output();
    match tokio::time::timeout(timeout, output_fut).await {
        Ok(Ok(output)) => Ok(output),
        // A missing binary means systemd itself isn't set up right -- an environment problem, not
        // a "try again" outcome.
        Ok(Err(e)) if e.kind() == std::io::ErrorKind::NotFound => Err("systemctl not found -- is systemd installed?".to_string()),
        Ok(Err(e)) => Err(e.to_string()),
        Err(_elapsed) => Err("systemctl timed out".to_string()),
    }
}

fn require_success(output: &std::process::Output) -> Result<(), String> {
    if output.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
    }
}

pub async fn shutdown(systemctl_bin: &str) -> Result<(), String> {
    let output = run_systemctl(systemctl_bin, &["poweroff"], COMMAND_TIMEOUT).await?;
    require_success(&output)
}

pub async fn reboot(systemctl_bin: &str) -> Result<(), String> {
    let output = run_systemctl(systemctl_bin, &["reboot"], COMMAND_TIMEOUT).await?;
    require_success(&output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn run_systemctl_captures_stdout_on_success() {
        let output = run_systemctl("sh", &["-c", "echo hello"], COMMAND_TIMEOUT).await.unwrap();
        assert!(output.status.success());
        assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "hello");
    }

    #[tokio::test]
    async fn run_systemctl_captures_stderr_on_a_nonzero_exit() {
        let output = run_systemctl("sh", &["-c", "echo boom >&2; exit 1"], COMMAND_TIMEOUT).await.unwrap();
        assert!(!output.status.success());
        assert_eq!(String::from_utf8_lossy(&output.stderr).trim(), "boom");
    }

    #[tokio::test]
    async fn run_systemctl_reports_a_clear_error_for_a_missing_binary() {
        let err = run_systemctl("definitely-not-a-real-binary-xyz", &[], COMMAND_TIMEOUT).await.unwrap_err();
        assert!(err.contains("not found"));
    }

    #[tokio::test]
    async fn run_systemctl_times_out_rather_than_hanging_forever() {
        let err = run_systemctl("sh", &["-c", "sleep 5"], Duration::from_millis(50)).await.unwrap_err();
        assert!(err.contains("timed out"));
    }

    #[test]
    fn require_success_passes_through_ok_and_captures_stderr_on_failure() {
        use std::os::unix::process::ExitStatusExt;
        let ok = std::process::Output { status: std::process::ExitStatus::from_raw(0), stdout: vec![], stderr: vec![] };
        assert!(require_success(&ok).is_ok());

        let failed =
            std::process::Output { status: std::process::ExitStatus::from_raw(1 << 8), stdout: vec![], stderr: b"boom".to_vec() };
        assert_eq!(require_success(&failed), Err("boom".to_string()));
    }

    // shutdown()/reboot() are one-line compositions of run_systemctl + require_success (both
    // already covered above) plus a fixed argument -- exercised here only via a missing binary,
    // which is safe (nothing runs). Never call these with a real "systemctl" from a test: unlike
    // nmcli/bluetoothctl, a real poweroff/reboot actually does what it says.
    #[tokio::test]
    async fn shutdown_reports_a_clear_error_for_a_missing_binary() {
        let err = shutdown("definitely-not-a-real-binary-xyz").await.unwrap_err();
        assert!(err.contains("not found"));
    }

    #[tokio::test]
    async fn reboot_reports_a_clear_error_for_a_missing_binary() {
        let err = reboot("definitely-not-a-real-binary-xyz").await.unwrap_err();
        assert!(err.contains("not found"));
    }
}
