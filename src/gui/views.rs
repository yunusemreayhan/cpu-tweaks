use super::app::CpuTweaksApp;
use super::theme::*;
use egui::{Color32, CornerRadius, Frame, Margin, Stroke, Vec2};

fn fmt_mhz(mhz: f32) -> String {
    if mhz >= 1000.0 { format!("{:.2} GHz", mhz / 1000.0) }
    else { format!("{:.0} MHz", mhz) }
}

fn fmt_khz(khz: u64) -> String {
    fmt_mhz(khz as f32 / 1000.0)
}

fn card_frame() -> Frame {
    Frame::new()
        .fill(BG_CARD)
        .corner_radius(CornerRadius::same(10))
        .inner_margin(Margin::same(14))
        .stroke(Stroke::new(1.0, Color32::from_rgb(50, 50, 70)))
}

fn section_label(ui: &mut egui::Ui, icon: &str, text: &str) {
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(icon).size(16.0).strong().color(ACCENT));
        ui.label(egui::RichText::new(text).size(16.0).strong().color(TEXT_PRIMARY));
    });
    ui.add_space(4.0);
}

pub fn draw(ctx: &egui::Context, app: &mut CpuTweaksApp) {
    use super::app::Tab;

    egui::TopBottomPanel::top("header").frame(
        Frame::new().fill(BG_PANEL).inner_margin(Margin::same(12))
    ).show(ctx, |ui| {
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("[=]").size(24.0).color(ACCENT));
            ui.label(egui::RichText::new("CPU Tweaks").size(22.0).strong().color(TEXT_PRIMARY));
            ui.separator();

            // Tabs
            let tab_btn = |ui: &mut egui::Ui, label: &str, tab: Tab, current: &mut Tab| {
                let sel = *current == tab;
                let text = egui::RichText::new(label).size(14.0)
                    .color(if sel { Color32::WHITE } else { TEXT_SECONDARY });
                if ui.add(egui::Button::new(text)
                    .fill(if sel { ACCENT } else { Color32::TRANSPARENT })
                    .corner_radius(CornerRadius::same(6))
                ).clicked() { *current = tab; }
            };
            tab_btn(ui, "Configurator", Tab::Configurator, &mut app.tab);
            tab_btn(ui, "Sensors", Tab::Sensors, &mut app.tab);

            ui.separator();
            ui.label(egui::RichText::new(&app.info.model_name).size(13.0).color(TEXT_SECONDARY));
            ui.label(egui::RichText::new(format!(
                "{}c  •  {}", app.info.cpu_count, app.info.driver
            )).size(13.0).color(TEXT_DIM));

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button(egui::RichText::new("~ Refresh").size(13.0)).clicked() {
                    app.refresh();
                    app.sync_from_info();
                    app.status_msg = "Refreshed from system.".into();
                    app.status_is_err = false;
                }
            });
        });
    });

    // Status bar
    egui::TopBottomPanel::bottom("status").frame(
        Frame::new().fill(BG_PANEL).inner_margin(Margin::same(8))
    ).show(ctx, |ui| {
        if !app.drifts.is_empty() {
            ui.horizontal_wrapped(|ui| {
                ui.label(egui::RichText::new("!! Settings not applied:")
                    .size(12.0).color(ORANGE).strong());
                for d in &app.drifts {
                    ui.label(egui::RichText::new(
                        format!("{}: {} -> sys: {}", d.label, d.expected, d.actual)
                    ).size(12.0).color(ORANGE));
                }
            });
        }
        ui.horizontal(|ui| {
            if !app.status_msg.is_empty() {
                let color = if app.status_is_err { RED } else { GREEN };
                let icon = if app.status_is_err { "x" } else { "v" };
                ui.label(egui::RichText::new(format!("{icon} {}", app.status_msg))
                    .size(13.0).color(color));
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(egui::RichText::new(format!(
                    "HW range: {} - {}", fmt_khz(app.info.hw_min_khz), fmt_khz(app.info.hw_max_khz)
                )).size(12.0).color(TEXT_DIM));
            });
        });
    });

    egui::CentralPanel::default().frame(
        Frame::new().fill(BG_DARK).inner_margin(Margin::same(16))
    ).show(ctx, |ui| {
        egui::ScrollArea::vertical().show(ui, |ui| {
            match app.tab {
                Tab::Configurator => {
                    ui.columns(2, |cols| {
                        cols[0].vertical(|ui| { draw_live_cores(ui, app); });
                        cols[1].vertical(|ui| { draw_controls(ui, app); });
                    });
                    ui.add_space(12.0);
                    draw_cpu_plot(ui, app);
                }
                Tab::Sensors => {
                    draw_sensors_tab(ui, app);
                }
            }
        });
    });
}

