"""Generate deterministic, local source-only performance fixtures; no dependencies or network.

The small built-in demo is curated in demo-project/ and must not be overwritten.
python -I -B fixtures/generate.py large --count 1000
"""
import argparse
from pathlib import Path

ROOT = Path(__file__).resolve().parent

def write(path, content):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(content, encoding="utf-8")

def demo():
    root = ROOT / "demo-project"
    # Retain the old command as a harmless pointer for existing instructions.
    print(f"Built-in demo is curated at {root}; no files changed.")

def large(count):
    root = ROOT / f"generated-{count}"
    for index in range(count):
        lines = [f"// Performance fixture module {index}"]
        for distance in [1, 7, 31]:
            if index >= distance: lines.append(f"import './module-{index - distance:05}.ts';")
        lines.append(f"export const value = {index};")
        write(root / f"module-{index:05}.ts", "\n".join(lines) + "\n")
    print(f"Generated {count} modules at {root}")

if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("mode", choices=["demo", "large"])
    parser.add_argument("--count", type=int, default=1000)
    args = parser.parse_args()
    if not 1 <= args.count <= 10000: parser.error("--count must be between 1 and 10000")
    demo() if args.mode == "demo" else large(args.count)
