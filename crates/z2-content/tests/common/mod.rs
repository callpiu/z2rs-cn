//! Shared ROM gate for the ROM-gated tests (test-only).
//!
//! `Z2_ROM` counts as absent when it is unset, empty (or all whitespace),
//! or does not name an existing file: the test prints one `skipping` line
//! and returns instead of panicking. An existing file that is not the
//! verified ROM still fails loudly.

use std::path::PathBuf;

/// The reference ROM path (`$Z2_ROM`), when it names an existing file.
pub fn rom_path() -> Option<PathBuf> {
    match std::env::var("Z2_ROM") {
        Ok(v) if !v.trim().is_empty() => {
            let p = PathBuf::from(v);
            if p.is_file() {
                Some(p)
            } else {
                eprintln!(
                    "skipping ROM-gated test: Z2_ROM ({}) is not an existing file",
                    p.display()
                );
                None
            }
        }
        _ => {
            eprintln!("skipping ROM-gated test: Z2_ROM not set to an existing file");
            None
        }
    }
}

/// The header-stripped, verified ROM body, read in place.
pub fn body() -> Option<Vec<u8>> {
    let p = rom_path()?;
    Some(z2_assets::rom::open_at(&p).expect("Z2_ROM must name the verified ROM"))
}