fn draw_live_cores(ui: &mut egui::Ui, app: &CpuTweaksApp) {
    card_frame().show(ui, |ui| {
        section_label(ui, ">>", "Live CPU Frequencies");

        let max_khz = app.info.hw_max_khz as f32;
        for core in &app.info.cores {
            let pct = if max_khz > 0.0 { core.cur_freq_khz as f32 / max_khz } else { 0.0 };
            let color = freq_color(pct);

            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(format!("CPU{:>2}", core.id))
                    .size(12.0).color(TEXT_SECONDARY).monospace());

                let avail = ui.available_width() - 90.0;
                let bar_width = (avail * pct).max(2.0);
                let (rect, _) = ui.allocate_exact_size(
                    Vec2::new(avail, 14.0), egui::Sense::hover()
                );
                ui.painter().rect_filled(rect, CornerRadius::same(3),
                    Color32::from_rgb(30, 30, 42));
                let mut fill_rect = rect;
                fill_rect.set_width(bar_width);
                ui.painter().rect_filled(fill_rect, CornerRadius::same(3), color);

                ui.label(egui::RichText::new(fmt_khz(core.cur_freq_khz))
                    .size(11.0).color(TEXT_SECONDARY).monospace());
            });
        }
    });

    ui.add_space(8.0);

    card_frame().show(ui, |ui| {
        section_label(ui, "--", "Current State");
        let core0 = app.info.cores.first();
        let items: Vec<(&str, String)> = vec![
            ("Governor", core0.map(|c| c.governor.clone()).unwrap_or_default()),
            ("EPP", core0.and_then(|c| c.epp.clone()).unwrap_or("n/a".into())),
            ("Turbo/Boost", if app.info.turbo_supported {
                if app.info.turbo_enabled { "Enabled".into() } else { "Disabled".into() }
            } else { "Not supported".into() }),
            ("Scaling Min", core0.map(|c| fmt_khz(c.scaling_min_khz)).unwrap_or_default()),
            ("Scaling Max", core0.map(|c| fmt_khz(c.scaling_max_khz)).unwrap_or_default()),
        ];
        for (label, val) in &items {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(format!("{label}:")).size(13.0).color(TEXT_DIM));
                ui.label(egui::RichText::new(val).size(13.0).color(TEXT_PRIMARY).strong());
            });
        }
        if let Some(pct) = app.info.min_perf_pct {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Min Perf %:").size(13.0).color(TEXT_DIM));
                ui.label(egui::RichText::new(format!("{pct}%")).size(13.0).color(TEXT_PRIMARY).strong());
            });
        }
        if let Some(pct) = app.info.max_perf_pct {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Max Perf %:").size(13.0).color(TEXT_DIM));
                ui.label(egui::RichText::new(format!("{pct}%")).size(13.0).color(TEXT_PRIMARY).strong());
            });
        }
    });
}

