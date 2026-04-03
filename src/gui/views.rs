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
        ui.label(egui::RichText::new(icon).size(18.0).color(ACCENT));
        ui.label(egui::RichText::new(text).size(16.0).strong().color(TEXT_PRIMARY));
    });
    ui.add_space(4.0);
}

pub fn draw(ctx: &egui::Context, app: &mut CpuTweaksApp) {
    egui::TopBottomPanel::top("header").frame(
        Frame::new().fill(BG_PANEL).inner_margin(Margin::same(12))
    ).show(ctx, |ui| {
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("⚙").size(24.0).color(ACCENT));
            ui.label(egui::RichText::new("CPU Tweaks").size(22.0).strong().color(TEXT_PRIMARY));
            ui.separator();
            ui.label(egui::RichText::new(&app.info.model_name).size(13.0).color(TEXT_SECONDARY));
            ui.separator();
            ui.label(egui::RichText::new(format!(
                "{} cores  •  Driver: {}",
                app.info.cpu_count, app.info.driver
            )).size(13.0).color(TEXT_DIM));

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button(egui::RichText::new("🔄 Refresh").size(13.0)).clicked() {
                    app.refresh();
                    app.sync_from_info();
                    app.status_msg = "Refreshed from system.".into();
                    app.status_is_err = false;
                }
            });
        });
    });

    // Status bar at bottom
    egui::TopBottomPanel::bottom("status").frame(
        Frame::new().fill(BG_PANEL).inner_margin(Margin::same(8))
    ).show(ctx, |ui| {
        // Drift warnings
        if !app.drifts.is_empty() {
            ui.horizontal_wrapped(|ui| {
                ui.label(egui::RichText::new("⚠ Settings not applied:")
                    .size(12.0).color(ORANGE).strong());
                for d in &app.drifts {
                    ui.label(egui::RichText::new(
                        format!("{}: {} → sys: {}", d.label, d.expected, d.actual)
                    ).size(12.0).color(ORANGE));
                    ui.label(egui::RichText::new("│").size(12.0).color(TEXT_DIM));
                }
            });
        }
        ui.horizontal(|ui| {
            if !app.status_msg.is_empty() {
                let color = if app.status_is_err { RED } else { GREEN };
                let icon = if app.status_is_err { "✗" } else { "✓" };
                ui.label(egui::RichText::new(format!("{icon} {}", app.status_msg))
                    .size(13.0).color(color));
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(egui::RichText::new(format!(
                    "HW range: {} – {}",
                    fmt_khz(app.info.hw_min_khz), fmt_khz(app.info.hw_max_khz)
                )).size(12.0).color(TEXT_DIM));
            });
        });
    });

    egui::CentralPanel::default().frame(
        Frame::new().fill(BG_DARK).inner_margin(Margin::same(16))
    ).show(ctx, |ui| {
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.columns(2, |cols| {
                cols[0].vertical(|ui| {
                    draw_live_cores(ui, app);
                });
                cols[1].vertical(|ui| {
                    draw_controls(ui, app);
                });
            });
        });
    });
}

fn draw_live_cores(ui: &mut egui::Ui, app: &CpuTweaksApp) {
    card_frame().show(ui, |ui| {
        section_label(ui, "📊", "Live CPU Frequencies");

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
        section_label(ui, "ℹ️", "Current State");
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
        section_label(ui, "🏛", "Governor");
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
            section_label(ui, "⚡", "Energy Performance Preference");
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
        section_label(ui, "📶", "Frequency Limits");
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
            section_label(ui, "🚀", "Turbo Boost");
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
            section_label(ui, "🔧", "Intel P-State Tuning");
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
