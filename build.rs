// Embeds every game asset (sprites, music, sfx, fonts) into the binary.
// Generates a table of (relative path, bytes) so code can look assets up by name,
// and so new files dropped into assets/ are picked up without code changes.
use std::{env, fs, path::Path};

fn walk(dir: &Path, root: &Path, out: &mut Vec<(String, String)>) {
    let Ok(rd) = fs::read_dir(dir) else { return };
    let mut entries: Vec<_> = rd.flatten().collect();
    entries.sort_by_key(|e| e.path());
    for e in entries {
        let p = e.path();
        if p.is_dir() {
            walk(&p, root, out);
        } else if let Some(ext) = p.extension().and_then(|x| x.to_str()) {
            if matches!(ext, "png" | "json" | "ogg" | "wav" | "txt") {
                let rel = p.strip_prefix(root).unwrap().to_string_lossy().replace('\\', "/");
                out.push((rel, p.canonicalize().unwrap().to_string_lossy().to_string()));
            }
        }
    }
}

fn main() {
    // GLINT_ASSETS lets a separate checkout (e.g. a git worktree) embed the live assets folder
    // of the main checkout, so code agents see art as soon as artists save it.
    println!("cargo:rerun-if-env-changed=GLINT_ASSETS");
    let root = match env::var("GLINT_ASSETS") {
        Ok(p) if !p.is_empty() => std::path::PathBuf::from(p),
        _ => Path::new(env!("CARGO_MANIFEST_DIR")).join("assets"),
    };
    let mut files = Vec::new();
    for sub in ["sprites", "music", "sfx", "fonts", "data"] {
        let d = root.join(sub);
        println!("cargo:rerun-if-changed={}", d.display());
        walk(&d, &root, &mut files);
    }
    println!("cargo:rerun-if-changed={}", root.display());
    let mut src = String::from("pub static ASSETS: &[(&str, &[u8])] = &[\n");
    for (rel, abs) in &files {
        src.push_str(&format!("    ({:?}, include_bytes!({:?})),\n", rel, abs));
    }
    src.push_str("];\n");
    let out = Path::new(&env::var("OUT_DIR").unwrap()).join("assets_gen.rs");
    fs::write(out, src).unwrap();
}
