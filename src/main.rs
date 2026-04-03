mod cpu;
mod gui;

extern crate gtk;

fn main() -> anyhow::Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn")).init();
    gtk::init().map_err(|e| anyhow::anyhow!("GTK init failed: {e}"))?;

    let icon_data = load_app_icon_rgba();
    let mut viewport = egui::ViewportBuilder::default()
        .with_inner_size([960.0, 640.0])
        .with_title("CPU Tweaks")
        .with_min_inner_size([700.0, 500.0]);
    if let Some((rgba, w, h)) = icon_data {
        viewport = viewport.with_icon(egui::IconData { rgba, width: w, height: h });
    }

    eframe::run_native(
        "CPU Tweaks",
        eframe::NativeOptions { viewport, ..Default::default() },
        Box::new(|_cc| Ok(Box::new(gui::app::CpuTweaksApp::new()))),
    ).map_err(|e| anyhow::anyhow!("eframe error: {e}"))
}

fn load_app_icon_rgba() -> Option<(Vec<u8>, u32, u32)> {
    let svg_bytes = include_bytes!("../assets/cpu-tweaks.svg");
    let tree = resvg::usvg::Tree::from_data(svg_bytes, &Default::default()).ok()?;
    let size = tree.size();
    let (w, h) = (size.width() as u32, size.height() as u32);
    let mut pixmap = resvg::tiny_skia::Pixmap::new(w, h)?;
    resvg::render(&tree, Default::default(), &mut pixmap.as_mut());
    // Convert RGBA premultiplied to straight RGBA
    let mut data = pixmap.take();
    for px in data.chunks_exact_mut(4) {
        let a = px[3] as f32 / 255.0;
        if a > 0.0 {
            px[0] = (px[0] as f32 / a).min(255.0) as u8;
            px[1] = (px[1] as f32 / a).min(255.0) as u8;
            px[2] = (px[2] as f32 / a).min(255.0) as u8;
        }
    }
    Some((data, w, h))
}
