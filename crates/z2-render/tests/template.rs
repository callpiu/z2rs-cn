//! Template sheets from (synthetic) CHR round-trip through the pack loader.

mod common;

use common::{contains, synthetic_chr};
use z2_render::chr::{chr_page, upscale_tile_rgba};
use z2_render::template::{MARKER_FILE, TEMPLATE_TEXT_KEY};
use z2_render::{
    decode_png_rgba, template_from_chr, tile_subpixels, HdPack, MasterPalette, TemplateError,
    TemplateOptions,
};

fn opts<'a>(pages: &'a [u8], scale: u32) -> TemplateOptions<'a> {
    TemplateOptions {
        name: "synthetic",
        scale,
        pages,
        colors: None,
    }
}

#[test]
fn template_round_trips_through_pack_loader() {
    let chr = synthetic_chr(32);
    let pages = [0u8, 5, 31];
    for scale in [1, 2, 3] {
        let out = template_from_chr(&chr, &opts(&pages, scale)).unwrap();
        assert_eq!(out.sheets.len(), 3);
        let pack = HdPack::from_files(&out.into_files()).unwrap();
        assert_eq!(pack.scale(), scale);
        assert_eq!(pack.tile_count(), 3 * 256);
        assert_eq!(pack.variant_count(), 0);
        for &page in &pages {
            let bytes = chr_page(&chr, page).unwrap();
            for t in 0..=255u8 {
                let cell = pack
                    .lookup(page, t, [0x0F, 0x16, 0x30])
                    .unwrap_or_else(|| panic!("page {page} tile {t} unresolved"));
                let want = upscale_tile_rgba(
                    &tile_subpixels(bytes, t),
                    z2_render::template::GRAY_RAMP,
                    scale,
                );
                assert_eq!(
                    pack.cell_pixels(cell).to_vec(),
                    want,
                    "page {page} tile {t} scale {scale}"
                );
            }
        }
        assert_eq!(pack.lookup(1, 0, [0; 3]), None, "page not templated");
    }
}

#[test]
fn template_files_carry_legal_markers() {
    let chr = synthetic_chr(2);
    let out = template_from_chr(&chr, &opts(&[1, 0, 1], 2)).unwrap();
    let names: Vec<&str> = out.sheets.iter().map(|(n, _)| n.as_str()).collect();
    assert_eq!(names, ["sheets/page00.png", "sheets/page01.png"]);
    let manifest: serde_json::Value = serde_json::from_str(&out.pack_json).unwrap();
    assert_eq!(manifest["format"], "z2rs-hdpack");
    assert_eq!(manifest["scale"], 2);
    for (name, png) in &out.sheets {
        assert!(contains(png, TEMPLATE_TEXT_KEY.as_bytes()), "{name}");
        let img = decode_png_rgba(png).unwrap();
        assert_eq!((img.width, img.height), (256, 256));
    }
    let files = out.into_files();
    assert_eq!(files[0].0, "pack.json");
    assert_eq!(files.last().unwrap().0, MARKER_FILE);
}

#[test]
fn explicit_colors_use_nes_palette() {
    let chr = synthetic_chr(1);
    let colors = [0x16, 0x27, 0x30];
    let mut o = opts(&[0], 1);
    o.colors = Some(colors);
    let pack = HdPack::from_files(&template_from_chr(&chr, &o).unwrap().into_files()).unwrap();
    let sub = tile_subpixels(&chr, 0);
    let cell = pack.cell_pixels(pack.lookup(0, 0, [0; 3]).unwrap());
    for (i, &s) in sub.iter().enumerate() {
        let px = cell.pixel(i as u32 % 8, i as u32 / 8);
        if s == 0 {
            assert_eq!(px[3], 0);
        } else {
            let [r, g, b] = MasterPalette::NES.rgb(colors[usize::from(s) - 1]);
            assert_eq!(px, [r, g, b, 255]);
        }
    }
}

#[test]
fn blank_chr_tiles_stay_unregistered() {
    let mut chr = synthetic_chr(1);
    chr[16 * 9..16 * 10].fill(0);
    let pack = HdPack::from_files(
        &template_from_chr(&chr, &opts(&[0], 1))
            .unwrap()
            .into_files(),
    )
    .unwrap();
    assert_eq!(pack.tile_count(), 255);
    assert_eq!(pack.lookup(0, 9, [0; 3]), None);
}

