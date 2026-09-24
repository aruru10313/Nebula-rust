mod core;
mod minecraft;
mod ui;

use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "nebulya_launcher=debug,info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1180.0, 760.0])
            .with_min_inner_size([980.0, 620.0])
            .with_title("Nebulya Launcher")
            .with_app_id("nebulya-launcher"),
        ..Default::default()
    };

    eframe::run_native(
        "Nebulya Launcher",
        options,
        Box::new(|cc| Ok(Box::new(ui::NebulyaApp::new(cc)))),
    )
    .map_err(|e| anyhow::anyhow!("eframe error: {e}"))?;

    Ok(())
}
