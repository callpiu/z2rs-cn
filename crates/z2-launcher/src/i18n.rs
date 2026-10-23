//! Launcher interface language.
//!
//! The launcher's text is small enough that a keyed table beats a runtime
//! catalogue: every string is a `const` array of `(english, translation)`
//! pairs, and [`Lang::t`] picks one by language. The key **is** the English
//! text, so a reader can match a call site to its translation without a
//! lookup, and the English table doubles as the documentation of what exists.
//!
//! Adding a language means adding one more array and one enum variant; the
//! tests below keep the arrays in step, so a missing translation is a test
//! failure rather than a blank label at runtime.

use serde::{Deserialize, Serialize};

/// Which language the launcher draws in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Lang {
    /// English (the upstream default).
    #[default]
    En,
    /// Simplified Chinese.
    Zh,
}

impl Lang {
    /// Every variant, for a selector widget.
    pub const ALL: [Lang; 2] = [Lang::En, Lang::Zh];

    /// The language's own name, for the selector.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Lang::En => "English",
            Lang::Zh => "简体中文",
        }
    }

    /// Look up `key` in this language's table.
    ///
    /// `key` is the English text. An unknown key returns the key itself, so a
    /// typo shows the raw string instead of an empty label — easier to spot in
    /// a screenshot than a blank. The borrow of `key` is only for that
    /// fallback, hence the tied lifetime.
    #[must_use]
    pub fn t<'a>(self, key: &'a str) -> &'a str {
        let table = match self {
            Lang::En => EN,
            Lang::Zh => ZH,
        };
        table
            .iter()
            .find(|(k, _)| *k == key)
            .map_or(key, |(_, v)| *v)
    }
}

