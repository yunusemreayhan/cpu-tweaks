use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// Detected CPU driver type.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Driver {
    IntelPstate,
    AmdPstate,
    AcpiCpufreq,
    Other,
}

impl std::fmt::Display for Driver {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::IntelPstate => write!(f, "intel_pstate"),
            Self::AmdPstate => write!(f, "amd-pstate"),
            Self::AcpiCpufreq => write!(f, "acpi-cpufreq"),
            Self::Other => write!(f, "unknown"),
        }
    }
}

/// Per-CPU live info.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuCore {
    pub id: u32,
    pub cur_freq_khz: u64,
    pub min_freq_khz: u64,
    pub max_freq_khz: u64,
    pub scaling_min_khz: u64,
    pub scaling_max_khz: u64,
    pub base_freq_khz: Option<u64>,
    pub governor: String,
    pub epp: Option<String>,
}

/// Global system CPU info.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuInfo {
    pub driver: Driver,
    pub model_name: String,
    pub cpu_count: u32,
    pub cores: Vec<CpuCore>,
    pub available_governors: Vec<String>,
    pub available_epp: Vec<String>,
    pub hw_min_khz: u64,
    pub hw_max_khz: u64,
    pub turbo_supported: bool,
    pub turbo_enabled: bool,
    /// Intel pstate: min_perf_pct / max_perf_pct
    pub min_perf_pct: Option<u32>,
    pub max_perf_pct: Option<u32>,
    pub hwp_dynamic_boost: Option<bool>,
}

fn read_str(p: &Path) -> Result<String> {
    fs::read_to_string(p)
        .map(|s| s.trim().to_string())
        .with_context(|| format!("reading {}", p.display()))
}

fn read_u64(p: &Path) -> Result<u64> {
    read_str(p)?
        .parse()
        .with_context(|| format!("parsing {}", p.display()))
}

fn try_read_str(p: &Path) -> Option<String> {
    read_str(p).ok()
}

fn try_read_u64(p: &Path) -> Option<u64> {
    read_u64(p).ok()
}

fn detect_driver() -> Driver {
    if let Ok(d) = read_str(Path::new("/sys/devices/system/cpu/cpu0/cpufreq/scaling_driver")) {
        match d.as_str() {
            "intel_pstate" => Driver::IntelPstate,
            "amd-pstate" | "amd-pstate-epp" | "amd_pstate" => Driver::AmdPstate,
            "acpi-cpufreq" => Driver::AcpiCpufreq,
            _ => Driver::Other,
        }
    } else {
        Driver::Other
    }
}

fn cpu_dirs() -> Vec<(u32, PathBuf)> {
    let mut cpus = vec![];
    if let Ok(entries) = fs::read_dir("/sys/devices/system/cpu") {
        for e in entries.flatten() {
            let name = e.file_name().to_string_lossy().to_string();
            if let Some(num) = name.strip_prefix("cpu").and_then(|s| s.parse::<u32>().ok()) {
                let freq_dir = e.path().join("cpufreq");
                if freq_dir.exists() {
                    cpus.push((num, freq_dir));
                }
            }
        }
    }
    cpus.sort_by_key(|(id, _)| *id);
    cpus
}

fn model_name() -> String {
    if let Ok(info) = fs::read_to_string("/proc/cpuinfo") {
        for line in info.lines() {
            if line.starts_with("model name") {
                if let Some(val) = line.split(':').nth(1) {
                    return val.trim().to_string();
                }
            }
        }
    }
    "Unknown CPU".into()
}

pub fn read_cpu_info() -> Result<CpuInfo> {
    let driver = detect_driver();
    let dirs = cpu_dirs();
    let cpu_count = dirs.len() as u32;

    let mut cores = Vec::with_capacity(dirs.len());
    let mut available_governors = vec![];
    let mut available_epp = vec![];
    let mut hw_min = u64::MAX;
    let mut hw_max = 0u64;

    for (id, dir) in &dirs {
        let cur = read_u64(&dir.join("scaling_cur_freq")).unwrap_or(0);
        let min = read_u64(&dir.join("cpuinfo_min_freq")).unwrap_or(0);
        let max = read_u64(&dir.join("cpuinfo_max_freq")).unwrap_or(0);
        let smin = read_u64(&dir.join("scaling_min_freq")).unwrap_or(min);
        let smax = read_u64(&dir.join("scaling_max_freq")).unwrap_or(max);
        let base = try_read_u64(&dir.join("base_frequency"));
        let gov = read_str(&dir.join("scaling_governor")).unwrap_or_default();
        let epp = try_read_str(&dir.join("energy_performance_preference"));

        if *id == 0 {
            if let Ok(g) = read_str(&dir.join("scaling_available_governors")) {
                available_governors = g.split_whitespace().map(String::from).collect();
            }
            if let Ok(e) = read_str(&dir.join("energy_performance_available_preferences")) {
                available_epp = e.split_whitespace().map(String::from).collect();
            }
        }

        hw_min = hw_min.min(min);
        hw_max = hw_max.max(max);

        cores.push(CpuCore {
            id: *id, cur_freq_khz: cur, min_freq_khz: min, max_freq_khz: max,
            scaling_min_khz: smin, scaling_max_khz: smax, base_freq_khz: base,
            governor: gov, epp,
        });
    }

    // Turbo / boost detection
    let (turbo_supported, turbo_enabled) = match driver {
        Driver::IntelPstate => {
            let p = Path::new("/sys/devices/system/cpu/intel_pstate/no_turbo");
            (p.exists(), try_read_str(p).map(|s| s == "0").unwrap_or(false))
        }
        _ => {
            let p = Path::new("/sys/devices/system/cpu/cpufreq/boost");
            (p.exists(), try_read_str(p).map(|s| s == "1").unwrap_or(false))
        }
    };

    let pstate_dir = Path::new("/sys/devices/system/cpu/intel_pstate");
    let min_perf_pct = try_read_str(&pstate_dir.join("min_perf_pct"))
        .and_then(|s| s.parse().ok());
    let max_perf_pct = try_read_str(&pstate_dir.join("max_perf_pct"))
        .and_then(|s| s.parse().ok());
    let hwp_dynamic_boost = try_read_str(&pstate_dir.join("hwp_dynamic_boost"))
        .map(|s| s == "1");

    Ok(CpuInfo {
        driver, model_name: model_name(), cpu_count, cores,
        available_governors, available_epp,
        hw_min_khz: if hw_min == u64::MAX { 0 } else { hw_min },
        hw_max_khz: hw_max,
        turbo_supported, turbo_enabled,
        min_perf_pct, max_perf_pct, hwp_dynamic_boost,
    })
}
