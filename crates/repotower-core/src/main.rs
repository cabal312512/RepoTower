use repotower_core::{analyze, impact, model::RepositoryAnalysis};
use serde::Deserialize;
use serde_json::{json, Value};
use std::io::{self, BufRead, Write};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Request {
    id: Value,
    command: String,
    path: Option<String>,
    module_id: Option<String>,
    #[serde(default)]
    excluded: Vec<String>,
}

fn emit(output: &mut impl Write, value: Value) -> io::Result<()> {
    serde_json::to_writer(&mut *output, &value)?;
    output.write_all(b"\n")?;
    output.flush()
}

fn main() {
    // stdout is protocol-only. Neither analysis nor impact ever runs repository code.
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut output = stdout.lock();
    let mut last: Option<RepositoryAnalysis> = None;
    for line in stdin.lock().lines() {
        let line = match line {
            Ok(line) => line,
            Err(error) => {
                eprintln!("IPC input error: {error}");
                break;
            }
        };
        if line.trim().is_empty() {
            continue;
        }
        if line.len() > 2 * 1024 * 1024 {
            if emit(
                &mut output,
                json!({"id": null, "type": "error", "error": "请求超过大小限制。"}),
            )
            .is_err()
            {
                break;
            }
            continue;
        }
        let request: Request = match serde_json::from_str(&line) {
            Ok(request) => request,
            Err(error) => {
                let id = serde_json::from_str::<Value>(&line)
                    .ok()
                    .and_then(|value| value.get("id").cloned())
                    .unwrap_or(Value::Null);
                if emit(
                    &mut output,
                    json!({"id": id, "type": "error", "error": format!("无效请求：{error}")}),
                )
                .is_err()
                {
                    break;
                }
                continue;
            }
        };
        let result: Result<Value, String> = match request.command.as_str() {
            "analyze" => {
                if let Some(path) = request.path {
                    let analysis = analyze(path, |progress| {
                        let _ = emit(
                            &mut output,
                            json!({"id": request.id, "type": "progress", "progress": progress}),
                        );
                    });
                    match analysis {
                        Ok(analysis) => {
                            let value =
                                serde_json::to_value(&analysis).map_err(|error| error.to_string());
                            last = Some(analysis);
                            value
                        }
                        Err(error) => Err(error),
                    }
                } else {
                    Err("缺少项目路径。".into())
                }
            }
            "impact" => {
                if let (Some(analysis), Some(module_id)) = (&last, request.module_id) {
                    impact(&analysis.modules, &module_id, &request.excluded).and_then(|result| {
                        serde_json::to_value(result).map_err(|error| error.to_string())
                    })
                } else {
                    Err("请先分析项目并指定模块。".into())
                }
            }
            _ => Err("未知命令。支持 analyze 和 impact。".into()),
        };
        let response = match result {
            Ok(result) => json!({"id": request.id, "type": "result", "result": result}),
            Err(error) => json!({"id": request.id, "type": "error", "error": error}),
        };
        if emit(&mut output, response).is_err() {
            break;
        }
    }
}
