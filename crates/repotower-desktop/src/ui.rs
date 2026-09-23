use crate::{
    app::{App, Popup},
    paint::*,
    text::{self, tr},
};
use eframe::egui::*;

impl App {
    pub fn resize_edges(&mut self, ui: &mut Ui, r: Rect, ctx: &Context) {
        if ctx.input(|i| i.viewport().maximized.unwrap_or(false)) {
            return;
        }
        for (id, rect, direction, cursor) in [
            (
                "resize-n",
                Rect::from_min_max(r.min + vec2(8.0, 0.0), r.right_top() + vec2(-8.0, 4.0)),
                ResizeDirection::North,
                CursorIcon::ResizeVertical,
            ),
            (
                "resize-s",
                Rect::from_min_max(r.left_bottom() + vec2(8.0, -4.0), r.max - vec2(8.0, 0.0)),
                ResizeDirection::South,
                CursorIcon::ResizeVertical,
            ),
            (
                "resize-w",
                Rect::from_min_max(r.min + vec2(0.0, 8.0), r.left_bottom() + vec2(4.0, -8.0)),
                ResizeDirection::West,
                CursorIcon::ResizeHorizontal,
            ),
            (
                "resize-e",
                Rect::from_min_max(r.right_top() + vec2(-4.0, 8.0), r.max - vec2(0.0, 8.0)),
                ResizeDirection::East,
                CursorIcon::ResizeHorizontal,
            ),
            (
                "resize-nw",
                Rect::from_min_size(r.min, vec2(8.0, 8.0)),
                ResizeDirection::NorthWest,
                CursorIcon::ResizeNwSe,
            ),
            (
                "resize-ne",
                Rect::from_min_size(r.right_top() - vec2(8.0, 0.0), vec2(8.0, 8.0)),
                ResizeDirection::NorthEast,
                CursorIcon::ResizeNeSw,
            ),
            (
                "resize-sw",
                Rect::from_min_size(r.left_bottom() - vec2(0.0, 8.0), vec2(8.0, 8.0)),
                ResizeDirection::SouthWest,
                CursorIcon::ResizeNeSw,
            ),
            (
                "resize-se",
                Rect::from_min_size(r.max - vec2(8.0, 8.0), vec2(8.0, 8.0)),
                ResizeDirection::SouthEast,
                CursorIcon::ResizeNwSe,
            ),
        ] {
            let response = ui
                .interact(rect, Id::new(id), Sense::drag())
                .on_hover_cursor(cursor);
            if response.drag_started() {
                ctx.send_viewport_cmd(ViewportCommand::BeginResize(direction));
            }
        }
    }
    pub fn style(&self, ctx: &Context) {
        let p = self.palette();
        let mut style = (*ctx.style()).clone();
        style.visuals = if self.prefs.theme == "dark" {
            Visuals::dark()
        } else {
            Visuals::light()
        };
        style.visuals.override_text_color = Some(p.text);
        style.visuals.panel_fill = p.bg;
        style.visuals.window_fill = p.popover;
        style.visuals.extreme_bg_color = p.field;
        style.visuals.selection.bg_fill = p.hover;
        style.visuals.selection.stroke = Stroke::new(1.0, p.accent);
        style.visuals.window_stroke = Stroke::new(1.0, p.line);
        style.visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, p.line);
        style.visuals.widgets.inactive.bg_fill = p.field;
        style.visuals.widgets.inactive.weak_bg_fill = p.field;
        style.visuals.widgets.hovered.weak_bg_fill = p.hover;
        style.visuals.widgets.active.weak_bg_fill = p.hover;
        style
            .text_styles
            .insert(TextStyle::Body, FontId::proportional(12.0));
        style
            .text_styles
            .insert(TextStyle::Button, FontId::proportional(12.0));
        style
            .text_styles
            .insert(TextStyle::Small, FontId::proportional(10.0));
        style.spacing.item_spacing = vec2(6.0, 6.0);
        ctx.set_style(style);
    }
    pub fn header(&mut self, ui: &mut Ui, rect: Rect, ctx: &Context) {
        let p = self.palette();
        let painter = ui.painter();
        painter.rect_filled(rect, 0, p.chrome);
        painter.line_segment(
            [rect.left_bottom(), rect.right_bottom()],
            Stroke::new(1.0, p.line),
        );
        let drag_rect = Rect::from_min_max(rect.min, pos2(rect.right() - 190.0, rect.bottom()));
        let drag = ui.interact(drag_rect, Id::new("title-drag"), Sense::click_and_drag());
        if drag.drag_started() {
            ctx.send_viewport_cmd(ViewportCommand::StartDrag);
        }
        if drag.double_clicked() {
            let max = ctx.input(|i| i.viewport().maximized.unwrap_or(false));
            ctx.send_viewport_cmd(ViewportCommand::Maximized(!max));
        }
        let origin = rect.min + vec2(18.0, 12.0);
        for (offset, tint) in [
            (vec2(0.0, 0.0), p.mint),
            (vec2(7.0, 10.0), p.blue),
            (vec2(14.0, 0.0), p.cut),
        ] {
            painter.rect_filled(
                Rect::from_min_size(origin + offset, vec2(6.0, 6.0)),
                1,
                tint,
            );
        }
        painter.line_segment(
            [origin + vec2(6.0, 3.0), origin + vec2(14.0, 3.0)],
            Stroke::new(1.0, p.subtle),
        );
        painter.line_segment(
            [origin + vec2(10.0, 3.0), origin + vec2(10.0, 10.0)],
            Stroke::new(1.0, p.subtle),
        );
        label(
            painter,
            rect.min + vec2(48.0, 20.0),
            "RepoTower",
            13.0,
            p.text,
        );
        label(painter, rect.min + vec2(127.0, 20.0), "06", 9.0, p.subtle);
        for (i, id, glyph, key) in [
            (0, "window-close", Icon::Close, "close"),
            (1, "window-max", Icon::Maximize, "maximize"),
            (2, "window-min", Icon::Minimize, "minimize"),
            (3, "settings", Icon::Settings, "settings"),
            (4, "open", Icon::Folder, "open"),
        ] {
            let r = Rect::from_min_size(
                pos2(rect.right() - 34.0 * (i + 1) as f32 - 4.0, rect.top() + 4.0),
                vec2(32.0, 32.0),
            );
            if button(
                ui,
                &mut self.controls,
                id,
                r,
                "",
                Some(glyph),
                p,
                false,
                true,
                false,
            )
            .on_hover_text(self.t(key))
            .clicked()
            {
                match id {
                    "window-close" => ctx.send_viewport_cmd(ViewportCommand::Close),
                    "window-min" => ctx.send_viewport_cmd(ViewportCommand::Minimized(true)),
                    "window-max" => ctx.send_viewport_cmd(ViewportCommand::Maximized(
                        !ctx.input(|i| i.viewport().maximized.unwrap_or(false)),
                    )),
                    "settings" => self.toggle(Popup::Settings),
                    _ => self.pick_folder(ctx),
                }
            }
        }
    }
    pub fn project_bar(&mut self, ui: &mut Ui, rect: Rect, ctx: &Context) {
        let p = self.palette();
        ui.painter().rect_filled(rect, 0, p.bg);
        ui.painter().line_segment(
            [rect.left_bottom(), rect.right_bottom()],
            Stroke::new(1.0, p.line),
        );
        icon(
            ui.painter(),
            rect.min + vec2(22.0, 22.0),
            Icon::Folder,
            p.muted,
            17.0,
        );
        if let Some(a) = self.analysis.clone() {
            label(
                ui.painter(),
                rect.min + vec2(38.0, 22.0),
                text::short(&a.repository_name, 26),
                13.0,
                p.text,
            );
            let title_width = ui
                .painter()
                .layout_no_wrap(
                    text::short(&a.repository_name, 26),
                    FontId::proportional(13.0),
                    p.text,
                )
                .size()
                .x;
            label(
                ui.painter(),
                rect.min + vec2(54.0 + title_width, 22.0),
                format!(
                    "{} {}  ·  {} {}",
                    a.total_modules,
                    self.t("files"),
                    a.total_edges,
                    self.t("links")
                ),
                10.0,
                p.subtle,
            );
        }
        let search_rect = Rect::from_min_size(
            pos2(rect.right() - 232.0, rect.top() + 7.0),
            vec2(182.0, 30.0),
        );
        ui.painter().rect_filled(search_rect, 6, p.field);
        ui.painter()
            .rect_stroke(search_rect, 6, Stroke::new(1.0, p.line), StrokeKind::Inside);
        icon(
            ui.painter(),
            search_rect.min + vec2(14.0, 15.0),
            Icon::Search,
            p.subtle,
            15.0,
        );
        let hint = self.t("find");
        let field = ui.put(
            Rect::from_min_max(
                search_rect.min + vec2(30.0, 5.0),
                search_rect.max - vec2(7.0, 4.0),
            ),
            TextEdit::singleline(&mut self.search)
                .id(Id::new("search-field"))
                .hint_text(hint)
                .frame(false)
                .font(FontId::proportional(11.0)),
        );
        register(&mut self.controls, "search", search_rect);
        if field.changed() || field.clicked() {
            self.popup = Some(Popup::Search);
            self.opened_popup = true;
        }
        if (field.has_focus() || field.lost_focus()) && ctx.input(|i| i.key_pressed(Key::Enter)) {
            if let Some(id) = self.search_results(false).first().cloned() {
                self.select(Some(id.clone()));
                self.popup = None;
                self.reveal_selected(&id);
                ctx.memory_mut(|m| m.surrender_focus(Id::new("search-field")));
            }
        }
        for (id, glyph, x, key) in [
            ("critical", Icon::Bolt, rect.right() - 269.0, "critical"),
            ("close-project", Icon::Close, rect.right() - 40.0, "home"),
        ] {
            if button(
                ui,
                &mut self.controls,
                id,
                Rect::from_min_size(pos2(x, rect.top() + 7.0), vec2(30.0, 30.0)),
                "",
                Some(glyph),
                p,
                false,
                true,
                false,
            )
            .on_hover_text(self.t(key))
            .clicked()
            {
                if id == "critical" {
                    self.toggle(Popup::Critical);
                } else {
                    self.close_project();
                }
            }
        }
    }
    pub fn welcome(&mut self, ui: &mut Ui, rect: Rect, ctx: &Context) {
        let p = self.palette();
        let lang = self.prefs.language.clone();
        let center = rect.center() - vec2(0.0, 24.0);
        let painter = ui.painter();
        let nodes = [
            ("config.ts", center + vec2(-174.0, -72.0), color(0x71d8cb)),
            ("http.ts", center + vec2(34.0, -100.0), color(0xaaa0ef)),
            ("auth.ts", center + vec2(34.0, -38.0), color(0xeeac70)),
        ];
        for destination in [nodes[1].1, nodes[2].1] {
            let points = crate::graph::route(nodes[0].1, destination);
            painter.add(Shape::CubicBezier(
                eframe::epaint::CubicBezierShape::from_points_stroke(
                    points,
                    false,
                    Color32::TRANSPARENT,
                    Stroke::new(1.0, p.line),
                ),
            ));
        }
        for (name, pos, tint) in nodes {
            let r = Rect::from_min_size(pos, crate::graph::NODE_SIZE);
            painter.rect(
                r,
                6,
                p.node,
                Stroke::new(1.0, blend(p.line, tint, 0.7)),
                StrokeKind::Inside,
            );
            painter.circle_filled(r.min + vec2(11.0, 22.0), 2.0, tint);
            painter.text(
                r.min + vec2(20.0, 22.0),
                Align2::LEFT_CENTER,
                name,
                FontId::monospace(12.0),
                p.text,
            );
        }
        let heading = if self.loading {
            self.progress
                .as_ref()
                .map(|p| self.t(&p.stage))
                .filter(|s| !s.is_empty())
                .unwrap_or(self.t("scan"))
        } else {
            self.t("landing")
        };
        painter.text(
            center + vec2(0.0, 40.0),
            Align2::CENTER_CENTER,
            heading,
            FontId::proportional(22.0),
            p.text,
        );
        painter.text(
            center + vec2(0.0, 75.0),
            Align2::CENTER_CENTER,
            "JS / TS  ·  Java  ·  Python  ·  C / C++  ·  Go  ·  Rust  ·  C#",
            FontId::proportional(11.0),
            p.muted,
        );
        if self.loading {
            if let Some(progress) = &self.progress {
                if let (Some(done), Some(total)) = (progress.completed, progress.total) {
                    painter.text(
                        center + vec2(0.0, 121.0),
                        Align2::CENTER_CENTER,
                        format!("{done} / {total}"),
                        FontId::monospace(12.0),
                        p.muted,
                    );
                }
            }
        } else {
            let r = Rect::from_center_size(center + vec2(-73.0, 121.0), vec2(140.0, 36.0));
            if button(
                ui,
                &mut self.controls,
                "welcome-open",
                r,
                tr(&lang, "choose"),
                Some(Icon::Folder),
                p,
                false,
                true,
                true,
            )
            .clicked()
            {
                self.pick_folder(ctx);
            }
            let r = Rect::from_center_size(center + vec2(83.0, 121.0), vec2(142.0, 36.0));
            if button(
                ui,
                &mut self.controls,
                "welcome-demo",
                r,
                tr(&lang, "demo"),
                None,
                p,
                false,
                true,
                false,
            )
            .clicked()
            {
                self.open_example("demo-project", ctx);
            }
            ui.painter().text(
                center + vec2(0.0, 168.0),
                Align2::CENTER_CENTER,
                self.t("drop"),
                FontId::proportional(11.0),
                p.subtle,
            );
            if button(
                ui,
                &mut self.controls,
                "examples",
                Rect::from_center_size(center + vec2(0.0, 203.0), vec2(110.0, 26.0)),
                tr(&lang, "example"),
                Some(Icon::Chevron),
                p,
                false,
                true,
                false,
            )
            .clicked()
            {
                self.toggle(Popup::Examples);
            }
        }
    }
    pub fn dock(&mut self, ui: &mut Ui, rect: Rect) {
        let p = self.palette();
        ui.painter().rect_filled(rect, 0, p.chrome);
        ui.painter().line_segment(
            [rect.left_top(), rect.right_top()],
            Stroke::new(1.0, p.line),
        );
        let icon_rect = Rect::from_min_size(rect.min + vec2(18.0, 24.0), vec2(34.0, 34.0));
        ui.painter().rect(
            icon_rect,
            9,
            p.field,
            Stroke::new(1.0, p.line),
            StrokeKind::Inside,
        );
        icon(ui.painter(), icon_rect.center(), Icon::File, p.blue, 21.0);
        let a = self.analysis.clone();
        let selected = self.simulation.selected.clone();
        let module = a
            .as_ref()
            .and_then(|a| {
                selected
                    .as_ref()
                    .and_then(|id| a.modules.iter().find(|m| m.id == *id))
            })
            .cloned();
        if let Some(m) = module {
            label(
                ui.painter(),
                rect.min + vec2(63.0, 24.0),
                text::short(&m.file_name, 35),
                13.0,
                p.text,
            );
            label(
                ui.painter(),
                rect.min + vec2(63.0, 43.0),
                text::short(&m.relative_path, 57),
                10.0,
                p.muted,
            );
            let mut x = rect.left() + 59.0;
            for (id, label, count, popup) in [
                (
                    "imports",
                    self.t("imports"),
                    m.imports.len(),
                    Popup::Relations(m.id.clone(), false),
                ),
                (
                    "callers",
                    self.t("callers"),
                    m.imported_by.len(),
                    Popup::Relations(m.id.clone(), true),
                ),
                (
                    "unresolved",
                    self.t("unresolved"),
                    m.unresolved_imports.len(),
                    Popup::Unresolved(m.id.clone()),
                ),
            ] {
                if id == "unresolved" && count == 0 {
                    continue;
                }
                let caption = format!("{count} {label}");
                let w = ui
                    .painter()
                    .layout_no_wrap(caption.clone(), FontId::proportional(11.0), p.muted)
                    .size()
                    .x
                    + 16.0;
                if button(
                    ui,
                    &mut self.controls,
                    id,
                    Rect::from_min_size(pos2(x, rect.top() + 55.0), vec2(w, 23.0)),
                    &caption,
                    None,
                    p,
                    false,
                    true,
                    false,
                )
                .clicked()
                {
                    self.toggle(popup);
                }
                x += w + 4.0;
            }
            let origin = self.simulation.origins.contains(&m.id)
                || self
                    .simulation
                    .live
                    .as_ref()
                    .is_some_and(|r| r.removed_module_id == m.id);
            let affected = self.simulation.unavailable.contains(&m.id) && !origin;
            let preview = self.preview.as_ref();
            let metric_x = rect.right() - 334.0;
            if metric_x > rect.left() + 430.0 {
                let title = if origin {
                    self.t("target")
                } else if affected {
                    self.t("affected")
                } else {
                    self.t("potential")
                };
                label(
                    ui.painter(),
                    pos2(metric_x, rect.top() + 28.0),
                    title,
                    10.0,
                    p.muted,
                );
                let value = preview
                    .map(|r| format!("{} {}", r.total_affected, self.t("otherFiles")))
                    .unwrap_or_else(|| {
                        if origin {
                            self.t("report").into()
                        } else {
                            self.t("restoreOrigin").into()
                        }
                    });
                label(
                    ui.painter(),
                    pos2(metric_x, rect.top() + 51.0),
                    text::short(&value, 25),
                    12.0,
                    if origin { p.cut } else { p.affected },
                );
            }
            let caption = if origin {
                self.t("restoreFile")
            } else if self.simulation.live.is_some() {
                self.t("finish")
            } else {
                self.t("disconnect")
            };
            let r = Rect::from_min_size(
                pos2(rect.right() - 198.0, rect.top() + 25.0),
                vec2(112.0, 34.0),
            );
            if button(
                ui,
                &mut self.controls,
                "file-action",
                r,
                caption,
                Some(if origin { Icon::Reset } else { Icon::Cut }),
                p,
                false,
                origin || !affected,
                true,
            )
            .clicked()
            {
                if origin {
                    self.restore(&m.id);
                } else if self.simulation.live.is_some() {
                    self.finish();
                } else {
                    self.disconnect();
                }
            }
        } else if let Some(report) = self.simulation.report().cloned() {
            label(
                ui.painter(),
                rect.min + vec2(63.0, 28.0),
                text::short(&report.removed_module_id, 46),
                12.0,
                p.text,
            );
            label(
                ui.painter(),
                rect.min + vec2(63.0, 52.0),
                format!(
                    "{} {}   ·   {} {}   ·   {} {}",
                    report.direct_affected,
                    self.t("direct"),
                    report.transitive_affected,
                    self.t("indirect"),
                    self.layout.untouched,
                    self.t("intact")
                ),
                11.0,
                p.muted,
            );
            if self.simulation.live.is_some() {
                let caption = self.t("finish");
                if button(
                    ui,
                    &mut self.controls,
                    "file-action",
                    Rect::from_min_size(
                        pos2(rect.right() - 198.0, rect.top() + 25.0),
                        vec2(112.0, 34.0),
                    ),
                    caption,
                    None,
                    p,
                    false,
                    true,
                    true,
                )
                .clicked()
                {
                    self.finish();
                }
            }
        } else {
            label(
                ui.painter(),
                rect.min + vec2(63.0, 33.0),
                self.t("select"),
                13.0,
                p.text,
            );
            label(
                ui.painter(),
                rect.min + vec2(63.0, 54.0),
                self.t("hint"),
                11.0,
                p.subtle,
            );
        }
        for (id, glyph, x, enabled, key) in [
            (
                "undo",
                Icon::Undo,
                rect.right() - 70.0,
                self.simulation.can_undo(),
                "undo",
            ),
            (
                "restore-all",
                Icon::Reset,
                rect.right() - 36.0,
                !self.simulation.origins.is_empty() || self.simulation.live.is_some(),
                "reset",
            ),
        ] {
            if button(
                ui,
                &mut self.controls,
                id,
                Rect::from_center_size(pos2(x + 8.0, rect.center().y), vec2(28.0, 30.0)),
                "",
                Some(glyph),
                p,
                false,
                enabled,
                false,
            )
            .on_hover_text(self.t(key))
            .clicked()
            {
                if id == "undo" {
                    self.undo();
                } else {
                    self.reset();
                }
            }
        }
    }
    pub fn status(&mut self, ui: &mut Ui, rect: Rect) {
        let p = self.palette();
        ui.painter().rect_filled(rect, 0, p.bg);
        ui.painter().line_segment(
            [rect.left_top(), rect.right_top()],
            Stroke::new(1.0, p.line),
        );
        ui.painter()
            .circle_filled(rect.min + vec2(19.0, 12.0), 2.0, p.mint);
        label(
            ui.painter(),
            rect.min + vec2(26.0, 12.0),
            format!("{}    {}", self.t("offline"), self.t("readOnly")),
            9.0,
            p.subtle,
        );
        if let Some(a) = self.analysis.clone() {
            let mut languages = Vec::new();
            for m in &a.modules {
                let l = match m.extension.as_str() {
                    "java" => "Java",
                    "py" | "pyi" => "Python",
                    "c" | "h" | "cc" | "cpp" | "cxx" | "hpp" | "hh" | "hxx" => "C/C++",
                    "go" => "Go",
                    "rs" => "Rust",
                    "cs" => "C#",
                    _ => "JS/TS",
                };
                if !languages.contains(&l) {
                    languages.push(l);
                }
            }
            languages.sort();
            ui.painter().text(
                rect.right_center() - vec2(17.0, 0.0),
                Align2::RIGHT_CENTER,
                languages.join(" · "),
                FontId::proportional(9.0),
                p.subtle,
            );
            if !a.warnings.is_empty() {
                let caption = format!("{} · {}", self.t("warnings"), a.warnings.len());
                if button(
                    ui,
                    &mut self.controls,
                    "notes",
                    Rect::from_min_size(rect.min + vec2(230.0, 1.0), vec2(155.0, 22.0)),
                    &caption,
                    None,
                    p,
                    false,
                    true,
                    false,
                )
                .clicked()
                {
                    self.toggle(Popup::Warnings);
                }
            }
        }
    }
}