#[test]
fn template_errors() {
    let chr = synthetic_chr(1);
    assert_eq!(
        template_from_chr(&chr, &opts(&[0], 0)),
        Err(TemplateError::Scale(0))
    );
    assert_eq!(
        template_from_chr(&chr, &opts(&[0], 9)),
        Err(TemplateError::Scale(9))
    );
    assert_eq!(
        template_from_chr(&chr, &opts(&[], 1)),
        Err(TemplateError::NoPages)
    );
    // A page number is accepted up to `MAX_CHR_PAGES - 1`; whether the CHR
    // image actually has it is checked separately (`ChrTooShort`). 32 is a
    // real page on a translated (MMC5) ROM's 512 KiB CHR image.
    assert_eq!(
        template_from_chr(&chr, &opts(&[255], 1)),
        Err(TemplateError::Page(255))
    );
    assert_eq!(
        template_from_chr(&chr, &opts(&[1], 1)),
        Err(TemplateError::ChrTooShort { page: 1, len: 4096 })
    );
    // Page 32 is in range but this 2-page fixture does not hold it.
    assert_eq!(
        template_from_chr(&chr, &opts(&[32], 1)),
        Err(TemplateError::ChrTooShort {
            page: 32,
            len: chr.len()
        })
    );
}

/// A translated (MMC5) ROM has a 512 KiB CHR image = 128 pages, and its
/// Chinese glyphs live in pages 32-127. Both the template generator and the
/// pack loader used to cap at 32, so HD art for those glyphs was either
/// rejected outright or silently dropped at lookup time.
#[test]
fn high_pages_of_a_512k_image_are_addressable() {
    // 128 pages x 4 KiB = 512 KiB, the translated ROM's CHR size.
    let chr = synthetic_chr(128);
    assert_eq!(chr.len(), 512 * 1024);

    // Templates can be generated for the glyph pages.
    let pages = [32u8, 64, 127];
    let out = template_from_chr(&chr, &opts(&pages, 1)).unwrap();
    assert_eq!(out.sheets.len(), 3);
    let pack = HdPack::from_files(&out.into_files()).unwrap();

    // ...and the pack resolves them (this is the lookup that used to return
    // `None` and fall back to the original art).
    for &page in &pages {
        let resolved = (0..8u8).any(|t| pack.lookup(page, t, [0x0F, 0x16, 0x30]).is_some());
        assert!(resolved, "page {page} resolved no tiles at all");
    }
    // A page just past the image is in range but empty — except through the
    // traditional-page fallback, which maps it to page 64 (see below).
    assert!(
        pack.lookup(200, 0, [0x0F, 0x16, 0x30]).is_none(),
        "page 200 has no cell and no simplified counterpart 64 pages below"
    );
}

/// The translated ROM's simplified/traditional switch shifts every CHR page by
/// `+64` without changing the artwork, so a pack built for the simplified
/// layout must still resolve the traditional pages.
///
/// Regression test: before the fallback, switching to traditional moved the
/// glyph pages from 26 to 90 and the terrain from 3 to 67, and **every** HD
/// lookup missed (0% coverage instead of 63%).
#[test]
fn traditional_pages_fall_back_to_simplified_art() {
    let chr = synthetic_chr(128);
    // Only the simplified pages get HD art.
    let pages = [0u8, 3, 26];
    let out = template_from_chr(&chr, &opts(&pages, 1)).unwrap();
    let pack = HdPack::from_files(&out.into_files()).unwrap();
    let cols = [0x0F, 0x16, 0x30];

    for &page in &pages {
        let direct = (0..8u8).any(|t| pack.lookup(page, t, cols).is_some());
        assert!(direct, "simplified page {page} should resolve");

        let trad = page + 64;
        let via_fallback = (0..8u8).any(|t| pack.lookup(trad, t, cols).is_some());
        assert!(
            via_fallback,
            "traditional page {trad} (simplified {page} + 64) resolved nothing"
        );
    }
}