/// The English strings, in the order the interface uses them. This array is
/// also the key set every other language must match.
const EN: &[(&str, &str)] = &[
    // -- window / tabs --------------------------------------------------
    ("z2rs launcher", "z2rs launcher"),
    ("Play", "Play"),
    ("Enhancements", "Enhancements"),
    ("Enhancements (on)", "Enhancements (on)"),
    ("ROM randomizer", "ROM randomizer"),
    ("ROM randomizer (on)", "ROM randomizer (on)"),
    ("Language", "Language"),
    (
        "Applies to the launcher only; the game itself is unchanged.",
        "Applies to the launcher only; the game itself is unchanged.",
    ),
    // -- play tab -------------------------------------------------------
    ("Your game file", "Your game file"),
    ("Path to your Zelda II .nes file", "Path to your Zelda II .nes file"),
    ("Browse…", "Browse…"),
    (
        "Use your own copy of Zelda II: The Adventure of Link (USA) as a .nes file. \
         The launcher only remembers where it is. The file is never copied or uploaded.",
        "Use your own copy of Zelda II: The Adventure of Link (USA) as a .nes file. \
         The launcher only remembers where it is. The file is never copied or uploaded.",
    ),
    ("Display", "Display"),
    ("Window size", "Window size"),
    ("Fullscreen", "Fullscreen"),
    ("Start in fullscreen", "Start in fullscreen"),
    ("Scaling", "Scaling"),
    ("Whole-number scaling only", "Whole-number scaling only"),
    ("Widescreen", "Widescreen"),
    ("4:3 (original)", "4:3 (original)"),
    ("16:10", "16:10"),
    ("16:9", "16:9"),
    ("21:9", "21:9"),
    ("Enemies use the wide screen", "Enemies use the wide screen"),
    (
        "Enemies can appear and move in the extra space at the sides. This changes \
         the game a little; online partners must pick the same.",
        "Enemies can appear and move in the extra space at the sides. This changes \
         the game a little; online partners must pick the same.",
    ),
    ("Draw sprites in the margins", "Draw sprites in the margins"),
    (
        "Show characters and items that are just off the original screen.",
        "Show characters and items that are just off the original screen.",
    ),
    (
        "F11 or Alt+Enter switches fullscreen while playing. The picture fills the \
         screen height; whole-number scaling is a little sharper but can leave black \
         borders. On a 16:10 screen (Steam Deck, Legion Go, many laptops) pick 16:10 \
         widescreen to fill it edge to edge.",
        "F11 or Alt+Enter switches fullscreen while playing. The picture fills the \
         screen height; whole-number scaling is a little sharper but can leave black \
         borders. On a 16:10 screen (Steam Deck, Legion Go, many laptops) pick 16:10 \
         widescreen to fill it edge to edge.",
    ),
    ("ROM file", "ROM file"),
    ("Browse", "Browse"),
    ("Not found", "Not found"),
    ("HD graphics pack", "HD graphics pack"),
    ("HD graphics pack (optional)", "HD graphics pack (optional)"),
    ("Custom folder…", "Custom folder…"),
    ("Folder that contains pack.json", "Folder that contains pack.json"),
    ("Pack", "Pack"),
    ("HD scale", "HD scale"),
    (
        "Use the scale the pack was drawn at (often 4). A scale the pack does not \
         support falls back to the pack's own.",
        "Use the scale the pack was drawn at (often 4). A scale the pack does not \
         support falls back to the pack's own.",
    ),
    (
        "Redrawn graphics. Unzip a pack into {} and press Refresh to list it \
         here, or choose Custom folder… to use one from anywhere.",
        "Redrawn graphics. Unzip a pack into {} and press Refresh to list it \
         here, or choose Custom folder… to use one from anywhere.",
    ),
    ("Open packs folder", "Open packs folder"),
    ("Refresh", "Refresh"),
    ("Player 2 uses gamepad number", "Player 2 uses gamepad number"),
    (
        "Gamepads count from 0 in the order they were connected. Leave this off \
         to let player 2 use the second gamepad, or the keyboard (W/A/S/D).",
        "Gamepads count from 0 in the order they were connected. Leave this off \
         to let player 2 use the second gamepad, or the keyboard (W/A/S/D).",
    ),
    ("Room name", "Room name"),
    ("e.g. link-and-zelda", "e.g. link-and-zelda"),
    ("Server", "Server"),
    (
        "Both players pick the same room name and use the same ROM, with matching \
         widescreen, \"Enemies use the wide screen\" and gameplay enhancement settings. \
         The host is player 1.",
        "Both players pick the same room name and use the same ROM, with matching \
         widescreen, \"Enemies use the wide screen\" and gameplay enhancement settings. \
         The host is player 1.",
    ),
    ("Advanced online settings", "Advanced online settings"),
    ("Sync", "Sync"),
    ("Rollback", "Rollback"),
    ("Lockstep", "Lockstep"),
    ("Input delay (frames)", "Input delay (frames)"),
    (
        "Rollback feels instant and fixes up the other player's moves (delay 0-3). \
         Lockstep waits for both players every frame (delay 0-8).",
        "Rollback feels instant and fixes up the other player's moves (delay 0-3). \
         Lockstep waits for both players every frame (delay 0-8).",
    ),
    ("Use the public server", "Use the public server"),
    ("ICE servers (optional)", "ICE servers (optional)"),
    (
        "blank = default; 'none' = same network only",
        "blank = default; 'none' = same network only",
    ),
    ("Command line", "Command line"),
    (
        "The exact command the Play button runs. Copy it into a script or shortcut.",
        "The exact command the Play button runs. Copy it into a script or shortcut.",
    ),
    ("Players", "Players"),
    ("Single player", "Single player"),
    ("Local co-op (same computer)", "Local co-op (same computer)"),
    ("Host online", "Host online"),
    ("Join online", "Join online"),
    ("Controls", "Controls"),
    (
        "Arrows move, Z = A, X = B, Enter = Start, Right Shift = Select",
        "Arrows move, Z = A, X = B, Enter = Start, Right Shift = Select",
    ),
    (
        "W/A/S/D move, G = A, F = B, T = Start, R = Select (local co-op)",
        "W/A/S/D move, G = A, F = B, T = Start, R = Select (local co-op)",
    ),
    (
        "Work automatically; the second one is player 2",
        "Work automatically; the second one is player 2",
    ),
    ("Fast-forward (hold)", "Fast-forward (hold)"),
    ("Save / load state", "Save / load state"),
    (
        "Choose save slot (shown in the title bar)",
        "Choose save slot (shown in the title bar)",
    ),
    (
        "Pause (. steps one frame while paused)",
        "Pause (. steps one frame while paused)",
    ),
    ("Toggle fullscreen", "Toggle fullscreen"),
    ("Quit", "Quit"),
    (
        "Saving, loading, pause and fast-forward are off during online play. \
         You can also drop a .nes file onto the game window.",
        "Saving, loading, pause and fast-forward are off during online play. \
         You can also drop a .nes file onto the game window.",
    ),
    (
        "Change keys and other options in the game's settings file:\n{}",
        "Change keys and other options in the game's settings file:\n{}",
    ),
    ("Open data folder", "Open data folder"),
    ("Advanced", "Advanced"),
    ("Game program", "Game program"),
    (
        "blank = find it next to the launcher",
        "blank = find it next to the launcher",
    ),
    ("Using: {}", "Using: {}"),
    ("Game output is saved to {}", "Game output is saved to {}"),
    ("Game running…", "Game running…"),
    ("Play!", "Play!"),
    (
        "Close launcher when the game starts",
        "Close launcher when the game starts",
    ),
    (
        "Close the game window to come back here.",
        "Close the game window to come back here.",
    ),
    // -- validation messages --------------------------------------------
    (
        "Choose your Zelda II ROM file (a .nes file) to play.",
        "Choose your Zelda II ROM file (a .nes file) to play.",
    ),
    ("The ROM file was not found: {}", "The ROM file was not found: {}"),
    (
        "That file does not look like a NES ROM (.nes). Pick your Zelda II (USA) \
         .nes file: {}",
        "That file does not look like a NES ROM (.nes). Pick your Zelda II (USA) \
         .nes file: {}",
    ),
    (
        "The ROM file could not be read ({}): {}",
        "The ROM file could not be read ({}): {}",
    ),
    ("Window size must be 1x to {}x.", "Window size must be 1x to {}x."),
    (
        "The HD pack folder was not found: {}",
        "The HD pack folder was not found: {}",
    ),
    (
        "That folder is not an HD pack (it has no pack.json inside): {}",
        "That folder is not an HD pack (it has no pack.json inside): {}",
    ),
    ("HD scale must be 1 to {}.", "HD scale must be 1 to {}."),
    (
        "Type a room name for online play.",
        "Type a room name for online play.",
    ),
    (
        "The room name cannot contain spaces.",
        "The room name cannot contain spaces.",
    ),
    (
        "The room name can only use letters, numbers, - and _ (up to {} characters).",
        "The room name can only use letters, numbers, - and _ (up to {} characters).",
    ),
    (
        "The server address must start with ws:// or wss://.",
        "The server address must start with ws:// or wss://.",
    ),
    (
        "The server address should look like wss://host or wss://host:port (no path).",
        "The server address should look like wss://host or wss://host:port (no path).",
    ),
    (
        "Input delay must be 0 to {} for {} mode.",
        "Input delay must be 0 to {} for {} mode.",
    ),
    (
        "Type a seed for the randomizer, or press Random seed.",
        "Type a seed for the randomizer, or press Random seed.",
    ),
    (
        "The randomizer options do not make a valid flag string ({}).",
        "The randomizer options do not make a valid flag string ({}).",
    ),
    (
        "A randomizer minimum is above its maximum (starting hearts, magic or palaces).",
        "A randomizer minimum is above its maximum (starting hearts, magic or palaces).",
    ),
    (
        "The sprite patch was not found: {}",
        "The sprite patch was not found: {}",
    ),
    (
        "The spoiler folder does not exist: {}",
        "The spoiler folder does not exist: {}",
    ),
    ("HD scale", "HD scale"),
    ("Players", "Players"),
    ("Single player", "Single player"),
    ("Local co-op", "Local co-op"),
    ("Online", "Online"),
    ("Room", "Room"),
    ("Signalling server", "Signalling server"),
    ("Sync", "Sync"),
    ("Input delay", "Input delay"),
    ("Command line", "Command line"),
    ("Copy", "Copy"),
    ("Copied", "Copied"),
    ("Controls", "Controls"),
    ("Advanced", "Advanced"),
    ("Close launcher on start", "Close launcher on start"),
    ("Game binary", "Game binary"),
    ("Play!", "Play!"),
    ("Quit", "Quit"),
    // -- generic --------------------------------------------------------
    ("Off", "Off"),
    ("On", "On"),
    ("None", "None"),
    ("Custom folder", "Custom folder"),
    ("OK", "OK"),
    ("Cancel", "Cancel"),
    ("Error", "Error"),
];

