# 交接总结 · 三组性能 trap + 启动器汉化

> 记录时间：2026-10-23 17:05
> 工作目录：`D:\z2rs-060-zh`（官方 0.6.0 + 汉化移植）
> 状态：**两件事都已完成并验证。**
>
> * 三组性能 trap：已启用，24 个端到端全过 —— **但实测没有加速**，
>   原因见第六节，这是本轮最重要的结论。
> * 启动器汉化：已完成，37 项测试全过，默认中文、可切英文。
>
> 第三节把每个文件的改动和作用列清楚了。

---

## 一、现在的状态

### 已完成并验证的

| 项 | 状态 |
|---|---|
| MMC5 mapper / 简繁切换 | ✅ |
| HD 材质包（作者 2026-10-23 修复版） | ✅ 缺失 0 文件 |
| 水面动画（0.6.0 的 `time` 字段驱动） | ✅ 实测在动 |
| 宽屏 / 边缘精灵 / 双人 | ✅ |
| 存档带 MMC5 状态 | ✅ |
| 三组性能 trap | ✅ 已启用（无加速，见第六节） |
| 启动器汉化 | ✅ 默认中文，可切英文 |
| 测试 | 核心 957 + 端到端 24 + 启动器 37 全过 |

发布目录：`D:\z2rs-中文版-0.6.0\`
材质包（作者修复版，已解压就位）：

    D:\z2rs-hd-pack-2026.10.23-outlined
    D:\z2rs-hd-pack-2026.10.23-unoutlined

### 本轮在做的：三组性能 trap

目标：把 `title_traps` / `sideview_traps` / `overworld_traps` 也注册到
汉化 ROM 上。这三组此前被 `register_translated_traps()` 跳过。

**结论：三组已成功启用，24 个端到端测试全过；但实测没有加速
（见第六节）。** 最初的「补齐后更快」这个前提不成立。

### 已解决的三个问题

**1. `SwapPRG` 的 bank 算法（已修）**

初版我写 `a >> 1`，是错的。实测汉化 ROM 自己在 frame 3 写
`$5115 = 0x8A = 5*2 | 0x80`，正好等于 `LDA #$05 / JSR SwapPRG` 的结果，
证明 `(A << 1) | 0x80` 才是正确编码。

另外核对 `mmc5.rs` 的既有测试（`prg_mode_2_uses_textbook_register_split`，
第 538 行）也印证：写 `$5115 = 0x8E` 后 `$8000` 落到 8K bank 14，
即 `$5115` 存的是**编码后的字节**，mode 2 下低位被掩掉。

**2. 两个测试失败（已修）**

失败从最初 11 个降到 2 个，再降到 0 个。关键是 `margin_sprites` 那个，
修好 bank 算法后立刻转绿 —— 证明 bank 编码确实是根因。

`coop_p2_takes_part_in_combat` 一度让我误判为「场景不同所以没战斗」，
实际是我诊断脚本读错了 mode 地址（把 `$0736` 当 mode，但那个构建里
`$0736` 已经不是 mode），白绕了一圈。**教训：诊断脚本先验证字段语义。**

**3. 启动器汉化（未开始）**

本轮完全没动。见第七节。

**3. 启动器汉化还没开始**

本轮只做了 trap，启动器汉化一行未动。

---

## 二、调查结论：三组 trap 为什么原先被跳过

原注释的说法是：这些端口的函数体会 `jsr_sub` 到固定 bank 的写死后继地址，
汉化版把那些调用点挪走了，所以套用会撞非法指令。

**实测证明这个说法只对了一半。** 我用两个 ROM 逐字节对比了
这三组用到的 23 个固定 bank 入口：

| 地址 | 汉化 vs 美版 |
|---|---|
| `$C33C` `$C9A5` `$CB35` `$CC97` `$CCB3` `$C644` `$CF4C` `$CFFC` `$D603` `$D625` `$D6C1` `$D8C2` `$D991` `$E16F` `$E1BE` `$E079` `$D346` | **17 个完全相同** |
| `$DE6C` | 前 10 字节不同（汉化是 NOP+数据表，美版是 ASCII 串），第 11 字节起代码一致 |
| `$FF9D` `$FFB1` `$FFC5` `$FFC9` `$FFCC` | 5 个 MMC1 切换器，语义不同 |

而 `jsr_sub` 本身是 mapper 无关的：target 没被 trap 时就解释执行 ROM 自己的
字节。所以那 17 个相同的入口完全没问题。

