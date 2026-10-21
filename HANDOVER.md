# z2rs 中文汉化 · 项目交接文档

> 最后更新：2026-10-21 12:20
> 项目根：`D:/下载/Compressed/main/z2rs-main`（Rust 源码）
> 发行目录：`D:/z2rs-中文版`（exe + 文档）
> 分析工作区：`C:/Users/ezplayer/WorkBuddy AI/2026-10-19-14-25-04`（脚本与图片）

---

## 一、一句话现状

**能玩的是「无中文对话框的纯净版」，已打包发布。**
**MMC5 移植在 `master` 分支进行中，尚未画出第一帧，也未部署。**

| 项目 | 状态 |
|---|---|
| 纯净版（保留剪贴板取词，对话显示原版英文） | ✅ 已发布、已打包 |
| 中文对话框（自己渲染汉字） | ⚠️ 代码在 `backup-cjk-full`，室内场景会乱码，**已放弃** |
| MMC5 移植（直接跑汉化版 ROM） | 🔧 进行中，见第六节 |

---

## 二、路径总览

### 2.1 源码

| 路径 | 说明 |
|---|---|
| `D:/下载/Compressed/main/z2rs-main` | 项目根（git 仓库） |
| `crates/z2-core/src/mmc5.rs` | **新增** MMC5 mapper（19614 字节，11 条测试） |
| `crates/z2-core/src/game.rs` | `MapperKind` 枚举、`Game::from_ines` 接受 mapper 5 |
| `crates/z2-core/src/cpu.rs` | PRG 读 / mapper 寄存器读写按 mapper 分派 |
| `crates/z2-core/src/ppu_bind.rs` | CHR 同步按 mapper 分派（MMC5 要拼 4 个 1KB bank） |
| `crates/z2-native/src/app.rs` | `emu_from_generic_ines()` 通用加载路径 |
| `crates/z2-native/tests/mmc5_zh_rom.rs` | 汉化 ROM 端到端诊断（4647 字节） |
| `deploy.sh` | 部署脚本：同步全部 exe + 逐字节校验 + 清理别名 |
| `MMC5-PROGRESS.md` | MMC5 移植进度（本文件第六节的详细版） |
| `CJK-DIALOG.md` | 中文对话框技术文档（已随功能移除，内容见 git 历史） |

### 2.2 编译产物

| 路径 | 说明 |
|---|---|
| `D:/z2rs-build/release/` | `CARGO_TARGET_DIR`，release 产物 |
| `D:/z2rs-build/release/z2-native.exe` | 游戏本体（59,168,334 字节 / 01:33，含 MMC5，**未部署**） |
| `D:/z2rs-build/release/z2-launcher.exe` | 启动器（24,749,918 字节 / 01:33） |
| `D:/z2rs-build/release/z2-signal.exe` | 信号辅助 |

### 2.3 发行目录 `D:/z2rs-中文版`

```
D:/z2rs-中文版/
├── Zelda II - The Adventure of Link (U) [20260806_NOKOH].nes   786,448  汉化版 ROM
├── z2rs.exe                    59,164,006  游戏本体（纯净版，无 MMC5）
├── z2-launcher.exe             24,746,293  启动器
├── SHA256SUMS                                zip 的校验值
├── z2rs-0.1.0-zh-CN-x86_64-pc-windows-gnu.zip   28.26 MB  发布包
├── HOW-TO-PLAY.txt / LUNAHOOK.md / README.zh-CN.md / 测试说明.txt
└── z2rs-0.1.0-zh-CN-x86_64-pc-windows-gnu/   解压后的发行目录
    ├── z2rs.exe                59,164,006
    ├── z2-launcher.exe         24,746,293
    ├── z2-signal.exe            5,263,447
    ├── 版本说明.txt / 汉化说明.txt / CJK-DIALOG.md
    └── HOW-TO-PLAY*.txt / LEGAL.md / LICENSE / README*.md / LUNAHOOK.md
```

**已部署版本校验**

```
z2rs.exe         59,164,006  sha256 935b0d8db6a2...
z2-launcher.exe  24,746,293  sha256 6f3eb922b6a8...
z2-signal.exe     5,263,447  sha256 98842f24bb3a...
发布包           28.26 MB    sha256 0646ed42530856a6...
```

### 2.4 分析工作区 `C:/Users/ezplayer/WorkBuddy AI/2026-10-19-14-25-04`

**结论类文档**

| 文件 | 说明 |
|---|---|
| `ROM-兼容性结论.md` | 早期关于汉化 ROM 能否复用的结论 |
| `glossary-zh.md` | 术语表 |
| `dialog_en.txt` / `dialog_en.json` | 从原版 ROM 解出的 98 段英文对话 |
| `fontchars.txt` | 字库字符集 |

