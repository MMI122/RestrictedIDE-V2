use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct SystemInfo {
    pub platform: String,
    pub arch: String,
    pub hostname: String,
    pub app_version: String,
}

#[tauri::command]
pub fn get_system_info() -> Result<SystemInfo, String> {
    Ok(SystemInfo {
        platform: std::env::consts::OS.into(),
        arch: std::env::consts::ARCH.into(),
        hostname: sysinfo::System::host_name().unwrap_or_else(|| "unknown".into()),
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
