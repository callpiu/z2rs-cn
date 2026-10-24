> 2026-10-24 菜单修复更新：后四种魔法名称重复已修复，游戏内 O 键菜单已汉化。验证、发布与限制见 [HANDOVER-MENU-FIX.md](HANDOVER-MENU-FIX.md)。下文为历史记录。

> 2026-10-23 大地图修复更新：进入大地图后的长柱/贴图错乱已定位并修复，见 [HANDOVER-OVERWORLD-FIX.md](HANDOVER-OVERWORLD-FIX.md)。下文关于 CHR 无切换和兼容分支的旧结论已被本次真实场景验证推翻。

> 2026-10-23 续作更新：增强页和随机化页汉化及编译修复已完成，新增验证与发布情况见 [HANDOVER-I18N-COMPLETE.md](HANDOVER-I18N-COMPLETE.md)。下文“当前工作树”与“未完成”描述保留为历史记录，以本更新为准。

# z2rs 汉化项目 · 全程汇总

> 记录时间：2026-10-23
> 源码目录：`D:\z2rs-060-zh`（官方 z2rs 0.6.0 + 汉化移植）
> 发布目录：`D:\z2rs-中文版-0.6.0`（可用） / `D:\z2rs-中文版`（旧版，保留对照）
>
> **当前状态：已发布版本可用；工作树有一次未完成的重构，编译失败。**
> 未完成部分的交接见 [`HANDOVER-I18N-SCOPED.md`](HANDOVER-I18N-SCOPED.md)。

---

## 一、起点与目标

起始任务是"继续这个项目"：让 **z2rs**（一个 Zelda II 的 Rust 模拟器）能运行**汉化版 ROM**
`Zelda II - The Adventure of Link (U) [20260806_NOKOH].nes`。

汉化版与原版的技术差异，是全部困难的来源：

| | 原版 美版 | 汉化版 |
|---|---|---|
| Mapper | MMC1 | **MMC5** |
| PRG | 128 KB | 256 KB |
| CHR | 128 KB（32 页） | **512 KB（128 页）** |
| 中文字形 | — | 页 0-63 简体 / 64-127 繁体，靠 `$5130` 平移 |

---

## 二、阶段一：MMC5 支持与一连串 bug（旧版基线）

### 1. MMC5 mapper 实现

`crates/z2-core/src/mmc5.rs`（697 行）：PRG 四种模式、CHR 四种模式、
`$5128-$512B`（四个 **8 位** CHR bank 寄存器）、`$5130`（CHR 高位 = 简繁切换）、
`$5105`（nametable 映射）、ExRAM、扫描线 IRQ、硬件乘法器。

### 2. 真凶：PRG 模式 1/2 的寄存器分配写错

之前"钉死 `$E000` 是最后一个 bank"的补丁是在绕错。改正为教科书语义
（模式 1/2 用 `$5115` 驱动 `$8000`，模式 2 的 `$E000` 由 `$5117` 驱动）后 ROM 一次跑通。

### 3. `$5105` 没接进 PPU

部分场景从左边 nametable 取右半边的图块 → 贴图错乱。
修法三处：`apply_mmc5` 调 `set_mirroring`、`bus_write` 触发范围加 `$5105`、缓存 key 并入 `nt_map`。

### 4. `$5130` 位宽错

写成 `<< 10` 应为 `<< 8`，页号取模 512 后绕回同一页 → 按右键只改 8 个像素。

### 5. 宽屏 / 双人 / HD 包在 MMC5 上静默失效

`emu_from_generic_ines` 把 `feats` 丢了，两个增强都注册不上。

### 6. 边缘精灵用错 CHR 页

MMC5 的 A 组（精灵）/ B 组（背景）必须分开取值。修了两次：
先加 `load_chr_4k_bg`，再发现 `record.page` 存的是 1 KiB bank 号而非 4 KiB 页号。

### 7. 宽屏边距是乱的（用美版 ROM 对照才定位）

根因：`area_bank`（`$0769`）是 **16 KiB** 索引，MMC5 硬件是 8 KiB，差了一个因子 2。
所以 `bank * 2 * 0x2000`，`$A000` 半边是 `+0x2000`。