**汉化 ROM 字库调查图（关键证据，别删）**

| 文件 | 说明 |
|---|---|
| `p96.png` `p97.png` `p100.png` `p110.png` | CHR 页 96/97/100/110 整页渲染 |
| `p96_raw.png` | 页 96 原始 128×128 位图（带 8px tile 网格） |
| `p96_16x24.png` | 按 16×24 切块后**能读出中文**的那张（"请让我帮忙，进来吧"） |
| `grid16.png` / `pitch18.png` / `line1_zoom.png` / `line97_zoom.png` | 网格/字距测量尝试 |
| `cand_A.png` ~ `cand_D.png` / `h16.png` ~ `h28.png` | 各种切法对比（**全部失败**） |
| `F32.png` `F96.png` | 页 32/96 整页 |
| `f128_off5.png` | PRG 区按 128 字节/字切分（**噪声，失败**） |
| `pp115.png` `pp116.png` `pp117.png` `pp124.png` | MMC5 换页用到的页（**全是 0 填充**） |
| `chr_atlas.png` / `crop_page*.png` | 早期 CHR 总览 |
| `glyphs16.json` / `quads.json` | 早期字形提取结果 |

**Python 分析脚本**：`atlas.py` `extract_cn.py` `extract_glyphs.py` `find_text.py`
`decode_en.py` `cmp_rom.py` `cmp_deep.py` `crack.py` `feasibility.py` 等。

---

## 三、Git 分支

| 分支 | HEAD | 内容 |
|---|---|---|
| **`master`** | `135fb48` | MMC5 移植进行中（当前所在） |
| `backup-cjk-full` | `4ccecd7` | 中文对话框**完整实现**（能显示，但室内乱码） |
| `backup-broken-path` | `7fdbd54` | 中文定位的失败尝试（字母块/边框策略） |
| `upstream/simtest` | `4f695dc` | 上游原始 |

**切换命令**

```bash
cd "D:/下载/Compressed/main/z2rs-main"
git checkout backup-cjk-full    # 回到中文对话框版本
git checkout master             # 回到 MMC5 版本
```

**关键提交**

```
135fb48  MMC5 移植（进行中）：mapper 实现 + 接入 Game + 通用 ROM 加载路径
daa89c2  回到无中文对话框功能的纯净版（858c629）
4ccecd7  去掉多余 exe 别名：每程序只保留一个名字
8b97ef1  回滚到 7758221 稳定点
7758221  端到端测试：真ROM+真存档跑3000帧，确认中文覆盖生效（61帧中文）
341f3e2  中文对话框：从原版 ROM 解码全文、自译 98 段…   ← 中文功能起点
858c629  汉化更新流程 + 待翻译扫描                      ← 纯净版基线
5bc4bef  重构取词：改为整句版面分析                      ← 剪贴板相关最后一个
74addbe  修复剪贴板导出导致的堆损坏崩溃
```

---

## 四、走过的三条路（含结论，避免重复踩坑）

### 路线 A：中文对话框（自己渲染汉字）— ❌ 已放弃

**做法**：从原版 ROM 解出英文对话 → 自译 98 段 → NotoSansSC 生成 16×16 字库
→ 在帧末把汉字 tile 写进 nametable 覆盖英文。

**结果**：室外能正常显示（端到端测试确认 61 帧中文），**室内场景始终乱码**。

**失败原因**：室内对话框的边框不是室外的 `$CA/$CB/$CC`（是白蓝双线），
定位逻辑找不到框，汉字画到错误位置，按 ROM 页取模就成了彩色乱码。

**试过的定位策略**（全在 `backup-broken-path`）：边框三角、上边框整行 `$CB`、
英文字母块、状态栏干扰排除、位置锁定复用…… 均未彻底解决。

**代码位置**：`backup-cjk-full` 分支，核心是 `crates/z2-core/src/cjk_dialog_ui.rs`。

### 路线 B：从汉化 ROM 提取字库 — ❌ 不可行

**调查结论**（证据见 2.4 的图片）：

1. **没有单字字库**。扫了全部 128 个 CHR 页，**0 页**有可切分的空列。
2. 有中文的是页 **32-50 和 96-114**（各 19 页，其余是镜像重复），
   但那是**已排版的连排文本图**：字距约 18px、上下行重叠、整页连成一片。
3. 用 16×24 切块能勉强读出内容（`p96_16x24.png`）：
   「请让我帮忙，进来吧。请 / 我可以回复 你的生命。/
   在这里躺作 休息吧。哈 / 我帮你回复魔法；随时再」
