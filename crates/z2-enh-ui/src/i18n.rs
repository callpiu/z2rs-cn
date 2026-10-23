//! Chinese text for the enhancements panel.
//!
//! The widgets in this crate are shared by the launcher and the game's in-game
//! options overlay, so the language has to be a parameter rather than a global:
//! two different windows can be open at once (a launcher next to a game) and
//! they must not fight over one setting.
//!
//! Every English string in this crate is also the key here, so a missing entry
//! renders the English original instead of a blank — and the table doubles as
//! the list of what needs translating.

use serde::{Deserialize, Serialize};

/// Which language the panel draws in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Lang {
    /// English (upstream default).
    #[default]
    En,
    /// Simplified Chinese.
    Zh,
}

impl Lang {
    /// Translate `english`, returning it unchanged when this is English or the
    /// phrase has no entry.
    #[must_use]
    pub fn tr(self, english: &str) -> &str {
        match self {
            Lang::En => english,
            Lang::Zh => ZH
                .iter()
                .find(|(k, _)| *k == english)
                .map_or(english, |(_, v)| *v),
        }
    }
}

/// `(english, 简体中文)`. Order follows the panel's layout so a reader can find
/// an entry by walking the UI top to bottom.
const ZH: &[(&str, &str)] = &[
    // -- item / spell names (start-items bit boxes) ---------------------
    ("Shield", "盾"),
    ("Jump", "跳跃"),
    ("Life", "生命"),
    ("Fairy", "妖精"),
    ("Fire", "火焰"),
    ("Reflect", "反射"),
    ("Spell", "魔法"),
    ("Thunder", "雷电"),
    ("Candle", "蜡烛"),
    ("Glove", "手套"),
    ("Raft", "木筏"),
    ("Boots", "靴子"),
    ("Flute", "笛子"),
    ("Cross", "十字架"),
    ("Hammer", "锤子"),
    ("Magic key", "魔法钥匙"),
    ("Downward thrust", "下刺"),
    ("Upward thrust", "上刺"),
    // -- header ---------------------------------------------------------
    (
        "Gameplay enhancements are on: online partners must use the same ones.",
        "玩法增强已开启：联机时对方必须使用相同的设置。",
    ),
    ("Gameplay: original game.", "玩法：原版游戏。"),
    ("Reset to original", "恢复原版"),
    (
        "Turn every enhancement off (the original NES game).",
        "关闭所有增强功能（即原版 NES 游戏）。",
    ),
    ("ZALiA preset", "ZALiA 预设"),
    ("Enhancements (ZALiA-inspired)", "增强功能（受 ZALiA 启发）"),
    // -- text & HUD -----------------------------------------------------
    ("Text & HUD", "文本与 HUD"),
    ("Dialogue speed", "对话速度"),
    ("Call the SHIELD spell PROTECT", "把盾魔法命名为 PROTECT"),
    (
        "Renames SHIELD to PROTECT in the spell menu. ZALiA: mod_SpellSHIELD_NAME.",
        "在魔法菜单里把 SHIELD 改名为 PROTECT。ZALiA：mod_SpellSHIELD_NAME。",
    ),
    ("Show the quest timer", "显示任务计时器"),
    // -- quality of life ------------------------------------------------
    ("Quality of life", "体验优化"),
    ("Continue from", "继续位置"),
    ("XP kept on game over (%)", "游戏结束后保留的经验值（%）"),
    (
        "0 = original (all XP lost). ZALiA keeps 25% (mod_Gameover_XP_PENALTY).",
        "0 = 原版（经验值全部丢失）。ZALiA 保留 25%（mod_Gameover_XP_PENALTY）。",
    ),
    ("Lives from Life Dolls", "生命数来自生命娃娃"),
    (
        "Enter towns from the side you came from",
        "从进入方向的一侧进入城镇",
    ),
    ("Wise men restore MP", "贤者恢复魔力"),
    (
        "No magic requirement to learn spells",
        "学习魔法不受魔力上限限制",
    ),
    ("Overworld softlock warp", "大地图卡死传送"),
    // -- engine fixes ---------------------------------------------------
    ("Engine fixes", "引擎修复"),
    ("Level-up softlock fixes", "修复升级卡死"),
    ("No skipped updates while invincible", "无敌期间不再跳帧更新"),
    ("Same jump in both directions", "左右跳跃一致"),
    ("Same shield hitbox both ways", "左右盾牌判定一致"),
    ("Crumbling bridges check both feet", "塌陷桥梁判定双脚"),
    ("XP drain fix", "修复经验值流失"),
    // -- enemies --------------------------------------------------------
    ("Enemies & bosses", "敌人与首领"),
    ("Iron Knuckle aggro rules", "铁甲骑士仇恨规则"),
    ("Ra with less HP", "Ra 生命值降低"),
    (
        "The flying Ra heads take fewer hits. ZALiA: mod_Ra_HP.",
        "飞行的 Ra 头部需要的攻击次数减少。ZALiA：mod_Ra_HP。",
    ),
    ("Stalfos upthrust fix", "骷髅上刺修复"),
    ("Wizards teleport anywhere", "法师可在任意位置传送"),
    ("Mago balance", "Mago 平衡调整"),
    ("Bosses wait before attacking", "首领出手前有延迟"),
    ("Helmethead head fix", "Helmethead 头部修复"),
    ("Carock vulnerable longer", "Carock 可攻击时间延长"),
    (
        "A longer window to hit Carock. ZALiA: Carock VulnDur.",
        "攻击 Carock 的时间窗口更长。ZALiA：Carock VulnDur。",
    ),
    ("Barba aims", "Barba 瞄准"),
    (
        "Barba's fire is aimed at Link. ZALiA: Barba Aim.",
        "Barba 的火焰会瞄准林克。ZALiA：Barba Aim。",
    ),
    (
        "Horsehead in Palace 5's false-wall room",
        "第 5 宫殿假墙房间里的马头",
    ),
    // -- abilities ------------------------------------------------------
    ("Abilities", "能力"),
    ("Double jump", "二段跳"),
    (
        "One extra jump in mid-air. ZALiA: the FEATHER item.",
        "可在空中多跳一次。ZALiA：FEATHER 道具。",
    ),
    ("Stab frenzy", "连刺"),
    (
        "Hold B to stab over and over. ZALiA: Options > Other > Stab frenzy.",
        "按住 B 连续突刺。ZALiA：Options > Other > Stab frenzy。",
    ),
    ("Extra sword reach (px)", "额外剑长（像素）"),
    (
        "0 = original. ZALiA's SWORD item adds 3 px (mod_PCSword2).",
        "0 = 原版。ZALiA 的 SWORD 道具加 3 像素（mod_PCSword2）。",
    ),
    ("Damage reduction (%)", "伤害减免（%）"),
    (
        "0 = original. ZALiA's RING halves damage; RING with PROTECT takes a third.",
        "0 = 原版。ZALiA 的 RING 使伤害减半；RING 配合 PROTECT 减到三分之一。",
    ),
    ("Slow magic regeneration", "缓慢魔力回复"),
    (
        "+2 MP every 128 frames. ZALiA: the PENDANT item.",
        "每 128 帧回复 2 点魔力。ZALiA：PENDANT 道具。",
    ),
    ("Stronger REFLECT", "强化反射"),
    ("Dash", "冲刺"),
    ("Rescue fairy", "救助妖精"),
    ("Flute warp", "笛子传送"),
    // -- randomizer & start ---------------------------------------------
    ("Randomizer & start", "随机化与初始状态"),
    ("Seed", "种子"),
    (
        "0 = original (level 1). ZALiA: RANDO > START ATTACK/MAGIC/LIFE.",
        "0 = 原版（1 级）。ZALiA：RANDO > START ATTACK/MAGIC/LIFE。",
    ),
    ("Start attack level", "初始攻击等级"),
    ("Start magic level", "初始魔法等级"),
    ("Start life level", "初始生命等级"),
    (
        "0 = original (4). ZALiA: RANDO > START ITEMS containers.",
        "0 = 原版（4 个）。ZALiA：RANDO > START ITEMS 容器。",
    ),
    ("Start heart containers", "初始心容器"),
    ("Start magic containers", "初始魔法容器"),
    ("Start spells", "初始魔法"),
    (
        "Spells known from the start. ZALiA: RANDO > START SPELLS.",
        "开局即掌握这些魔法。ZALiA：RANDO > START SPELLS。",
    ),
    ("Start items and skills", "初始道具与技能"),
    ("Enemy HP", "敌人生命值"),
    (
        "0 = original. ZALiA: RANDO > ENEMY HP +/-25%.",
        "0 = 原版。ZALiA：RANDO > ENEMY HP ±25%。",
    ),
    ("Enemy damage", "敌人伤害"),
    (
        "0 = original. ZALiA: RANDO > ENEMY DAMAGE +/-25%.",
        "0 = 原版。ZALiA：RANDO > ENEMY DAMAGE ±25%。",
    ),
    ("XP gained", "获得的经验值"),
    (
        "0 = original. ZALiA: RANDO > XP +/-25%.",
        "0 = 原版。ZALiA：RANDO > XP ±25%。",
    ),
    ("Level-up cost", "升级所需经验"),
    (
        "0 = original. ZALiA: RANDO > LEVEL COSTS +/-25%.",
        "0 = 原版。ZALiA：RANDO > LEVEL COSTS ±25%。",
    ),
    ("Spell cost", "魔法消耗"),
    (
        "0 = original. ZALiA: RANDO > SPELL COSTS.",
        "0 = 原版。ZALiA：RANDO > SPELL COSTS。",
    ),
    ("Palette randomizer", "调色板随机化"),
    ("Shuffle item locations", "打乱道具位置"),
    (
        "Items are placed at random, always beatable. ZALiA: RANDO > ITEMS.",
        "道具随机放置，但一定可以通关。ZALiA：RANDO > ITEMS。",
    ),
    // -- cheats ---------------------------------------------------------
    ("Cheats", "作弊"),
    ("Invincible", "无敌"),
    (
        "Link takes no damage. ZALiA: dev invulnerability (I key).",
        "林克不会受伤。ZALiA：开发者无敌（I 键）。",
    ),
    ("Infinite magic", "无限魔力"),
    (
        "Magic never runs out. ZALiA: dev stab-to-cheat jars.",
        "魔力永不耗尽。ZALiA：开发者「刺罐子加魔力」。",
    ),
    ("Infinite lives", "无限生命"),
    ("The life counter never goes down.", "生命计数永不减少。"),
    ("Max stats", "属性全满"),
    // -- new areas ------------------------------------------------------
    ("New areas (ZALiA-inspired)", "新增区域（受 ZALiA 启发）"),
    ("New areas", "新增区域"),
    ("Parapa annex", "帕拉帕附属区"),
    ("Challenge rooms", "挑战房间"),
    ("Minigames", "小游戏"),
    ("Hidden caves", "隐藏洞穴"),
    ("Dark Isle", "暗黑岛"),
    ("Developer", "开发者"),
    ("Test room", "测试房间"),
    // -- graphics -------------------------------------------------------
    ("Graphics effects", "画面特效"),
    (
        "Display only: these never change the game.",
        "仅影响显示：不会改变游戏内容。",
    ),
    ("Screen shake", "屏幕震动"),
    ("Flash colour", "闪光颜色"),
    ("Camera zoom", "镜头缩放"),
    ("Post effects", "后期效果"),
    ("Brightness", "亮度"),
    ("ZALiA GEE default 0.05.", "ZALiA GEE 默认值 0.05。"),
    ("Saturation", "饱和度"),
    ("ZALiA GEE default -0.075.", "ZALiA GEE 默认值 -0.075。"),
    ("Scanlines", "扫描线"),
    ("ZALiA GEE default 0.5.", "ZALiA GEE 默认值 0.5。"),
    ("Bloom", "泛光"),
    ("ZALiA GEE bloom.", "ZALiA GEE 泛光。"),
    ("Blur", "模糊"),
    ("ZALiA GEE default 0.85.", "ZALiA GEE 默认值 0.85。"),
    // -- audio ----------------------------------------------------------
    ("Audio", "音频"),
    ("Music volume", "音乐音量"),
    (
        "10 = unchanged. ZALiA: music volume 0-10.",
        "10 = 不变。ZALiA：音乐音量 0-10。",
    ),
    ("Sound effects volume", "音效音量"),
    (
        "10 = unchanged. ZALiA: sound volume 0-10.",
        "10 = 不变。ZALiA：音效音量 0-10。",
    ),
    ("Quieter low-health beep", "降低低血量提示音"),
    // -- dev tools ------------------------------------------------------
    ("Dev tools", "开发者工具"),
    ("Show collision boxes", "显示碰撞框"),
    (
        "Body (green), shield (purple), sword (yellow), enemies (red), and terrain probes (cyan). Uses game collision tables, including HD actor animations.",
        "身体（绿）、盾牌（紫）、剑（黄）、敌人（红）和地形探测（青）。使用游戏自身的碰撞表，含高清角色动画。",
    ),
    ("Show XY points", "显示 XY 坐标点"),
    (
        "Draw object positions. ZALiA: Dev tools > XY points.",
        "绘制物体位置。ZALiA：Dev tools > XY points。",
    ),
    ("Show enemy HP", "显示敌人生命值"),
    (
        "Draw enemy hit points. ZALiA: Dev tools > HP.",
        "绘制敌人血量。ZALiA：Dev tools > HP。",
    ),
    ("Show frame count", "显示帧数"),
    (
        "Draw the frame counter. ZALiA: Dev tools > App frame count.",
        "绘制帧计数器。ZALiA：Dev tools > App frame count。",
    ),
    ("drawing alone never edits anything", "仅绘制，不会修改任何数据"),
    ("Optional changes inspired by ZALiA, HoverBat's Zelda II remake. Everything is off by default, which is the original game. Gameplay options change the game: online partners must pick exactly the same, and movie playback ignores them. Hover an option for details. Options not implemented yet are saved but have no effect.", "可选改动参考了 HoverBat 制作的《塞尔达2》重制版 ZALiA。默认全部关闭，即原版玩法。玩法选项会改变游戏，联机双方必须完全一致，录像播放时会忽略这些选项。悬停可查看详情。尚未实现的选项仅保存设置，不产生效果。"),
    ("ZALiA's own defaults: dialogue speed 2, continue from the palace entrance or last town, keep 25% XP, lives from dolls, enter towns from the side, wise men restore MP, every engine fix and enemy tweak, the new areas except the Dark Isle, screen shake and a red flash.", "采用 ZALiA 的默认配置：对话速度 2、从宫殿入口或上次城镇继续、保留 25% 经验、生命人偶永久增加初始生命、按方向进入城镇、贤者补满魔力、全部引擎修复和敌人调整、除暗黑岛外的新增区域、画面震动和红色闪光。"),
    ("0 = original speed, 5 = instant. ZALiA: mod_DLG_SPEED (default 2); the delay between letters drops from 5 frames to 4, 3, 2, 1, 0.", "0 为原版速度，5 为立即显示。对应 ZALiA 的 mod_DLG_SPEED（默认 2）；逐字间隔从 5 帧依次降为 4、3、2、1、0 帧。"),
    ("Speedrun-style 00:00:00.00 timer since the quest started (display only). ZALiA: Options > Other > Quest timer.", "显示从冒险开始计时的竞速计时器，格式为 00:00:00.00，仅影响显示。参考 ZALiA 的任务计时器。"),
    ("Where you continue after a game over. Original: always the North Palace. ZALiA (mod_ContinueFrom): the palace entrance after dying in a palace, the last town you saved in after dying elsewhere.", "游戏结束后的继续位置。原版始终从北方宫殿开始；ZALiA 的 mod_ContinueFrom 在宫殿内死亡后从入口继续，在其他地方死亡后从上次存档的城镇继续。"),
    ("Lives start at 3 plus one per Life Doll found, so dolls count for good. ZALiA: mod_START_RUN_LIVES.", "初始生命数为 3 加上找到的生命人偶数，人偶效果永久保留。对应 ZALiA 的 mod_START_RUN_LIVES。"),
    ("Start at the left or right end of a town depending on your approach. The original always starts at the right end. ZALiA: mod_EnterTownSideOption.", "根据接近方向从城镇左端或右端进入。原版始终从右端进入。对应 ZALiA 的 mod_EnterTownSideOption。"),
    ("A wise man refills your magic when he teaches his spell. ZALiA: mod_WISEMEN_RESTORE_MP.", "贤者教授魔法时补满魔力。对应 ZALiA 的 mod_WISEMEN_RESTORE_MP。"),
    ("Learn a spell without the minimum number of magic containers. ZALiA: mod_AcquireSpellRequirement.", "学习魔法不再要求最低魔法容器数。对应 ZALiA 的 mod_AcquireSpellRequirement。"),
    ("Hold a button chord for 3 seconds while paused on the overworld to warp to the continue screen. ZALiA's softlock failsafe.", "在大地图暂停时按住组合键 3 秒，返回继续画面。参考 ZALiA 的卡关脱困功能。"),
    ("The level-up window can no longer open during a room change or a dialogue. ZALiA: mod_FIX_SOFTLOCK_LVLUP_1/2.", "切换房间或对话期间不再弹出升级窗口。对应 ZALiA 的 mod_FIX_SOFTLOCK_LVLUP_1/2。"),
    ("The original skips camera and collision updates on alternate frames when the invincibility timer is 1 or 2. ZALiA: mod_PCUpdate1.", "修复原版无敌计时为 1 或 2 时隔帧跳过镜头和碰撞更新的问题。对应 ZALiA 的 mod_PCUpdate1。"),
    ("Full jump speed is slightly easier to reach moving right in the original. ZALiA: mod_PCJumpDirBalancing.", "原版向右移动时更容易达到完整跳跃速度，此项平衡左右差异。对应 ZALiA 的 mod_PCJumpDirBalancing。"),
    ("The right-facing shield hitbox sits 2 px further out in the original. ZALiA: PC_init.", "修复原版朝右时盾牌碰撞框向外偏移 2 像素的问题。参考 ZALiA 的 PC_init。"),
    ("Crumble tiles react to either foot, not only the middle point. ZALiA: mod_CRUMBLE_TILES.", "易碎地板检测双脚，不再只检测中点。对应 ZALiA 的 mod_CRUMBLE_TILES。"),
    ("With 10 or less XP pending, the counter drains one at a time and leaves no leftover. ZALiA: update_xp.", "待结算经验不超过 10 点时逐点结算，避免残留。参考 ZALiA 的 update_xp。"),
    ("Off-screen Iron Knuckles must face Link to attack and give up once he is below them. ZALiA: mod_IronKnuckle_AggroAI.", "画面外的铁甲骑士必须朝向林克才会攻击，林克低于它们时会停止攻击。对应 ZALiA 的 mod_IronKnuckle_AggroAI。"),
    ("The Stalfos upthrust immunity no longer swallows your other sword hits. ZALiA: mod_STALFOS_CONTROL1.", "骷髅兵的上刺免疫不再吞掉其他剑攻击。对应 ZALiA 的 mod_STALFOS_CONTROL1。"),
    ("Wizards teleport across the whole room, spread out, instead of the first screen only. ZALiA: mod_Wizard_TELEPORT_AREA.", "巫师可分散传送到整个房间，不再限于第一屏。对应 ZALiA 的 mod_Wizard_TELEPORT_AREA。"),
    ("Mago shots spawn evenly on both sides and a lone Mago teleports closer. ZALiA: Mago ADJ1/ADJ3.", "马戈的弹丸在两侧均匀生成，单独出现的马戈会传送得更近。参考 ZALiA 的 Mago ADJ1/ADJ3。"),
    ("Carock, Gooma, Helmethead and Horsehead pause before their first attack. ZALiA boss changes.", "卡洛克、古玛、盔头和马头首领在第一次攻击前短暂停顿。参考 ZALiA 的首领调整。"),
    ("Helmethead's flying head rises back to the right height. ZALiA: mod_FenserFix1.", "盔头首领的飞头会升回正确高度。对应 ZALiA 的 mod_FenserFix1。"),
    ("As in the Japanese FDS version (the US game has a blue Iron Knuckle). ZALiA: mod_P5HorseHead.", "与日版 FDS 相同；美版此处为蓝色铁甲骑士。对应 ZALiA 的 mod_P5HorseHead。"),
    ("REFLECT bounces more kinds of shots, and bounced shots hurt enemies. ZALiA: mod_REFLECT_more_obj.", "反射魔法可反弹更多种弹丸，反弹弹丸可伤害敌人。对应 ZALiA 的 mod_REFLECT_more_obj。"),
    ("Run faster on the ground while holding the dash button. ZALiA: Dev tools > Faster movement speed.", "在地面按住冲刺键可加速奔跑。参考 ZALiA 开发者工具的移动加速。"),
    ("Falling into a pit or lava puts you back on safe ground instead of costing a life. ZALiA: the RESCUE FAIRY item.", "落入深坑或熔岩时返回安全地面，不损失生命。参考 ZALiA 的救援妖精道具。"),
    ("On the overworld, Select + B cycles through towns you have visited. ZALiA: randomizer flute warping.", "在大地图按 Select+B，循环传送到已访问的城镇。参考 ZALiA 随机化模式的笛子传送。"),
    ("Seed for the shuffles. Same seed, same game. ZALiA: File Select > RANDO > EDIT SEED.", "打乱所用种子。相同种子生成相同游戏。参考 ZALiA 存档选择页的随机化种子编辑。"),
    ("Items and sword skills owned from the start. ZALiA: RANDO > START ITEMS / START SKILLS.", "开局持有的道具和剑术。参考 ZALiA 的初始道具和初始技能设置。"),
    ("0 off, 1 Link and palaces, 2 every scene (stable per seed). ZALiA: PALETTE RANDO.", "0 为关闭，1 为林克和宫殿，2 为全部场景；每个种子的配色固定。参考 ZALiA 的配色随机化。"),
    ("Item shuffle is off while the ROM randomizer is on: that seed already placed the items, and this shuffle assumes the original locations. The other options here apply on top of the seed.", "ROM 随机化开启时，此处的道具打乱自动关闭：种子已经安排道具，而此功能依赖原版位置。这里的其他选项仍可叠加生效。"),
    ("Attack, magic and life at level 8 with every container. ZALiA: dev stab-to-cheat levels and containers.", "攻击、魔法、生命均为 8 级，并拥有全部容器。参考 ZALiA 的开发者等级与容器作弊功能。"),
    ("Rooms that are not in the original game. Every layout is original to z2rs, inspired by ZALiA (HoverBat's Zelda II remake); nothing is copied from it. In progress: areas not finished yet are saved but have no effect.", "原版以外的房间。所有布局均为 z2rs 原创，灵感来自 HoverBat 的 ZALiA 重制版，未复制其素材。尚未完成的区域仅保存选项，不产生效果。"),
    ("Master switch: lets the game enter rooms that are not in the original (virtual rooms). The areas below need it. Layouts are original, inspired by ZALiA.", "总开关：允许进入原版以外的虚拟房间。下方区域均需要开启。布局为受 ZALiA 启发的原创设计。"),
    ("Extra rooms off Parapa Palace (Palace 1). Original layouts, inspired by ZALiA's expanded Palace 1.", "帕拉帕宫殿（第一宫殿）的额外房间。原创布局，参考 ZALiA 扩展的第一宫殿。"),
    ("Optional rooms with a hard fight or platforming test and a reward. Original layouts, inspired by ZALiA's challenge rooms.", "提供高难度战斗或跳台挑战，并给予奖励的可选房间。原创布局，参考 ZALiA 的挑战房间。"),
    ("Small optional games for a prize. Original designs, inspired by ZALiA's minigames.", "可赢取奖品的可选小游戏。原创设计，参考 ZALiA 的小游戏。"),
    ("Secret caves holding heart or magic containers and other treasure. Original layouts, inspired by ZALiA's caves.", "藏有生命容器、魔法容器及其他宝物的秘密洞穴。原创布局，参考 ZALiA 的洞穴。"),
    ("A whole extra island with a monster town and a palace. Original layouts, inspired by ZALiA's Palace 8 and Bulblin town. Not part of the ZALiA preset.", "包含怪物城镇和宫殿的全新岛屿。原创布局，参考 ZALiA 的第八宫殿和布布林城镇。不包含在 ZALiA 预设内。"),
    ("A developer room (a few platforms, a Bot, a P-bag) between Great Palace maps 1 and 2, for testing virtual rooms.", "位于大神殿地图 1 和 2 之间的开发测试房，内有平台、史莱姆和经验袋，用于测试虚拟房间。"),
    ("With the ROM randomizer on, areas whose spare room slot the seed uses are skipped.", "ROM 随机化开启时，若种子使用了某区域需要的空闲房间槽位，则跳过该区域。"),
    ("Shake the picture on boss explosions and big hits. ZALiA: Options > Other > Screen shake (on there).", "首领爆炸和重击时画面震动。参考 ZALiA 的画面震动选项，该作默认开启。"),
    ("Colour the background flashes on spell casts and explosions. None turns the flashing off (easier on the eyes). ZALiA: Background flashing color (red there).", "设置施法与爆炸时的背景闪光颜色。“无”关闭闪光，减轻视觉刺激。参考 ZALiA 的背景闪光颜色，该作默认红色。"),
    ("Magnify the gameplay view around Link (display only). The status bar and the boss life meter stay at their normal size; menus and dialogue zoom back out. 2x with a 4x HD pack shows each pack pixel as a crisp block.", "以林克为中心放大游戏画面，仅影响显示。状态栏和首领生命条维持原大小，菜单与对话时恢复原倍率。4 倍高清材质配合 2 倍镜头，可将材质像素显示为清晰的方块。"),
    ("Off", "关闭"),
    ("Turn on the five effects below. ZALiA: Graphics Effects Editor (off there by default).", "开启下方五种效果。参考 ZALiA 的画面效果编辑器，该作默认关闭。"),
    ("The low-health beep plays less often after a few repeats. ZALiA: mod_LOW_HP_SOUND.", "低生命警报重复几次后降低播放频率。对应 ZALiA 的 mod_LOW_HP_SOUND。"),
    ("Original (North Palace)", "原版（北方宫殿）"),
    ("Palace entrance", "宫殿入口"),
    ("Last town visited", "上次访问的城镇"),
    ("Palace entrance or last town", "宫殿入口或上次城镇"),
    ("Original", "原版"),
    ("None (no flashing)", "无（关闭闪光）"),
    ("Gray", "灰色"),
    ("Red", "红色"),
    ("Violet", "紫色"),
    ("Green", "绿色"),
];