### 8. P2 打不出伤害

汉化路径没注册任何 trap，coop 的 3 个覆盖项无从挂载（`register_coop_traps`
只在已有基础 trap 时才包装）。加上后又发现 `bank7` 的 MMC1 串行切换器
在汉化版上是数据 → 改为 byte-check（`uses_mmc5_switcher`）跳过。

### 9. 对话框中文乱码（修了 5 轮才找对）

这是最曲折的一个：

| 轮次 | 尝试 | 结果 |
|---|---|---|
| 1 | 整帧丢 HD | 太突兀 |
| 2 | 按行丢 HD | 横条仍突兀 |
| 3 | 只丢 `background_pixel` 的 cell | 仍乱码 |
| 4 | 发现单位搞错（像素列 vs slot 索引）| 仍乱码 |
| **5** | **真凶：back 层从没被 `text_box` 挡住** | 修好 |

中文笔画空隙是透明的，HD 的**整屏 back 层**从缝隙里透出来。
这完美解释"室内正常、室外乱码"——室内场景没有 back 层。

最终方案加了一道**兜底**：渲染完一行后，用 NES 原始像素无条件覆盖窗口区域
（`restore_nes_span`），不依赖"是否找全了绘制路径"。

### 10. 城镇地面用原版材质

用户提供的日志显示地面是 `4Bp47` —— tile 75 但 **page 0x47（71）**。
71−64 = 7，正是 HD 包定义的页。

**繁体偏移 64 页**，而 `covers_tile`（`over_tiles` 白名单）漏了这个回退。
`HdPack::lookup` 有 −64 回退，`covers_tile` 没有。

### 11. 简繁搞反了

最初从 `$9E70` 汇编反推"右键→0"，用户实测反馈"默认还是繁体"。
逐键实测结果：

| 按键 | `$5130` | 字形 |
|---|---|---|
| 右键 `0x80` | 1 | **繁体** |
| 左键 `0x40` | 0 | **简体** |
| 不按 | 0 | 简体（启动默认）|

而且**右键不是 toggle** —— 连按 4 次仍是繁体，要回简体必须按左键。

原因：`$F7` 是移位后的位序，与 `poll_pad1` 的输出不是同一套编号。
**这类结论必须以实测为准，不能靠反汇编推断。**

用户选择保留切换能力，所以只修正了文档和测试，没改逻辑。

### 12. 存档不带 MMC5 状态

`GameState` 只有 `mmc1`。受影响的不是 F7（`.z2snap` 本来只存 RAM+WRAM，
所有 mapper 一致），而是：

* `netplay.rs` 的回滚 `save_state`/`load_state` → 恢复不了 CHR bank、简繁位、IRQ、ExRAM
* `hash_state(to_bytes())` → 连 MMC5 漂移都看不见，**不同步检测不出来**

修法：`GameState` 加 `mmc5` + `mmc5_in_use`，`STATE_VERSION` 3→4，
`load_state` 还原寄存器后调 `sync_from_mapper_mmc5` 重算 CHR 槽位和 `$5105` 镜像。

---

## 三、阶段二：迁移到官方 0.6.0

旧仓库没有配置 git remote，源码是用户下载的 `z2rs-0.6.0.zip`。

### 做法

1. 解压 0.6.0，`git init` + 提交为 `736f8e1 upstream z2rs 0.6.0`
2. 旧仓库里把未提交的改动落成提交 `4c0b6dc`
3. 新仓库 `git remote add old <旧仓库>`、`fetch`，再 `git cherry-pick daa89c2..old/master`

这样 git 能用"父提交"做真正的三方合并。
`git apply --3way` 走不通 —— 报 `repository lacks the necessary blob`，
因为新仓库没有基线对象。

### 6 处冲突

| 文件 | 冲突内容 | 解法 |
|---|---|---|
| `z2-core/src/cpu.rs` | 0.6.0 在 `$6000-$7FFF` 加了 `wram-probe` | 两边都要 |
| `z2-core/src/ppu_bind.rs` | `mapper_key` 类型 + `text_writes` 字段 | 用我们的 `Option<u64>`；`text_writes` **删掉**（0.6.0 已移除该机制）|
| `z2-render/src/compositor.rs` | `ComposeInput` 加 `time`（上游）/ `text_rect`（我们）| 两个字段都留 |
| `z2-render/src/presenter.rs` | 同上，构造处 | 两个都传 |
| `z2-render/tests/bench.rs` | 同上 | 同上 |
| `z2-render/tests/compositor.rs` | 同上 + 函数错位 | 见下 |

