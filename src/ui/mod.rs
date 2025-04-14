mod app;
mod log;
mod theme;

use std::sync::Arc;

use anyhow::Result;
use app::App;
use flume::Receiver;
use theme::apply_theme;

use crate::context::Context;
use crate::context::ContextNotification;

pub use app::ProjectDescription;

pub fn run_ui(
    context: Context,
    receiver: Receiver<ContextNotification>,
    project_descriptions: Vec<ProjectDescription>,
) -> Result<()> {
    let base_width = 800.0;
    let base_height = 600.0;

    let d = eframe::icon_data::from_png_bytes(include_bytes!("../../assets/icon.png"))
        .expect("The icon data must be valid");
    let mut options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size(egui::vec2(base_width, base_height))
            .with_min_inner_size(egui::vec2(base_width, base_height)),
        ..Default::default()
    };
    options.viewport.icon = Some(Arc::new(d));

    let app_factory = Box::new(|cc: &eframe::CreationContext<'_>| {
        apply_theme(&cc.egui_ctx);

        let app = App::new(context, receiver, project_descriptions).with_creation_context(cc);

        Ok(Box::new(app) as Box<dyn eframe::App>)
    });

    eframe::run_native("Cursor Rust Tools", options, app_factory)
        .map_err(|e| anyhow::anyhow!("Failed to run eframe: {}", e))
}
