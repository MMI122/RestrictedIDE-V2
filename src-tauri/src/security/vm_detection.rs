//! VM / hypervisor detection.
//!
//! Checks multiple indicators and returns a result with details.
//! Policy can decide whether to block or just log.

use std::process::Command;

#[derive(Debug, Clone, serde::Serialize)]
pub struct VmCheckResult {
    pub is_vm: bool,
    pub indicators: Vec<String>,
}

/// Run all VM detection heuristics and return a combined result.
pub fn detect_vm() -> VmCheckResult {
    let mut indicators: Vec<String> = Vec::new();

    // 1) Check well-known VM registry keys
    check_registry_indicators(&mut indicators);

    // 2) Check system model / manufacturer via WMI (wmic)
    check_wmi_indicators(&mut indicators);

    // 3) Check known VM-related processes
    check_vm_processes(&mut indicators);

    // 4) Check MAC address OUI prefixes for VM NICs
    check_mac_address(&mut indicators);

    let is_vm = !indicators.is_empty();

    if is_vm {
        log::warn!(
            "[Security] VM detected — {} indicator(s): {:?}",
            indicators.len(),
            indicators
        );
    } else {
        log::info!("[Security] VM check passed — no virtualization indicators found");
    }

    VmCheckResult { is_vm, indicators }
}

/// Check Windows registry for VM-specific keys/values.
fn check_registry_indicators(indicators: &mut Vec<String>) {
    // Registry paths that indicate virtualisation
    let registry_checks: Vec<(&str, &str, &[&str])> = vec![
        (
            r"HKLM\SOFTWARE\Microsoft\Virtual Machine\Guest\Parameters",
            "Hyper-V Guest Parameters key",
            &[],
        ),
        (
            r"HKLM\SOFTWARE\Oracle\VirtualBox Guest Additions",
            "VirtualBox Guest Additions key",
            &[],
        ),
        (
            r"HKLM\SOFTWARE\VMware, Inc.\VMware Tools",
            "VMware Tools key",
            &[],
        ),
        (
            r"HKLM\SYSTEM\CurrentControlSet\Services\VBoxGuest",
            "VBoxGuest service",
            &[],
        ),
        (
            r"HKLM\SYSTEM\CurrentControlSet\Services\vmci",
            "VMware VMCI service",
            &[],
        ),
        (
            r"HKLM\SYSTEM\CurrentControlSet\Services\vmhgfs",
            "VMware HGFS service",
            &[],
        ),
    ];

    for (path, description, _) in registry_checks {
        if registry_key_exists(path) {
            indicators.push(format!("Registry: {}", description));
        }
    }
}

/// Test if a registry key exists using `reg query`.
fn registry_key_exists(path: &str) -> bool {
    Command::new("reg")
        .args(["query", path])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Use `wmic` to check ComputerSystem Manufacturer and Model.
fn check_wmi_indicators(indicators: &mut Vec<String>) {
    let vm_strings = [
        "virtualbox",
        "vmware",
        "qemu",
        "kvm",
        "virtual machine",
        "xen",
        "parallels",
        "hyper-v",
        "bochs",
        "innotek",
    ];

    // Check manufacturer
    if let Some(manufacturer) = wmic_value("computersystem", "manufacturer") {
        let lower = manufacturer.to_lowercase();
        for pattern in &vm_strings {
            if lower.contains(pattern) {
                indicators.push(format!("WMI Manufacturer: {}", manufacturer.trim()));
                break;
            }
        }
    }

    // Check model
    if let Some(model) = wmic_value("computersystem", "model") {
        let lower = model.to_lowercase();
        for pattern in &vm_strings {
            if lower.contains(pattern) {
                indicators.push(format!("WMI Model: {}", model.trim()));
                break;
            }
        }
    }

    // Check BIOS serial (some VMs have distinctive serials)
    if let Some(serial) = wmic_value("bios", "serialnumber") {
        let lower = serial.to_lowercase();
        let vm_serials = ["vmware", "virtualbox", "parallels", "0", "none"];
        for pattern in &vm_serials {
            if lower.contains(pattern) {
                indicators.push(format!("WMI BIOS serial: {}", serial.trim()));
                break;
            }
        }
    }
}

/// Run `wmic <alias> get <property>` and return the value line.
fn wmic_value(alias: &str, property: &str) -> Option<String> {
    let output = Command::new("wmic")
        .args([alias, "get", property])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout);
    // wmic output: first line is header, second line is value
    text.lines()
        .skip(1)
        .find(|line| !line.trim().is_empty())
        .map(|s| s.trim().to_string())
}

/// Check for VM-specific processes.
fn check_vm_processes(indicators: &mut Vec<String>) {
    let vm_processes = [
        "vmtoolsd.exe",
        "vmwaretray.exe",
        "vmwareuser.exe",
        "vboxservice.exe",
        "vboxtray.exe",
        "vboxtray.exe",
        "qemu-ga.exe",
        "xenservice.exe",
        "prl_tools.exe",
        "prl_cc.exe",
    ];

    let output = match Command::new("tasklist")
        .args(["/FO", "CSV", "/NH"])
        .output()
    {
        Ok(o) => o,
        Err(_) => return,
    };

    let text = String::from_utf8_lossy(&output.stdout).to_lowercase();
    for proc in &vm_processes {
        if text.contains(&proc.to_lowercase()) {
            indicators.push(format!("Process: {}", proc));
        }
    }
}

/// Check network adapter MAC address OUI prefixes known to belong to VMs.
fn check_mac_address(indicators: &mut Vec<String>) {
    let vm_mac_prefixes = [
        "00:05:69", // VMware
        "00:0c:29", // VMware
        "00:1c:14", // VMware
        "00:50:56", // VMware
        "08:00:27", // VirtualBox
        "00:15:5d", // Hyper-V
        "00:16:3e", // Xen
        "52:54:00", // QEMU/KVM
        "00:1c:42", // Parallels
    ];

    let output = match Command::new("getmac")
        .args(["/FO", "CSV", "/NH", "/V"])
        .output()
    {
        Ok(o) => o,
        Err(_) => return,
    };

    let text = String::from_utf8_lossy(&output.stdout).to_lowercase();
    // Normalize MAC separators (getmac uses -)
    let text_colons = text.replace('-', ":");

    for prefix in &vm_mac_prefixes {
        if text_colons.contains(&prefix.to_lowercase()) {
            indicators.push(format!("MAC prefix: {}", prefix));
        }
    }
}