**`tests/compositor.rs` 的做法**：不要逐块合并。以 0.6.0 的文件为基准，
把我们新增的 5 个测试函数**追加**到末尾。逐块拼会把 `compose_at` 的函数体挤掉、
`ComposeInput` 字面量字段重复或缺失。

### 移植后连带修的三处

1. **`Enhancements` 新增 `content` 字段**（官方新 crate `z2-content`），
   任何完整结构体字面量都要补 `content: Default::default()`
2. **`ComposeInput` 恢复 `Copy`**（0.6.0 是 `Copy`，我们旧版改成了 `Clone`），
   测试里大量 `..input` 结构体更新要求 `Copy`
3. **`sprite_page` 回退**：0.6.0 的测试只设 `chr_page`，`sprite_page` 停在
   `NO_PAGE` 就画成黑色。改成 `NO_PAGE` 时回退 `chr_page`

---

## 四、阶段三：材质包适配

### 作者 2026-10-22 首发版缺 4 个文件

`environment/layers/W_0.png ~ W_3.png` —— 19 个水面动画层用的 4 帧。
数字完全对得上：

| 项 | 首发版 | 修复版 |
|---|---|---|
| `pack.json` 引用 PNG | 22,920 | 22,920 |
| 磁盘实际 PNG | 22,916 | **22,920** |
| `RELEASE.json` 声明 | 22,916 | **22,920** |
| 缺失 | 4 | **0** |
| `W_0~3.png` | 无 | 有（各 1024×832）|

**为什么报错就废掉整个包**：0.6.0 的加载器是预读所有图层帧的，
遇到第一个不存在的文件直接中止，且**整帧都不输出**。

### 我做的过渡补丁

`E:\z2rs-hd-pack-2026.10.22-unoutlined-playable` —— 用**目录联接**指向原包
（不复制 203 MB），只重写 `pack.json` 剔除 19 个缺帧动画层。

作者补档后已验证修复版可用，**这个补丁目录已不需要，可删**。

### 修复版验证

* 两个变体（outlined / unoutlined）均加载正常，`interpreter faults: 0`
* 宫殿场景 HD 差异 603,191（unoutlined）/ 603,473（outlined）像素
* 宽屏 16:9 输出 1728×960，扩展区 HD 正常
* **水面动画真的在动**：四帧互不相同（各差约 8,000-9,000 像素），
  渲染随时间推进（相邻 12 帧差异 7,232 / 8,704 / 12,432 像素）

最后一条同时证明：移植时保留的 0.6.0 `ComposeInput::time` 字段确实在驱动动画。

---

## 五、阶段四：三组性能 trap —— 含一次交付错误

### 调查结论

汉化版固定 bank 的 23 个入口里 **17 个与美版逐字节相同**，
`$DE6C` 只差前 10 字节（NOP/数据填充），真正卡住的只有 4 个 MMC1 切换器。

| 地址 | 汉化版 | 美版 |
|---|---|---|
| `$FFCC` | `0A 09 80 8D 15 51 60`（`ASL A`/`ORA #$80`/`STA $5115`/`RTS`）| MMC1 串行 |
| `$FFC9` | `AD 69 07 0A 09 80 8D 15 51 60` | MMC1 串行 |
| `$FFC5` | `A9 00 F0 03 AD 69 07 0A 09 80` | MMC1 串行 |
| `$FFB1` | `52 68 AA 68 40` ← **数据，不是代码** | MMC1 串行 |

`jsr_sub` 本身是 mapper 无关的（target 没被 trap 就解释执行 ROM 自己的字节），
所以那 17 个相同的入口完全没问题。

**另一个关键事实**：汉化版固定 bank 里**没有任何一处写 `$5120-$512B`**
（正则搜 `8D 2X 51`，零命中），只在 `$DDD7` 写 `$5130`。
说明 CHR 是 512KB 全静态装载，靠 `$5130` 整体平移 —— **`SwapCHR` 是空操作**。

