#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]
mod app;
mod automation;
mod canvas;
mod graph;
mod menus;
mod paint;
mod runtime;
mod state;
mod text;
mod ui;

fn main() -> eframe::Result {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let data = args
        .iter()
        .position(|a| a == "--data-dir")
        .and_then(|i| args.get(i + 1))
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| runtime::portable_root().expect("Cannot locate executable"));
    if let Err(error) = runtime::prepare(&data) {
        rfd::MessageDialog::new()
            .set_title("RepoTower")
            .set_description(&error)
            .set_level(rfd::MessageLevel::Error)
            .show();
        return Ok(());
    }
    let automation = if args.iter().any(|a| a == "--automation-stdio") {
        Some(automation::Automation::start())
    } else {
        None
    };
    let project = args
        .iter()
        .position(|a| a == "--project")
        .and_then(|i| args.get(i + 1))
        .map(std::path::PathBuf::from);
    let demo = args.iter().any(|a| a == "--demo");
    let icon =
        eframe::icon_data::from_png_bytes(include_bytes!("../../../desktop/assets/repotower.png"))
            .ok();
    let mut viewport = eframe::egui::ViewportBuilder::default()
        .with_title("RepoTower")
        .with_app_id("io.github.cabal312512.RepoTower")
        .with_inner_size([980.0, 680.0])
        .with_min_inner_size([760.0, 520.0])
        .with_decorations(false);
    if let Some(icon) = icon {
        viewport = viewport.with_icon(icon);
    }
    let options = eframe::NativeOptions {
        viewport,
        renderer: eframe::Renderer::Wgpu,
        persist_window: false,
        ..Default::default()
    };
    eframe::run_native(
        "RepoTower",
        options,
        Box::new(move |cc| {
            cc.egui_ctx.set_fonts(runtime::fonts()?);
            let mut app = app::App::new(data, automation);
            if let Some(project) = project {
                app.open(project, &cc.egui_ctx);
            } else if demo {
                app.open_example("demo-project", &cc.egui_ctx);
            }
            Ok(Box::new(app))
        }),
    )
}
