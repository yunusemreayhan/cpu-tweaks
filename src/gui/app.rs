use crate::cpu::apply::{apply, ApplyRequest};
use crate::cpu::sysfs::{self, CpuInfo, CpuJiffies};
use crate::cpu::sensors::{self, SensorData};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::time::Instant;

#[derive(Debug, Clone)]
pub struct Drift {
    pub label: String,
    pub expected: String,
    pub actual: String,
}

#[derive(Debug, Clone)]
pub struct Sample {
    pub time_secs: f64,
    pub cpu_util_pct: f64,
    pub avg_freq_mhz: f64,
}

/// Per-sensor temperature history keyed by "hwmon/label".
pub type TempHistory = BTreeMap<String, VecDeque<(f64, f64)>>; // key -> [(time_secs, temp_c)]

pub struct CpuTweaksApp {
    pub info: CpuInfo,
    pub sensors: SensorData,
    pub last_refresh: Instant,
    pub app_start: Instant,
    pub sel_governor: usize,
    pub sel_epp: usize,
    pub turbo: bool,
    pub min_freq_mhz: f32,
    pub max_freq_mhz: f32,
    pub min_perf_pct: f32,
    pub max_perf_pct: f32,
    pub hwp_dynamic_boost: bool,
    pub status_msg: String,
    pub status_is_err: bool,
    pub theme_applied: bool,
    pub drifts: Vec<Drift>,
    pub history: VecDeque<Sample>,
    pub temp_history: TempHistory,
    pub selected_temps: BTreeSet<String>,
    pub history_window_secs: f64,
    pub tab: Tab,
    prev_jiffies: CpuJiffies,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Tab { Configurator, Sensors }

const MAX_HISTORY: usize = 7200;

/// Build a sensor key like "coretemp/Package id 0"
pub fn sensor_key(hwmon: &str, label: &str) -> String {
    format!("{hwmon}/{label}")
}

/// Is this a "summary" sensor worth selecting by default?
fn is_default_sensor(hwmon: &str, label: &str) -> bool {
    // Package/die temps, NVMe composite, GPU junction
    label.contains("Package") || label.contains("Tctl") || label.contains("Tdie")
        || label.contains("Composite") || label.contains("junction")
        || (hwmon == "amdgpu" && label.contains("edge"))
        || hwmon == "iwlwifi_1"
}

impl CpuTweaksApp {
    pub fn new() -> Self {
        let info = sysfs::read_cpu_info().unwrap_or_else(|e| {
            eprintln!("Failed to read CPU info: {e}");
            std::process::exit(1);
        });
        let jiffies = sysfs::read_cpu_jiffies();
        let sensor_data = sensors::read_sensors();

        // Auto-select default sensors
        let selected: BTreeSet<String> = sensor_data.temps.iter()
            .filter(|t| t.temp_mc != 0 && is_default_sensor(&t.hwmon, &t.label))
            .map(|t| sensor_key(&t.hwmon, &t.label))
            .collect();

        let mut app = Self {
            sel_governor: 0, sel_epp: 0, turbo: false,
            min_freq_mhz: 0.0, max_freq_mhz: 0.0,
            min_perf_pct: 0.0, max_perf_pct: 100.0,
            hwp_dynamic_boost: false,
            status_msg: String::new(), status_is_err: false,
            theme_applied: false, drifts: vec![],
            history: VecDeque::with_capacity(MAX_HISTORY),
            temp_history: BTreeMap::new(),
            selected_temps: selected,
            history_window_secs: 120.0,
            prev_jiffies: jiffies,
            sensors: sensor_data,
            tab: Tab::Configurator,
            app_start: Instant::now(),
            last_refresh: Instant::now(), info,
        };
        app.sync_from_info();
        app
    }

    pub fn refresh(&mut self) {
        if let Ok(new_info) = sysfs::read_cpu_info() {
            self.info = new_info;
            self.last_refresh = Instant::now();
            self.sensors = sensors::read_sensors();
            self.record_sample();
        }
    }

    fn record_sample(&mut self) {
        let now_jiffies = sysfs::read_cpu_jiffies();
        let dt = now_jiffies.total().saturating_sub(self.prev_jiffies.total());
        let db = now_jiffies.busy().saturating_sub(self.prev_jiffies.busy());
        let util = if dt > 0 { (db as f64 / dt as f64) * 100.0 } else { 0.0 };
        self.prev_jiffies = now_jiffies;

        let avg_freq = if self.info.cores.is_empty() { 0.0 } else {
            self.info.cores.iter().map(|c| c.cur_freq_khz as f64).sum::<f64>()
                / self.info.cores.len() as f64 / 1000.0
        };

        let t = self.app_start.elapsed().as_secs_f64();

        if self.history.len() >= MAX_HISTORY { self.history.pop_front(); }
        self.history.push_back(Sample { time_secs: t, cpu_util_pct: util, avg_freq_mhz: avg_freq });

        // Record per-sensor temps
        for sensor in &self.sensors.temps {
            if sensor.temp_mc == 0 { continue; }
            let key = sensor_key(&sensor.hwmon, &sensor.label);
            let q = self.temp_history.entry(key).or_insert_with(|| VecDeque::with_capacity(MAX_HISTORY));
            if q.len() >= MAX_HISTORY { q.pop_front(); }
            q.push_back((t, sensor.temp_c()));
        }
    }