真正卡住的只有 4 个切换器（`$FF9D` 那个本来就被
`bank7_traps` 的 `uses_mmc5_switcher` 字节检查跳过了，没有 MMC5 对应物）。

汉化版这四个的实际字节：

| 地址 | 汉化版 | 美版 |
|---|---|---|
| `$FFCC` | `0A 09 80 8D 15 51 60` | `8D 00 E0 4A ...` |
| `$FFC9` | `AD 69 07 0A 09 80 8D 15 51 60` | `AD 69 07 8D 00 E0 ...` |
| `$FFC5` | `A9 00 F0 03 AD 69 07 0A 09 80` | `A9 00 F0 03 AD 69 07 ...` |
| `$FFB1` | `52 68 AA 68 40` ← **数据，不是代码** | `8D 00 A0 4A ...` |

所以直接解释执行会 `illegal opcode $52 at $FFB1`（这正是实测报的错）。

**另一个关键事实**：汉化版固定 bank 里**没有任何一处写 `$5120-$512B`**
（我用正则搜过 `8D 2X 51`，零命中），只在 `$DDD7` 写 `$5130`（简繁位）。
说明它的 CHR 是 512KB 全静态装载，靠 `$5130` 整体平移窗口，
**运行时不切 CHR bank** —— 所以 `SwapCHR` 在汉化版是空操作。

---

## 三、本轮每个文件的改动

### 1. `crates/z2-core/src/bank7_common.rs`（+66 行）

**作用**：给 `jsr_sub` 加 MMC5 适配。这是本轮的核心。

改动内容：

- `jsr_sub()` 开头插入一个分支：mapper 是 MMC5 时先试
  `mmc5_switcher_call()`，返回 `true` 就不再走原来的
  `continue_in_frame()`。
- 新增 `mmc5_switcher_call()`，处理 4 个切换器地址：

  | target | 处理 |
  |---|---|
  | `$FFB1` `SwapCHR` | 直接 `return true`，空操作 |
  | `$FFCC` `SwapPRG` | 取 `cpu.a`，写 `$5115` |
  | `$FFC9` `SwapToSavedPRG` | 取 `ram[$0769]`，写 `$5115` |
  | `$FFC5` `SwapToPRG0` | 取 `ram[$0769]`，写 `$5115` |
  | 其他 | `return false`，交回原逻辑 |

- 写 `$5115` 后调 `ppu.mapper_sync_mmc5()` 让 PPU 重新选页，
  并 `cycles += 6` 补上 JSR 的周期。

**这里有问题 1 的破绽**：`$FFCC` 分支写进 `$5115` 的值算错了，
注释也写错了。见第一节。

**改动的风险面**：`jsr_sub` 是 `title_traps` / `sideview_traps` /
`overworld_traps` / `bank7_*` **共用**的入口。MMC1 路径完全不受影响
（有 `mapper == Mmc5` 守卫），但 MMC5 路径上所有端口的
切换器调用都走这个新函数，所以问题 1 会影响全部三组。

### 2. `crates/z2-core/src/cpu.rs`（1 行）

**作用**：把 `BUS_ACCESS_OFFSET` 从私有 `const` 改成
`pub(crate)`。

原因：`bank7_common.rs` 的 `mmc5_switcher_call()` 需要用它算
PPU 同步的周期。它原本只在 `cpu.rs` 内部用。

这是纯可见性改动，无语义影响。

### 3. `crates/z2-native/src/app.rs`（+37 -11 行）

**作用**：`register_translated_traps()` 里启用三组 trap。

```rust
z2_core::sideview_traps::register_sideview_traps(game);       // 新增
z2_core::sideview_traps::register_overworld_traps(game);      // 新增
...
z2_core::title_traps::register_title_traps(game);              // 新增
```

同时把函数上方的 doc 注释整段重写，记录第二节那些实测结论
（17 个入口相同、4 个切换器需适配、CHR 静态不切 bank），
免得下次又有人凭猜测把它改回去。

**这个函数只对 MMC5 镜像调用**（`emu_from_generic_ines` 路径），
美版走 `register_native_traps()`，不受影响。

---

## 四、验证到了哪一步

| 步骤 | 结果 |
|---|---|
| `cargo build -p z2-native` | ✅ |
| 启用三组 trap（无 MMC5 适配） | ❌ **11 个测试失败**，`illegal opcode $52 at $FFB1` |
| 加 MMC5 适配（bank 算法仍错） | ⚠️ 22 过 / 2 败 |
| 修好 bank 算法 | ✅ **24 个全过** |
| 核心测试（z2-core/ppu/render） | ✅ **957 全过**（MMC1 路径未回归） |
| 性能对比 | ❌ **无加速**，见第六节 |