4. PRG `0x12900..0x17B00` 的 20KB **不是字模**：试过 16/32/64/128 字节每字
   × 64 种偏移，全部渲染成噪声；Big5/GB2312/GBK/SJIS 解码全是乱码。
5. 容量也对不上：38 页 × 约 28 字 ≈ 1064 字，但原版对话约 3000 字。

**要照抄它需要**：OCR 38 页连排图 + 人工校对，成本高于收益。

### 路线 C：MMC5 移植（直接跑汉化版 ROM）— 🔧 进行中

**动机**：让 ROM 自己处理一切（字形、译文、位置、调色板），不逆向、不 OCR。

**已完成**：见第六节。

---

## 五、汉化版 ROM 的技术档案

**文件**：`D:/z2rs-中文版/Zelda II - The Adventure of Link (U) [20260806_NOKOH].nes`

```
iNES: mapper 5 (MMC5), PRG 256KB (16×16KB), CHR 512KB (64×8KB)
      mirroring=horizontal, battery=True
      文件大小 786,448 = 16 + 262144 + 524288（无 trainer）
```

**与原版的关键差异**

| 项目 | 原版 | 汉化版 |
|---|---|---|
| mapper | 1 (MMC1) | 5 (MMC5) |
| PRG | 128KB | 256KB |
| CHR | 128KB（实际文件 256KB） | 512KB |
| 固定 bank 代码 | `$C000-$FFFF` | `$E000-$FFFF`（**只有 8KB**） |
| 固定 bank 与原版差异 | — | **51.31%（8407/16384 字节）** |

**PRG 结构（32 个 8KB bank）**

- bank 0-1：启动 + 数据
- bank 14/15：初始化代码（`$C000` 入口）
- bank 30：**全 `$FF`（空）**
- bank 31：**唯一的合法向量表**（NMI=`$C07B`、RESET=`$FF4C`、IRQ=`$FF9C`）
- bank 1,3,5,7,9,11,13 都有 `NMI=$C07B / RESET=$BF70 / IRQ=$FFF0` 的副本

**MMC5 寄存器实际用量**

```
$5100 PRG模式  $5101 CHR模式  $5102/$5103 PRG-RAM保护
$5105 nametable映射(8次)      $5113 PRG-RAM bank
$5115 $5116 PRG bank          $5124-$5127 CHR bank
$5128-$512B CHR高位           $5130 CHR高位
$5203/$5204 扫描线IRQ         $5205/$5206 乘法器
```
**注意：`STA $5117` 一次都没出现** → 只用 PRG 模式 2。

**启动链路（已实测走通）**

```
$FF4C: SEI / CLD / LDX #$00 / STX $2000
$FF54: LDA $2002 / BPL（等 vblank 两次）/ TXS
$FF5D: LDA #$02 / STA $5100      ← PRG 模式 2
$FF62: LDA #$40 / STA $4017
$FF67: LDA #$01 / STA $5103
$FF6C: LDA #$02 / JMP $FDCB

$FDCB: STA $5102
$FDCE: LDA $01D4 / CMP #$01
$FDD7: STA $5130
$FDDA: LDA #$8E / STA $5116      ← $C000 用 bank 14（$8E 取低5位=14）
$FDDF: LDA #$03 / STA $5101      ← CHR 模式 3（1KB）
$FDE4: LDA #$00 / STA $5204      ← 关 IRQ
$FDE9: JSR $FEB0
$FDEC: LDA #$8E / STA $5115
$FDF1: LDA #$00 / STA $5113
$FDF6: CLI / JMP $C000           ← 进 bank 14 初始化

$C000: LDA #$00 / STA $2001 / STA $2000
$C008: LDA #$05 / JSR $FFCC
$C00D: JSR $A6A0
$C010: LDA $0736 / CMP #$08 / BEQ $C904
$C018: CMP #$14 / BNE $C010      ← 卡在这个等待循环
$C01E: LDA $076C / CMP #$01 / BNE $C010
```

---

## 六、MMC5 移植进度

### 已完成

`crates/z2-core/src/mmc5.rs`（11 条单元测试全过）：

