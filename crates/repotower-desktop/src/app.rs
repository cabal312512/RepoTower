use crate::{
    automation::Automation,
    graph::{Camera, Layout, View, WAVE_SECONDS},
    paint::*,
    runtime::{self, Preferences},
    state::Simulation,
    text::{self, tr},
};
use eframe::egui::*;
use repotower_core::model::{ImpactResult, RepositoryAnalysis, ScanProgress};
use serde_json::{json, Value};
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{
        mpsc::{self, Receiver, Sender},
        Arc,
    },
    time::{Duration, Instant},
};

pub enum Work {
    Progress(u64, ScanProgress),
    Complete(u64, Result<RepositoryAnalysis, String>),
    Folder(Option<PathBuf>),
}
#[derive(Clone, PartialEq)]
pub enum Popup {
    Settings,
    Examples,
    Help,
    Positions,
    Reports,
    Search,
    Critical,
    Relations(String, bool),
    Unresolved(String),
    Warnings,
    Licenses,
}
pub struct Drag {
    pub start: Pos2,
    pub last: Pos2,
    pub button: PointerButton,
    pub node: Option<String>,
    pub moved: bool,
}

pub struct App {
    pub data: PathBuf,
    pub prefs: Preferences,
    pub analysis: Option<Arc<RepositoryAnalysis>>,
    pub simulation: Simulation,
    pub layout: Layout,
    pub layout_dirty: bool,
    pub preview: Option<ImpactResult>,
    pub view: View,
    pub neighbor: Option<String>,
    pub offsets: HashMap<String, Vec2>,
    pub camera: Camera,
    pub cameras: HashMap<String, Camera>,
    pub camera_key: String,
    pub graph_size: Vec2,
    pub needs_fit: bool,
    pub z_order: Vec<String>,
    pub playhead: f32,
    pub playing: bool,
    pub last_time: f64,
    pub hovered: Option<String>,
    pub hover_candidate: Option<String>,
    pub hover_since: f64,
    pub drag: Option<Drag>,
    pub last_node_click: Option<(String, f64)>,
    pub controls: Controls,
    pub popup: Option<Popup>,
    pub popup_rect: Rect,
    pub opened_popup: bool,
    pub search: String,
    pub error: Option<String>,
    pub loading: bool,
    pub progress: Option<ScanProgress>,
    pub folder_dialog: bool,
    generation: u64,
    sender: Sender<Work>,
    receiver: Receiver<Work>,
    pub automation: Option<Automation>,
}
impl App {
    pub fn new(data: PathBuf, automation: Option<Automation>) -> Self {
        let prefs = Preferences::load(&data);
        let (sender, receiver) = mpsc::channel();
        Self {
            data,
            prefs,
            analysis: None,
            simulation: Simulation::default(),
            layout: Layout::default(),
            layout_dirty: true,
            preview: None,
            view: View::All,
            neighbor: None,
            offsets: HashMap::new(),
            camera: Camera::default(),
            cameras: HashMap::new(),
            camera_key: String::new(),
            graph_size: Vec2::ZERO,
            needs_fit: true,
            z_order: Vec::new(),
            playhead: 0.0,
            playing: false,
            last_time: 0.0,
            hovered: None,
            hover_candidate: None,
            hover_since: 0.0,
            drag: None,
            last_node_click: None,
            controls: Controls::new(),
            popup: None,
            popup_rect: Rect::NOTHING,
            opened_popup: false,
            search: String::new(),
            error: None,
            loading: false,
            progress: None,
            folder_dialog: false,
            generation: 0,
            sender,
            receiver,
            automation,
        }
    }
    pub fn palette(&self) -> Palette {
        Palette::new(self.prefs.theme == "dark")
    }
    pub fn t(&self, key: &str) -> &'static str {
        tr(&self.prefs.language, key)
    }
    pub fn save_preferences(&mut self) {
        if let Err(e) = self.prefs.save(&self.data) {
            self.error = Some(e);
        }
    }
    pub fn close_project(&mut self) {
        self.generation += 1;
        self.analysis = None;
        self.loading = false;
        self.progress = None;
        self.simulation.reset();
        self.layout = Layout::default();
        self.view = View::All;
        self.neighbor = None;
        self.offsets.clear();
        self.cameras.clear();
        self.camera_key.clear();
        self.needs_fit = true;
        self.layout_dirty = true;
        self.search.clear();
        self.popup = None;
        self.playing = false;
        self.hovered = None;
        self.hover_candidate = None;
        self.drag = None;
        self.z_order.clear();
    }
    pub fn open(&mut self, path: PathBuf, ctx: &Context) {
        self.close_project();
        self.error = None;
        self.loading = true;
        let generation = self.generation;
        let sender = self.sender.clone();
        let context = ctx.clone();
        std::thread::spawn(move || {
            let mut last = Instant::now();
            let mut stage = String::new();
            let result = repotower_core::analyze(&path, |progress| {
                if last.elapsed() > Duration::from_millis(40) || stage != progress.stage {
                    stage = progress.stage.clone();
                    last = Instant::now();
                    let _ = sender.send(Work::Progress(generation, progress));
                    context.request_repaint();
                }
            });
            let _ = sender.send(Work::Complete(generation, result));
            context.request_repaint();
        });
    }
    pub fn open_example(&mut self, name: &str, ctx: &Context) {
        match runtime::example(&self.data, name) {
            Ok(path) => self.open(path, ctx),
            Err(e) => self.error = Some(e),
        }
    }
    pub fn pick_folder(&mut self, ctx: &Context) {
        if self.folder_dialog {
            return;
        }
        self.folder_dialog = true;
        self.popup = None;
        let context = ctx.clone();
        let sender = self.sender.clone();
        let title = self.t("open");
        std::thread::spawn(move || {
            let path = rfd::FileDialog::new().set_title(title).pick_folder();
            let _ = sender.send(Work::Folder(path));
            context.request_repaint();
        });
    }
    fn receive(&mut self, ctx: &Context) {
        while let Ok(work) = self.receiver.try_recv() {
            match work {
                Work::Progress(g, p) if g == self.generation => self.progress = Some(p),
                Work::Complete(g, result) if g == self.generation => {
                    self.loading = false;
                    self.progress = None;
                    match result {
                        Ok(a) => {
                            self.analysis = Some(Arc::new(a));
                            self.layout_dirty = true;
                            self.needs_fit = true;
                        }
                        Err(e) => self.error = Some(e),
                    }
                }
                Work::Folder(path) => {
                    self.folder_dialog = false;
                    if let Some(path) = path {
                        self.open(path, ctx);
                    }
                }
                _ => {}
            }
        }
    }
    pub fn toggle(&mut self, popup: Popup) {
        if self.popup.as_ref() == Some(&popup) {
            self.popup = None;
        } else {
            self.popup = Some(popup);
            self.opened_popup = true;
        }
    }
    pub fn select(&mut self, id: Option<String>) {
        if self.simulation.selected != id {
            self.simulation.selected = id;
            self.layout_dirty = true;
            self.hovered = None;
            self.hover_candidate = None;
        }
    }
    pub fn set_view(&mut self, view: View) {
        if view == View::Nearby {
            if self.simulation.selected.is_none() {
                return;
            }
            self.neighbor = self.simulation.selected.clone();
        }
        if view == View::Impact && self.simulation.report().is_none() {
            return;
        }
        self.view = view;
        self.layout_dirty = true;
        self.hovered = None;
        self.hover_candidate = None;
        self.popup = None;
    }
    pub fn disconnect(&mut self) {
        let Some(a) = self.analysis.clone() else {
            return;
        };
        let Some(id) = self.simulation.selected.clone() else {
            return;
        };
        if self.simulation.begin(&a, &id) {
            self.playhead = 0.0;
            self.playing = !self.prefs.reduced_motion;
            self.view = View::Impact;
            self.layout_dirty = true;
            self.popup = None;
            if self.prefs.reduced_motion {
                self.finish();
            }
        }
    }
    pub fn finish(&mut self) {
        self.playhead = self.max_wave();
        self.playing = false;
        self.simulation.finish();
        self.layout_dirty = true;
    }
    pub fn max_wave(&self) -> f32 {
        self.simulation
            .report()
            .map(|r| r.waves.len().saturating_sub(1))
            .unwrap_or(0) as f32
    }
    pub fn restore(&mut self, id: &str) {
        if let Some(a) = &self.analysis {
            if self.simulation.restore(a, id) {
                self.playhead = self.max_wave();
                self.playing = false;
                self.layout_dirty = true;
                if self.simulation.report().is_none() && self.view == View::Impact {
                    self.view = View::All;
                }
            }
        }
    }
    pub fn undo(&mut self) {
        if self.simulation.undo() {
            self.playing = false;
            self.playhead = self.max_wave();
            self.layout_dirty = true;
            if self.simulation.report().is_none() && self.view == View::Impact {
                self.view = View::All;
            }
        }
    }
    pub fn reset(&mut self) {
        self.simulation.reset();
        self.playing = false;
        self.playhead = 0.0;
        self.view = View::All;
        self.layout_dirty = true;
        self.needs_fit = true;
        self.cameras.clear();
    }
    fn animate(&mut self, ctx: &Context) {
        let now = ctx.input(|i| i.time);
        let delta = (now - self.last_time).clamp(0.0, 0.1) as f32;
        self.last_time = now;
        if self.playing {
            self.playhead = (self.playhead + delta / WAVE_SECONDS).min(self.max_wave());
            if self.playhead >= self.max_wave() {
                self.finish();
            } else {
                ctx.request_repaint_after(Duration::from_millis(16));
            }
        }
    }
    pub fn search_results(&self, critical: bool) -> Vec<String> {
        let Some(a) = &self.analysis else {
            return Vec::new();
        };
        if critical {
            return a.critical_modules.iter().take(16).cloned().collect();
        }
        let query = self.search.to_lowercase();
        let mut result: Vec<_> = a
            .modules
            .iter()
            .filter(|m| m.relative_path.to_lowercase().contains(&query))
            .collect();
        result.sort_by_key(|m| m.id.as_str());
        result.into_iter().take(80).map(|m| m.id.clone()).collect()
    }
    pub fn reveal_selected(&mut self, id: &str) {
        if self.view != View::All && !self.layout.nodes.iter().any(|n| n.id == id) {
            self.view = View::All;
            self.layout_dirty = true;
        }
        // Center explicit search/navigation results only when they lie outside the viewport.
        self.rebuild();
        if let Some(node) = self.layout.nodes.iter().find(|n| n.id == id) {
            let point = node.pos
                + self.offsets.get(id).copied().unwrap_or_default()
                + crate::graph::NODE_SIZE / 2.0;
            let screen = point.to_vec2() * self.camera.scale + self.camera.offset;
            if screen.x < 30.0
                || screen.x > self.graph_size.x - 30.0
                || screen.y < 60.0
                || screen.y > self.graph_size.y - 60.0
            {
                self.camera.offset = self.graph_size / 2.0 - point.to_vec2() * self.camera.scale;
            }
        }
    }
    fn shortcuts(&mut self, ctx: &Context) {
        if ctx.wants_keyboard_input() {
            return;
        }
        if ctx.input(|i| i.key_pressed(Key::R)) {
            self.needs_fit = true;
        }
        if ctx.input(|i| i.modifiers.command && i.key_pressed(Key::Z)) {
            self.undo();
        }
        if ctx.input(|i| i.modifiers.command && i.key_pressed(Key::O)) {
            self.pick_folder(ctx);
        }
        if ctx.input(|i| i.key_pressed(Key::Delete) || i.key_pressed(Key::Backspace)) {
            let id = self.simulation.selected.clone();
            if let Some(id) = id {
                if self.simulation.origins.contains(&id) {
                    self.restore(&id);
                } else {
                    self.disconnect();
                }
            }
        }
        if ctx.input(|i| i.key_pressed(Key::Space)) && self.simulation.report().is_some() {
            if self.playhead >= self.max_wave() {
                self.playhead = 0.0;
            }
            self.playing = !self.playing;
        }
    }
    pub fn snapshot(&self) -> Value {
        json!({"version":env!("CARGO_PKG_VERSION"),"renderer":"native-wgpu","theme":self.prefs.theme,"language":self.prefs.language,"dataDir":self.data,"loading":self.loading,"error":self.error,
            "repository":self.analysis.as_ref().map(|a|&a.repository_name),"modules":self.analysis.as_ref().map(|a|a.total_modules),"edgeCount":self.analysis.as_ref().map(|a|a.total_edges),
            "selected":self.simulation.selected,"hovered":self.hovered,"view":format!("{:?}",self.view),"camera":{"x":self.camera.offset.x,"y":self.camera.offset.y,"scale":self.camera.scale},
            "playing":self.playing,"playhead":self.playhead,"live":self.simulation.live.is_some(),"origins":self.simulation.origins,"unavailable":self.simulation.unavailable,"reports":self.simulation.reports,"report":self.simulation.report(),"controls":self.controls,
            "nodes":self.layout.nodes.iter().map(|n|{let pos=n.pos+if self.view==View::All{self.offsets.get(&n.id).copied().unwrap_or_default()}else{Vec2::ZERO};json!({"id":n.id,"x":pos.x,"y":pos.y,"wave":n.wave})}).collect::<Vec<_>>(),
            "edges":self.layout.edges.iter().map(|e|json!({"from":self.layout.nodes[e.from].id,"to":self.layout.nodes[e.to].id,"causal":e.causal})).collect::<Vec<_>>()})
    }
}
impl eframe::App for App {
    fn raw_input_hook(&mut self, ctx: &Context, input: &mut RawInput) {
        if let Some(automation) = &mut self.automation {
            if automation.pending.is_none() && automation.screenshot.is_none() {
                if let Ok(command) = automation.receiver.try_recv() {
                    Automation::inject(&command, input);
                    match command["action"].as_str().unwrap_or("") {
                        "resize" => ctx.send_viewport_cmd(ViewportCommand::InnerSize(vec2(
                            command["width"].as_f64().unwrap_or(980.0) as f32,
                            command["height"].as_f64().unwrap_or(680.0) as f32,
                        ))),
                        "screenshot" => {
                            automation.screenshot = Some(command.clone());
                            ctx.send_viewport_cmd(ViewportCommand::Screenshot(Default::default()));
                        }
                        "quit" => ctx.send_viewport_cmd(ViewportCommand::Close),
                        _ => {}
                    }
                    automation.pending = Some(command);
                }
            }
        }
    }
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        self.receive(ctx);
        self.style(ctx);
        self.animate(ctx);
        self.shortcuts(ctx);
        self.controls.clear();
        self.opened_popup = false;
        for path in ctx.input(|i| {
            i.raw
                .dropped_files
                .iter()
                .filter_map(|f| f.path.clone())
                .collect::<Vec<_>>()
        }) {
            if path.is_dir() {
                self.open(path, ctx);
                break;
            } else {
                self.error = Some(
                    match self.prefs.language.as_str() {
                        "zh" => "请拖入项目文件夹。",
                        "ja" => "プロジェクトのフォルダーをドロップしてください。",
                        _ => "Drop a project folder.",
                    }
                    .into(),
                );
            }
        }
        if ctx.input(|i| i.key_pressed(Key::Escape)) {
            self.popup = None;
            self.error = None;
            self.hovered = None;
            ctx.memory_mut(|m| m.surrender_focus(Id::new("search-field")));
        }
        let screen = ctx.content_rect();
        let p = self.palette();
        CentralPanel::default()
            .frame(Frame::new().fill(p.bg))
            .show(ctx, |ui| {
                self.header(
                    ui,
                    Rect::from_min_size(screen.min, vec2(screen.width(), 40.0)),
                    ctx,
                );
                let status = Rect::from_min_size(
                    pos2(screen.left(), screen.bottom() - 24.0),
                    vec2(screen.width(), 24.0),
                );
                if self.analysis.is_some() {
                    let bar = Rect::from_min_size(
                        screen.min + vec2(0.0, 40.0),
                        vec2(screen.width(), 44.0),
                    );
                    self.project_bar(ui, bar, ctx);
                    let dock = Rect::from_min_size(
                        pos2(screen.left(), status.top() - 82.0),
                        vec2(screen.width(), 82.0),
                    );
                    let canvas = Rect::from_min_max(
                        pos2(screen.left(), bar.bottom()),
                        pos2(screen.right(), dock.top()),
                    );
                    self.canvas(ui, canvas, ctx);
                    self.dock(ui, dock);
                } else {
                    self.welcome(
                        ui,
                        Rect::from_min_max(
                            screen.min + vec2(0.0, 40.0),
                            status.left_top() + vec2(screen.width(), 0.0),
                        ),
                        ctx,
                    );
                }
                self.status(ui, status);
                self.resize_edges(ui, screen, ctx);
            });
        self.popup_ui(ctx, screen);
        if self.popup.is_some()
            && !self.opened_popup
            && ctx.input(|i| {
                i.pointer.any_click()
                    && i.pointer
                        .interact_pos()
                        .is_some_and(|p| !self.popup_rect.contains(p))
            })
        {
            self.popup = None;
        }
        if let Some(error) = self.error.clone() {
            let mut open = true;
            Window::new(self.t("failed"))
                .collapsible(false)
                .resizable(false)
                .open(&mut open)
                .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
                .show(ctx, |ui| {
                    ui.set_max_width(500.0);
                    ui.label(text::diagnostic(&error, &self.prefs.language));
                });
            if !open {
                self.error = None;
            }
        }
        if ctx.input(|i| !i.raw.hovered_files.is_empty()) {
            let painter = ctx.layer_painter(LayerId::new(Order::Tooltip, Id::new("drop-overlay")));
            painter.rect_filled(screen.shrink(4.0), 8, p.bg.gamma_multiply(0.96));
            painter.rect_stroke(
                screen.shrink(15.0),
                8,
                Stroke::new(2.0, p.mint),
                StrokeKind::Inside,
            );
            painter.text(
                screen.center(),
                Align2::CENTER_CENTER,
                self.t("dropNow"),
                FontId::proportional(24.0),
                p.text,
            );
        }
        let screenshots = ctx.input(|i| {
            i.events
                .iter()
                .filter_map(|e| {
                    if let Event::Screenshot { image, .. } = e {
                        Some(image.clone())
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>()
        });
        if let Some(automation) = &mut self.automation {
            for screenshot in screenshots {
                if let Some(request) = automation.screenshot.take() {
                    let result = request["path"]
                        .as_str()
                        .ok_or("Missing screenshot path".to_string())
                        .and_then(|path| {
                            let bytes: Vec<_> = screenshot
                                .pixels
                                .iter()
                                .flat_map(|c| c.to_array())
                                .collect();
                            image::save_buffer(
                                path,
                                &bytes,
                                screenshot.width() as u32,
                                screenshot.height() as u32,
                                image::ColorType::Rgba8,
                            )
                            .map_err(|e| e.to_string())
                        });
                    Automation::reply(
                        json!({"id":request["id"],"screenshot":result.is_ok(),"error":result.err()}),
                    );
                }
            }
        }
        let request = self.automation.as_mut().and_then(|a| a.pending.take());
        if let Some(request) = request {
            if request["action"] != "screenshot" {
                Automation::reply(json!({"id":request["id"],"state":self.snapshot()}));
            }
        }
        if self.automation.is_some() {
            ctx.request_repaint_after(Duration::from_millis(16));
        }
        if self.loading {
            ctx.request_repaint_after(Duration::from_millis(100));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn late_scan_results_cannot_replace_new_or_closed_projects() {
        let dir = tempfile::tempdir().unwrap();
        let mut app = App::new(dir.path().into(), None);
        let ctx = Context::default();
        let analysis = repotower_core::analyze(
            concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/demo-project"),
            |_| {},
        )
        .unwrap();
        app.generation = 2;
        app.loading = true;
        app.sender
            .send(Work::Complete(1, Ok(analysis.clone())))
            .unwrap();
        app.sender
            .send(Work::Progress(1, ScanProgress::new("parse", 3, Some(23))))
            .unwrap();
        app.receive(&ctx);
        assert!(app.analysis.is_none());
        assert!(app.loading);
        assert!(app.progress.is_none());
        app.sender
            .send(Work::Complete(2, Ok(analysis.clone())))
            .unwrap();
        app.receive(&ctx);
        assert_eq!(app.analysis.as_ref().unwrap().total_modules, 23);
        assert!(!app.loading);
        app.close_project();
        app.sender.send(Work::Complete(2, Ok(analysis))).unwrap();
        app.receive(&ctx);
        assert!(app.analysis.is_none());
        assert!(!app.loading);
    }
}