/// Every translated English key, for tests and tooling.
#[must_use]
pub fn translated_keys() -> &'static [(&'static str, &'static str)] {
    ZH
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chinese_differs_from_english() {
        for (en, zh) in ZH {
            assert_ne!(en, zh, "key {en:?} 的译文与原文相同");
            assert!(
                zh.chars().any(|c| ('\u{4e00}'..='\u{9fff}').contains(&c)),
                "key {en:?} 的译文没有汉字: {zh:?}"
            );
        }
    }

    #[test]
    fn no_duplicate_keys() {
        let mut seen = std::collections::BTreeSet::new();
        for (en, _) in ZH {
            assert!(seen.insert(*en), "重复的键: {en:?}");
        }
    }

    #[test]
    fn english_passes_through_unchanged() {
        assert_eq!(Lang::En.tr("Dialogue speed"), "Dialogue speed");
        assert_eq!(Lang::En.tr("no such key"), "no such key");
    }

    #[test]
    fn unknown_key_falls_back_to_english() {
        assert_eq!(Lang::Zh.tr("no such key"), "no such key");
    }

    #[test]
    fn chinese_lookup_works() {
        assert_eq!(Lang::Zh.tr("Dialogue speed"), "对话速度");
        assert_eq!(Lang::Zh.tr("Cheats"), "作弊");
    }
}
