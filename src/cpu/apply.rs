use super::sysfs::{CpuInfo, Driver};
use anyhow::{bail, Result};
use std::process::Command;

/// Build a shell script that applies all requested changes atomically.
/// Runs via `pkexec bash -c '...'` for privilege escalation.
pub fn apply(info: &CpuInfo, req: &ApplyRequest) -> Result<String> {
    let mut cmds: Vec<String> = vec![];

    // Governor
    if let Some(gov) = &req.governor {
        if !info.available_governors.contains(gov) {
            bail!("Governor '{gov}' not available");
        }
        for core in &info.cores {
            cmds.push(format!(
                "echo '{gov}' > /sys/devices/system/cpu/cpu{}/cpufreq/scaling_governor",
                core.id
            ));
        }
    }

    // Frequency limits
    if let Some(min_khz) = req.min_freq_khz {
        for core in &info.cores {
            cmds.push(format!(
                "echo '{min_khz}' > /sys/devices/system/cpu/cpu{}/cpufreq/scaling_min_freq",
                core.id
            ));
        }
    }
    if let Some(max_khz) = req.max_freq_khz {
        for core in &info.cores {
            cmds.push(format!(
                "echo '{max_khz}' > /sys/devices/system/cpu/cpu{}/cpufreq/scaling_max_freq",
                core.id
            ));
        }
    }

    // EPP
    if let Some(epp) = &req.epp {
        if !info.available_epp.contains(epp) {
            bail!("EPP '{epp}' not available");
        }
        for core in &info.cores {
            cmds.push(format!(
                "echo '{epp}' > /sys/devices/system/cpu/cpu{}/cpufreq/energy_performance_preference",
                core.id
            ));
        }
    }

    // Turbo / boost
    if let Some(turbo) = req.turbo_enabled {
        match info.driver {
            Driver::IntelPstate => {
                let val = if turbo { "0" } else { "1" }; // no_turbo: 0=enabled
                cmds.push(format!(
                    "echo '{val}' > /sys/devices/system/cpu/intel_pstate/no_turbo"
                ));
            }
            _ => {
                let val = if turbo { "1" } else { "0" };
                cmds.push(format!(
                    "echo '{val}' > /sys/devices/system/cpu/cpufreq/boost"
                ));
            }
        }
    }

    // Intel pstate perf percentages
    if let Some(pct) = req.min_perf_pct {
        cmds.push(format!(
            "echo '{pct}' > /sys/devices/system/cpu/intel_pstate/min_perf_pct"
        ));
    }
    if let Some(pct) = req.max_perf_pct {
        cmds.push(format!(
            "echo '{pct}' > /sys/devices/system/cpu/intel_pstate/max_perf_pct"
        ));
    }

    // HWP dynamic boost
    if let Some(en) = req.hwp_dynamic_boost {
        let val = if en { "1" } else { "0" };
        cmds.push(format!(
            "echo '{val}' > /sys/devices/system/cpu/intel_pstate/hwp_dynamic_boost"
        ));
    }

    if cmds.is_empty() {
        return Ok("Nothing to apply.".into());
    }

    let script = cmds.join(" && ");
    let output = Command::new("pkexec")
        .args(["bash", "-c", &script])
        .output()?;

    if output.status.success() {
        Ok(format!("Applied {} changes successfully.", cmds.len()))
    } else {
        let err = String::from_utf8_lossy(&output.stderr);
        if err.contains("dismissed") || err.contains("Not authorized") {
            bail!("Authentication cancelled by user.")
        }
        bail!("Failed to apply: {err}")
    }
}

/// What the user wants to change.
#[derive(Debug, Default, Clone)]
pub struct ApplyRequest {
    pub governor: Option<String>,
    pub min_freq_khz: Option<u64>,
    pub max_freq_khz: Option<u64>,
    pub epp: Option<String>,
    pub turbo_enabled: Option<bool>,
    pub min_perf_pct: Option<u32>,
    pub max_perf_pct: Option<u32>,
    pub hwp_dynamic_boost: Option<bool>,
}