    pub fn check_drifts(&mut self) {
        let mut drifts = vec![];
        let core0 = match self.info.cores.first() {
            Some(c) => c,
            None => { self.drifts = drifts; return; }
        };
        if let Some(ui_gov) = self.info.available_governors.get(self.sel_governor) {
            if *ui_gov != core0.governor {
                drifts.push(Drift { label: "Governor".into(), expected: ui_gov.clone(), actual: core0.governor.clone() });
            }
        }
        if let (Some(ui_epp), Some(live_epp)) = (self.info.available_epp.get(self.sel_epp), core0.epp.as_ref()) {
            if ui_epp != live_epp {
                drifts.push(Drift { label: "EPP".into(), expected: ui_epp.clone(), actual: live_epp.clone() });
            }
        }
        if self.info.turbo_supported && self.turbo != self.info.turbo_enabled {
            drifts.push(Drift { label: "Turbo".into(),
                expected: if self.turbo { "Enabled" } else { "Disabled" }.into(),
                actual: if self.info.turbo_enabled { "Enabled" } else { "Disabled" }.into() });
        }
        let live_min = core0.scaling_min_khz / 1000;
        let live_max = core0.scaling_max_khz / 1000;
        if (self.min_freq_mhz as u64).abs_diff(live_min) > 100 {
            drifts.push(Drift { label: "Min Freq".into(), expected: format!("{} MHz", self.min_freq_mhz as u64), actual: format!("{live_min} MHz") });
        }
        if (self.max_freq_mhz as u64).abs_diff(live_max) > 100 {
            drifts.push(Drift { label: "Max Freq".into(), expected: format!("{} MHz", self.max_freq_mhz as u64), actual: format!("{live_max} MHz") });
        }
        if let Some(live) = self.info.min_perf_pct {
            if (self.min_perf_pct as u32) != live { drifts.push(Drift { label: "Min Perf %".into(), expected: format!("{}%", self.min_perf_pct as u32), actual: format!("{live}%") }); }
        }
        if let Some(live) = self.info.max_perf_pct {
            if (self.max_perf_pct as u32) != live { drifts.push(Drift { label: "Max Perf %".into(), expected: format!("{}%", self.max_perf_pct as u32), actual: format!("{live}%") }); }
        }
        if let Some(live) = self.info.hwp_dynamic_boost {
            if self.hwp_dynamic_boost != live { drifts.push(Drift { label: "HWP Dynamic Boost".into(),
                expected: if self.hwp_dynamic_boost { "On" } else { "Off" }.into(),
                actual: if live { "On" } else { "Off" }.into() }); }
        }
        self.drifts = drifts;
    }

    pub fn apply_changes(&mut self) {
        let mut req = ApplyRequest::default();
        if let Some(gov) = self.info.available_governors.get(self.sel_governor) {
            if self.info.cores.first().map(|c| &c.governor) != Some(gov) { req.governor = Some(gov.clone()); }
        }
        if !self.info.available_epp.is_empty() {
            if let Some(epp) = self.info.available_epp.get(self.sel_epp) {
                if self.info.cores.first().and_then(|c| c.epp.as_ref()) != Some(epp) { req.epp = Some(epp.clone()); }
            }
        }
        if self.info.turbo_supported && self.turbo != self.info.turbo_enabled { req.turbo_enabled = Some(self.turbo); }
        let cur_min = self.info.cores.first().map(|c| c.scaling_min_khz).unwrap_or(0);
        let cur_max = self.info.cores.first().map(|c| c.scaling_max_khz).unwrap_or(0);
        let new_min = (self.min_freq_mhz as u64) * 1000;
        let new_max = (self.max_freq_mhz as u64) * 1000;
        if new_min != cur_min { req.min_freq_khz = Some(new_min); }
        if new_max != cur_max { req.max_freq_khz = Some(new_max); }
        if self.info.min_perf_pct.is_some() {
            let cur = self.info.min_perf_pct.unwrap_or(0);
            if self.min_perf_pct as u32 != cur { req.min_perf_pct = Some(self.min_perf_pct as u32); }
        }
        if self.info.max_perf_pct.is_some() {
            let cur = self.info.max_perf_pct.unwrap_or(100);
            if self.max_perf_pct as u32 != cur { req.max_perf_pct = Some(self.max_perf_pct as u32); }
        }
        if self.info.hwp_dynamic_boost.is_some() {
            let cur = self.info.hwp_dynamic_boost.unwrap_or(false);
            if self.hwp_dynamic_boost != cur { req.hwp_dynamic_boost = Some(self.hwp_dynamic_boost); }
        }

        match apply(&self.info, &req) {
            Ok(msg) => {
                self.status_msg = msg; self.status_is_err = false;
                self.refresh(); self.check_drifts();
                if !self.drifts.is_empty() {
                    let names: Vec<_> = self.drifts.iter().map(|d| d.label.as_str()).collect();
                    self.status_msg = format!("Applied, but {} not confirmed: {}", self.drifts.len(), names.join(", "));
                    self.status_is_err = true;
                }
            }
            Err(e) => { self.status_msg = e.to_string(); self.status_is_err = true; }
        }
    }

    pub fn sync_from_info(&mut self) {
        self.sel_governor = self.info.available_governors.iter()
            .position(|g| self.info.cores.first().map(|c| &c.governor) == Some(g)).unwrap_or(0);
        self.sel_epp = self.info.available_epp.iter()
            .position(|e| self.info.cores.first().and_then(|c| c.epp.as_ref()) == Some(e)).unwrap_or(0);
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
        ctx.request_repaint_after(std::time::Duration::from_secs(5));
        if self.last_refresh.elapsed().as_secs() >= 5 {
            self.refresh();
            self.check_drifts();
        }
        super::views::draw(ctx, self);
    }
}
