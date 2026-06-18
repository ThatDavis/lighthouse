use anyhow::{Context, Result};
use std::process::Command;

pub fn status() -> Result<bool> {
    let output = Command::new("systemctl")
        .args(["is-active", "--quiet", "lighthouse"])
        .output()
        .context("failed to run systemctl is-active")?;
    Ok(output.status.success())
}

pub fn start() -> Result<String> {
    run_systemctl(["start", "lighthouse.service"])
}

pub fn stop() -> Result<String> {
    run_systemctl(["stop", "lighthouse.service"])
}

pub fn restart() -> Result<String> {
    run_systemctl(["restart", "lighthouse.service"])
}

fn run_systemctl<I, S>(args: I) -> Result<String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<std::ffi::OsStr>,
{
    let output = Command::new("systemctl")
        .args(args)
        .output()
        .context("failed to run systemctl")?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(anyhow::anyhow!("systemctl failed: {}", stderr))
    }
}