fn draw_controls(ui: &mut egui::Ui, app: &mut CpuTweaksApp) {
    // Governor
    card_frame().show(ui, |ui| {
        section_label(ui, "G:", "Governor");
        ui.horizontal_wrapped(|ui| {
            for (i, gov) in app.info.available_governors.iter().enumerate() {
                let selected = i == app.sel_governor;
                let text = egui::RichText::new(gov).size(13.0)
                    .color(if selected { Color32::WHITE } else { TEXT_SECONDARY });
                let btn = egui::Button::new(text)
                    .fill(if selected { ACCENT } else { BG_PANEL })
                    .corner_radius(CornerRadius::same(6));
                if ui.add(btn).clicked() {
                    app.sel_governor = i;
                }
            }
        });
        ui.add_space(2.0);
        ui.label(egui::RichText::new(governor_desc(
            app.info.available_governors.get(app.sel_governor).map(|s| s.as_str()).unwrap_or("")
        )).size(11.0).color(TEXT_DIM).italics());
    });

    ui.add_space(6.0);

    // EPP
    if !app.info.available_epp.is_empty() {
        card_frame().show(ui, |ui| {
            section_label(ui, "E:", "Energy Performance Preference");
            ui.horizontal_wrapped(|ui| {
                for (i, epp) in app.info.available_epp.iter().enumerate() {
                    let selected = i == app.sel_epp;
                    let text = egui::RichText::new(epp).size(13.0)
                        .color(if selected { Color32::WHITE } else { TEXT_SECONDARY });
                    let btn = egui::Button::new(text)
                        .fill(if selected { ACCENT } else { BG_PANEL })
                        .corner_radius(CornerRadius::same(6));
                    if ui.add(btn).clicked() {
                        app.sel_epp = i;
                    }
                }
            });
        });
        ui.add_space(6.0);
    }

    // Frequency limits
    card_frame().show(ui, |ui| {
        section_label(ui, "F:", "Frequency Limits");
        let hw_min = app.info.hw_min_khz as f32 / 1000.0;
        let hw_max = app.info.hw_max_khz as f32 / 1000.0;

        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Min:").size(13.0).color(TEXT_SECONDARY));
            ui.add(egui::Slider::new(&mut app.min_freq_mhz, hw_min..=hw_max)
                .suffix(" MHz").step_by(100.0).custom_formatter(|v, _| fmt_mhz(v as f32)));
        });
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Max:").size(13.0).color(TEXT_SECONDARY));
            ui.add(egui::Slider::new(&mut app.max_freq_mhz, hw_min..=hw_max)
                .suffix(" MHz").step_by(100.0).custom_formatter(|v, _| fmt_mhz(v as f32)));
        });

        if app.min_freq_mhz > app.max_freq_mhz {
            app.min_freq_mhz = app.max_freq_mhz;
        }
    });

    ui.add_space(6.0);

    // Turbo / Boost
    if app.info.turbo_supported {
        card_frame().show(ui, |ui| {
            section_label(ui, "T:", "Turbo Boost");
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Turbo:").size(13.0).color(TEXT_SECONDARY));
                let label = if app.turbo { "Enabled" } else { "Disabled" };
                let color = if app.turbo { GREEN } else { TEXT_DIM };
                if ui.add(egui::Button::new(
                    egui::RichText::new(label).size(13.0).color(Color32::WHITE)
                ).fill(color).corner_radius(CornerRadius::same(6))).clicked() {
                    app.turbo = !app.turbo;
                }
            });
        });
        ui.add_space(6.0);
    }

    // Intel pstate specific
    if app.info.min_perf_pct.is_some() {
        card_frame().show(ui, |ui| {
            section_label(ui, "P:", "Intel P-State Tuning");
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Min Perf %:").size(13.0).color(TEXT_SECONDARY));
                ui.add(egui::Slider::new(&mut app.min_perf_pct, 0.0..=100.0)
                    .suffix("%").step_by(1.0));
            });
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Max Perf %:").size(13.0).color(TEXT_SECONDARY));
                ui.add(egui::Slider::new(&mut app.max_perf_pct, 0.0..=100.0)
                    .suffix("%").step_by(1.0));
            });
            if app.min_perf_pct > app.max_perf_pct {
                app.min_perf_pct = app.max_perf_pct;
            }
            if app.info.hwp_dynamic_boost.is_some() {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("HWP Dynamic Boost:").size(13.0).color(TEXT_SECONDARY));
                    ui.checkbox(&mut app.hwp_dynamic_boost, "");
                });
            }
        });
        ui.add_space(6.0);
    }

    // Apply button
    ui.add_space(8.0);
    ui.horizontal(|ui| {
        let apply_btn = egui::Button::new(
            egui::RichText::new("✓  Apply Changes").size(16.0).strong().color(Color32::WHITE)
        ).fill(ACCENT).corner_radius(CornerRadius::same(8)).min_size(Vec2::new(200.0, 40.0));

        if ui.add(apply_btn).clicked() {
            app.apply_changes();
        }

        ui.add_space(8.0);

        let reset_btn = egui::Button::new(
            egui::RichText::new("↺  Reset to Current").size(13.0).color(TEXT_SECONDARY)
        ).fill(BG_PANEL).corner_radius(CornerRadius::same(8));

        if ui.add(reset_btn).clicked() {
            app.refresh();
            app.sync_from_info();
            app.status_msg = "Reset to current system state.".into();
            app.status_is_err = false;
        }
    });
}

