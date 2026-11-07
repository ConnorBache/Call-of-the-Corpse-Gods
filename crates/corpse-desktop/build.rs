//! Build-only indexing of compact distribution plates; canonical RGBA PNGs remain authoring inputs.
use std::{env, fs, path::PathBuf, process::Command};
fn main() {
    let root = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap());
    let out = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    // Some Linux runtime images provide SDL2 without the development linker alias.
    if env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("linux") {
        let runtime = std::path::Path::new("/lib/x86_64-linux-gnu/libSDL2-2.0.so.0");
        if runtime.exists() {
            let alias = out.join("libSDL2.so");
            if !alias.exists() {
                #[cfg(unix)]
                std::os::unix::fs::symlink(runtime, &alias).expect("SDL2 linker alias");
            }
            println!("cargo:rustc-link-search=native={}", out.display());
        }
    }
    let assets = root.join("assets");
    println!("cargo:rerun-if-changed=assets/creatures-rgba");
    println!("cargo:rerun-if-changed=assets/creatures-runtime");
    println!("cargo:rerun-if-changed=assets/creature-runtime-manifest.json");
    println!("cargo:rerun-if-changed=assets/creature-alpha-manifest.json");
    let mut paths: Vec<_> = fs::read_dir(assets.join("creatures-runtime"))
        .unwrap()
        .map(|p| p.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "webp"))
        .collect();
    paths.sort_by(|a, b| a.file_stem().cmp(&b.file_stem()));
    let originals: std::collections::BTreeSet<_> = fs::read_dir(assets.join("creatures"))
        .unwrap()
        .map(|p| p.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "png"))
        .map(|p| p.file_stem().unwrap().to_os_string())
        .collect();
    let runtime: std::collections::BTreeSet<_> = paths
        .iter()
        .map(|p| p.file_stem().unwrap().to_os_string())
        .collect();
    assert_eq!(
        originals, runtime,
        "every authored plate must have an embedded RGBA resource"
    );
    let mut source = String::from("pub static CREATURES: &[(&str, &[u8])] = &[\n");
    for path in paths {
        let canonical = assets
            .join("creatures-rgba")
            .join(path.file_stem().unwrap())
            .with_extension("png");
        let bytes = fs::read(&canonical).expect("canonical RGBA PNG bytes");
        assert!(
            bytes.starts_with(b"\x89PNG\r\n\x1a\n") && bytes.get(25) == Some(&6),
            "canonical creature must be true RGBA PNG: {}",
            canonical.display()
        );
        let bytes = fs::read(&path).expect("compact resource bytes");
        assert!(
            bytes.starts_with(b"RIFF") && bytes.get(8..12) == Some(b"WEBP"),
            "runtime creature must be WebP: {}",
            path.display()
        );
        source.push_str(&format!(
            "({:?}, include_bytes!({:?})),\n",
            path.file_stem().unwrap().to_str().unwrap(),
            path
        ));
    }
    source.push_str("];\n");
    fs::write(out.join("creature_assets.rs"), source).unwrap();
    println!("cargo:rerun-if-changed=assets/corpse-gods.ico");
    println!("cargo:rerun-if-changed=assets/corpse-gods.manifest");
    println!("cargo:rerun-if-env-changed=WINDRES");
    if env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let resource = out.join("corpse-gods.rc");
        fs::write(
            &resource,
            format!(
                "1 ICON {:?}\n1 24 {:?}\n",
                assets
                    .join("corpse-gods.ico")
                    .to_string_lossy()
                    .replace('\\', "/"),
                assets
                    .join("corpse-gods.manifest")
                    .to_string_lossy()
                    .replace('\\', "/")
            ),
        )
        .unwrap();
        let object = out.join("corpse-gods-icon.o");
        let sdk_tool =
            root.join("../../../native-windows-sdk/sysroot/usr/bin/x86_64-w64-mingw32-windres");
        let tool = env::var_os("WINDRES")
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                if sdk_tool.exists() {
                    sdk_tool
                } else {
                    PathBuf::from("windres")
                }
            });
        let mut cmd = Command::new(&tool);
        // The local cross SDK is relocatable; its preprocessor must be selected explicitly.
        let cpp = tool.with_file_name("x86_64-w64-mingw32-cpp-posix");
        if cpp.exists() {
            cmd.arg(format!("--preprocessor={}", cpp.display()));
        }
        let status = cmd
            .args(["-i"])
            .arg(resource)
            .args(["-o"])
            .arg(&object)
            .args(["-O", "coff"])
            .status()
            .expect("Windows icon resource compiler (set WINDRES)");
        assert!(status.success(), "Windows icon resource compilation failed");
        println!("cargo:rustc-link-arg={}", object.display());
    }
}
