//! Loading a CJK-capable font into egui.
//!
//! egui ships Latin-only faces (Ubuntu-Light, NotoEmoji, Hack), so every
//! Chinese label rendered as a hollow box (the "tofu" glyph) until one was
//! added. This module finds a system font that has Han glyphs and registers it
//! as the **last** fallback in both families, so Latin text keeps egui's own
//! font (better hinting at small sizes) and only characters it cannot draw
//! come from the system face.
//!
//! The launcher does not bundle a font: a CJK face is several megabytes, and
//! shipping one would also mean shipping its licence. Every desktop this runs
//! on already has one, so it is loaded from the usual paths per platform.

use std::path::PathBuf;
use std::sync::Arc;

/// System fonts to try, best first.
///
/// The order matters: on Windows `msyh.ttc` (Microsoft YaHei) covers simplified
/// Chinese and is present on every install since Vista; `simhei.ttf` and
/// `simsun.ttc` are the fallbacks. macOS has `PingFang` and `Hiragino`. Linux
/// desktops vary the most, so several Noto CJK weights are listed.
fn candidates() -> Vec<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        let root = std::env::var_os("WINDIR").map_or_else(
            || PathBuf::from("C:/Windows"),
            PathBuf::from,
        );
        let fonts = root.join("Fonts");
        ["msyh.ttc", "msyh.ttf", "simhei.ttf", "simsun.ttc", "Deng.ttf"]
            .iter()
            .map(|f| fonts.join(f))
            .collect()
    }
    #[cfg(target_os = "macos")]
    {
        [
            "/System/Library/Fonts/PingFang.ttc",
            "/System/Library/Fonts/Hiragino Sans GB.ttc",
            "/Library/Fonts/Arial Unicode.ttf",
        ]
        .iter()
        .map(PathBuf::from)
        .collect()
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        [
            "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
            "/usr/share/fonts/opentype/noto/NotoSansCJKsc-Regular.otf",
            "/usr/share/fonts/truetype/noto/NotoSansCJK-Regular.ttc",
            "/usr/share/fonts/truetype/wqy/wqy-zenhei.ttc",
            "/usr/share/fonts/truetype/arphic/uming.ttc",
        ]
        .iter()
        .map(PathBuf::from)
        .collect()
    }
}

/// Read the first candidate that exists and is non-empty.
fn load_bytes() -> Option<(PathBuf, Vec<u8>)> {
    for path in candidates() {
        match std::fs::read(&path) {
            Ok(bytes) if bytes.len() > 1024 => return Some((path, bytes)),
            _ => {}
        }
    }
    None
}

/// Register a system CJK font as the last fallback for both families.
///
/// Returns the path that was loaded, or `None` when no candidate exists — in
/// which case the interface keeps egui's own fonts and Chinese would still show
/// as boxes, so the caller should say so once instead of failing silently.
pub fn install(ctx: &egui::Context) -> Option<PathBuf> {
    let (path, bytes) = load_bytes()?;
    let mut fonts = egui::FontDefinitions::default();
    // A `.ttc` is a collection; epaint's font parser (ab_glyph) reads the first
    // face, which for msyh/PingFang/Noto CJK is the one with Han coverage.
    fonts
        .font_data
        .insert("system-cjk".to_owned(), Arc::new(egui::FontData::from_owned(bytes)));
    for family in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
        fonts
            .families
            .entry(family)
            .or_default()
            .push("system-cjk".to_owned());
    }
    ctx.set_fonts(fonts);
    Some(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn candidates_are_absolute_and_non_empty() {
        let c = candidates();
        assert!(!c.is_empty(), "每个平台都应至少列出一个候选字体");
        for p in &c {
            assert!(p.is_absolute(), "候选路径应为绝对路径: {}", p.display());
        }
    }

    /// 这台机器上应该能找到中文字体，否则界面会全是方块。
    #[test]
    fn a_cjk_font_is_available_on_this_machine() {
        match load_bytes() {
            Some((p, b)) => eprintln!("找到中文字体: {} ({} 字节)", p.display(), b.len()),
            None => panic!("没找到任何中文字体，界面会显示成方块"),
        }
    }
}
