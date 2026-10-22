//! Embeds the fonts of the optional craft-fonts build input (`CRAFT_FONTS_DIR`,
//! <https://github.com/685ks/craft-fonts>) as `CRAFT_FONTS`, through the shared
//! `craft-fonts-build` helper (legacy mode: `src/craft.rs` defines `CraftFont`). Unset,
//! `CRAFT_FONTS` is empty; `CRAFT_FONTS_REQUIRED=1` turns a bad checkout into a build error. It
//! only reads the local checkout (no network). On wasm32 it embeds only the UI faces (BIZ
//! UDPGothic Regular and any `Arab`, `Telu`, `Hebr` or `Thai` face), to keep the web build within
//! hosting limits (Cloudflare Pages: 25 MiB per file).

use std::path::PathBuf;

use craft_fonts_build::{Builder, DIR_ENV, Entry, REQUIRED_ENV};

/// Family-name prefixes of the faces AGENTS.md §1.1 forbids: Adobe's Source families and Noto
/// CJK, which is Source Han under another name. The test in `src/craft.rs` has the same list.
const BARRED_FAMILIES: [&str; 5] = ["Source Han", "Source Serif", "Source Sans", "Noto Sans CJK", "Noto Serif CJK"];

/// Whether `e` is a barred family (never embedded, whatever its licence).
fn barred(e: &Entry) -> bool {
    BARRED_FAMILIES.iter().any(|b| e.family.starts_with(b))
}

/// The faces the web build keeps: the Japanese UI face and the small Arabic, Telugu, Hebrew and
/// Thai interface faces.
fn web_face(e: &Entry) -> bool {
    let script = |s: &str| e.scripts.iter().any(|x| x == s);
    (e.family == "BIZ UDPGothic" && e.style == "Regular") || ["Arab", "Telu", "Hebr", "Thai"].into_iter().any(script)
}

fn main() {
    println!("cargo::rerun-if-env-changed={DIR_ENV}");
    println!("cargo::rerun-if-env-changed={REQUIRED_ENV}");
    let dir = std::env::var_os(DIR_ENV).filter(|d| !d.is_empty()).map(PathBuf::from);
    let required = std::env::var_os(REQUIRED_ENV).is_some();
    let wasm = std::env::var("CARGO_CFG_TARGET_ARCH").is_ok_and(|a| a == "wasm32");
    // The web selection is by script, so it is made here, with the target passed as native.
    let builder = Builder::new().legacy(true).include(move |e| {
        if barred(e) {
            println!("cargo::warning=craft-fonts: not embedding {} {} (AGENTS.md §1.1)", e.family, e.style);
            return false;
        }
        !wasm || web_face(e)
    });
    let mut rerun = Vec::new();
    let src = match builder.generate(dir.as_deref(), false, &mut rerun) {
        Ok(src) => src,
        Err(e) => {
            let shown = dir.as_deref().map(|d| d.display().to_string()).unwrap_or_default();
            if required {
                println!("cargo::error={DIR_ENV}={shown}: {e}");
            } else {
                println!("cargo::warning=building without craft-fonts: {DIR_ENV}={shown}: {e}");
            }
            builder.generate(None, false, &mut Vec::new()).unwrap_or_else(|_| "pub static CRAFT_FONTS: &[CraftFont] = &[];\n".into())
        }
    };
    for path in rerun {
        println!("cargo::rerun-if-changed={}", path.display());
    }
    let out = PathBuf::from(std::env::var_os("OUT_DIR").unwrap_or_default()).join("craft_fonts.rs");
    // Unchanged output keeps its mtime, so an unchanged build does not recompile this crate.
    if std::fs::read_to_string(&out).is_ok_and(|old| old == src) {
        return;
    }
    if let Err(e) = std::fs::write(&out, src) {
        println!("cargo::error=writing {}: {e}", out.display());
    }
}
