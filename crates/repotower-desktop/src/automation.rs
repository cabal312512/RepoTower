//! Explicit, opt-in GUI verification through stdin. Normal launches expose no listener.
use eframe::egui::*;
use serde_json::{json, Value};
use std::{
    io::{BufRead, Write},
    sync::mpsc::{self, Receiver},
};

pub struct Automation {
    pub receiver: Receiver<Value>,
    pub pending: Option<Value>,
    pub screenshot: Option<Value>,
}
impl Automation {
    pub fn start() -> Self {
        let (sender, receiver) = mpsc::channel();
        std::thread::spawn(move || {
            for line in std::io::stdin().lock().lines().map_while(Result::ok) {
                match serde_json::from_str(&line) {
                    Ok(value) => {
                        if sender.send(value).is_err() {
                            break;
                        }
                    }
                    Err(e) => Self::reply(json!({"error":e.to_string()})),
                }
            }
        });
        Self {
            receiver,
            pending: None,
            screenshot: None,
        }
    }
    pub fn reply(value: Value) {
        let mut out = std::io::stdout().lock();
        let _ = writeln!(out, "{value}");
        let _ = out.flush();
    }
    pub fn inject(command: &Value, input: &mut RawInput) {
        let point = || {
            pos2(
                command["x"].as_f64().unwrap_or(0.0) as f32,
                command["y"].as_f64().unwrap_or(0.0) as f32,
            )
        };
        let modifiers = Modifiers {
            ctrl: command["ctrl"].as_bool().unwrap_or(false),
            command: command["ctrl"].as_bool().unwrap_or(false),
            shift: command["shift"].as_bool().unwrap_or(false),
            ..Default::default()
        };
        match command["action"].as_str().unwrap_or("") {
            "move" => input.events.push(Event::PointerMoved(point())),
            "button" => {
                input.events.push(Event::PointerMoved(point()));
                input.events.push(Event::PointerButton {
                    pos: point(),
                    button: if command["button"] == "right" {
                        PointerButton::Secondary
                    } else {
                        PointerButton::Primary
                    },
                    pressed: command["down"].as_bool().unwrap_or(false),
                    modifiers,
                });
            }
            "wheel" => {
                input.events.push(Event::PointerMoved(point()));
                input.events.push(Event::MouseWheel {
                    unit: MouseWheelUnit::Point,
                    delta: vec2(0.0, command["delta"].as_f64().unwrap_or(0.0) as f32),
                    modifiers,
                });
            }
            "text" => input
                .events
                .push(Event::Text(command["text"].as_str().unwrap_or("").into())),
            "key" => {
                if let Some(key) = Key::from_name(command["key"].as_str().unwrap_or("")) {
                    input.events.push(Event::Key {
                        key,
                        physical_key: None,
                        pressed: command["down"].as_bool().unwrap_or(true),
                        repeat: false,
                        modifiers,
                    });
                }
                input.modifiers = modifiers;
            }
            "drop" => input.dropped_files.push(DroppedFile {
                path: command["path"].as_str().map(std::path::PathBuf::from),
                ..Default::default()
            }),
            _ => {}
        }
    }
}