| 功能 | 状态 |
|---|---|
| PRG 四模式（32K / 16+16 / 16+8+8 / 8×4） | ✅ |
| CHR 四模式（8K/4K/2K/1K）+ `$5128-$512B` / `$5130` 高位 | ✅ |
| nametable 映射 `$5105`（解码） | ✅ |
| ExRAM `$5C00-$5FFF` / `$5104` | ✅（仅 RAM） |
| 扫描线 IRQ `$5203`/`$5204` | ✅（`on_scanline`） |
| 乘法器 `$5205`/`$5206` | ✅ |
| PRG-RAM bank `$5113`、保护 `$5102/$5103` | ✅ |
| ExRAM 扩展属性（逐格调色板） | ❌ 未实现（只影响颜色） |
| 填充模式 `$5106/$5107` | ⚠️ 只存值 |

### 踩过的三个真 bug（都已修）

**① `bus_write` 把 MMC5 寄存器写入全吞了**

```rust
// crates/z2-core/src/cpu.rs 原来
0x4020..=0x5FFF => {} // Expansion stub: ignore.
```

MMC5 全部寄存器（`$5100-$5FFF`）都落在这个区间 → **所有 bank 切换无效**。

**② MMC5 上电时所有 bank 寄存器为 0**

`$FFFC` 从 bank 0 读 → `$9658` → 那里是 `$FF` → 直接跑飞。
修法：`Mmc5::init_fix_last()`，上电固定 `$8000`=bank 0、`$E000`=最后一个。

**③ `$E000-$FFFF` 必须钉死最后一个 bank**

不是教科书 MMC5（那里 `$5116` 驱动 `$E000`），但这个 ROM 需要。
证据：`STA $5117` 从未出现；`STA $5116` 之后代码仍在 `$E000` 区继续执行；
写的 `$8E` 取低 5 位=14，bank 14 的 `$C000` 确实是初始化代码。

### 当前卡点

```
PC=$C024  $0736=00  $076C=00  ctrl=$00  mask=$00
每帧周期 = 0（解释器 ExecError 后冻结）
```

- PPU 从未被启用
- `$0736`/`$076C`（游戏状态机）没被中断服务程序更新

### 下一步排查清单

1. **先看 `exec_errors` / `last_exec_error`**：确认报错地址。
   如果是 `IllegalOpcode`，对照 6502 指令表找漏掉的 opcode。
2. **单步跟踪 `$C000..$C024`**：确认 `JSR $FFCC`、`JSR $A6A0` 正确返回。
3. **查 `$5204` 读取**：读会清 pending，返回值影响分支判断。
4. **搜 `STA $0736`**：找谁写这个变量（主循环自己 or NMI/IRQ）。
5. **查 `$FFCC` 做什么**：PPU 为何一直没开。

---

## 七、常用命令

```bash
# 环境
export PATH="/c/Users/ezplayer/.cargo/bin:/c/msys64/ucrt64/bin:$PATH"
export CARGO_TARGET_DIR="D:/z2rs-build"
cd "D:/下载/Compressed/main/z2rs-main"

# 单元测试（不依赖 ROM）
cargo test -j 4 -p z2-core --lib            # 182 条，全部应通过
cargo test -j 4 -p z2-core --lib mmc5       # 只跑 MMC5 的 11 条

# 汉化 ROM 端到端诊断（需要 ROM 在指定路径）
cargo test -j 4 -p z2-native --test mmc5_zh_rom -- --nocapture

# 编译 release
cargo build --release -j 4 -p z2-native -p z2-launcher -p z2-signal

# 部署（同步全部 exe + 校验 + 清理别名）
bash deploy.sh
```

**注意**：只编译 `-p z2-native` 会让 exe 从 59MB 掉到 53MB（feature 解析不同），
必须带齐三个 `-p`。

---

## 八、重要提醒

1. **已部署的 exe 不含 MMC5**。发行目录里的 `z2rs.exe`（59,164,006 / 00:49）
   是纯净版；`D:/z2rs-build/release/`（59,168,334 / 01:33）才是含 MMC5 的。
   要部署 MMC5 版需先跑 `bash deploy.sh`，但**它现在还跑不起来**，别急着部署。

2. **原版 MMC1 路径完全没动**。`z2-core` 182 条单元测试全过。

3. **汉化版走纯解释器（无 trap）**，比原版慢很多。这是唯一正确做法 ——
   汉化版固定 bank 有 51% 与原版不同，按原版地址硬编码的 Rust trap 全部会跑错。

4. **不要再试"从汉化 ROM 提取字库"**，见第四节路线 B。

5. **exe 命名规则**：每个程序只有一个名字，不造别名。
   `z2rs.exe`（游戏）/ `z2-launcher.exe`（启动器）/ `z2-signal.exe`（辅助）。
   曾经复制过 `z2rs-launcher.exe`、`z2rs-native.exe`，导致同目录两份启动器
   一新一旧，用户点到旧的会以为更新没生效。