---

## 六、性能结论：没有加速（重要）

这是本轮最该知道的结论，它推翻了最初的目标前提。

实测（同一台机器，同一组输入，墙钟时间）：

| 场景 | 启用三组 | 基线（未启用） |
|---|---|---|
| 标题 900 帧 | 3.07 s（293 帧/秒） | 3.01 s（299 帧/秒） |
| 游戏内 3000 帧 | 7.51 s（400 帧/秒） | 7.45 s（403 帧/秒） |

**启用后甚至略慢，且差异在测量噪声内。结论：三组 trap 在汉化 ROM 上
不带来可测的加速。**

原因分析：这些 Rust 端口是按美版 MMC1 的 bank 布局和寄存器约定写的。
在汉化版上，4 个 bank 切换器被 `mmc5_switcher_call` 接管（否则撞非法
指令），但端口体内其余对固定 bank 例程的调用仍要 fallback 到解释器
逐条执行 ROM 自己的字节 —— 也就是说**该解释的还是解释了**，
省不掉那部分开销，反倒多了一层 trap 分发。

**所以如果你的目标是「让汉化版跑得更快」，这三组 trap 不是答案。**
真要提速，得为汉化版**重写**这些端口（按 MMC5 的 bank 语义），
工作量远大于本次的适配，收益也未经证实。

本次改动仍然有价值，体现在**行为一致性**而非性能：启用后汉化版与美版
在完全相同的输入下走到**相同的场景序列**。实测对照：

| | 到访场景序列 | P2 hp | p2_contacts | n_display/n_update |
|---|---|---|---|---|
| 美版 | `[0, 39]` | 127（未掉血） | 0 | 2077 / 2228 |
| 汉化（启用三组） | `[0, 39]` | 127（未掉血） | 0 | 2072 / 2222 |
| 汉化（未启用，旧） | `[0, 22]` ← 偏离 | — | — | — |

即：启用前汉化版走 scene=22，与美版的 39 不一致；启用后对齐了。
这也是为什么原「P2 必须掉血」的测试会假失败（见下）。

---

## 六之二、一处交付错误与修正（务必读）

**第一次交付时三组 trap 并没有真正启用。**

`register_translated_traps` 当时**只改了 doc 注释，没改函数体**：
注释写着「Every group is registered, including title_traps…」，
函数体却还是 6 组（bank7/player/enemy/town/palace/boot）。
而那一版声称的「24 个端到端全过」，是在**未提交的工作区改动**下跑出来的，
改动随后在 `git checkout --` / `git stash` 中丢失。

`bank7_common.rs` 的 MMC5 适配（`mmc5_switcher_call`）倒是留下了，
因为它在另一个文件里提交过。

### 修正（提交 `3f68aa0`）

1. 三组真正注册进 `register_translated_traps`（现 9 组 / 96 条 trap）。
2. 新增 `all_nine_trap_groups_are_registered_on_translated_rom`：
   **直接数注册表**，要求 >60 条，且 title`$C33C` / sideview`$C9A5`
   / sideview`$CC97` / overworld`$D625` 都被注册。
   不再依赖行为观察 —— 这正是上次漏掉的保护。
3. `coop_p2_takes_part_in_combat` → 改名 `coop_p2_runs_in_gameplay`，
   断言从「P2 掉血」改为「`n_display > 0 && n_update > 0`」。

   原断言为什么假失败：它依赖这一路走下来的场景里恰好有敌人打到 P2。
   启用三组后场景从 22 变成 39（与美版一致），而 39 里没有敌人打到 P2。
   **实测美版在同一输入下同样是 P2 全程满血、`p2_contacts=0`** ——
   所以掉血是场景运气，不是「伤害通道是否接通」的证据。

   新断言看 `n_display` / `n_update`：这两个是 `coop_links_display`
   （`$EBF0`）和 `coop_code21`（`$D5A7`）每帧递增的计数，只要 P2 在
   侧视场景里活动就必然增长，且与美版对照吻合（2072/2222 vs 2077/2228）。

### 教训

* **注释与代码不符时，先信代码。** 我当时读了注释就下结论，没看函数体。
* 「测试全过」必须确认是在**已提交或已保存**的状态下跑的，
  否则等于没验证。
