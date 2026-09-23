use std::{env, fs, path::Path};

fn collect(root: &Path, dir: &Path, entries: &mut Vec<String>) {
    let mut paths: Vec<_> = fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    paths.sort();
    for path in paths {
        if path.is_dir() {
            collect(root, &path, entries);
        } else {
            let relative = path
                .strip_prefix(root)
                .unwrap()
                .to_str()
                .unwrap()
                .replace('\\', "/");
            entries.push(format!(
                "({relative:?}, include_bytes!({:?})),",
                path.to_str().unwrap()
            ));
        }
    }
}

fn main() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let font = root.join(".cache/native-assets/NotoSansCJKsc-Regular.otf.gz");
    assert!(
        font.exists(),
        "Run `node scripts/prepare-native-assets.mjs` before building the desktop app."
    );
    println!("cargo:rerun-if-changed={}", font.display());
    println!("cargo:rustc-env=REPOTOWER_FONT={}", font.display());
    let mut entries = Vec::new();
    for example in ["demo-project", "language-samples"] {
        let path = root.join("fixtures").join(example);
        println!("cargo:rerun-if-changed={}", path.display());
        collect(&root.join("fixtures"), &path, &mut entries);
    }
    fs::write(
        Path::new(&env::var("OUT_DIR").unwrap()).join("examples.rs"),
        format!(
            "pub static EXAMPLES: &[(&str, &[u8])] = &[\n{}\n];",
            entries.join("\n")
        ),
    )
    .unwrap();
}
