use crate::{
    app::{App, Drag, Popup},
    graph::{self, Camera, Layout, View, NODE_SIZE},
    paint::*,
    text::{self, tr},
};
use eframe::egui::*;
use std::{
    collections::{HashMap, HashSet},
    time::Duration,
};

impl App {
    pub fn rebuild(&mut self) {
        if !self.layout_dirty {
            return;
        }
        if let Some(a) = &self.analysis {
            self.layout = Layout::build(
                a,
                self.view,
                self.simulation.selected.as_deref(),
                self.neighbor.as_deref(),
                self.simulation.report(),
                &self.simulation.unavailable,
            );
            self.preview = self
                .simulation
                .selected
                .as_deref()
                .and_then(|id| self.simulation.preview(a, id));
        }
        self.layout_dirty = false;
    }
    fn camera_scope(&mut self, size: Vec2) {
        let key = match self.view {
            View::All => "all".to_string(),
            View::Nearby => format!("nearby:{}", self.neighbor.as_deref().unwrap_or("")),
            View::Impact => format!(
                "impact:{}",
                self.simulation
                    .report()
                    .map(|r| r.removed_module_id.as_str())
                    .unwrap_or("")
            ),
        };
        if self.camera_key != key {
            if !self.camera_key.is_empty() {
                self.cameras.insert(self.camera_key.clone(), self.camera);
            }
            if let Some(camera) = self.cameras.get(&key) {
                self.camera = *camera;
            } else {
                self.needs_fit = true;
            }
            self.camera_key = key;
        }
        if self.graph_size != size {
            self.graph_size = size;
            self.needs_fit = true;
        }
        if self.needs_fit {
            let empty = HashMap::new();
            self.camera = Camera::fit(
                &self.layout,
                if self.view == View::All {
                    &self.offsets
                } else {
                    &empty
                },
                size,
                self.simulation.report().is_some(),
            );
            self.needs_fit = false;
        }
    }
    fn node_position(&self, node: &graph::Node) -> Pos2 {
        node.pos
            + if self.view == View::All {
                self.offsets.get(&node.id).copied().unwrap_or_default()
            } else {
                Vec2::ZERO
            }
    }
    fn hit_node(&self, point: Pos2, origin: Pos2) -> Option<String> {
        let world = ((point - origin - self.camera.offset) / self.camera.scale).to_pos2();
        let contains = |n: &&graph::Node| {
            Rect::from_min_size(self.node_position(n), NODE_SIZE).contains(world)
        };
        if self.view == View::All {
            for id in self.z_order.iter().rev() {
                if self
                    .layout
                    .nodes
                    .iter()
                    .filter(|n| n.id == *id)
                    .any(|n| contains(&n))
                {
                    return Some(id.clone());
                }
            }
        }
        self.layout
            .nodes
            .iter()
            .rev()
            .find(contains)
            .map(|n| n.id.clone())
    }
    fn move_drag(&mut self, at: Pos2) {
        let Some(drag) = &mut self.drag else { return };
        if (at - drag.start).length() > 3.0 {
            drag.moved = true;
        }
        if drag.moved {
            let delta = at - drag.last;
            if let Some(id) = &drag.node {
                *self.offsets.entry(id.clone()).or_default() += delta / self.camera.scale;
            } else {
                self.camera.offset += delta;
            }
        }
        drag.last = at;
    }
    fn graph_input(&mut self, ctx: &Context, rect: Rect) {
        let has_report = self.simulation.report().is_some();
        let input_rect = Rect::from_min_max(
            rect.min + vec2(0.0, 50.0),
            rect.max - vec2(0.0, if has_report { 66.0 } else { 0.0 }),
        );
        let zoom_rect = Rect::from_min_size(
            rect.right_bottom() - vec2(112.0, if has_report { 115.0 } else { 50.0 }),
            vec2(100.0, 35.0),
        );
        let inside = |p: Pos2| input_rect.contains(p) && !zoom_rect.contains(p);
        let now = ctx.input(|i| i.time);
        if self.popup.is_some() || self.error.is_some() {
            self.hovered = None;
            self.hover_candidate = None;
            self.drag = None;
            self.last_node_click = None;
            return;
        }
        // Preserve event order: a slow frame can contain a complete drag or both clicks.
        for event in ctx.input(|i| i.raw.events.clone()) {
            match event {
                Event::PointerMoved(at) => self.move_drag(at),
                Event::PointerButton {
                    pos: at,
                    button,
                    pressed: true,
                    ..
                } => {
                    if !inside(at) {
                        self.last_node_click = None;
                        continue;
                    }
                    if self.drag.is_some()
                        || !matches!(
                            button,
                            PointerButton::Primary
                                | PointerButton::Secondary
                                | PointerButton::Middle
                        )
                    {
                        continue;
                    }
                    let target = self.hit_node(at, rect.min);
                    let move_node = button == PointerButton::Secondary && self.view == View::All;
                    if move_node {
                        if let Some(id) = &target {
                            self.select(Some(id.clone()));
                            self.z_order.retain(|other| other != id);
                            self.z_order.push(id.clone());
                        }
                    }
                    self.drag = Some(Drag {
                        start: at,
                        last: at,
                        button,
                        node: if move_node { target } else { None },
                        moved: false,
                    });
                }
                Event::PointerButton {
                    pos: at,
                    button,
                    pressed: false,
                    ..
                } => {
                    if !self.drag.as_ref().is_some_and(|drag| drag.button == button) {
                        continue;
                    }
                    self.move_drag(at);
                    let drag = self.drag.take().unwrap();
                    let target = if inside(at) {
                        self.hit_node(at, rect.min)
                    } else {
                        None
                    };
                    if drag.moved {
                        self.last_node_click = None;
                    } else if button == PointerButton::Primary {
                        self.select(target.clone());
                        if let Some(id) = target {
                            let double =
                                self.last_node_click
                                    .as_ref()
                                    .is_some_and(|(previous, time)| {
                                        *previous == id && now - *time <= 0.35
                                    });
                            self.last_node_click = if double { None } else { Some((id, now)) };
                            if double {
                                self.set_view(View::Nearby);
                            }
                        } else {
                            self.last_node_click = None;
                        }
                    } else if button == PointerButton::Secondary
                        && self.view == View::All
                        && target.is_some()
                    {
                        self.toggle(Popup::Positions);
                        break;
                    }
                }
                Event::PointerGone | Event::WindowFocused(false) => {
                    self.drag = None;
                    self.hovered = None;
                    self.hover_candidate = None;
                    self.last_node_click = None;
                }
                _ => {}
            }
        }
        if self.popup.is_some() {
            self.hovered = None;
            self.hover_candidate = None;
            return;
        }
        let point = ctx.input(|i| i.pointer.hover_pos());
        if let Some(at) = point {
            if self.drag.is_some() {
                ctx.set_cursor_icon(CursorIcon::Grabbing);
                return;
            }
            if inside(at) {
                let wheel = ctx.input(|i| i.raw_scroll_delta.y);
                if wheel != 0.0 {
                    self.camera.zoom((wheel * 0.0015).exp(), at - rect.min);
                }
            }
            let target = if inside(at) {
                self.hit_node(at, rect.min)
            } else {
                None
            };
            if self.hover_candidate != target {
                self.hover_candidate = target.clone();
                self.hover_since = now;
                self.hovered = None;
            }
            let wait = if self.simulation.selected.is_some() && self.simulation.selected != target {
                0.35
            } else {
                0.0
            };
            if now - self.hover_since >= wait {
                self.hovered = target.clone();
            } else {
                ctx.request_repaint_after(Duration::from_secs_f64(
                    (wait - (now - self.hover_since)).max(0.001),
                ));
            }
            if target.is_some() {
                ctx.set_cursor_icon(CursorIcon::PointingHand);
            } else if inside(at) {
                ctx.set_cursor_icon(CursorIcon::Grab);
            }
        } else {
            self.hovered = None;
            self.hover_candidate = None;
        }
    }
    pub fn canvas(&mut self, ui: &mut Ui, rect: Rect, ctx: &Context) {
        self.rebuild();
        self.camera_scope(rect.size());
        self.graph_input(ctx, rect);
        self.rebuild();
        self.camera_scope(rect.size());
        let p = self.palette();
        let lang = self.prefs.language.clone();
        let painter = ui.painter().with_clip_rect(rect);
        painter.rect_filled(rect, 0, p.graph);
        register(&mut self.controls, "canvas", rect);
        let dot = blend(p.graph, p.dot, 0.23);
        for row in 0..(rect.height() / 22.0).ceil() as i32 {
            for col in 0..(rect.width() / 22.0).ceil() as i32 {
                painter.circle_filled(
                    rect.min + vec2(col as f32 * 22.0 + 1.0, row as f32 * 22.0 + 1.0),
                    0.65,
                    dot,
                );
            }
        }
        let transform = |v: Pos2| rect.min + self.camera.offset + v.to_vec2() * self.camera.scale;
        let camera_scale = self.camera.scale;
        let content_clip = Rect::from_min_max(
            rect.min + vec2(0.0, 48.0),
            rect.max
                - vec2(
                    0.0,
                    if self.simulation.report().is_some() {
                        67.0
                    } else {
                        0.0
                    },
                ),
        );
        let graph_painter = painter.with_clip_rect(content_clip);
        let focus = self.hovered.as_ref().or(self.simulation.selected.as_ref());
        let mut connected: HashSet<&str> = HashSet::new();
        let show_trace = self.simulation.report().is_some()
            && (self.view == View::Impact
                || focus.is_none()
                || self.playing
                || self.simulation.live.is_some());
        if let Some(id) = focus {
            connected.insert(id);
            for edge in &self.layout.edges {
                let from = &self.layout.nodes[edge.from].id;
                let to = &self.layout.nodes[edge.to].id;
                if from == id || to == id {
                    connected.insert(from);
                    connected.insert(to);
                }
            }
        }
        for (index, x) in &self.layout.columns {
            let heading = match self.view {
                View::All => format!("{} {index}", tr(&lang, "graph.depth")),
                View::Nearby => tr(
                    &lang,
                    match index {
                        0 => "graph.depends",
                        1 => "graph.self",
                        _ => "graph.callers",
                    },
                )
                .into(),
                View::Impact => match index {
                    0 => tr(&lang, "graph.origin").into(),
                    1 => tr(&lang, "graph.direct").into(),
                    _ => format!("{} {index}", tr(&lang, "graph.wave")),
                },
            };
            graph_painter.text(
                transform(pos2(*x + NODE_SIZE.x / 2.0, 16.0)),
                Align2::CENTER_CENTER,
                heading,
                FontId::proportional((20.0 * camera_scale).min(10.0)),
                p.muted,
            );
        }
        for edge in &self.layout.edges {
            let from = &self.layout.nodes[edge.from];
            let to = &self.layout.nodes[edge.to];
            let points =
                graph::route(self.node_position(from), self.node_position(to)).map(transform);
            let highlight = focus.is_some_and(|id| *id == from.id || *id == to.id);
            let dim = focus.is_some() && !highlight && !show_trace;
            let reached = edge.causal && to.wave.is_some_and(|wave| self.playhead >= wave as f32);
            let tint = if reached {
                p.affected
            } else if highlight {
                p.accent
            } else {
                p.wire
            };
            let tint = blend(
                p.graph,
                tint,
                if dim {
                    0.18
                } else if edge.causal || highlight {
                    0.92
                } else {
                    0.65
                },
            );
            let stroke = Stroke::new(if highlight || reached { 1.45 } else { 1.0 }, tint);
            if show_trace && !edge.causal {
                let curve: Vec<_> = (0..36)
                    .map(|i| graph::curve_point(points, i as f32 / 35.0))
                    .collect();
                graph_painter.extend(Shape::dashed_line(
                    &curve,
                    Stroke::new(1.0, blend(p.graph, p.wire, 0.25)),
                    3.0,
                    5.0,
                ));
            } else {
                graph_painter.add(Shape::CubicBezier(
                    eframe::epaint::CubicBezierShape::from_points_stroke(
                        points,
                        false,
                        Color32::TRANSPARENT,
                        stroke,
                    ),
                ));
            }
            let dir = (points[3] - points[2]).normalized();
            let normal = vec2(-dir.y, dir.x);
            graph_painter.line_segment([points[3] - dir * 5.0 + normal * 2.5, points[3]], stroke);
            graph_painter.line_segment([points[3] - dir * 5.0 - normal * 2.5, points[3]], stroke);
            if edge.causal {
                if let Some(wave) = to.wave {
                    let t = self.playhead - (wave as f32 - 1.0);
                    if t > 0.0 && t < 1.0 {
                        let pos = graph::curve_point(points, t);
                        graph_painter.circle_filled(pos, 7.0, blend(p.graph, p.affected, 0.12));
                        graph_painter.circle_filled(pos, 3.0, p.affected);
                        graph_painter.circle_filled(pos, 1.3, p.node);
                    }
                }
            }
        }
        let Some(analysis) = self.analysis.clone() else {
            return;
        };
        let mut draw_order: Vec<_> = self.layout.nodes.iter().collect();
        if self.view == View::All {
            draw_order.sort_by_key(|n| {
                self.z_order
                    .iter()
                    .position(|id| *id == n.id)
                    .map(|i| i + 1)
                    .unwrap_or(0)
            });
        }
        for node in draw_order {
            let module = &analysis.modules[node.module];
            let r = Rect::from_min_size(
                transform(self.node_position(node)),
                NODE_SIZE * camera_scale,
            );
            register(&mut self.controls, &format!("node:{}", node.id), r);
            if !content_clip.intersects(r) {
                continue;
            }
            let origin = self.simulation.origins.contains(&node.id) || node.wave == Some(0);
            let reached = node
                .wave
                .is_some_and(|wave| wave > 0 && self.playhead >= wave as f32);
            let pending = node
                .wave
                .is_some_and(|wave| wave > 0 && self.playhead < wave as f32);
            let affected = !origin
                && (reached
                    || node.wave.is_none() && self.simulation.unavailable.contains(&node.id));
            let selected = self.simulation.selected.as_ref() == Some(&node.id);
            let dim = focus.is_some() && !connected.contains(node.id.as_str()) && !show_trace;
            let color = node_color(&module.directory);
            let border = if origin {
                p.cut
            } else if affected {
                p.affected
            } else if selected {
                p.accent
            } else {
                blend(p.graph_border, color, 0.55)
            };
            let fill = if origin {
                blend(p.node, p.cut, 0.055)
            } else if affected {
                blend(p.node, p.affected, 0.08)
            } else if selected {
                blend(p.node, p.accent, 0.08)
            } else {
                p.node
            };
            let alpha = if dim { 0.35 } else { 1.0 };
            let border = blend(p.graph, border, if pending { alpha * 0.55 } else { alpha });
            let fill = blend(p.graph, fill, alpha);
            let radius = 5.0 * camera_scale;
            if selected {
                graph_painter.rect_stroke(
                    r.expand(3.0),
                    radius,
                    Stroke::new(3.0, blend(p.graph, p.accent, 0.1)),
                    StrokeKind::Outside,
                );
            }
            graph_painter.rect_filled(r, radius, fill);
            if origin {
                let corners = [
                    r.left_top(),
                    r.right_top(),
                    r.right_bottom(),
                    r.left_bottom(),
                    r.left_top(),
                ];
                graph_painter.extend(Shape::dashed_line(
                    &corners,
                    Stroke::new(1.2, border),
                    4.0,
                    4.0,
                ));
            } else {
                graph_painter.rect_stroke(
                    r,
                    radius,
                    Stroke::new(
                        if selected {
                            1.7
                        } else if affected {
                            1.4
                        } else {
                            1.0
                        },
                        border,
                    ),
                    StrokeKind::Inside,
                );
            }
            graph_painter.circle_filled(
                r.min + vec2(11.0, 22.0) * camera_scale,
                2.0 * camera_scale,
                blend(
                    p.graph,
                    if origin {
                        p.cut
                    } else if affected {
                        p.affected
                    } else {
                        color
                    },
                    alpha,
                ),
            );
            let tint = if origin {
                p.cut
            } else if affected {
                p.affected
            } else if pending {
                p.muted
            } else {
                p.text
            };
            let text_painter =
                graph_painter.with_clip_rect(content_clip.intersect(r.shrink2(vec2(5.0, 0.0))));
            let node_font = (11.5 / camera_scale).clamp(12.0, 23.0) * camera_scale;
            let limit = ((NODE_SIZE.x - 30.0) * camera_scale / (node_font * 0.6)).floor() as usize;
            text_painter.text(
                r.min + vec2(22.0, 22.0) * camera_scale,
                Align2::LEFT_CENTER,
                text::short(&module.file_name, limit),
                FontId::monospace(node_font),
                blend(p.graph, tint, alpha),
            );
            if module.cycle_id.is_some() {
                graph_painter.circle_stroke(
                    r.right_top() + vec2(-5.0, 5.0) * camera_scale,
                    1.5 * camera_scale,
                    Stroke::new(0.75, blend(p.graph, p.cut, alpha)),
                );
            }
        }
        if self.layout.nodes.is_empty() {
            painter.text(
                rect.center(),
                Align2::CENTER_CENTER,
                tr(&lang, "emptyTitle"),
                FontId::proportional(15.0),
                p.muted,
            );
        }
        if self.layout.hidden > 0 {
            label(
                &painter,
                rect.min + vec2(22.0, 61.0),
                format!(
                    "{} {} · {} {}",
                    self.layout.nodes.len(),
                    tr(&lang, "files"),
                    self.layout.hidden,
                    tr(&lang, "graph.hidden")
                ),
                10.0,
                p.muted,
            );
        }
        if let Some(id) = self.hovered.as_ref() {
            if let Some(m) = analysis.modules.iter().find(|m| m.id == *id) {
                let y = rect.bottom()
                    - if self.simulation.report().is_some() {
                        90.0
                    } else {
                        29.0
                    };
                let parents: Vec<_> = self
                    .layout
                    .edges
                    .iter()
                    .filter(|e| e.causal && self.layout.nodes[e.to].id == *id)
                    .map(|e| {
                        analysis.modules[self.layout.nodes[e.from].module]
                            .file_name
                            .as_str()
                    })
                    .collect();
                let value = if parents.is_empty() {
                    m.relative_path.clone()
                } else {
                    format!(
                        "{}   ·   {} {}",
                        m.relative_path,
                        tr(&lang, "graph.via"),
                        parents.join(", ")
                    )
                };
                let value = text::short(&value, 90);
                let galley = painter.layout_no_wrap(value, FontId::proportional(10.0), p.muted);
                let r = Rect::from_min_size(
                    pos2(rect.left() + 20.0, y - 12.0),
                    galley.size() + vec2(16.0, 12.0),
                );
                painter.rect(
                    r,
                    5,
                    p.node,
                    Stroke::new(1.0, p.graph_border),
                    StrokeKind::Inside,
                );
                painter.galley(r.min + vec2(8.0, 6.0), galley, p.muted);
            }
        }
        // Paint controls after the circuit so nodes never cover actions.
        for (id, glyph, x, key) in [
            ("help", Icon::Help, 20.0, "graph.help"),
            ("positions", Icon::Grid, 52.0, "graph.position"),
        ] {
            if button(
                ui,
                &mut self.controls,
                id,
                Rect::from_min_size(rect.min + vec2(x, 14.0), vec2(28.0, 28.0)),
                "",
                Some(glyph),
                p,
                false,
                true,
                false,
            )
            .on_hover_text(tr(&lang, key))
            .clicked()
            {
                self.toggle(if id == "help" {
                    Popup::Help
                } else {
                    Popup::Positions
                });
            }
        }
        let report = self.simulation.report().cloned();
        let tabs_width = if report.is_some() { 234.0 } else { 112.0 };
        let tabs = Rect::from_min_size(
            pos2(rect.right() - tabs_width - 20.0, rect.top() + 14.0),
            vec2(tabs_width, 28.0),
        );
        painter.rect(
            tabs,
            5,
            p.node,
            Stroke::new(1.0, p.graph_border),
            StrokeKind::Inside,
        );
        let mut x = tabs.left() + 3.0;
        for (id, view, key, w, enabled) in [
            ("view-all", View::All, "graph.overview", 40.0, true),
            (
                "view-nearby",
                View::Nearby,
                "graph.neighbors",
                65.0,
                self.simulation.selected.is_some(),
            ),
            (
                "view-impact",
                View::Impact,
                "graph.impact",
                118.0,
                report.is_some(),
            ),
        ] {
            if view == View::Impact && report.is_none() {
                continue;
            }
            if button(
                ui,
                &mut self.controls,
                id,
                Rect::from_min_size(pos2(x, tabs.top() + 3.0), vec2(w, 22.0)),
                tr(&lang, key),
                None,
                p,
                self.view == view,
                enabled,
                false,
            )
            .clicked()
            {
                self.set_view(view);
            }
            x += w;
        }
        if let Some(report) = report.as_ref() {
            let r = Rect::from_min_size(pos2(tabs.left() - 157.0, tabs.top()), vec2(149.0, 28.0));
            let name = report.removed_module_id.rsplit('/').next().unwrap_or("");
            if button(
                ui,
                &mut self.controls,
                "reports",
                r,
                &text::short(name, 17),
                Some(Icon::Chevron),
                p,
                false,
                self.simulation.live.is_none(),
                false,
            )
            .on_hover_text(tr(&lang, "report"))
            .clicked()
            {
                self.toggle(Popup::Reports);
            }
        }
        let zoom = Rect::from_min_size(
            rect.right_bottom() - vec2(103.0, if report.is_some() { 109.0 } else { 50.0 }),
            vec2(85.0, 32.0),
        );
        painter.rect(
            zoom,
            6,
            p.node,
            Stroke::new(1.0, p.graph_border),
            StrokeKind::Inside,
        );
        for (i, id, glyph, key) in [
            (0, "zoom-out", Icon::Minus, "graph.zoomOut"),
            (1, "fit", Icon::Fit, "graph.fit"),
            (2, "zoom-in", Icon::Plus, "graph.zoomIn"),
        ] {
            if button(
                ui,
                &mut self.controls,
                id,
                Rect::from_min_size(
                    zoom.min + vec2(i as f32 * 27.0 + 2.0, 3.0),
                    vec2(27.0, 26.0),
                ),
                "",
                Some(glyph),
                p,
                false,
                true,
                false,
            )
            .on_hover_text(tr(&lang, key))
            .clicked()
            {
                match i {
                    0 => self.camera.zoom(1.0 / 1.2, rect.size() / 2.0),
                    2 => self.camera.zoom(1.2, rect.size() / 2.0),
                    _ => self.needs_fit = true,
                }
            }
        }
        if report.is_some() {
            self.timeline(ui, rect);
        }
    }
    fn timeline(&mut self, ui: &mut Ui, rect: Rect) {
        let p = self.palette();
        let lang = self.prefs.language.clone();
        let bar = Rect::from_min_size(
            rect.left_bottom() + vec2(20.0, -61.0),
            vec2(rect.width() - 40.0, 44.0),
        );
        ui.painter().rect(
            bar,
            7,
            p.node,
            Stroke::new(1.0, p.graph_border),
            StrokeKind::Inside,
        );
        for (i, id, glyph, key) in [
            (0, "previous", Icon::Prev, "graph.prev"),
            (
                1,
                "play",
                if self.playing {
                    Icon::Pause
                } else {
                    Icon::Play
                },
                if self.playing {
                    "graph.pause"
                } else {
                    "graph.play"
                },
            ),
            (2, "next", Icon::Next, "graph.next"),
            (3, "replay", Icon::Reset, "graph.replay"),
        ] {
            if button(
                ui,
                &mut self.controls,
                id,
                Rect::from_min_size(bar.min + vec2(8.0 + i as f32 * 29.0, 8.0), vec2(27.0, 28.0)),
                "",
                Some(glyph),
                p,
                false,
                true,
                false,
            )
            .on_hover_text(tr(&lang, key))
            .clicked()
            {
                match i {
                    0 => {
                        self.playing = false;
                        self.playhead = (self.playhead.ceil() - 1.0).max(0.0);
                    }
                    1 => {
                        if self.playhead >= self.max_wave() {
                            self.playhead = 0.0;
                        }
                        self.playing = !self.playing;
                    }
                    2 => {
                        self.playing = false;
                        self.playhead = (self.playhead.floor() + 1.0).min(self.max_wave());
                        if self.playhead >= self.max_wave() {
                            self.finish();
                        }
                    }
                    _ => {
                        self.playhead = 0.0;
                        self.playing = true;
                    }
                }
            }
        }
        let slider = Rect::from_min_max(
            bar.min + vec2(139.0, 11.0),
            bar.right_bottom() - vec2(255.0, 11.0),
        );
        let max = self.max_wave().max(1.0);
        ui.painter().line_segment(
            [slider.left_center(), slider.right_center()],
            Stroke::new(2.0, p.graph_border),
        );
        let head = pos2(
            slider.left() + slider.width() * self.playhead / max,
            slider.center().y,
        );
        ui.painter()
            .line_segment([slider.left_center(), head], Stroke::new(2.0, p.affected));
        ui.painter().circle_filled(head, 4.0, p.affected);
        register(&mut self.controls, "scrub", slider);
        let response = ui.interact(
            slider.expand(4.0),
            Id::new("scrub"),
            Sense::click_and_drag(),
        );
        if response.clicked() || response.dragged() {
            if let Some(pos) = response.interact_pointer_pos() {
                self.playing = false;
                self.playhead =
                    ((pos.x - slider.left()) / slider.width() * max).clamp(0.0, self.max_wave());
                if self.playhead >= self.max_wave() {
                    self.finish();
                }
            }
        }
        let count = self
            .simulation
            .report()
            .map(|r| {
                r.waves
                    .iter()
                    .enumerate()
                    .filter(|(i, _)| *i > 0 && *i as f32 <= self.playhead)
                    .map(|(_, w)| w.len())
                    .sum::<usize>()
            })
            .unwrap_or(0);
        label(
            ui.painter(),
            pos2(bar.right() - 236.0, bar.center().y),
            format!(
                "{} {} / {}  ·  {} {}",
                tr(&lang, "graph.wave"),
                self.playhead.floor() as usize,
                self.max_wave() as usize,
                count,
                tr(&lang, "affected")
            ),
            10.0,
            p.muted,
        );
        if button(
            ui,
            &mut self.controls,
            "skip",
            Rect::from_min_size(pos2(bar.right() - 32.0, bar.top() + 8.0), vec2(26.0, 28.0)),
            "",
            Some(Icon::Next),
            p,
            false,
            true,
            false,
        )
        .on_hover_text(tr(&lang, "graph.skip"))
        .clicked()
        {
            self.finish();
        }
    }
}