### 修法

给共用的 `jsr_sub` 加 `mmc5_switcher_call`，按汉化版真实语义写 `$5115`。

实测证明编码是 `(A << 1) | 0x80`：ROM 自己在 frame 3 写 `$5115 = 0x8A`，
正好等于 `LDA #$05 / JSR SwapPRG` 的结果。

（我第一版写 `a >> 1` 是错的，导致 11 个测试失败；修正后降到 2 个，最后归零。）

### 性能结论是负面的

| 场景 | 启用三组 | 基线（未启用）|
|---|---|---|
| 标题 900 帧 | 3.07 s（293 帧/秒）| 3.01 s（299 帧/秒）|
| 游戏内 3000 帧 | 7.51 s（400 帧/秒）| 7.45 s（403 帧/秒）|

**启用后甚至略慢（噪声范围内）。三组 trap 在汉化 ROM 上不带来可测的加速。**

原因：Rust 端口按美版 MMC1 的 bank 布局和寄存器约定写，
汉化版上 4 个切换器被接管（否则撞非法指令），但端口体内其余对固定 bank
例程的调用仍要 fallback 到解释器 —— 该解释的还是解释了，反倒多了一层 trap 分发。

### 保留它们的理由：行为一致性

启用后汉化版与美版在**完全相同的输入**下走到**相同的场景序列**：

| | 到访场景序列 | P2 hp | p2_contacts | n_display/n_update |
|---|---|---|---|---|
| 美版 | `[0, 39]` | 127（未掉血）| 0 | 2077 / 2228 |
| 汉化（启用三组）| `[0, 39]` | 127（未掉血）| 0 | 2072 / 2222 |
| 汉化（未启用，旧）| `[0, 22]` ← 偏离 | — | — | — |

### ⚠️ 我犯的交付错误

第一次交付时 `register_translated_traps` **只改了 doc 注释、没改函数体** ——
注释写着"Every group is registered, including title_traps…"，函数体还是 6 组。

那次声称的"24 个端到端全过"是在**未提交的工作区改动**下跑的，
改动随后在 `git checkout --` / `git stash` 中丢失。
只有 `bank7_common.rs` 的 MMC5 适配留下了（在另一个文件里提交过）。

下一轮读注释准备"核实"时差点又照抄，翻函数体才发现。已修正并补了
**直接数注册表**的防回归测试 `all_nine_trap_groups_are_registered_on_translated_rom`
（要求 >60 条，且 title/sideview/overworld 代表地址都被注册）。

### 顺带修正的一个假失败测试

`coop_p2_takes_part_in_combat` 断言"P2 必须掉血"。启用三组后场景从 22 变 39，
39 里没有敌人打到 P2 → 假失败。

**实测美版在同一输入下也是 P2 全程满血、`p2_contacts=0`** ——
所以掉血是场景运气，不是"伤害通道是否接通"的证据。

改名 `coop_p2_runs_in_gameplay`，断言改为 `n_display > 0 && n_update > 0`
（每帧必跑的 P2 显示/更新通道）。

---

## 六、阶段五：启动器汉化

### 主界面

新增 `crates/z2-launcher/src/i18n.rs`：`Lang` 枚举 + 两张 `(英文, 译文)` 常量表，
**键就是英文原文**（调用点不用查表，英文表本身就是"有哪些字符串"的清单）。

覆盖：三个页签、ROM/显示/HD 包/玩家/联机全部区块、操作说明表、命令行、
高级设置、窗口标题（切语言同步变）、以及**校验提示**（"找不到 ROM 文件"这类
点开始前最常看到的）。

默认中文，可在「开始游戏」页顶部切 English，选择存入 `launcher.json`，
旧配置文件无该字段也能读。

**有意保留英文**：材质包名称、随机化模块名（来自用户目录和 `z2-rando` 标识）。

### 字体问题（用户截图反馈"全是方块"）

**原因不是 i18n** —— 文字确实换成中文了，问题是
**egui 内置字体（Ubuntu-Light / NotoEmoji / Hack）不含汉字**，
每个汉字都落到"无字形"分支，画成空心框。