fn governor_desc(gov: &str) -> &'static str {
    match gov {
        "performance" => "Run all CPUs at maximum frequency. Best for benchmarks and low-latency workloads.",
        "powersave" => "Let the CPU scale dynamically. With intel_pstate, EPP controls the bias.",
        "ondemand" => "Scale frequency based on CPU load. Classic Linux governor.",
        "conservative" => "Like ondemand but ramps up gradually. Smoother power transitions.",
        "schedutil" => "Scheduler-driven frequency scaling. Modern and efficient.",
        "userspace" => "Manual frequency control. Requires explicit frequency setting.",
        _ => "",
    }
}

fn draw_cpu_plot(ui: &mut egui::Ui, app: &mut CpuTweaksApp) {
    use egui_plot::{Line, Plot, PlotPoints};

    // Shared time window selector
    card_frame().show(ui, |ui| {
        ui.horizontal(|ui| {
            section_label(ui, "~", "CPU Monitor");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if let Some(last) = app.history.back() {
                    ui.label(egui::RichText::new(format!(
                        "CPU: {:.0}%  Freq: {:.0} MHz", last.cpu_util_pct, last.avg_freq_mhz
                    )).size(12.0).color(TEXT_SECONDARY).monospace());
                }
                ui.separator();
                for &(label, secs) in &[("1m", 60.0), ("2m", 120.0), ("5m", 300.0), ("15m", 900.0), ("1h", 3600.0)] {
                    let sel = (app.history_window_secs - secs).abs() < 1.0;
                    let text = egui::RichText::new(label).size(11.0)
                        .color(if sel { Color32::WHITE } else { TEXT_DIM });
                    if ui.add(egui::Button::new(text)
                        .fill(if sel { ACCENT_DIM } else { BG_PANEL })
                        .corner_radius(CornerRadius::same(4))
                    ).clicked() {
                        app.history_window_secs = secs;
                    }
                }
            });
        });

        if app.history.len() < 2 {
            ui.label(egui::RichText::new("Collecting data...").size(12.0).color(TEXT_DIM));
            return;
        }

        let now = app.app_start.elapsed().as_secs_f64();
        let t_min = now - app.history_window_secs;

        let util_points: PlotPoints = app.history.iter()
            .filter(|s| s.time_secs >= t_min)
            .map(|s| [s.time_secs - now, s.cpu_util_pct])
            .collect();
        let freq_max = app.info.hw_max_khz as f64 / 1000.0;
        let freq_points: PlotPoints = app.history.iter()
            .filter(|s| s.time_secs >= t_min)
            .map(|s| [s.time_secs - now, (s.avg_freq_mhz / freq_max) * 100.0])
            .collect();

        Plot::new("cpu_plot")
            .height(150.0)
            .include_y(0.0).include_y(100.0)
            .include_x(-app.history_window_secs).include_x(0.0)
            .allow_drag(false).allow_zoom(false).allow_scroll(false)
            .x_axis_label("seconds ago").y_axis_label("%")
            .legend(egui_plot::Legend::default())
            .show(ui, |plot_ui| {
                plot_ui.line(Line::new(util_points).name("CPU %").color(ACCENT).width(2.0));
                plot_ui.line(Line::new(freq_points).name("Freq %").color(GREEN).width(1.5));
            });
    });
}

