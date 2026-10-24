//! SCRATCH: 验证启动器默认中文、可切换、能持久化。

use z2_launcher::{build_args, validate, validate_lang, Lang, Settings};

const ZH: &str = r"D:/z2rs-中文版/Zelda II - The Adventure of Link (U) [20260806_NOKOH].nes";

#[test]
fn default_is_chinese() {
    let s = Settings::default();
    assert_eq!(s.language, Lang::Zh, "默认应为中文");
}

#[test]
fn language_round_trips_through_json() {
    let mut s = Settings::default();
    s.rom_path = ZH.to_string();
    s.language = Lang::En;
    let json = serde_json::to_string(&s).expect("序列化");
    let back: Settings = serde_json::from_str(&json).expect("反序列化");
    assert_eq!(back.language, Lang::En, "切换英文应能存下来");

    s.language = Lang::Zh;
    let json = serde_json::to_string(&s).expect("序列化");
    let back: Settings = serde_json::from_str(&json).expect("反序列化");
    assert_eq!(back.language, Lang::Zh, "切回中文应能存下来");
}

/// 旧版 launcher.json 没有 language 字段时应默认中文（serde default）。
#[test]
fn old_settings_file_without_language_loads_as_chinese() {
    let json = r#"{"rom_path":"x.nes","scale":3}"#;
    let s: Settings = serde_json::from_str(json).expect("旧文件应能解析");
    assert_eq!(s.language, Lang::Zh, "缺字段应落到默认中文");
}

/// 语言只影响 `--ui-language`，不碰其它启动参数。
#[test]
fn language_does_not_change_args() {
    let mut a = Settings::default();
    a.rom_path = ZH.to_string();
    a.language = Lang::Zh;
    let args_zh = build_args(&a);
    a.language = Lang::En;
    let args_en = build_args(&a);
    // The only difference is the UI language flag the game window reads.
    let only_flag = |args: &[String]| -> Vec<String> {
        let mut out = Vec::new();
        let mut i = 0;
        while i < args.len() {
            if args[i] == "--ui-language" {
                i += 2;
            } else {
                out.push(args[i].clone());
                i += 1;
            }
        }
        out
    };
    assert_eq!(
        only_flag(&args_zh),
        only_flag(&args_en),
        "语言只该通过 --ui-language 影响启动参数"
    );
    assert!(args_zh.windows(2).any(|w| w == ["--ui-language", "zh"]));
    assert!(args_en.windows(2).any(|w| w == ["--ui-language", "en"]));
}

/// 校验信息在中文下必须是中文，英文下必须是英文。
/// 这条专门盯「键加了但翻译没接上」的情况。
#[test]
fn validation_messages_are_translated() {
    let mut s = Settings::default();
    // 没选 ROM：中英文都应给出提示
    let zh = validate_lang(&s, Lang::Zh);
    let en = validate_lang(&s, Lang::En);
    assert_ne!(zh, en, "中英文提示不应相同");
    assert!(
        zh.iter().all(|m| m.chars().any(|c| c as u32 > 0x2E80)),
        "中文提示里应含汉字: {zh:?}"
    );
    assert!(
        en.iter().all(|m| !m.chars().any(|c| c as u32 > 0x2E80)),
        "英文提示里不应含汉字: {en:?}"
    );

    // 有 ROM 但文件不存在：占位符要被填进去
    s.rom_path = r"D:\definitely\not\here.nes".to_string();
    let zh = validate_lang(&s, Lang::Zh);
    assert!(
        zh.iter().any(|m| m.contains(r"D:\definitely\not\here.nes")),
        "路径应出现在提示里: {zh:?}"
    );

    // 窗口大小越界：提示说明允许范围（1x..=6x）
    s.rom_path = ZH.to_string();
    s.scale = 99;
    let zh = validate_lang(&s, Lang::Zh);
    eprintln!("scale=99 的中文提示: {zh:?}");
    assert!(
        zh.iter().any(|m| m.contains('6')),
        "提示应说明允许的上限: {zh:?}"
    );
}