新增 `crates/z2-launcher/src/font.rs`：

* 按平台列候选路径，逐个 `fs::read`，第一个读成功且 >1KB 的用上
* `FontData::from_owned(bytes)` 塞进 `font_data`
* 把 `"system-cjk"` **append**（不是 insert 到开头）到
  `Proportional` 和 `Monospace` 两个族末尾
* `apply_style()` 里调用；找不到时打印提示而非静默

**为什么 append 到末尾**：拉丁字符继续用 egui 自带字体（小字号 hinting 更好），
只有它画不出来的字符才 fallback 到系统字体。

**为什么不打包字体**：中文字体好几 MB，还要带授权；每台桌面机器本来就有。

候选顺序：

| 平台 | 顺序 |
|---|---|
| Windows | `msyh.ttc`（微软雅黑）→ `msyh.ttf` → `simhei.ttf` → `simsun.ttc` → `Deng.ttf` |
| macOS | `PingFang.ttc` → `Hiragino Sans GB.ttc` → `Arial Unicode.ttf` |
| Linux | `NotoSansCJK-Regular.ttc` → `NotoSansCJKsc-Regular.otf` → `wqy-zenhei.ttc` → `uming.ttc` |

本机实测命中 `C:\Windows\Fonts\msyh.ttc`（19.7 MB）。

### 字体验证（这条值得单独说）

只检查"字体文件找到了"是**不够的**：`.ttc` 是字体集合，
epaint 用 ab_glyph 只读**第一个 face**，而第一个 face 未必含汉字。

`crates/z2-launcher/tests/font_render.rs` 真的用 egui 排版一次并量宽度：

```
「开始游戏」= 64.00   「Play」= 29.28   缺字对照 = 82.28
```

判据：**汉字是全角的，16px 字号下每个正好 1 个 em**，4 字 = 64px。
断言 `|实测 - em×字数| <= 0.5em`。

`缺字对照` 用私有区码位（`\u{E000}` 起）量，得到 82.28 ——
**豆腐块反而比汉字宽**。我最初写的"中文应比缺字宽"是反的，
被测试自己抓出来，改成"约等于 em×字数"。

### 增强页与随机化页（进行中，见第九节）

---

## 七、生成物清单与作用

### 可运行的发布物

#### `D:\z2rs-中文版-0.6.0\` —— 当前可用版本（对应提交 `0b489d7`）

| 文件 | 大小 | 作用 |
|---|---|---|
| `z2rs.exe` | 59,616,796 | **游戏本体**。含 MMC5 支持，可直接跑汉化 ROM |
| `z2-launcher.exe` | 25,236,891 | **启动器**。图形界面配置并启动游戏 |
| `z2-signal.exe` | 5,263,495 | **联机信令服务器**。网络对战时双方握手中转 |
| `z2rs-中文版-0.6.0.zip` | 29,941,152 | 上面三个 + 文档的打包，可直接分发 |
| `SHA256SUMS` | 237 | 三个 exe 的校验和 |
| `版本说明.txt` | 11,042 | 完整变更记录、已知问题、使用说明 |
| `README-官方.md` | 6,935 | 官方 0.6.0 说明 |
| `HOW-TO-PLAY.txt` | 3,970 | 官方操作说明 |
| `LEGAL.md` / `LICENSE` | 2,840 / 1,091 | 法律与授权 |

#### `D:\z2rs-中文版\` —— 旧版（0.5.x 基线），保留作对照

| 文件 | 大小 | 作用 |
|---|---|---|
| `z2rs.exe` | 59,191,894 | 旧版游戏本体 |
| `z2-launcher.exe` | 24,756,997 | 旧版启动器 |
| `z2-signal.exe` | 5,263,447 | 旧版信令服务器 |
| `z2rs-0.1.0-zh-CN-...zip` | 29,787,434 | 旧版打包 |
| **`Zelda II ... [20260806_NOKOH].nes`** | 786,448 | **汉化版 ROM**，所有测试都用它 |
| `交接文档.md` | 14,260 | 旧版交接记录 |
| `测试说明.txt` | 1,033 | 旧版测试说明 |
| `LUNAHOOK.md` / `README.zh-CN.md` | — | 文本钩子与中文说明 |

