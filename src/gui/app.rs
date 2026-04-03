use crate::cpu::apply::{apply, ApplyRequest};
use crate::cpu::sysfs::{self, CpuInfo};
use std::time::Instant;

/// A mismatch between what the user requested and what sysfs reports.
#[derive(Debug, Clone)]
pub struct Drift {
    pub label: String,
    pub expected: String,
    pub actual: String,
}

pub struct CpuTweaksApp {
    pub info: CpuInfo,
    pub last_refresh: Instant,
    // Editable state
    pub sel_governor: usize,
    pub sel_epp: usize,
    pub turbo: bool,
    pub min_freq_mhz: f32,
    pub max_freq_mhz: f32,
    pub min_perf_pct: f32,
    pub max_perf_pct: f32,
    pub hwp_dynamic_boost: bool,
    // Status
    pub status_msg: String,
    pub status_is_err: bool,
    pub theme_applied: bool,
    /// Warnings when UI state differs from live sysfs.
    pub drifts: Vec<Drift>,
}

impl CpuTweaksApp {
    pub fn new() -> Self {
        let info = sysfs::read_cpu_info().unwrap_or_else(|e| {
            eprintln!("Failed to read CPU info: {e}");
            std::process::exit(1);
        });
        let mut app = Self {
            sel_governor: 0, sel_epp: 0, turbo: false,
            min_freq_mhz: 0.0, max_freq_mhz: 0.0,
            min_perf_pct: 0.0, max_perf_pct: 100.0,
            hwp_dynamic_boost: false,
            status_msg: String::new(), status_is_err: false,
            theme_applied: false, drifts: vec![],
            last_refresh: Instant::now(), info,
        };
        app.sync_from_info();
        app
    }

    pub fn refresh(&mut self) {
        if let Ok(new_info) = sysfs::read_cpu_info() {
            self.info = new_info;
            self.last_refresh = Instant::now();
        }
    }

    /// Compare UI selections against live sysfs and populate drifts.
    pub fn check_drifts(&mut self) {
        let mut drifts = vec![];
        let core0 = match self.info.cores.first() {
            Some(c) => c,
            None => { self.drifts = drifts; return; }
        };

        // Governor
        if let Some(ui_gov) = self.info.available_governors.get(self.sel_governor) {
            if *ui_gov != core0.governor {
                drifts.push(Drift {
                    label: "Governor".into(),
                    expected: ui_gov.clone(),
                    actual: core0.governor.clone(),
                });
            }
        }

        // EPP
        if let (Some(ui_epp), Some(live_epp)) = (
            self.info.available_epp.get(self.sel_epp),
            core0.epp.as_ref(),
        ) {
            if ui_epp != live_epp {
                drifts.push(Drift {
                    label: "EPP".into(),
                    expected: ui_epp.clone(),
                    actual: live_epp.clone(),
                });
            }
        }

        // Turbo
        if self.info.turbo_supported && self.turbo != self.info.turbo_enabled {
            drifts.push(Drift {
                label: "Turbo".into(),
                expected: if self.turbo { "Enabled" } else { "Disabled" }.into(),
                actual: if self.info.turbo_enabled { "Enabled" } else { "Disabled" }.into(),
            });
        }

        // Freq limits (allow 100 MHz tolerance for rounding)
        let live_min = core0.scaling_min_khz / 1000;
        let live_max = core0.scaling_max_khz / 1000;
        let ui_min = self.min_freq_mhz as u64;
        let ui_max = self.max_freq_mhz as u64;
        if ui_min.abs_diff(live_min) > 100 {
            drifts.push(Drift {
                label: "Min Freq".into(),
                expected: format!("{ui_min} MHz"),
                actual: format!("{live_min} MHz"),
            });
        }
        if ui_max.abs_diff(live_max) > 100 {
            drifts.push(Drift {
                label: "Max Freq".into(),
                expected: format!("{ui_max} MHz"),
                actual: format!("{live_max} MHz"),
            });
        }

        // Perf pct
        if let Some(live_min_pct) = self.info.min_perf_pct {
            if (self.min_perf_pct as u32) != live_min_pct {
                drifts.push(Drift {
                    label: "Min Perf %".into(),
                    expected: format!("{}%", self.min_perf_pct as u32),
                    actual: format!("{live_min_pct}%"),
                });
            }
        }
        if let Some(live_max_pct) = self.info.max_perf_pct {
            if (self.max_perf_pct as u32) != live_max_pct {
                drifts.push(Drift {
                    label: "Max Perf %".into(),
                    expected: format!("{}%", self.max_perf_pct as u32),
                    actual: format!("{live_max_pct}%"),
                });
            }
        }

        // HWP dynamic boost
        if let Some(live_hwp) = self.info.hwp_dynamic_boost {
            if self.hwp_dynamic_boost != live_hwp {
                drifts.push(Drift {
                    label: "HWP Dynamic Boost".into(),
                    expected: if self.hwp_dynamic_boost { "On" } else { "Off" }.into(),
                    actual: if live_hwp { "On" } else { "Off" }.into(),
                });
            }
        }

        self.drifts = drifts;
    }

