"""Measure the shipped Rust JSON-lines sidecar against deterministic local fixtures."""
import json
import os
from pathlib import Path
import subprocess
import time

ROOT = Path(__file__).resolve().parent.parent
EXE = ROOT / "target" / "release" / ("repotower-core.exe" if os.name == "nt" else "repotower-core")
results = []
for fixture, module_id in [("demo-project", "src/foundation/logger.ts"), ("generated-1000", "module-00000.ts")]:
    process = subprocess.Popen([str(EXE)], stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, encoding="utf-8")
    start = time.perf_counter()
    process.stdin.write(json.dumps({"id": 1, "command": "analyze", "path": str(ROOT / "fixtures" / fixture)}) + "\n")
    process.stdin.flush()
    while True:
        line = process.stdout.readline()
        if not line: raise RuntimeError(process.stderr.read())
        message = json.loads(line)
        if message["type"] == "error": raise RuntimeError(message["error"])
        if message["type"] == "result": break
    analysis_ms = (time.perf_counter() - start) * 1000
    analysis = message["result"]
    start = time.perf_counter()
    process.stdin.write(json.dumps({"id": 2, "command": "impact", "moduleId": module_id, "excluded": []}) + "\n")
    process.stdin.flush()
    impact = json.loads(process.stdout.readline())["result"]
    impact_ms = (time.perf_counter() - start) * 1000
    process.stdin.close()
    process.wait(timeout=10)
    result = {"fixture": fixture, "modules": analysis["totalModules"], "edges": analysis["totalEdges"], "cycles": analysis["cycleCount"], "analysisMs": round(analysis_ms, 2), "impactMs": round(impact_ms, 2), "affected": impact["totalAffected"], "cascadeDepth": impact["maxCascadeDepth"], "score": analysis["stabilityScore"]}
    results.append(result)
    print(json.dumps(result))
output = ROOT / ".cache" / "core-benchmark.json"
output.parent.mkdir(parents=True, exist_ok=True)
output.write_text(json.dumps(results, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
