//! 验证中文字体真的能渲染出字形（而不是 tofu 方块）。

use egui::Color32;

/// 用 egui 实际布局一段中文，测量它占的宽度。
///
/// `.ttc` 是字体集合，epaint 只读第一个 face —— 那个 face 未必有汉字。
/// 所以"字体文件找到了"不等于"中文能画出来"，必须真的排一次版看宽度：
/// 5 个汉字挤成 1 个字的宽度就说明拿到的是豆腐块。
#[test]
fn chinese_text_lays_out_with_the_system_font() {
    let ctx = egui::Context::default();
    z2_launcher::app::apply_style(&ctx);

    let mut cjk_w = 0.0f32;
    let mut latin_w = 0.0f32;
    let mut tofu_w = 0.0f32;
    let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
        let measure = |text: &str| {
            ui.painter()
                .layout_no_wrap(
                    text.to_owned(),
                    egui::FontId::proportional(16.0),
                    Color32::WHITE,
                )
                .rect
                .width()
        };
        cjk_w = measure("开始游戏");
        latin_w = measure("Play");
        // A private-use codepoint no font has: this is what tofu measures like.
        tofu_w = measure("\u{E000}\u{E001}\u{E002}\u{E003}\u{E004}");
    });
    // epaint asserts on drop if the run produced texture updates nobody took;
    // a real backend applies them, a headless test has to discard them.
    out.textures_delta.clear();

    eprintln!("「开始游戏」={cjk_w:.2}  「Play」={latin_w:.2}  缺字对照={tofu_w:.2}");

    // 判据：汉字是**全角**的，在 16px 字号下每个正好占 1 个 em。
    // 4 个汉字 = 64px，这是真字形的特征。
    // 完全缺字时 epaint 画的是替代框，宽度与 em 对不上（实测 5 个缺字码位
    // 占 82px，比 5×16=80 还宽一点），所以"宽度约等于 em×字数"才是
    // 命中真字形的证据，不能反过来拿"比豆腐宽"当判据。
    let em = 16.0f32;
    let expected = em * 4.0;
    assert!(
        (cjk_w - expected).abs() <= em * 0.5,
        "4 个汉字宽 {cjk_w:.1}，预期约 {expected:.1}（全角 em）；\
         偏差过大说明画的不是真字形"
    );
    assert!(
        (cjk_w - tofu_w).abs() > 1.0,
        "中文宽度与缺字对照相同（{cjk_w:.1} vs {tofu_w:.1}），渲染的是方块"
    );
    assert!(latin_w > 0.0, "英文宽度为 0，字体根本没装上");
}