    pub fn apply_changes(&mut self) {
        let mut req = ApplyRequest::default();

        if let Some(gov) = self.info.available_governors.get(self.sel_governor) {
            if self.info.cores.first().map(|c| &c.governor) != Some(gov) {
                req.governor = Some(gov.clone());
            }
        }
        if !self.info.available_epp.is_empty() {
            if let Some(epp) = self.info.available_epp.get(self.sel_epp) {
                if self.info.cores.first().and_then(|c| c.epp.as_ref()) != Some(epp) {
                    req.epp = Some(epp.clone());
                }
            }
        }
        if self.info.turbo_supported && self.turbo != self.info.turbo_enabled {
            req.turbo_enabled = Some(self.turbo);
        }
        let cur_min = self.info.cores.first().map(|c| c.scaling_min_khz).unwrap_or(0);
        let cur_max = self.info.cores.first().map(|c| c.scaling_max_khz).unwrap_or(0);
        let new_min = (self.min_freq_mhz as u64) * 1000;
        let new_max = (self.max_freq_mhz as u64) * 1000;
        if new_min != cur_min { req.min_freq_khz = Some(new_min); }
        if new_max != cur_max { req.max_freq_khz = Some(new_max); }
        if self.info.min_perf_pct.is_some() {
            let cur = self.info.min_perf_pct.unwrap_or(0);
            let new_val = self.min_perf_pct as u32;
            if new_val != cur { req.min_perf_pct = Some(new_val); }
        }
        if self.info.max_perf_pct.is_some() {
            let cur = self.info.max_perf_pct.unwrap_or(100);
            let new_val = self.max_perf_pct as u32;
            if new_val != cur { req.max_perf_pct = Some(new_val); }
        }
        if self.info.hwp_dynamic_boost.is_some() {
            let cur = self.info.hwp_dynamic_boost.unwrap_or(false);
            if self.hwp_dynamic_boost != cur {
                req.hwp_dynamic_boost = Some(self.hwp_dynamic_boost);
            }
        }

        match apply(&self.info, &req) {
            Ok(msg) => {
                self.status_msg = msg;
                self.status_is_err = false;
                // Re-read sysfs and verify
                self.refresh();
                self.check_drifts();
                if !self.drifts.is_empty() {
                    let names: Vec<_> = self.drifts.iter().map(|d| d.label.as_str()).collect();
                    self.status_msg = format!("Applied, but {} not confirmed: {}", self.drifts.len(), names.join(", "));
                    self.status_is_err = true;
                }
            }
            Err(e) => {
                self.status_msg = e.to_string();
                self.status_is_err = true;
            }
        }
    }

    pub fn sync_from_info(&mut self) {
        self.sel_governor = self.info.available_governors.iter()
            .position(|g| self.info.cores.first().map(|c| &c.governor) == Some(g))
            .unwrap_or(0);
        self.sel_epp = self.info.available_epp.iter()
            .position(|e| self.info.cores.first().and_then(|c| c.epp.as_ref()) == Some(e))
            .unwrap_or(0);
        self.turbo = self.info.turbo_enabled;
        self.min_freq_mhz = self.info.cores.first().map(|c| c.scaling_min_khz / 1000).unwrap_or(0) as f32;
        self.max_freq_mhz = self.info.cores.first().map(|c| c.scaling_max_khz / 1000).unwrap_or(0) as f32;
        self.min_perf_pct = self.info.min_perf_pct.unwrap_or(0) as f32;
        self.max_perf_pct = self.info.max_perf_pct.unwrap_or(100) as f32;
        self.hwp_dynamic_boost = self.info.hwp_dynamic_boost.unwrap_or(false);
        self.drifts.clear();
    }
}

impl eframe::App for CpuTweaksApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if !self.theme_applied {
            super::theme::apply_theme(ctx);
            self.theme_applied = true;
        }

        // Auto-refresh every 5s + check drifts
        ctx.request_repaint_after(std::time::Duration::from_secs(5));
        if self.last_refresh.elapsed().as_secs() >= 5 {
            self.refresh();
            self.check_drifts();
        }

        super::views::draw(ctx, self);
    }
}
