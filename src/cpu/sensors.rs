use std::fs;
use std::path::Path;

/// A temperature sensor reading.
#[derive(Debug, Clone)]
pub struct TempSensor {
    pub hwmon: String,       // e.g. "coretemp", "nvme", "amdgpu"
    pub label: String,       // e.g. "Package id 0", "Core 0", "Composite"
    pub temp_mc: i64,        // millidegrees C
    pub crit_mc: Option<i64>,
    pub max_mc: Option<i64>,
}

impl TempSensor {
    pub fn temp_c(&self) -> f64 { self.temp_mc as f64 / 1000.0 }
    pub fn crit_c(&self) -> Option<f64> { self.crit_mc.map(|v| v as f64 / 1000.0) }
}

/// A fan sensor reading.
#[derive(Debug, Clone)]
pub struct FanSensor {
    pub hwmon: String,
    pub label: String,
    pub rpm: Option<u64>,    // None if unreadable
}

/// Fan PWM control.
#[derive(Debug, Clone)]
pub struct FanControl {
    pub hwmon: String,
    pub pwm_path: String,
    pub enable_path: String,
    pub pwm_value: u8,       // 0-255
    pub enable_mode: u8,     // 0=full, 1=manual, 2=auto
}

/// Battery info.
#[derive(Debug, Clone)]
pub struct BatteryInfo {
    pub present: bool,
    pub status: String,       // "Charging", "Discharging", "Not charging", "Full"
    pub capacity_pct: u8,
    pub power_watts: Option<f64>,
}

/// All sensor readings.
#[derive(Debug, Clone, Default)]
pub struct SensorData {
    pub temps: Vec<TempSensor>,
    pub fans: Vec<FanSensor>,
    pub fan_controls: Vec<FanControl>,
    pub battery: Option<BatteryInfo>,
    /// Convenience: CPU package temp in C (first coretemp/k10temp Package reading)
    pub cpu_temp_c: Option<f64>,
}

fn read_str(p: &Path) -> Option<String> {
    fs::read_to_string(p).ok().map(|s| s.trim().to_string())
}

fn read_i64(p: &Path) -> Option<i64> {
    read_str(p)?.parse().ok()
}

fn read_u64(p: &Path) -> Option<u64> {
    read_str(p)?.parse().ok()
}

pub fn read_sensors() -> SensorData {
    let mut data = SensorData::default();

    // Scan hwmon devices
    if let Ok(entries) = fs::read_dir("/sys/class/hwmon") {
        for entry in entries.flatten() {
            let dir = entry.path();
            let name = read_str(&dir.join("name")).unwrap_or_default();

            // Temperature sensors: temp{N}_input
            for i in 1..=32 {
                let input = dir.join(format!("temp{i}_input"));
                if !input.exists() { continue; }
                let temp_mc = match read_i64(&input) {
                    Some(v) => v,
                    None => continue,
                };
                let label = read_str(&dir.join(format!("temp{i}_label")))
                    .unwrap_or_else(|| format!("temp{i}"));
                let crit = read_i64(&dir.join(format!("temp{i}_crit")));
                let max = read_i64(&dir.join(format!("temp{i}_max")));

                data.temps.push(TempSensor {
                    hwmon: name.clone(), label, temp_mc, crit_mc: crit, max_mc: max,
                });
            }

            // Fan sensors: fan{N}_input
            for i in 1..=8 {
                let input = dir.join(format!("fan{i}_input"));
                if !input.exists() { continue; }
                let rpm = read_u64(&input);
                let label = read_str(&dir.join(format!("fan{i}_label")))
                    .unwrap_or_else(|| format!("Fan {i}"));
                data.fans.push(FanSensor { hwmon: name.clone(), label, rpm });
            }

            // PWM fan control: pwm{N}, pwm{N}_enable
            for i in 1..=8 {
                let pwm_path = dir.join(format!("pwm{i}"));
                let enable_path = dir.join(format!("pwm{i}_enable"));
                if !enable_path.exists() { continue; }
                let pwm_value = read_str(&pwm_path).and_then(|s| s.parse().ok()).unwrap_or(0);
                let enable_mode = read_str(&enable_path).and_then(|s| s.parse().ok()).unwrap_or(2);
                data.fan_controls.push(FanControl {
                    hwmon: name.clone(),
                    pwm_path: pwm_path.to_string_lossy().into(),
                    enable_path: enable_path.to_string_lossy().into(),
                    pwm_value, enable_mode,
                });
            }
        }
    }

    // Sort temps: coretemp/k10temp first, then by label
    data.temps.sort_by(|a, b| {
        let a_cpu = matches!(a.hwmon.as_str(), "coretemp" | "k10temp");
        let b_cpu = matches!(b.hwmon.as_str(), "coretemp" | "k10temp");
        b_cpu.cmp(&a_cpu).then(a.label.cmp(&b.label))
    });

    // Extract CPU package temp
    data.cpu_temp_c = data.temps.iter()
        .find(|t| (t.hwmon == "coretemp" || t.hwmon == "k10temp")
            && (t.label.contains("Package") || t.label.contains("Tctl") || t.label.contains("Tdie")))
        .map(|t| t.temp_c());

    // Battery
    let bat_path = Path::new("/sys/class/power_supply/BAT0");
    if bat_path.exists() {
        let status = read_str(&bat_path.join("status")).unwrap_or_default();
        let capacity = read_str(&bat_path.join("capacity"))
            .and_then(|s| s.parse().ok()).unwrap_or(0);
        let power = read_u64(&bat_path.join("power_now"))
            .map(|uw| uw as f64 / 1_000_000.0); // microwatts to watts
        data.battery = Some(BatteryInfo {
            present: true, status, capacity_pct: capacity, power_watts: power,
        });
    }

    data
}
