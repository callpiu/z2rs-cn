//! SCRATCH: 端到端验证 HD 材质包能作用于汉化 ROM 的高页号字形。
//!
//! 用真实汉化 ROM 的 CHR 生成模板包（页 32-127 是中文），再用合成帧验证
//! `HdPack::lookup` 能解析这些页 —— 修复前索引只到 32 页，会静默返回 None。

use std::path::Path;

use z2_render::{template_from_chr, HdPack, TemplateOptions};

const ROM: &str =
    r"D:/z2rs-中文版/Zelda II - The Adventure of Link (U) [20260806_NOKOH].nes";

fn chr_rom() -> Option<Vec<u8>> {
    let p = Path::new(ROM);
    if !p.exists() {
        eprintln!("跳过：找不到汉化版 ROM");
        return None;
    }
    let b = std::fs::read(p).unwrap();
    // iNES: 16 字节头 + PRG(16 x 16KiB) + CHR
    let prg = usize::from(b[4]) * 0x4000;
    Some(b[16 + prg..].to_vec())
}

#[test]
fn hd_pack_covers_translated_glyph_pages() {
    let Some(chr) = chr_rom() else { return };
    eprintln!("CHR 长度 {} = {} 页", chr.len(), chr.len() / 4096);
    assert_eq!(chr.len(), 512 * 1024);

    // 汉化 ROM 的中文字形在 4KB 页 32-127（实测简繁各占一半）。
    let pages: Vec<u8> = (32u8..128).collect();
    let out = template_from_chr(
        &chr,
        &TemplateOptions {
            name: "translated",
            scale: 1,
            pages: &pages,
            colors: None,
        },
    )
    .expect("为 32-127 页生成模板");
    assert_eq!(out.sheets.len(), 96, "应生成 96 页模板");

    let pack = HdPack::from_files(&out.into_files()).expect("加载模板包");
    eprintln!("pack scale={}", pack.scale());

    // 每个高页号都必须能解析出图块（这正是修复前静默失败的地方）。
    let mut resolved_pages = 0;
    for &page in &pages {
        let any = (0..=255u8).any(|t| pack.lookup(page, t, [0x0F, 0x16, 0x30]).is_some());
        if any {
            resolved_pages += 1;
        }
    }
    eprintln!("可解析的高页号: {resolved_pages} / {}", pages.len());
    assert_eq!(
        resolved_pages,
        pages.len(),
        "有高页号解析不到图块（HD 材质包对中文字形失效）"
    );

    // 注意：这里不断言低页号。模板只为**有像素**的图块建条目，而原版页
    // （0/5/31）在测试用颜色键 `0x0F,0x16,0x30` 下可能全是空白 tile。
    // 本测试要锁的是高页号（中文字形）能被索引到 —— 这正是修复前失效的点。
}
