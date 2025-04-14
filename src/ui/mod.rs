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
pub use app::Settings;

pub fn run_ui(
    context: Context,
    receiver: Receiver<ContextNotification>,
    project: Vec<ProjectDescription>,
    settings: Settings,
) -> Result<()> {
    let width = 800.0 * settings.scale;
    let height = 600.0 * settings.scale;

    let scale = settings.scale;

    let scale = if scale >= 0.5 && scale <= 2.5 {
        scale
    } else {
        1.0
    };

    let d = eframe::icon_data::from_png_bytes(include_bytes!("../../assets/icon.png"))
        .expect("The icon data must be valid");

    let viewport = egui::ViewportBuilder::default()
        .with_inner_size(egui::vec2(width, height))
        .with_min_inner_size(egui::vec2(width, height));

    let mut options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };
    options.viewport.icon = Some(Arc::new(d));

    let scale_for_closure = scale;
    let context_for_closure = context.clone();
    let app_factory = Box::new(move |cc: &eframe::CreationContext<'_>| {
        apply_theme(&cc.egui_ctx);

        let app = App::new(context_for_closure, receiver, project, settings);

        cc.egui_ctx.set_pixels_per_point(scale_for_closure);

        Ok(Box::new(app) as Box<dyn eframe::App>)
    });

    eframe::run_native("Cursor Rust Tools", options, app_factory)
        .map_err(|e| anyhow::anyhow!("Failed to run eframe: {}", e))
}
