//! Inspect real widget output, including groups below the normal viewport.
use z2_launcher::{i18n::Lang, rando_ui};
use z2_rando::flags::{Flags, Preset};

fn has_cjk(s: &str) -> bool {
    s.chars().any(|c| ('\u{4e00}'..='\u{9fff}').contains(&c))
}

fn render(lang: Lang, preset: Preset) -> Vec<String> {
    let mut f: Flags = preset.flags();
    let before = f.clone();
    let ctx = egui::Context::default();
    ctx.enable_accesskit();
    let mut texts = Vec::new();
    for group in 0..12 {
      for frame in 0..3 {
        let mut out = ctx.run_ui(egui::RawInput {
            time: Some((group * 3 + frame) as f64),
            screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1200.0, 20000.0))),
            ..Default::default()
        }, |ui| {
            match group {
                0 => rando_ui::start::ui(ui, lang, &mut f.start),
                1 => rando_ui::overworld::ui(ui, lang, &mut f.overworld),
                2 => rando_ui::palaces::ui(ui, lang, &mut f.palaces),
                3 => rando_ui::items::ui(ui, lang, &mut f.items),
                4 => rando_ui::enemies::ui(ui, lang, &mut f.enemies),
                5 => rando_ui::stats::ui(ui, lang, &mut f.stats),
                6 => rando_ui::spells::ui(ui, lang, &mut f.spells),
                7 => rando_ui::drops::ui(ui, lang, &mut f.drops),
                8 => rando_ui::hints::ui(ui, lang, &mut f.hints),
                9 => rando_ui::towns::ui(ui, lang, &mut f.towns),
                10 => rando_ui::qol::ui(ui, lang, &mut f.qol),
                _ => {
                    rando_ui::cosmetic::sprite_patch(ui, lang, &mut String::new());
                    rando_ui::cosmetic::ui(ui, lang, &mut f.cosmetic);
                }
            }
        });
        if let Some(update) = out.platform_output.accesskit_update.take() {
            for (_, node) in update.nodes {
                if let Some(s) = node.label().filter(|s| !s.is_empty()) {
                    texts.push(s.to_owned());
                }
            }
        }
        out.textures_delta.clear();
      }
    }
    assert_eq!(f, before, "drawing or switching language must not change randomizer flags");
    texts
}

#[test]
fn every_randomizer_group_is_chinese_in_every_preset() {
    for preset in Preset::ALL {
        let texts = render(Lang::Zh, *preset);
        assert!(texts.len() > 150, "missing rendered groups: {}", texts.len());
        let left: Vec<_> = texts.iter().filter(|s| !has_cjk(s) && s.chars().any(|c| c.is_ascii_alphabetic()) && !s.starts_with('$')).collect();
        assert!(left.is_empty(), "{preset:?}: untranslated {left:?}");
    }
}

#[test]
fn english_randomizer_has_no_chinese() {
    let texts = render(Lang::En, Preset::Standard);
    assert!(texts.len() > 150);
    assert!(!texts.iter().any(|s| has_cjk(s)));
}