### 源码仓库

#### `D:\z2rs-060-zh\` —— 官方 0.6.0 + 汉化移植（git 仓库）

11 个提交，`736f8e1 upstream z2rs 0.6.0` → `0b489d7`。
**当前工作树脏且有编译错误**（见第九节）。

#### `D:\下载\Compressed\main\z2rs-main\` —— 旧仓库（0.5.x 基线）

保留历史，旧版译文（`z2-enh-ui` 的 173 条中文）在这里可查。

#### `D:\z2rs-build-060\` —— 构建产物

`release/` 下是编译好的 exe；`zh.patch`、`enh_zh.json`、`enh_strings.json`
等是过程中的中间数据。

### 文档

| 文件 | 大小 | 作用 |
|---|---|---|
| `HANDOVER.md` | 47,803 | **完整交接**：MMC5 全链路、简繁实测、材质包、移植过程 |
| `HANDOVER-TRAPS.md` | 18,386 | **三组 trap 全过程** + 交付错误复盘 + 启动器汉化 + 字体问题 |
| `HANDOVER-I18N-SCOPED.md` | 8,689 | **当前未完成状态的接手文档**（增强页/随机化页）|
| `MMC5-PROGRESS.md` | 11,587 | MMC5 专项进度与踩坑记录 |
| `PROJECT-SUMMARY.md` | 本文件 | 全程汇总 |
| `README.md` / `CONTRIBUTING.md` / `LEGAL.md` / `PROVENANCE.md` | — | 官方文档 |

### 材质包（作者提供，非本项目产物）

| 目录 | PNG | 作用 |
|---|---|---|
| `D:\z2rs-hd-pack-2026.10.23-unoutlined` | 22,920 | 作者修复版，**无描边**（推荐）|
| `D:\z2rs-hd-pack-2026.10.23-outlined` | 22,920 | 作者修复版，**有描边** |
| `E:\z2rs-hd-pack-2026.10.22-unoutlined-playable` | 0（目录联接）| **我做的过渡补丁**，作者补档后**已不需要，可删** |

材质包构成：824 图层 / 737 角色动画 / 113 图集 / 22,920 PNG / 约 204 MB。

### 测试文件（源码内）

| 文件 | 项数 | 作用 |
|---|---|---|
| `crates/z2-native/tests/mmc5_zh_rom.rs` | 25 | **汉化 ROM 端到端**：启动、简繁切换、宽屏、双人、边缘精灵、HD 让位、存档 MMC5、增强补丁、9 组 trap 注册 |
| `crates/z2-enh-ui/tests/i18n_render.rs` | 2 | 用 accesskit 收集一帧**实际渲染文本**，断言中文模式无残留英文 |
| `crates/z2-launcher/tests/font_render.rs` | 1 | 量中文排版宽度，证明是真字形而非豆腐块 |
| `crates/z2-launcher/tests/i18n_settings.rs` | 5 | 默认中文、JSON 往返、旧文件兼容、语言不影响启动参数、提示真翻译 |
| 核心测试（`z2-core`/`z2-ppu`/`z2-render`）| 957 | 单元与集成测试 |
| 启动器测试 | 40 | UI 渲染、设置校验、args 生成 |

### 关键代码改动及作用

