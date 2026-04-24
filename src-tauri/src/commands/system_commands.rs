use serde::Serialize;
use sha2::{Digest, Sha256};
use std::process::Command;

#[derive(Debug, Serialize)]
pub struct SystemInfo {
    pub platform: String,
    pub arch: String,
    pub hostname: String,
    pub device_fingerprint: String,
    pub app_version: String,
}

fn normalized_token(s: &str) -> Option<String> {
    let t = s.trim().to_lowercase();
    if t.is_empty() || t == "unknown" || t == "none" || t == "n/a" {
        None
    } else {
        Some(t)
    }
}

#[cfg(target_os = "windows")]
fn wmic_value(alias: &str, property: &str) -> Option<String> {
    let output = Command::new("wmic")
        .args([alias, "get", property])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout);
    text.lines()
        .skip(1)
        .find(|line| !line.trim().is_empty())
        .map(|s| s.trim().to_string())
}

#[cfg(target_os = "windows")]
fn machine_guid() -> Option<String> {
    let output = Command::new("reg")
        .args([
            "query",
            r"HKLM\SOFTWARE\Microsoft\Cryptography",
            "/v",
            "MachineGuid",
        ])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }

    let text = String::from_utf8_lossy(&output.stdout);
    for line in text.lines() {
        if !line.contains("MachineGuid") {
            continue;
        }
        let parts: Vec<&str> = line.split_whitespace().collect();
        if let Some(last) = parts.last() {
            if let Some(v) = normalized_token(last) {
                return Some(v);
            }
        }
    }
    None
}

fn compute_device_fingerprint(hostname: &str) -> String {
    let mut components: Vec<String> = Vec::new();

    if let Some(host) = normalized_token(hostname) {
        components.push(format!("host:{}", host));
    }
    components.push(format!("os:{}", std::env::consts::OS));
    components.push(format!("arch:{}", std::env::consts::ARCH));

    #[cfg(target_os = "windows")]
    {
        if let Some(v) = machine_guid() {
            components.push(format!("guid:{}", v));
        }
        if let Some(v) = wmic_value("bios", "serialnumber").and_then(|s| normalized_token(&s)) {
            components.push(format!("bios:{}", v));
        }
        if let Some(v) = wmic_value("baseboard", "serialnumber").and_then(|s| normalized_token(&s)) {
            components.push(format!("board:{}", v));
        }
        if let Some(v) = wmic_value("cpu", "processorid").and_then(|s| normalized_token(&s)) {
            components.push(format!("cpu:{}", v));
        }
    }

    let canonical = components.join("|");
    let mut hasher = Sha256::new();
    hasher.update(canonical.as_bytes());
    format!("{:x}", hasher.finalize())
}

#[tauri::command]
pub fn get_system_info() -> Result<SystemInfo, String> {
    let hostname = sysinfo::System::host_name().unwrap_or_else(|| "unknown".into());
    let device_fingerprint = compute_device_fingerprint(&hostname);

    Ok(SystemInfo {
        platform: std::env::consts::OS.into(),
        arch: std::env::consts::ARCH.into(),
        hostname,
        device_fingerprint,
        app_version: env!("CARGO_PKG_VERSION").into(),
    })
}

#[derive(Debug, Serialize)]
pub struct SystemStatus {
    pub cpu_usage: f32,
    pub memory_used: u64,
    pub memory_total: u64,
    pub uptime: u64,
}

#[tauri::command]
pub fn get_system_status() -> Result<SystemStatus, String> {
    use sysinfo::System;

    let mut sys = System::new();
    sys.refresh_memory();
    sys.refresh_cpu_usage();

    // Small pause so CPU usage has data
    std::thread::sleep(std::time::Duration::from_millis(200));
    sys.refresh_cpu_usage();

    Ok(SystemStatus {
        cpu_usage: sys.global_cpu_usage(),
        memory_used: sys.used_memory(),
        memory_total: sys.total_memory(),
        uptime: System::uptime(),
    })
}
