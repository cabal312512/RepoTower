use eframe::egui::*;
use std::collections::BTreeMap;

pub type Controls = BTreeMap<String, [f32; 4]>;
pub fn register(controls: &mut Controls, id: &str, rect: Rect) {
    controls.insert(
        id.into(),
        [rect.min.x, rect.min.y, rect.width(), rect.height()],
    );
}
pub fn color(hex: u32) -> Color32 {
    Color32::from_rgb((hex >> 16) as u8, (hex >> 8) as u8, hex as u8)
}
pub fn blend(a: Color32, b: Color32, t: f32) -> Color32 {
    Color32::from_rgb(
        (a.r() as f32 * (1.0 - t) + b.r() as f32 * t) as u8,
        (a.g() as f32 * (1.0 - t) + b.g() as f32 * t) as u8,
        (a.b() as f32 * (1.0 - t) + b.b() as f32 * t) as u8,
    )
}
#[derive(Clone, Copy)]
pub struct Palette {
    pub bg: Color32,
    pub chrome: Color32,
    pub text: Color32,
    pub muted: Color32,
    pub subtle: Color32,
    pub line: Color32,
    pub hover: Color32,
    pub mint: Color32,
    pub blue: Color32,
    pub cut: Color32,
    pub affected: Color32,
    pub primary: Color32,
    pub primary_text: Color32,
    pub popover: Color32,
    pub field: Color32,
    pub graph: Color32,
    pub dot: Color32,
    pub node: Color32,
    pub wire: Color32,
    pub graph_border: Color32,
    pub accent: Color32,
}
impl Palette {
    pub fn new(dark: bool) -> Self {
        let c = if dark {
            [
                0x161a22, 0x1d222d, 0xe2e9f4, 0x98a6bb, 0x5d6d83, 0x303948, 0x2b3544, 0x78e4bf,
                0x88a6ff, 0xf27e94, 0xffa779, 0xace9d3, 0x17372f, 0x242c39, 0x171d28, 0x171c21,
                0x627b88, 0x1d262b, 0x54676c, 0x303b41, 0x84dfc2,
            ]
        } else {
            [
                0xf3f4f5, 0xe9edef, 0x273643, 0x667988, 0x92a0aa, 0xcfd9de, 0xdfe8eb, 0x138e73,
                0x527ccf, 0xcf4c70, 0xce652c, 0x243e45, 0xe6fcf6, 0xfcfdfd, 0xf6f9fa, 0xf1f3ef,
                0x8d9a91, 0xfafbf8, 0x8a9c93, 0xd0d8d0, 0x1b8971,
            ]
        };
        Self {
            bg: color(c[0]),
            chrome: color(c[1]),
            text: color(c[2]),
            muted: color(c[3]),
            subtle: color(c[4]),
            line: color(c[5]),
            hover: color(c[6]),
            mint: color(c[7]),
            blue: color(c[8]),
            cut: color(c[9]),
            affected: color(c[10]),
            primary: color(c[11]),
            primary_text: color(c[12]),
            popover: color(c[13]),
            field: color(c[14]),
            graph: color(c[15]),
            dot: color(c[16]),
            node: color(c[17]),
            wire: color(c[18]),
            graph_border: color(c[19]),
            accent: color(c[20]),
        }
    }
}
#[derive(Clone, Copy)]
pub enum Icon {
    Folder,
    Settings,
    Minimize,
    Maximize,
    Close,
    Help,
    Grid,
    Minus,
    Plus,
    Fit,
    Undo,
    Reset,
    Bolt,
    Search,
    File,
    Cut,
    Play,
    Pause,
    Next,
    Prev,
    Chevron,
}
pub fn icon(p: &Painter, center: Pos2, kind: Icon, tint: Color32, size: f32) {
    let o = center - vec2(size / 2.0, size / 2.0);
    let at = |x: f32, y: f32| o + vec2(x, y) * (size / 20.0);
    let stroke = Stroke::new(1.25, tint);
    let line = |points: &[(f32, f32)]| {
        p.add(Shape::line(
            points.iter().map(|(x, y)| at(*x, *y)).collect(),
            stroke,
        ));
    };
    match kind {
        Icon::Folder => {
            line(&[
                (2.0, 16.0),
                (2.0, 5.0),
                (7.0, 5.0),
                (9.0, 7.0),
                (17.0, 7.0),
                (17.0, 9.0),
            ]);
            line(&[
                (2.0, 16.0),
                (16.0, 16.0),
                (19.0, 9.0),
                (5.0, 9.0),
                (2.0, 16.0),
            ]);
        }
        Icon::Settings => {
            line(&[(3.0, 5.0), (17.0, 5.0)]);
            line(&[(3.0, 15.0), (17.0, 15.0)]);
            p.circle(
                at(7.0, 5.0),
                2.5 * size / 20.0,
                Color32::TRANSPARENT,
                stroke,
            );
            p.circle(
                at(13.0, 15.0),
                2.5 * size / 20.0,
                Color32::TRANSPARENT,
                stroke,
            );
        }
        Icon::Minimize | Icon::Minus => line(&[(4.0, 10.0), (16.0, 10.0)]),
        Icon::Plus => {
            line(&[(4.0, 10.0), (16.0, 10.0)]);
            line(&[(10.0, 4.0), (10.0, 16.0)]);
        }
        Icon::Maximize => {
            line(&[(4.0, 8.0), (4.0, 16.0), (12.0, 16.0)]);
            line(&[(8.0, 4.0), (16.0, 4.0), (16.0, 12.0)]);
            line(&[(4.0, 16.0), (9.0, 11.0)]);
            line(&[(16.0, 4.0), (11.0, 9.0)]);
        }
        Icon::Close => {
            line(&[(5.0, 5.0), (15.0, 15.0)]);
            line(&[(15.0, 5.0), (5.0, 15.0)]);
        }
        Icon::Help => {
            p.circle_stroke(center, size * 0.42, stroke);
            line(&[
                (7.5, 7.0),
                (9.0, 5.5),
                (11.5, 5.5),
                (13.0, 7.5),
                (10.0, 10.0),
                (10.0, 11.0),
            ]);
            p.circle_filled(at(10.0, 14.0), 0.85, tint);
        }
        Icon::Grid => {
            for (x, y) in [(3.0, 3.0), (12.0, 3.0), (3.0, 12.0), (12.0, 12.0)] {
                p.rect_stroke(
                    Rect::from_min_max(at(x, y), at(x + 5.0, y + 5.0)),
                    0,
                    stroke,
                    StrokeKind::Middle,
                );
            }
        }
        Icon::Fit => {
            for (x, y, dx, dy) in [
                (3.0, 3.0, 1.0, 1.0),
                (17.0, 3.0, -1.0, 1.0),
                (3.0, 17.0, 1.0, -1.0),
                (17.0, 17.0, -1.0, -1.0),
            ] {
                line(&[(x + 4.0 * dx, y), (x, y), (x, y + 4.0 * dy)]);
            }
        }
        Icon::Undo => {
            line(&[(7.0, 5.0), (3.0, 9.0), (7.0, 13.0)]);
            line(&[
                (3.0, 9.0),
                (12.0, 9.0),
                (16.0, 11.0),
                (16.0, 15.0),
                (12.0, 17.0),
                (9.0, 17.0),
            ]);
        }
        Icon::Reset => {
            let pts: Vec<_> = (0..22)
                .map(|i| {
                    let t = (i as f32 / 21.0 * 5.2) - 2.7;
                    center + vec2(t.cos(), t.sin()) * size * 0.34
                })
                .collect();
            p.add(Shape::line(pts, stroke));
            line(&[(3.0, 3.0), (3.0, 8.0), (8.0, 8.0)]);
        }
        Icon::Bolt => line(&[
            (12.0, 2.0),
            (4.0, 11.0),
            (9.0, 11.0),
            (8.0, 18.0),
            (16.0, 8.0),
            (11.0, 8.0),
            (12.0, 2.0),
        ]),
        Icon::Search => {
            p.circle_stroke(at(8.0, 8.0), 5.0 * size / 20.0, stroke);
            line(&[(12.0, 12.0), (17.0, 17.0)]);
        }
        Icon::File => {
            line(&[
                (5.0, 18.0),
                (3.0, 16.0),
                (3.0, 2.0),
                (12.0, 2.0),
                (17.0, 7.0),
                (17.0, 18.0),
                (13.0, 18.0),
            ]);
            line(&[(12.0, 2.0), (12.0, 7.0), (17.0, 7.0)]);
            line(&[(7.0, 11.0), (4.0, 14.0), (7.0, 17.0)]);
            line(&[(10.0, 11.0), (13.0, 14.0), (10.0, 17.0)]);
        }
        Icon::Cut => {
            line(&[(3.0, 6.0), (7.0, 10.0), (3.0, 14.0)]);
            line(&[(17.0, 6.0), (13.0, 10.0), (17.0, 14.0)]);
            line(&[(12.0, 3.0), (8.0, 17.0)]);
        }
        Icon::Play => {
            p.add(Shape::convex_polygon(
                vec![at(6.0, 3.0), at(16.0, 10.0), at(6.0, 17.0)],
                tint,
                Stroke::NONE,
            ));
        }
        Icon::Pause => {
            line(&[(7.0, 4.0), (7.0, 16.0)]);
            line(&[(13.0, 4.0), (13.0, 16.0)]);
        }
        Icon::Next => {
            line(&[(6.0, 5.0), (11.0, 10.0), (6.0, 15.0)]);
            line(&[(14.0, 5.0), (14.0, 15.0)]);
        }
        Icon::Prev => {
            line(&[(14.0, 5.0), (9.0, 10.0), (14.0, 15.0)]);
            line(&[(6.0, 5.0), (6.0, 15.0)]);
        }
        Icon::Chevron => line(&[(5.0, 8.0), (10.0, 13.0), (15.0, 8.0)]),
    }
}
pub fn label(p: &Painter, pos: Pos2, text: impl ToString, size: f32, tint: Color32) {
    p.text(
        pos,
        Align2::LEFT_CENTER,
        text,
        FontId::proportional(size),
        tint,
    );
}
pub fn button(
    ui: &mut Ui,
    controls: &mut Controls,
    id: &str,
    rect: Rect,
    caption: &str,
    glyph: Option<Icon>,
    p: Palette,
    active: bool,
    enabled: bool,
    primary: bool,
) -> Response {
    register(controls, id, rect);
    let response = ui.interact(
        rect,
        Id::new(id),
        if enabled {
            Sense::click()
        } else {
            Sense::hover()
        },
    );
    response.widget_info(|| WidgetInfo::labeled(WidgetType::Button, enabled, caption));
    let bg = if primary {
        p.primary
    } else if active || response.hovered() && enabled {
        p.hover
    } else {
        Color32::TRANSPARENT
    };
    ui.painter().rect_filled(rect, 5, bg);
    let tint = if !enabled {
        blend(p.subtle, p.bg, 0.45)
    } else if primary {
        p.primary_text
    } else if active {
        p.text
    } else {
        p.muted
    };
    if let Some(glyph) = glyph {
        icon(
            ui.painter(),
            if caption.is_empty() {
                rect.center()
            } else {
                pos2(rect.left() + 15.0, rect.center().y)
            },
            glyph,
            tint,
            16.0,
        );
    }
    if !caption.is_empty() {
        ui.painter().text(
            rect.center() + vec2(if glyph.is_some() { 8.0 } else { 0.0 }, 0.0),
            Align2::CENTER_CENTER,
            caption,
            FontId::proportional(12.0),
            tint,
        );
    }
    if enabled {
        response.on_hover_cursor(CursorIcon::PointingHand)
    } else {
        response
    }
}
pub fn node_color(directory: &str) -> Color32 {
    let hash = directory
        .encode_utf16()
        .fold(0_i32, |h, c| h.wrapping_mul(31).wrapping_add(c as i32));
    color(
        [0x71d8cb, 0xa6cb70, 0xaaa0ef, 0xeeac70, 0x6cb8e1, 0xe68fb9]
            [hash.unsigned_abs() as usize % 6],
    )
}
