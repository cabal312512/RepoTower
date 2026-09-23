use serde_json::{json, Value};
use std::{
    io::Write,
    path::PathBuf,
    process::{Command, Stdio},
};

#[test]
fn persistent_json_lines_handles_errors_and_preserves_latest_graph() {
    let mut process = Command::new(env!("CARGO_BIN_EXE_repotower-core"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/linear");
    let requests = [
        json!({"id": 1, "command": "analyze", "path": root}),
        json!({"id": "missing-path", "command": "analyze"}),
        json!({"id": 3, "command": "impact", "moduleId": "foundation.ts", "excluded": []}),
        json!({"id": 4, "command": "impact", "moduleId": "foundation.ts", "excluded": ["service.ts"]}),
    ];
    let mut input = process.stdin.take().unwrap();
    for request in requests {
        writeln!(input, "{request}").unwrap();
    }
    drop(input);
    let output = process.wait_with_output().unwrap();
    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    let responses: Vec<Value> = String::from_utf8(output.stdout)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert!(responses
        .iter()
        .any(|r| r["type"] == "progress" && r["id"] == 1));
    let finals: Vec<_> = responses
        .iter()
        .filter(|r| r["type"] != "progress")
        .collect();
    assert_eq!(finals.len(), 4);
    assert_eq!(finals[0]["result"]["totalModules"], 3);
    assert_eq!(finals[1]["type"], "error");
    assert_eq!(finals[1]["id"], "missing-path");
    assert_eq!(finals[2]["result"]["totalAffected"], 2);
    assert_eq!(finals[3]["result"]["totalAffected"], 0);
}