| 文件 | 作用 |
|---|---|
| `z2-core/src/mmc5.rs` | MMC5 mapper 全部实现（PRG/CHR 模式、寄存器、ExRAM、IRQ、乘法器）|
| `z2-core/src/ppu_bind.rs` | A/B 两组 CHR 分开装载；`$5105` 接进 PPU 镜像 |
| `z2-core/src/state.rs` | 存档加 MMC5 块，`STATE_VERSION` 3→4 |
| `z2-core/src/bank7_common.rs` | `jsr_sub` 的 MMC5 切换器适配（`mmc5_switcher_call`）|
| `z2-core/src/wide_margins.rs` | 宽屏边距的 mapper 感知 PRG 读取（修正 16K/8K 因子）|
| `z2-core/src/wide_sprites.rs` | 边缘精灵 shadow 同步 mapper |
| `z2-ppu/src/wide.rs` | 边缘精灵用 `sprite_page`，`NO_PAGE` 回退 `chr_page` |
| `z2-render/src/compositor.rs` | `text_box` 检测 + `restore_nes_span` 兜底 |
| `z2-render/src/pack.rs` | `covers_tile` 的繁体 −64 页回退 |
| `z2-render/src/presenter.rs` | 传递 `text_rect` |
| `z2-native/src/app.rs` | `register_translated_traps`（9 组）、`refuse_unsupported_features` |
| `z2-launcher/src/i18n.rs` | 启动器字符串表 + `Lang` |
| `z2-launcher/src/font.rs` | 系统 CJK 字体加载 |
| `z2-launcher/src/ui.rs` / `settings.rs` / `app.rs` | 界面接 i18n、`validate_lang`、窗口标题同步 |
| `z2-enh-ui/src/i18n.rs` | 148 条增强页译文（**未完成接线**）|

---

## 八、当前可用版本的能力与验证

### 功能

MMC5 支持 / 简繁切换 / HD 材质包 / 宽屏（16:10、16:9、21:9）/ 边缘精灵 /
双人合作 / 即时存档（带 MMC5 状态）/ ZALiA 式增强 / 显示增强 /
**启动器主界面中文**

### 验证（`0b489d7`）

```
核心测试（z2-core + z2-ppu + z2-render）   957 项全过
汉化 ROM 端到端                              25 项全过
启动器                                       40 项全过
材质包修复版                                 实测可行，水面动画在动
解压后 汉化+材质包 / 美版                    均 faults 0
```

### 已知限制

1. **随机化器**只支持美版 ROM，汉化版上会提示并跳过
2. **文本钩子**（`--export-text`）只认原版英文字母表，汉化版抓不到
3. **三组 trap 无加速**（见第五节）
4. **游戏增强页 / ROM 随机化页仍是英文**（进行中）

---

## 九、未完成部分（工作树）

**增强页和随机化页汉化。**

* 148 条增强页译文已写好（`crates/z2-enh-ui/src/i18n.rs`）
* 接线代码完成（`Opts` 加 `lang`、6 个叶子函数、`group()`、`intro()`、`preset_buttons()`）
* 渲染测试曾验证 **81/81 全中文，0 条残留英文**
* **但最后一步破坏性重构导致编译失败**：删掉了 launcher 自己的 `Lang` 枚举
  想共用类型，而 `t()`/`label()`/`ALL` 只存在于被删的那份定义里

| 目标 | 状态 |
|---|---|
| `cargo build -p z2-enh-ui` | ✅ 通过 |
| `cargo test -p z2-enh-ui` | ❌ 2 个（`Opts` 缺 `lang`）|
| `cargo build -p z2-launcher` | ❌ **98 个** |
| `z2-native` overlay | 未测（函数签名改了，必然要改）|
| `rando_ui`（约 190 处英文）| **一行未动** |

**接手第一步**：按 [`HANDOVER-I18N-SCOPED.md`](HANDOVER-I18N-SCOPED.md)
第三节「修法 A」恢复 launcher 的 `Lang` 枚举（`git show 0b489d7:crates/z2-launcher/src/i18n.rs`
能拿到原文），就能回到可编译状态。

---

## 十、我犯过的错（值得记住）

1. **注释与代码不符时，我信了注释**
   三组 trap 那次，函数体没改只改注释，我读注释就下结论，
   还让"测试全过"发生在未提交状态。**教训：先信代码；「测试全过」必须确认
   是在已提交/已保存状态下跑的。**

2. **做界面本地化时没想到字体**
   文字换成中文但字体画不出，整屏方块。**字体和文本是两件事。**

3. **断言方向写反**
   "中文应比豆腐宽"是错的，豆腐反而更宽。被测试自己抓出来。

4. **合成测试代替真实场景**
   对话乱码修了 5 轮，因为合成测试没构造"back 层 + 透明笔画"这个组合。
   **用户的两次日志输出才是决定性的。**

5. **验证结构性事实应直接检查状态**
   "某组 trap 是否启用"应该数注册表，而不是靠行为间接推断。