fn draw_sensors_tab(ui: &mut egui::Ui, app: &mut CpuTweaksApp) {
    use egui_plot::{Line, Plot, PlotPoints};
    use super::app::sensor_key;

    let s = &app.sensors;

    // Top row: sensor readings + fans + battery
    ui.columns(3, |cols| {
        // Column 1: Temperatures
        cols[0].vertical(|ui| {
            card_frame().show(ui, |ui| {
                section_label(ui, "^", "Temperatures");
                if !s.temps.is_empty() {
                    let mut last_hwmon = String::new();
                    for t in &s.temps {
                        if t.temp_mc == 0 { continue; }
                        if t.hwmon != last_hwmon {
                            ui.label(egui::RichText::new(&t.hwmon).size(11.0).color(ACCENT).strong());
                            last_hwmon = t.hwmon.clone();
                        }
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new(format!("  {}:", t.label)).size(12.0).color(TEXT_DIM));
                            let temp = t.temp_c();
                            let color = if temp > 85.0 { RED } else if temp > 70.0 { ORANGE } else { GREEN };
                            ui.label(egui::RichText::new(format!("{temp:.0}C")).size(12.0).color(color).strong());
                            if let Some(crit) = t.crit_c() {
                                ui.label(egui::RichText::new(format!("(crit {crit:.0})")).size(10.0).color(TEXT_DIM));
                            }
                        });
                    }
                } else {
                    ui.label(egui::RichText::new("No temperature sensors found").size(12.0).color(TEXT_DIM));
                }
            });
        });

        // Column 2: Fans
        cols[1].vertical(|ui| {
            card_frame().show(ui, |ui| {
                section_label(ui, ">", "Fans");
                if !s.fans.is_empty() {
                    for f in &s.fans {
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new(format!("{}:", f.label)).size(12.0).color(TEXT_DIM));
                            match f.rpm {
                                Some(rpm) => { ui.label(egui::RichText::new(format!("{rpm} RPM")).size(12.0).color(TEXT_PRIMARY)); }
                                None => { ui.label(egui::RichText::new("N/A").size(12.0).color(TEXT_DIM)); }
                            };
                        });
                    }
                } else {
                    ui.label(egui::RichText::new("No fan sensors found").size(12.0).color(TEXT_DIM));
                }
                for fc in &s.fan_controls {
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new(format!("{} mode:", fc.hwmon)).size(12.0).color(TEXT_DIM));
                        let mode = match fc.enable_mode { 0 => "Full Speed", 1 => "Manual", 2 => "Auto", _ => "Unknown" };
                        ui.label(egui::RichText::new(mode).size(12.0).color(TEXT_SECONDARY));
                        if fc.enable_mode == 1 {
                            ui.label(egui::RichText::new(format!("PWM {}/255", fc.pwm_value)).size(11.0).color(TEXT_DIM));
                        }
                    });
                }
            });
        });

        // Column 3: Battery + summary
        cols[2].vertical(|ui| {
            card_frame().show(ui, |ui| {
                section_label(ui, "=", "Power");
                if let Some(bat) = &s.battery {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Battery:").size(12.0).color(TEXT_DIM));
                        let color = if bat.capacity_pct < 20 { RED } else if bat.capacity_pct < 50 { ORANGE } else { GREEN };
                        ui.label(egui::RichText::new(format!("{}%", bat.capacity_pct)).size(14.0).color(color).strong());
                    });
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Status:").size(12.0).color(TEXT_DIM));
                        ui.label(egui::RichText::new(&bat.status).size(12.0).color(TEXT_PRIMARY));
                    });
                    if let Some(w) = bat.power_watts {
                        if w > 0.1 {
                            ui.horizontal(|ui| {
                                ui.label(egui::RichText::new("Power:").size(12.0).color(TEXT_DIM));
                                ui.label(egui::RichText::new(format!("{w:.1} W")).size(12.0).color(TEXT_PRIMARY));
                            });
                        }
                    }
                } else {
                    ui.label(egui::RichText::new("No battery detected").size(12.0).color(TEXT_DIM));
                }
            });
        });
    });

    ui.add_space(12.0);

    // CPU plot (same as configurator tab)
    draw_cpu_plot(ui, app);

    // Temperature plot
    if app.temp_history.is_empty() { return; }

    ui.add_space(8.0);
    card_frame().show(ui, |ui| {
        ui.horizontal(|ui| {
            section_label(ui, "^", "Temperature History");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                for key in app.selected_temps.iter().rev() {
                    if let Some(q) = app.temp_history.get(key) {
                        if let Some(&(_, t)) = q.back() {
                            let short = key.split('/').last().unwrap_or(key);
                            let color = if t > 85.0 { RED } else if t > 70.0 { ORANGE } else { GREEN };
                            ui.label(egui::RichText::new(format!("{short}: {t:.0}C"))
                                .size(11.0).color(color).monospace());
                        }
                    }
                }
            });
        });

        // Sensor selector
        ui.horizontal_wrapped(|ui| {
            let all_keys: Vec<String> = app.sensors.temps.iter()
                .filter(|t| t.temp_mc != 0)
                .map(|t| sensor_key(&t.hwmon, &t.label))
                .collect();
            for key in &all_keys {
                let selected = app.selected_temps.contains(key);
                let short = key.split('/').last().unwrap_or(key);
                let text = egui::RichText::new(short).size(10.0)
                    .color(if selected { Color32::WHITE } else { TEXT_DIM });
                if ui.add(egui::Button::new(text)
                    .fill(if selected { ACCENT_DIM } else { BG_PANEL })
                    .corner_radius(CornerRadius::same(3))
                ).clicked() {
                    if selected { app.selected_temps.remove(key); }
                    else { app.selected_temps.insert(key.clone()); }
                }
            }
        });

        if app.selected_temps.is_empty() {
            ui.label(egui::RichText::new("Select sensors above to plot").size(11.0).color(TEXT_DIM));
            return;
        }

        let now = app.app_start.elapsed().as_secs_f64();
        let t_min = now - app.history_window_secs;
        let colors = [
            Color32::from_rgb(244, 67, 54), Color32::from_rgb(255, 152, 0),
            Color32::from_rgb(255, 235, 59), Color32::from_rgb(76, 175, 80),
            Color32::from_rgb(0, 188, 212), Color32::from_rgb(156, 39, 176),
            Color32::from_rgb(233, 30, 99), Color32::from_rgb(121, 85, 72),
        ];

        Plot::new("temp_plot")
            .height(200.0)
            .include_y(20.0).include_y(100.0)
            .include_x(-app.history_window_secs).include_x(0.0)
            .allow_drag(false).allow_zoom(false).allow_scroll(false)
            .x_axis_label("seconds ago").y_axis_label("C")
            .legend(egui_plot::Legend::default())
            .show(ui, |plot_ui| {
                for (i, key) in app.selected_temps.iter().enumerate() {
                    if let Some(q) = app.temp_history.get(key) {
                        let points: PlotPoints = q.iter()
                            .filter(|(t, _)| *t >= t_min)
                            .map(|(t, temp)| [t - now, *temp])
                            .collect();
                        let short = key.split('/').last().unwrap_or(key);
                        plot_ui.line(Line::new(points).name(short).color(colors[i % colors.len()]).width(1.5));
                    }
                }
            });
    });
}