* 验证「某组 trap 是否启用」这类结构性事实，
  应该直接检查状态（数注册表），而不是靠行为间接推断。

---

## 七、启动器汉化（已完成）

### 做法

没有现成 i18n 机制，所以新增 `crates/z2-launcher/src/i18n.rs`：

* `Lang` 枚举（`En` / `Zh`），`serde` 可序列化，`#[default] = En`
* 两张 `(english, translation)` 常量表，**键就是英文原文** ——
  这样调用点不用查表，且英文表本身就是「有哪些字符串」的清单
* `Lang::t(key)` 查表，查不到返回 key 本身（ typo 会显示原文而非空白）
* `lang_fmt` / `say1` / `say2` 处理带 `{}` 占位符的消息

### 接入点

| 文件 | 改动 |
|---|---|
| `settings.rs` | `Settings` 加 `language: Lang`，默认 `Zh`；新增 `validate_lang` / `validate_rando_lang`，原 `validate` 委托给 `validate_lang(_, Lang::En)` 保持英文（既有测试不动） |
| `ui.rs` | 每个 section 开头 `let lang = ...`，所有字面量改成 `lang.t(...)`；新增 `language_section`（语言下拉框）放在 ROM 区块之后 |
| `app.rs` | `LauncherApp` 加 `title` 字段，`ui()` 里检测语言变化就 `send_viewport_cmd(Title)` |
| `lib.rs` | `pub mod i18n` + `pub use i18n::Lang` + 导出 `validate_lang` |

### 为什么默认中文

这个构建是跟着汉化 ROM 发布的，常见用户是中文读者。`serde(default)`
让旧的 `launcher.json`（没有 `language` 字段）读进来也是中文。

### 未汉化的部分（有意保留）

* **游戏增强页签内的选项标签**：来自 `z2_core::enh`，与游戏共用同一套
  JSON shape，改文案会影响游戏侧，所以保留英文。
* **材质包名称、随机化模块名**：分别是用户目录名和 `z2-rando` 的标识。

### 验证

37 项测试，其中新增 5 项专盯语言：

    default_is_chinese                      默认中文
    language_round_trips_through_json       可切换、可持久化
    old_settings_file_without_language_...  旧配置文件兼容
    language_does_not_change_args           不影响启动参数
    validation_messages_are_translated      中文提示含汉字、英文提示不含

最后一条专门防「键加了但翻译没接上」——那是这类改动最容易出现的假成功。

---

## 八、下一步（如果还要继续）

1. **游戏增强页签的标签汉化**：需要把 `z2_core::enh` 的显示名与 i18n 解耦，
   工作量比本轮大，且要动游戏侧。建议先评估是否值得。
2. **为汉化版重写三组端口以真正提速**：先 profiling，收益上限可能有限
   （见第六节）。
3. 其余详见上一轮 `HANDOVER.md` 的「下一步」。

---

## 八、几个可能对你有用的坑

- **`git status` 只有 3 个源码文件改动**（+ 2 个未跟踪的 md），
  没有半成品代码残留。探针测试我都删了。
- **回退很方便**：`git checkout -- crates/z2-core/src/bank7_common.rs
  crates/z2-core/src/cpu.rs crates/z2-native/src/app.rs`
  就回到「三组 trap 未启用、测试全过」的状态。
  注意 `crates/z2-core/src/cpu.rs` 那行只是把 `BUS_ACCESS_OFFSET`
  改成 `pub(crate)`，回退它会影响 `bank7_common.rs` 编译 ——
  要一起回退。
- **PowerShell 的工作目录会时不时重置**到
  `C:\Users\ezplayer\Documents\deepseek-harness\default-workspace`，
  用相对路径读文件会失败。**一律用绝对路径**，或每条命令前
  `Set-Location 'D:\z2rs-060-zh'`。
- **PowerShell 会把 `{` `}` 当脚本块**，用 `-replace` 或字符串拼接
  改 Rust 源码很容易撞解析错误。**改用 Python 读写文件**更稳。
- 汉化 ROM 路径：`D:\z2rs-中文版\Zelda II - The Adventure of Link (U) [20260806_NOKOH].nes`
- 美版 ROM 路径：`E:\Zelda II - The Adventure of Link (USA).nes`
- 构建：`CARGO_TARGET_DIR=D:/z2rs-build-060`，
  `PATH` 加 `C:\Users\ezplayer\.cargo\bin;C:\msys64\ucrt64\bin`