/// The Simplified Chinese translations, same key order.
const ZH: &[(&str, &str)] = &[
    ("z2rs launcher", "z2rs 启动器"),
    ("Play", "开始游戏"),
    ("Enhancements", "游戏增强"),
    ("Enhancements (on)", "游戏增强（已开）"),
    ("ROM randomizer", "ROM 随机化"),
    ("ROM randomizer (on)", "ROM 随机化（已开）"),
    ("Language", "语言"),
    (
        "Applies to the launcher only; the game itself is unchanged.",
        "仅作用于启动器界面，不影响游戏本身。",
    ),
    ("Your game file", "你的游戏文件"),
    (
        "Path to your Zelda II .nes file",
        "你的塞尔达传说 II .nes 文件路径",
    ),
    ("Browse…", "浏览…"),
    (
        "Use your own copy of Zelda II: The Adventure of Link (USA) as a .nes file. \
         The launcher only remembers where it is. The file is never copied or uploaded.",
        "请使用你自己拥有的《塞尔达传说 II：林克的冒险》（美版）ROM 文件（.nes）。\
         启动器只会记住它的位置，不会复制或上传该文件。",
    ),
    ("Display", "显示"),
    ("Window size", "窗口大小"),
    ("Fullscreen", "全屏"),
    ("Start in fullscreen", "以全屏启动"),
    ("Scaling", "缩放方式"),
    ("Whole-number scaling only", "仅整数倍缩放"),
    ("Widescreen", "宽屏"),
    ("4:3 (original)", "4:3（原始）"),
    ("16:10", "16:10"),
    ("16:9", "16:9"),
    ("21:9", "21:9"),
    ("Enemies use the wide screen", "敌人使用宽屏空间"),
    (
        "Enemies can appear and move in the extra space at the sides. This changes \
         the game a little; online partners must pick the same.",
        "敌人会在两侧多出的空间里出现和活动。这会略微改变游戏内容；\
         联机时双方必须选择相同设置。",
    ),
    ("Draw sprites in the margins", "在边缘绘制精灵"),
    (
        "Show characters and items that are just off the original screen.",
        "显示刚好在原始画面之外的角色和物品。",
    ),
    (
        "F11 or Alt+Enter switches fullscreen while playing. The picture fills the \
         screen height; whole-number scaling is a little sharper but can leave black \
         borders. On a 16:10 screen (Steam Deck, Legion Go, many laptops) pick 16:10 \
         widescreen to fill it edge to edge.",
        "游戏中按 F11 或 Alt+Enter 切换全屏。画面会填满屏幕高度；\
         整数倍缩放更锐利，但可能留下黑边。\
         在 16:10 屏幕（Steam Deck、Legion Go、许多笔记本）上\
         选择 16:10 宽屏即可铺满。",
    ),
    ("ROM file", "ROM 文件"),
    ("Browse", "浏览"),
    ("Not found", "找不到"),
    ("HD graphics pack", "HD 材质包"),
    ("HD graphics pack (optional)", "HD 材质包（可选）"),
    ("Custom folder…", "自定义文件夹…"),
    ("Folder that contains pack.json", "包含 pack.json 的文件夹"),
    ("Pack", "材质包"),
    ("HD scale", "HD 缩放"),
    (
        "Use the scale the pack was drawn at (often 4). A scale the pack does not \
         support falls back to the pack's own.",
        "请使用材质包绘制时的缩放级别（通常是 4）。\
         若所选缩放不被支持，会自动回退到材质包自身的级别。",
    ),
    (
        "Redrawn graphics. Unzip a pack into {} and press Refresh to list it \
         here, or choose Custom folder… to use one from anywhere.",
        "重绘的高清图形。把材质包解压到 {} 后点「刷新」即可在此列出，\
         也可以选「自定义文件夹…」使用任意位置的材质包。",
    ),
    ("Open packs folder", "打开材质包目录"),
    ("Refresh", "刷新"),
    ("Player 2 uses gamepad number", "2P 使用指定编号的手柄"),
    (
        "Gamepads count from 0 in the order they were connected. Leave this off \
         to let player 2 use the second gamepad, or the keyboard (W/A/S/D).",
        "手柄从 0 开始编号，按连接顺序排列。关闭此项时，\
         2P 会使用第二个手柄，或用键盘（W/A/S/D）操作。",
    ),
    ("Room name", "房间名"),
    ("e.g. link-and-zelda", "例如 link-and-zelda"),
    ("Server", "服务器"),
    (
        "Both players pick the same room name and use the same ROM, with matching \
         widescreen, \"Enemies use the wide screen\" and gameplay enhancement settings. \
         The host is player 1.",
        "双方需填写相同的房间名、使用相同的 ROM，\
         并保持宽屏、「敌人使用宽屏空间」和游戏增强设置一致。\
         主机为 1P。",
    ),
    ("Advanced online settings", "联机高级设置"),
    ("Sync", "同步方式"),
    ("Rollback", "回滚（Rollback）"),
    ("Lockstep", "锁步（Lockstep）"),
    ("Input delay (frames)", "输入延迟（帧）"),
    (
        "Rollback feels instant and fixes up the other player's moves (delay 0-3). \
         Lockstep waits for both players every frame (delay 0-8).",
        "回滚操作即时响应，并会修正对方的输入（延迟 0-3）。\
         锁步每帧都等待双方输入（延迟 0-8）。",
    ),
    ("Use the public server", "使用公共服务器"),
    ("ICE servers (optional)", "ICE 服务器（可选）"),
    (
        "blank = default; 'none' = same network only",
        "留空 = 默认；'none' = 仅同一网络",
    ),
    ("Command line", "命令行"),
    (
        "The exact command the Play button runs. Copy it into a script or shortcut.",
        "「开始游戏」按钮实际执行的命令。可复制到脚本或快捷方式中。",
    ),
    ("Players", "玩家人数"),
    ("Single player", "单人游戏"),
    ("Local co-op (same computer)", "本地双人（同一台电脑）"),
    ("Host online", "创建联机房间"),
    ("Join online", "加入联机房间"),
    ("Controls", "操作说明"),
    (
        "Arrows move, Z = A, X = B, Enter = Start, Right Shift = Select",
        "方向键移动，Z = A，X = B，Enter = Start，右 Shift = Select",
    ),
    (
        "W/A/S/D move, G = A, F = B, T = Start, R = Select (local co-op)",
        "W/A/S/D 移动，G = A，F = B，T = Start，R = Select（本地双人）",
    ),
    (
        "Work automatically; the second one is player 2",
        "自动识别；第二个手柄归 2P 使用",
    ),
    ("Fast-forward (hold)", "快进（按住）"),
    ("Save / load state", "存档 / 读档"),
    (
        "Choose save slot (shown in the title bar)",
        "选择存档槽位（显示在标题栏）",
    ),
    (
        "Pause (. steps one frame while paused)",
        "暂停（暂停中按 . 单帧步进）",
    ),
    ("Toggle fullscreen", "切换全屏"),
    ("Quit", "退出"),
    (
        "Saving, loading, pause and fast-forward are off during online play. \
         You can also drop a .nes file onto the game window.",
        "联机对战时存档、读档、暂停和快进不可用。\
         也可以把 .nes 文件直接拖到游戏窗口上。",
    ),
    (
        "Change keys and other options in the game's settings file:\n{}",
        "按键和其他选项可在游戏设置文件中修改：\n{}",
    ),
    ("Open data folder", "打开数据目录"),
    ("Advanced", "高级"),
    ("Game program", "游戏程序"),
    (
        "blank = find it next to the launcher",
        "留空 = 在启动器旁边自动查找",
    ),
    ("Using: {}", "正在使用：{}"),
    ("Game output is saved to {}", "游戏日志保存在 {}"),
    ("Game running…", "游戏运行中…"),
    ("Play!", "开始游戏！"),
    (
        "Close launcher when the game starts",
        "游戏启动后关闭启动器",
    ),
    (
        "Close the game window to come back here.",
        "关闭游戏窗口即可回到这里。",
    ),
    // -- validation messages --------------------------------------------
    (
        "Choose your Zelda II ROM file (a .nes file) to play.",
        "请选择你的《塞尔达传说 II》ROM 文件（.nes）再开始游戏。",
    ),
    ("The ROM file was not found: {}", "找不到 ROM 文件：{}"),
    (
        "That file does not look like a NES ROM (.nes). Pick your Zelda II (USA) \
         .nes file: {}",
        "这个文件看起来不是 NES ROM（.nes）。请选择你的《塞尔达传说 II》（美版）\
         .nes 文件：{}",
    ),
    (
        "The ROM file could not be read ({}): {}",
        "无法读取 ROM 文件（{}）：{}",
    ),
    ("Window size must be 1x to {}x.", "窗口大小必须在 1x 到 {}x 之间。"),
    ("The HD pack folder was not found: {}", "找不到 HD 材质包目录：{}"),
    (
        "That folder is not an HD pack (it has no pack.json inside): {}",
        "这个目录不是 HD 材质包（里面没有 pack.json）：{}",
    ),
    ("HD scale must be 1 to {}.", "HD 缩放必须在 1 到 {} 之间。"),
    ("Type a room name for online play.", "请填写联机房间名。"),
    ("The room name cannot contain spaces.", "房间名不能包含空格。"),
    (
        "The room name can only use letters, numbers, - and _ (up to {} characters).",
        "房间名只能使用字母、数字、- 和 _（最多 {} 个字符）。",
    ),
    (
        "The server address must start with ws:// or wss://.",
        "服务器地址必须以 ws:// 或 wss:// 开头。",
    ),
    (
        "The server address should look like wss://host or wss://host:port (no path).",
        "服务器地址应形如 wss://host 或 wss://host:port（不能带路径）。",
    ),
    (
        "Input delay must be 0 to {} for {} mode.",
        "{} 模式的输入延迟必须在 0 到 {} 之间。",
    ),
    (
        "Type a seed for the randomizer, or press Random seed.",
        "请填写随机化种子，或点「随机种子」。",
    ),
    (
        "The randomizer options do not make a valid flag string ({}).",
        "当前随机化选项无法组成有效的标志字符串（{}）。",
    ),
    (
        "A randomizer minimum is above its maximum (starting hearts, magic or palaces).",
        "某项随机化下限高于其上限（初始红心、魔法或宫殿数量）。",
    ),
    ("The sprite patch was not found: {}", "找不到角色贴图补丁：{}"),
    ("The spoiler folder does not exist: {}", "剧透日志目录不存在：{}"),
    ("HD scale", "HD 缩放"),
    ("Players", "玩家人数"),
    ("Single player", "单人游戏"),
    ("Local co-op", "本地双人"),
    ("Online", "联机"),
    ("Room", "房间"),
    ("Signalling server", "信令服务器"),
    ("Sync", "同步方式"),
    ("Input delay", "输入延迟"),
    ("Command line", "命令行"),
    ("Copy", "复制"),
    ("Copied", "已复制"),
    ("Controls", "操作说明"),
    ("Advanced", "高级"),
    ("Close launcher on start", "启动后关闭启动器"),
    ("Game binary", "游戏程序"),
    ("Play!", "开始游戏！"),
    ("Quit", "退出"),
    ("Off", "关"),
    ("On", "开"),
    ("None", "无"),
    ("Custom folder", "自定义文件夹"),
    ("OK", "确定"),
    ("Cancel", "取消"),
    ("Error", "错误"),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_english_key_has_a_chinese_translation() {
        for (k, _) in EN {
            assert!(
                ZH.iter().any(|(zk, _)| zk == k),
                "key {k:?} missing from the Chinese table"
            );
        }
    }

    #[test]
    fn no_extra_chinese_keys() {
        for (k, _) in ZH {
            assert!(
                EN.iter().any(|(ek, _)| ek == k),
                "Chinese table has key {k:?} that English does not"
            );
        }
    }

    #[test]
    fn lookup_returns_the_translation() {
        assert_eq!(Lang::En.t("Play"), "Play");
        assert_eq!(Lang::Zh.t("Play"), "开始游戏");
    }

    #[test]
    fn unknown_key_returns_the_key_itself() {
        assert_eq!(Lang::Zh.t("no such key"), "no such key");
    }

    #[test]
    fn tables_stay_the_same_length() {
        assert_eq!(EN.len(), ZH.len());
    }
}
