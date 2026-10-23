//! 验证增强面板在中文模式下真的渲染出中文。

use egui::Context;
use z2_core::enh::{display::DisplayEnh, Enhancements};
use z2_enh_ui::{enhancements_section, group_ui, Group, Lang, Opts};

/// 用 egui 的 accessibility 层收集这一帧实际画出的所有文本。
///
/// 直接检查"标签是不是中文"，而不是相信代码里都包了 `tr()` ——
/// 漏掉一处 `lang.tr()` 在这里就会暴露。
fn rendered_texts(lang: Lang, force_open: bool) -> Vec<String> {
    let ctx = Context::default();
    ctx.enable_accesskit();
    let mut e = Enhancements::default();
    let mut d = DisplayEnh::default();
    let opts = Opts {
        force_open,
        gameplay_editable: true,
        rom_randomized: false,
        lang,
    };
    let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
        enhancements_section(ui, &mut e, &mut d, opts);
    });
    let mut texts = Vec::new();
    if let Some(update) = out.platform_output.accesskit_update.take() {
        for (_id, node) in &update.nodes {
            if let Some(name) = node.label() {
                if !name.is_empty() {
                    texts.push(name.to_owned());
                }
            }
        }
    }
    out.textures_delta.clear();
    texts
}

fn has_cjk(s: &str) -> bool {
    s.chars().any(|c| ('\u{4e00}'..='\u{9fff}').contains(&c))
}

#[test]
fn chinese_mode_draws_chinese_labels() {
    let zh = rendered_texts(Lang::Zh, true);
    assert!(!zh.is_empty(), "没有抓到任何文本，a11y 层可能没开");
    let cjk = zh.iter().filter(|s| has_cjk(s)).count();
    eprintln!("中文模式: 共 {} 条文本，其中含汉字 {} 条", zh.len(), cjk);
    for s in zh.iter().filter(|s| has_cjk(s)).take(8) {
        eprintln!("   ✓ {s}");
    }
    let leftover: Vec<&String> = zh
        .iter()
        .filter(|s| !has_cjk(s) && s.chars().filter(|c| c.is_alphabetic()).count() > 3)
        .collect();
    eprintln!("仍为英文的 {} 条:", leftover.len());
    for s in leftover.iter().take(12) {
        eprintln!("   ✗ {s}");
    }
    assert!(cjk > 30, "中文标签只有 {cjk} 条，翻译可能没接上");
    assert!(
        leftover.is_empty(),
        "还有 {} 条英文没翻译，前几条: {:?}",
        leftover.len(),
        &leftover[..leftover.len().min(5)]
    );
}

#[test]
fn english_mode_stays_english() {
    let en = rendered_texts(Lang::En, true);
    assert!(!en.is_empty());
    let cjk = en.iter().filter(|s| has_cjk(s)).count();
    eprintln!("英文模式: 共 {} 条，含汉字 {} 条", en.len(), cjk);
    assert_eq!(cjk, 0, "英文模式下不该出现汉字");
}

#[test]
fn every_group_and_conditional_control_is_translated() {
    for lang in [Lang::Zh, Lang::En] {
        for group in Group::ALL {
            let ctx = Context::default();
            ctx.enable_accesskit();
            let mut e = Enhancements::zalia_preset();
            let mut d = DisplayEnh::zalia_preset();
            d.effects_enabled = true;
            e.content.virtual_rooms = true;
            let mut texts = Vec::new();
            // Collapsing headers animate open; inspect their settled output.
            for frame in 0..3 {
                let mut out = ctx.run_ui(egui::RawInput {
                    time: Some(frame as f64),
                    screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1200.0, 20000.0))),
                    ..Default::default()
                }, |ui| group_ui(ui, group, &mut e, &mut d, Opts {
                    force_open: true, gameplay_editable: true, rom_randomized: true, lang,
                }));
                if let Some(update) = out.platform_output.accesskit_update.take() {
                    for (_, node) in update.nodes {
                        if let Some(s) = node.label().filter(|s| !s.is_empty()) { texts.push(s.to_owned()); }
                    }
                }
                out.textures_delta.clear();
            }
            assert!(!texts.is_empty(), "{group:?} did not render");
            if lang == Lang::Zh {
                let left: Vec<_> = texts.iter().filter(|s| !has_cjk(s) && s.chars().filter(|c| c.is_ascii_alphabetic()).count() > 2).collect();
                assert!(left.is_empty(), "{group:?}: untranslated {left:?}");
            } else {
                assert!(!texts.iter().any(|s| has_cjk(s)), "{group:?}");
            }
        }
    }
}
