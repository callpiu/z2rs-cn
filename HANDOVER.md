# z2rs 中文汉化 · 项目交接文档

> 最后更新：2026-10-21 13:20
> 项目根：`D:/下载/Compressed/main/z2rs-main`（Rust 源码）
> 发行目录：`D:/z2rs-中文版`（exe + 文档）
> 分析工作区：`C:/Users/ezplayer/WorkBuddy AI/2026-10-19-14-25-04`（脚本与图片）

---

## 一、一句话现状

**MMC5 移植已跑通：z2rs 能直接运行汉化版 ROM，标题画面和存档菜单正常显示汉字。**
已编译、已部署、已实测。

| 项目 | 状态 |
|---|---|
| 纯净版（保留剪贴板取词，对话显示原版英文） | ✅ 已发布 |
| 中文对话框（自己渲染汉字） | ⚠️ 代码在 `backup-cjk-full`，室内场景会乱码，**已放弃** |
| **MMC5 移植（直接跑汉化版 ROM）** | **✅ 已跑通、已部署**，见第六节 |

---

## 二、路径总览

### 2.1 源码

| 路径 | 说明 |
|---|---|
| `D:/下载/Compressed/main/z2rs-main` | 项目根（git 仓库） |
| `crates/z2-core/src/mmc5.rs` | **MMC5 mapper**（13 条单元测试） |
| `crates/z2-core/src/game.rs` | `MapperKind` 枚举、`Game::from_ines` 接受 mapper 5 |
| `crates/z2-core/src/cpu.rs` | PRG 读 / mapper 寄存器读写按 mapper 分派 |
| `crates/z2-core/src/ppu_bind.rs` | CHR 同步按 mapper 分派（MMC5 要拼 4 个 1KB bank） |
| `crates/z2-native/src/app.rs` | `emu_from_generic_ines()` 通用加载路径 |
| `crates/z2-native/tests/mmc5_zh_rom.rs` | 汉化 ROM 端到端测试（7 条） |
| `deploy.sh` | 部署脚本：同步全部 exe + 逐字节校验 + 清理别名 |
| `MMC5-PROGRESS.md` | MMC5 移植进度（本文件第六节的详细版） |
| `CJK-DIALOG.md` | 中文对话框技术文档（已随功能移除，内容见 git 历史） |

### 2.2 编译产物

| 路径 | 说明 |
|---|---|
| `D:/z2rs-build/release/` | `CARGO_TARGET_DIR`，release 产物 |
| `D:/z2rs-build/release/z2-native.exe` | 游戏本体（59,191,894 字节 / 13:12，含 MMC5，**已部署**） |
| `D:/z2rs-build/release/z2-launcher.exe` | 启动器（24,758,819 字节 / 13:12） |
| `D:/z2rs-build/release/z2-signal.exe` | 信号辅助（5,263,447） |

### 2.3 发行目录 `D:/z2rs-中文版`

```
D:/z2rs-中文版/
├── Zelda II - The Adventure of Link (U) [20260806_NOKOH].nes   786,448  汉化版 ROM
├── z2rs.exe                    59,191,894  游戏本体（含 MMC5，能跑汉化 ROM）
├── z2-launcher.exe             24,758,819  启动器
├── SHA256SUMS                                zip 的校验值
├── z2rs-0.1.0-zh-CN-x86_64-pc-windows-gnu.zip   28.26 MB  发布包（旧，需重打包）
├── HOW-TO-PLAY.txt / LUNAHOOK.md / README.zh-CN.md / 测试说明.txt
└── z2rs-0.1.0-zh-CN-x86_64-pc-windows-gnu/   解压后的发行目录
    ├── z2rs.exe                59,191,894
    ├── z2-launcher.exe         24,758,819
    ├── z2-signal.exe            5,263,447
    ├── 版本说明.txt / 汉化说明.txt / CJK-DIALOG.md
    └── HOW-TO-PLAY*.txt / LEGAL.md / LICENSE / README*.md / LUNAHOOK.md
```

**已部署版本校验（2026-10-21 13:12 部署，逐字节核对通过）**

```
z2rs.exe         59,191,894  sha256 2827dacf7f88...
z2-launcher.exe  24,758,819  sha256 fe4b1267fe28...
z2-signal.exe     5,263,447  sha256 98842f24bb3a...
发布包           29.78 MB    sha256 90adcc1c55eb...
```

MMC5 版已实证：`z2rs.exe --headless --rom <汉化ROM> --frames 300` → 标题画面
正确渲染、`interpreter faults: 0`。

调试截图在 `D:/z2rs-diag/mmc5sh/`（`deployed_title.png`、`g700.png` 等）。

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
| **`master`** | `48e384b`（+ 未提交的 MMC5 修复） | MMC5 移植**已跑通**（当前所在） |
| `backup-cjk-full` | `4ccecd7` | 中文对话框**完整实现**（能显示，但室内乱码） |
| `backup-broken-path` | `7fdbd54` | 中文定位的失败尝试（字母块/边框策略） |
| `upstream/simtest` | `4f695dc` | 上游原始 |

> ⚠️ 这台机器的 PATH 里**没有 `git`**（msys64 装了，但没有 `git.exe`）。
> 需要提交时先装 Git for Windows，或者在别的机器上提交。

**切换命令**

```bash
cd "D:/下载/Compressed/main/z2rs-main"
git checkout backup-cjk-full    # 回到中文对话框版本
git checkout master             # 回到 MMC5 版本
```

**关键提交**

```
135fb48  MMC5 移植（进行中）：mapper 实现 + 接入 Game + 通用 ROM 加载路径
48e384b  新增交接文档 HANDOVER.md
daa89c2  回到无中文对话框功能的纯净版（858c629）
4ccecd7  去掉多余 exe 别名：每程序只保留一个名字
8b97ef1  回滚到 7758221 稳定点
7758221  端到端测试：真ROM+真存档跑3000帧，确认中文覆盖生效（61帧中文）
341f3e2  中文对话框：从原版 ROM 解码全文、自译 98 段…   ← 中文功能起点
858c629  汉化更新流程 + 待翻译扫描                      ← 纯净版基线
5bc4bef  重构取词：改为整句版面分析                      ← 剪贴板相关最后一个
74addbe  修复剪贴板导出导致的堆损坏崩溃
```

**未提交改动（2026-10-21 13:12）**

- `crates/z2-core/src/mmc5.rs`：修正 PRG 模式 1/2/3 寄存器分配；`$5204` bit 6
  改为跟踪 in-frame；新增/修正 3 条单元测试
- `crates/z2-native/tests/mmc5_zh_rom.rs`：强化启动测试，新增
  `start_button_leaves_the_title_screen`
- `HANDOVER.md` / `MMC5-PROGRESS.md`：文档更新

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

### 路线 C：MMC5 移植（直接跑汉化版 ROM）— ✅ 已跑通

**动机**：让 ROM 自己处理一切（字形、译文、位置、调色板），不逆向、不 OCR。

**结果**：标题画面 + 存档菜单正常，汉字清晰，零解释器错误。已部署。

**关键突破**：不是"ROM 需要特殊处理"，而是 MMC5 的 PRG 模式寄存器分配写错
（模式 1/2 该用 `$5115` 驱动 `$8000`，模式 2 的 `$E000` 由 `$5117` 驱动）。
详见第六节 ③。

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
- bank 16-30：**全 `$FF`（空）**
- bank 31：**唯一的合法向量表**（NMI=`$C07B`、RESET=`$FF4C`、IRQ=`$FF9C`）
- bank 1,3,5,7,9,11,13 都有 `NMI=$C07B / RESET=$BF70 / IRQ=$FFF0` 的副本

**MMC5 寄存器实际用量**

```
$5100 PRG模式  $5101 CHR模式  $5102/$5103 PRG-RAM保护
$5105 nametable映射(8次)      $5113 PRG-RAM bank
$5115 $5116 PRG bank          $5120-$5127 CHR bank
$5128-$512B CHR高位           $5130 CHR高位
$5203/$5204 扫描线IRQ         $5205/$5206 乘法器
```

**注意**：`STA $5117`（字节 `8D 17 51`）在整个 PRG 里**一次都没出现**，
所以只用 PRG 模式 2，且 `$E000-$FFFF` 始终停在上电时 `init_fix_last`
给的 bank 31 —— 这正是向量表和共享子程序（`$FEB0`/`$FFCC`）一直可用的原因。

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
$FDD9: LDA #$8E / STA $5116      ← $C000 用 bank 14（$8E 取低5位=14）
$FDDF: LDA #$03 / STA $5101      ← CHR 模式 3（1KB）
$FDE4: LDA #$00 / STA $5204      ← 关 IRQ
$FDE9: JSR $FEB0                 ← 在 $E000-$FFFF，靠 $5117 仍指向 bank 31
$FDEB: LDA #$8E / STA $5115      ← $8000 也换 bank 14
$FDF1: LDA #$00 / STA $5113
$FDF6: CLI / JMP $C000           ← 进 bank 14 初始化

$C000: LDA #$00 / STA $2001 / STA $2000
$C008: LDA #$05 / JSR $FFCC
$C00D: JSR $A6A0
$C010: LDA $0736 / CMP #$08 / BEQ $C904
$C018: CMP #$14 / BNE $C010      ← 状态机等待（实测 0→1→2→3 正常推进）
$C01E: LDA $076C / CMP #$01 / BNE $C010
```

---

## 六、MMC5 移植进度

### 状态：✅ 跑通

```
300 帧后：非零像素 55201 / 61440     ctrl=$B2 mask=$1E   exec_errors=0
按 Start：$0736 03 -> 00，画面变化 18121 像素
```

- 标题画面（剑插石头）正常渲染
- 按 Start 进存档菜单，「選擇紀錄 / 名字 / 等級 / 註冊你的名字 / 刪除模式」
  汉字清晰
- 已部署到 `D:/z2rs-中文版/`，用部署后的 exe 实测通过

### 已完成

`crates/z2-core/src/mmc5.rs`（14 条单元测试全过）：

| 功能 | 状态 |
|---|---|
| PRG 四模式（32K / 16+16 / 16+8+8 / 8×4），寄存器分配正确 | ✅ |
| CHR 四模式（8K/4K/2K/1K） | ✅ |
| `$5128-$512B` 四个 **8 位**背景 bank 寄存器 | ✅ |
| `$5130` CHR 高位（bit 8-9）= 简繁切换 | ✅ |
| nametable 映射 `$5105` → **真正接进 PPU 镜像** | ✅ |
| ExRAM `$5C00-$5FFF` / `$5104` | ✅（仅 RAM；ROM 未用 `$5104`） |
| 扫描线 IRQ `$5203`/`$5204`（含 bit 6 in-frame） | ✅（`on_scanline`） |
| 乘法器 `$5205`/`$5206` | ✅ |
| PRG-RAM bank `$5113`、保护 `$5102/$5103` | ✅ |
| ExRAM 扩展属性（逐格调色板） | ❌ 未实现（ROM 未用，实测无影响） |
| 填充模式 `$5106/$5107` | ⚠️ 只存值（ROM 未用） |

### 踩过的六个真 bug（都已修）

**① `bus_write` 把 MMC5 寄存器写入全吞了**

```rust
// crates/z2-core/src/cpu.rs 原来
0x4020..=0x5FFF => {} // Expansion stub: ignore.
```

MMC5 全部寄存器（`$5100-$5FFF`）都落在这个区间 → **所有 bank 切换无效**。

**② MMC5 上电时所有 bank 寄存器为 0**

`$FFFC` 从 bank 0 读 → `$9658` → 那里是 `$FF` → 直接跑飞。
修法：`Mmc5::init_fix_last()`，上电固定 `$8000`=bank 0、`$C000`=倒数第二、
`$E000`=最后一个（bank 31）。

**③ ★ 真凶：PRG 模式 1/2 的寄存器分配写错了**

这是让 ROM 一直不出画面的原因。原来错在两点：模式 1/2 用 `$5114` 驱动
`$8000`（应该是 `$5115`），以及把 `$E000-$FFFF` 硬钉在最后一个 bank。
正确分配（NESdev）：

| 模式 | `$8000` | `$A000` | `$C000` | `$E000` |
|---|---|---|---|---|
| 0（32K） | `$5114` | `$5114`+1 | `$5114`+2 | `$5114`+3 |
| 1（16+16） | `$5115` | `$5115`+1 | `$5116` | `$5116`+1 |
| 2（16+8+8） | `$5115` | `$5115`+1 | `$5116` | **`$5117`** |
| 3（8×4） | `$5114` | `$5115` | `$5116` | `$5117` |

修好之前 PC 停在 `$FF63`，从 `$FF62` 起全是 `$FF`（非法 opcode）——
因为 `STA $5116` 在错误语义下把 `$E000` 换成了 bank 30（全 `$FF`）。

**④ `$5204` 的 bit 6 是"在帧内"，不是"NMI 使能"**

原来读 `$5204` 返回 `irq_enabled << 6`。硬件上 bit 6 表示束流是否在可见帧
内（vblank 带 240-260 为 0），bit 7 才是 pending。修法：`on_scanline`
里跟踪 `in_frame`，读时返回 `pending<<7 | in_frame<<6`。

**⑤ `$5105` 没接进 PPU → 部分场景贴图错乱**

`$5105` 决定 4 个逻辑屏幕各取哪块 nametable（启动写 `$44`，某些场景
改写成 `$50`）。原来只把值存进 `Mmc5::nt_map`，`nametable_source()`
**没有任何调用者**，PPU 一直用自己的默认镜像 → 某些场景从左 nametable
取右半边的图块，表现为贴图错乱。

修法三处：`PpuBind::apply_mmc5()` 里调 `set_mirroring`；`bus_write` 的
触发范围加上 `addr == 0x5105`；缓存 key 并入 `nt_map`。
注意 `$44` 是**竖**镜像、`$50` 是**横**镜像。

**⑥ `$5130` 位宽错（简繁切换失效）**

`chr_bank_1k` 写成 `hi2 << 10`（应为 bit 8-9 = `<< 8`），页号 +1024 后
取模 512 个 1KB bank **绕回同一页**，按右键只改 8 个像素。
另外 `$5128-$512B` 被当成"每寄存器 2 位高位"，实际是**四个 8 位 bank
寄存器**（ROM 写 `104,105,106,107`）。

修好后按 → 键，8 个 bank 随时 +256，画面差异 1452 像素。

**⑦ HD 材质包只索引 32 页（中文字形取不到）**

`z2-render` 的 `Pack::index` 固定 `CHR_PAGES * 256` = 32 页（原版 128 KiB
CHR），`check_page` 也拒绝 page ≥ 32。汉化版有 128 页，中文字形在 32-127，
所以材质包对它们**静默退回原始画面**。

修法：索引改 `MAX_CHR_PAGES * 256`（254 页，`u8` 除 `255` 外的全部），
`check_page` 相应放宽，模板生成器同步。`CHR_PAGES` 保留 32 作为"原版布局"
的默认值。新增测试 `high_pages_of_a_512k_image_are_addressable` 用 128 页
CHR 验证 32/64/127 页都能被解析。

### 已排除：A/B 两组 CHR bank 不是问题

`ctrl=$B2`（8×16 精灵）时两组值确实不同（A=`0,1,2,3,104..107`，
B=`104..107`），一度怀疑背景该用 B 组。**实测推翻了这个假设**：

- 把背景切换到 B 组后，标题文字反而变成残字。
- 回退后发现那个"残字"画面在旧版同样存在 —— 那是标题动画的过渡帧
  （第 700 帧），与 CHR 分组无关。

现在两条路径都用 A 组，与实测正确画面一致。`Ppu::load_chr_4k_bg` /
`Mmc5::chr_bg_slot_4k` / `chr_sets_differ` 接口保留但未接线，等有真机
对照证据再启用。

### 已验证：真实侧视场景贴图正常

用正确手柄位序可以进游戏（注册界面选空槽后**按 Down 再按 A** 触发
「結束」，Start/B/Up/方向键都不行），进入后 `mode=$0B`（侧视）、
`nt_map=$44`、非零像素 55,932：

- 宫殿内部：石柱、帷幔、地毯、火盆、Link 全部正确
- 状态栏「魔法 / 生命 / 下一级」汉字清晰
- 跑 120 帧后画面稳定，`exec_errors=0`

这一条已固化成测试 `reaches_sideview_scene_with_correct_tiles`。

### 已启用：宽屏「游戏性」扩展 + 双人合作

上一轮把这两项列为「无法支持」，其实**判断过早**。重新核对后它们可以支持，
理由与 ZALiA 有本质区别：

| | ZALiA 增强 | 宽屏游戏性 / 双人 |
|---|---|---|
| 怎么改游戏 | 按原版**字节偏移**直接 patch PRG | 挂 CPU 地址的 trap + 原样跑 ROM + 叠加扩展 |
| 有没有身份校验 | **没有** | PRG 改写前校验目标字节 |
| 译制版能否用 | ❌ 会写坏代码 | ✅ 可用 |

**踩到的关键 bug**：`wide_gameplay::bank0_mapped()` 和 `rom_table()` 读的是
`game.mmc1.map_prg`。译制 ROM 用 MMC5，`mmc1` 还是未初始化的上电默认值，
于是 bank-0 判断全错。改成按 `game.mapper` 分派到 `mmc1`/`mmc5`。

（`TOWN_OFFSETS_PRG = 3 * 0x4000 + (0x96DC - 0x8000)` 一度以为要改，实测
汉化 ROM 的 `E0 08 FF 01` 恰好也落在 PRG 偏移 55004 —— 因为 16 KiB 的 bank 3
等于 8 KiB 的 bank 6，字节偏移不变。所以无需修改。）

**⑧ 宽屏两侧画面乱码（`prg_read` 用错了 mapper 布局）**

宽屏边缘是「读 PRG 里的区域图块表 → 按世界坐标重新解码」出来的。
`wide_margins::prg_read` 硬编码了 MMC1 布局：**16 KiB bank，最后 16 KiB
固定**。译制 ROM 是 MMC5：**8 KiB bank，只有最后 8 KiB 固定**
（`$8000`/`$C000` 各自可切换）。

于是一个「bank 6 `$8500`」的读请求：
- MMC1 布局 → PRG 偏移 `6*0x4000 + 0x500` = `0x18500`
- MMC5 布局 → PRG 偏移 `6*0x2000 + 0x500` = `0x3500`

读到的是完全不同的字节，解出来的图块 ID 全错 → 两侧就是成片的错误图案。

修法：新增 `PrgLayout::{Mmc1, Mmc5}` 与 `prg_read_layout` /
`prg_read16_layout`，`build_margins_layout` 接收它，`Game::wide_margins`
用 `PrgLayout::of(self.mapper)` 传入。`prg_read` 保留为 MMC1 默认（不影响
原版路径与既有测试）。已加单元测试
`mmc5_layout_uses_8k_banks_and_last_8k_fixed`。

修复后宫殿场景 21:9：两侧各 152×240，非零像素 35,660/侧，画面是连贯的
石柱/帷幔/地砖，而不是之前的重复乱码。

**⑨ ★ 宽屏两侧是「文字碎片」的真凶：`record.page` 存了 1 KiB bank 号**

修完 ⑧ 之后两侧仍然是乱码，只是图案从「重复方块」变成了**汉字笔画碎片**。
这个症状本身就是线索：两侧取到的图块来自译制 ROM 的**文本页**。

根因在 `PpuBind::apply_mmc5`：

```rust
// 错
let bank = (mmc5.chr_bank_1k(page * 4) & 0xFF) as u8;  // 1 KiB bank 号
self.ppu.load_chr_4k(page, bank, &data);
```

`chr_bank_1k` 返回 **1 KiB 银行号**（译制 ROM 是 `104`），而
`record.page` 的语义是 **4 KiB 页号** —— `wide::chr_row` / `recorder` /
HD 包全部按 `page * 4096` 索引。于是第二槽被读成「页 104」：

```
旧: page=104 -> 偏移 104*4096 = 425,984   ← 落在汉字文本页
新: page=104/4=26 -> 偏移 26*4096 = 106,496 ✓ 正好等于 bank 104 * 1024
```

修法：`page_no = chr_bank_1k(page * 4) / 4`。修复后 `record.chr_page`
从 `[2, 104]` 变成 `[2, 3]`，宫殿场景两侧变成真实地形。

已加端到端测试 `wide_margins_use_4k_chr_page_numbers`。

**教训**：「1 KiB bank 号」和「4 KiB 页号」是两个不同的坐标系，
`record.page` 属于后者。这个错误不会让游戏画面出错（PPU 用的是
`chr` 数组本身，不经 `page`），只影响所有**读 record 的消费者**——
宽屏边距、HD 材质包、渲染记录。所以它藏得很深。

**⑩ ★★ 真凶：MMC5 的 A/B 两组 CHR bank 必须分开（有美版对照才定位到）**

修完 ⑧⑨ 后宫殿场景正确了，但**野外/城镇场景**（蓝天草地那种）两侧
仍然错。用美版 ROM（`E:\Zelda II - The Adventure of Link (USA).nes`）
在同一场景对比才看清：

```
汉化版野外 (mode=$0B, world=00, ctrl=$B1):
  A 组(精灵 $5120-$5127) 1k = [8,9,10,11, 12,13,14,15]
  B 组(背景 $5128-$512B) 1k = [12,13,14,15, 12,13,14,15]
```

**两组选的是不同的 4 KiB 页。** 硬件在 CHR 模式 3 + 8×16 精灵时，
精灵取 A 组、**背景取 B 组**。原来两条路径都用 A 组，于是：

- 主画面「看起来」正常（因为背景确实用了 B 组，`chr_bg` 是我后来才加的）
- 但 `record.page` 记的是 **A 组**的页，边距解码按它取背景像素 → 错

宫殿场景碰巧 A/B 的页相同（都落在页 3），所以之前测不出来。

修法：
1. `PpuBind::apply_mmc5` 在 `chr_sets_differ()` 时用 `load_chr_4k_bg`
   装 B 组；
2. `Ppu` 新增 `bg_page_no(table)`，背景取图时返回 B 组页号；
3. `l.chr_page` 改存 `bg_page_no(0..1)`，让 record 描述**背景**实际用的页。

验证：汉化版野外 `chr_page=[3,3]`（B 组），宫殿场景两版画面完全一致；
测试 `wide_margins_use_4k_chr_page_numbers` 改为断言
「record 页号 == B 组页号」且 `has_chr_bg()` 为真。

**为什么这次能定位**：美版 ROM 一对比就发现两版在野外场景的 CHR bank
完全不同，而宫殿场景恰好相同 —— 这解释了「为什么有的场景对、有的错」。

**⑪ ★★★ `area_bank` 的单位是 16 KiB，不是 8 KiB**

修完 ⑩ 后宫殿和洞穴都对了，但用户报告「大地图正常，其他场景仍错」。
大地图走 `OverworldSource`（读 WRAM），侧视走 `SideviewSource`（读 PRG）——
差异在 PRG 读取。

用美版逐项对比 metatile 指针表（`$8500`，`area_bank=1`）：

```
美版: $8417  $8417  $849F  $937D   ← 全部落在 $8000-$9FFF，合法
汉化: $D006  $AD0F  $078B  $0AF0   ← 完全不对
```

关键洞察：**`$0769` 里的 `area_bank` 是 16 KiB 单位**（原版 MMC1 的 bank
粒度，ROM 沿用了这套索引）。译制镜像只是把 ROM 重排成 8 KiB 单位，
所以索引 `n` 对应 8 KiB bank `2n` / `2n+1`。

```rust
// 错：把 area_bank 当成 8 KiB bank 号
let base = usize::from(bank) * 0x2000;
// 对：16 KiB 索引 -> 8 KiB 对
let base = usize::from(bank) * 2 * 0x2000;
```

验证：`bank=1` 时读 8 KiB bank 2，指针表回到 `$8417`，**与美版逐字节一致**。

另外 `$8000-$BFFF` 是 **16 KiB 窗口**，`$A000` 应切到下一半（`+0x2000`），
不是 `+0x1000`。

**教训**：「bank 号」很容易假设成单一粒度。这里存在三套：
`$0769` 用 16 KiB 索引、MMC5 硬件用 8 KiB、CHR 又用 1 KiB。
`record.page` 还是 4 KiB 页号。每次跨层传递都要问「这是哪个单位的」。

已加单元测试 `mmc5_layout_uses_16k_window_and_last_8k_fixed`。
实测确认正常：宫殿、洞穴（草地+树干）、大地图三类场景两侧都连贯。

**双人的激活条件**（`coop::observe`）：只在侧视场景激活
（`$0736==$0B && $00B5==1 && $0503==0`），大地图上 P2 不出现是正常的。
另外 `$D6C1`/`$E677`/`$E558` 是「没有原版实现就不覆盖」的地址，不必强求注册。

**实测证据**：

```
宽屏 trap 命中 76,576 次   mode=$0B  errors=0
coop trap 命中 19,200 次   P2 active=true alive=true hp=127/127
```

双人画面确认有两个 Link（左侧绿色 P1、右侧偏白 P2）。

已固化成三条测试：`wide_gameplay_traps_fire_on_translated_rom`、
`coop_activates_second_link_on_translated_rom`、`reaches_overworld`。

### 已修复：边缘精灵（margin sprites）

`wide_sprites` 用一个**影子 Game** 重跑 ROM 的 `bank7_Display`（`$EF11`），
把落在 256px 窗口外的敌人收集起来画进边距。两处对译制镜像不成立：

1. `set_enabled` 用 `Game::new()` 建 shadow，而它的 `mapper` 默认是
   **MMC1**。译制镜像是 MMC5，shadow 读 PRG 的 bank 全错 → 一个精灵都
   收集不到。修法：`shadow.mapper = game.mapper`，并复制 `mmc1`/`mmc5`。
2. `observe` 每帧只同步了 `sh.mmc1`，**漏了 `sh.mmc5`**。ROM 每帧都在切
   bank，影子副本跟不上。修法：补 `sh.mmc5 = game.mmc5.clone()`。

顺带修了 `compose_wide`：它还在调 `build_margins`（MMC1 布局），
没走 mapper 感知的 `build_margins_layout`。

**排查陷阱**：`shown`（收集到的边缘精灵）在 `$4014` DMA 时填充，
必须在 `step` 之后**立刻**读；隔一帧再读就被下一轮 DMA 覆盖了。
我第一版探针就是这样，误判成「功能没生效」。

实测：`shown 最大 10, 进图 10`，`shadow_errors=0`。洞穴场景左侧边缘
能看到窗口外的红色敌人，右侧也有。已固化成
`margin_sprites_reach_the_margins` 与 `wide_gameplay_blobs_reach_the_margins`。

### 已修复：边缘精灵的贴图错乱（`sprite_page` 与 `chr_page` 混用）

边缘精灵能画出来了，但**贴图是文字碎片**。原因是我在 ⑩ 里把
`LineRecord::chr_page` 改成了**背景**页（B 组），而
`wide::margin_sprite_on_line` 拿它当**精灵**页用：

```rust
let page = rec.chr_page[table];   // 背景页，错
```

译制 ROM 的野外场景 A 组（精灵）= `[8..15]`、B 组（背景）= `[12..15]`，
页号不同，于是精灵图案从错误页取 → 画出来是汉字笔画。

修法：`LineRecord` 新增 `sprite_page: [u8; 2]`，`state.rs` 里写
`l.sprite_page = self.chr_page_no`（A 组），`chr_page` 继续存背景页
（B 组）。`margin_sprite_on_line` 改用 `sprite_page`。

**教训**：`chr_page` 这个名字太含糊。PPU 有两个独立的图案表选择器
（`$2000` 的 BG/sprite 表位），而 MMC5 又让两者指向不同的 bank 组，
所以「一个 chr_page」根本不够表达。字段已加注释说明用途，
新增测试 `margin_sprites_use_the_sprite_page_not_the_background_page`。

实测（`world=00` 野外，就是你截图那个场景）：边缘精灵最多 12 个，
贴图是正常的红色小怪，不再是文字碎片。

### 已修复：P2 打不出伤害（译制路径根本没有 trap）

`emu_from_generic_ines`（MMC5 路径）**从未注册任何 trap**，而
`coop::register_coop_traps` 的三个覆盖项只在「目标地址已有基础 trap」时
才覆盖：

```rust
for (name, addr, f) in overrides {
    if let Some(cycles) = game.traps.get(addr).map(|t| t.cycles) {
        game.trap_register_cycles(name, Some(7), addr, f, cycles);
    }
}
```

于是 `$D6C1`（接触）、`$E677`（剑击）、`$E558`（敌武器）全部跳过 ——
P2 能走能跳能显示，但对敌人完全无害，也不会被敌人打。

修法分两步，第二步是踩坑后加的：

1. 在译制路径注册 trap 组（`register_translated_traps`）。
2. **`bank7_traps` 的 MMC1 串行切换器必须跳过。** 译制镜像在
   `$FF9D`/`$FFB1`/`$FFC5`/`$FFC9`/`$FFCC` 上放的是 MMC5 代码：

   ```
   汉化 $FFCC: 0A 09 80 8D 15 51   ; ASL A / ORA #$80 / STA $5115
   美版 $FFCC: 8D 00 E0 4A 8D 00    ; MMC1 串行写 $E000
   ```

   把 MMC1 的移植版盖上去会写一个不存在的寄存器、MMC5 bank 没切换，
   CPU 随后执行到未映射区 —— 表现为 `SwapPRG` 命中一次、PC 一路
   `$A6A1 → $A9BD` 递增、每帧一个错误。判据是 `uses_mmc5_switcher()`：
   只在 `$FFCC` 恰好是上面那 6 字节时才跳过，合成 fixture（全零 blob）
   仍走原路径，`ow_saved_bank_wrappers_bracket_the_banked_call` 因此不受影响。

**为什么没有注册全部组**：二分测试逐个跑过，`title_traps` 和
`sideview_traps` 会 `jsr_sub` 进固定区地址、再按硬编码的后继地址恢复
（如原版 `$C341` 的 `JSR $FFB1` 在译制版是 `JSR $C03D`），照原序列重放
会落到 MMC5 数据上（`illegal opcode $52 at $FFB1`）。所以
`register_translated_traps` 只装 `bank7`(已过滤)/`player`/`enemy`/
`town`/`palace`/`boot` 六组，实测均 0 错误。

实测：`P2 hp 127 -> 47 (deaths=1) mode=$0B errors=0` —— P2 会受伤、会死亡，
说明接触与剑击通道都真的走到了 P2。已固化成
`coop_p2_takes_part_in_combat`。

### HD 材质包：简繁两态都要能覆盖

HD 包本身**已经能用在汉化版上**（宫殿场景 `5184/8160 = 63%`，与美版逐 tile
相同）。先前误判「没生效」是因为用了 `--dump-frame` —— 它明确不带 pack；
`--dump-present` / 窗口才是完整渲染路径。

真正的 bug 在简繁切换：

**① 繁体页号整体 +64，HD 全部落空**

`$5130` bits 1-0 给 CHR bank 加 `+256` 个 1 KiB（= `+64` 个 4 KiB 页），
**图案不变、只是页号平移**。实测：

| 场景 | 简体页 | 繁体页 |
|---|---|---|
| 标题 | 26 | 90 |
| 宫殿 | 3 | 67 |

HD 包按简体布局索引，繁体下命中率从 63% 直接掉到 **0%**。

修法：`HdPack::lookup` / `lookup_variant` / `lookup_default` 在精确页查不到时
回退到 `page - TRADITIONAL_PAGE_OFFSET`（64）。实测繁体回到 `5184/8160`，
与简体完全一致。

**② 每次启动没有回到简体**

简繁选择存在 **RAM `$01D4`**，ROM 初始化时用它恢复 `$5130`：

```asm
$9E8E: AD D4 01   LDA $01D4
       C9 01      CMP #$01
       F0 02      BEQ +
       A9 00      LDA #$00
    +: 8D 30 51   STA $5130
```

`mmc5` 和 `$01D4` 都跨 `reset` 存活，所以只清 `$5130` 不够 —— 下一帧就被
写回（实测 reset 后第 2 帧 `upper2` 又变 1）。修法：`Game::reset` 在
`Cpu::reset` **之后**同时清 `$5130` 与 `$01D4`。

已固化成 `every_start_begins_in_simplified_chinese` 与
`traditional_pages_still_get_hd_art`。

**③ 文本帧被 HD 地形覆盖 → 对话乱码**

译制 ROM 把**中文字形放进了原版地形的 `(page, tile)` 槽里**。实测：
HD 包中 `page 3, tiles 179-184` 是**云朵**（`sheet-16` 的 32×32 块），而
译制版那一带是对话框文字。HD 照常覆盖，就把中文对话画成一片云和地面碎片
—— 这正是「室外对话乱码、室内正常」的原因（室内场景那几行刚好没有 HD 覆盖）。

修法（**第一版，范围过大**）：`Game::is_showing_text()` 判定文本帧

```rust
self.ram[0x074C] != 0     // 对话框
 || self.ram[0x0524] != 0 // 暂停菜单
 || !matches!(self.ram[0x0736], 0x05 | 0x0B)  // 标题/名字注册/存档菜单
```

`Display::present` 把它传给 `Presenter::set_text_mode`，`Compositor::compose`
在文本帧把 `input.pack` 当作 `None`（整帧丢掉 pack，走原版 CHR）。

**用户反馈**：一开对话整屏 HD 全没了，画面来回跳变，太突兀 —— 应该只让文本框区域失效。

**修法（当前）**：改成**按行**控制。`Game::text_rows()` 返回受影响的行范围：

| 状态 | 行范围 |
|---|---|
| 对话框（` != 0`） | 32..96 |
| 暂停菜单（` != 0`） | 104..208 |
| 标题/名字注册/存档 | 0..240（整屏都是文本） |

`ComposeInput::text_rows` 传进 `Compositor::compose`，渲染循环逐行判断：
文本框那几行走原版 CHR，其余行照常叠 HD。

**用户反馈（第二次）**：按行还是太宽 —— 整条横带失效，看着突兀。

**修法（当前）**：改成**矩形**。`Game::text_rect()` 返回 `z2_ppu::TextRect`：

| 状态 | 矩形 (x, y) |
|---|---|
| 对话框 | x 112..224, y 32..96 |
| 暂停菜单 | x 112..224, y 104..208 |
| 标题/名字注册/存档 | 整屏 0,0..256,240 |

列范围**不硬编码**：`Compositor` 每行用 `text_box_span`（既有的函数，
从边框 tile `//` 反推窗口左右边界）算出来，所以任何窗口
（对话/告示牌/暂停）都能自动跟随。`paint_slot` 逐像素判断
`self.text_box`，落在窗口内就跳过 HD。

实测：暂停菜单下**塞尔达公主的 HD 立绘、石柱雕花、帷幔、红毯**
全部保留，只有窗口矩形内回退原版。

**用户反馈（第三次）**：室外对话又乱码了。

**根因**：`resolve_columns`（算窗口列范围的函数）只在**宽屏边距路径**上
被调用，普通窗口模式下 `self.text_box` 恒为 `None` —— 所以边框检测
根本没跑过，HD 一如既往盖住文字。

而且「室外对话」不设 ``（实测 `dlg=00` 却有窗口），所以
靠游戏状态判断本身就不可靠。

**修法（当前）**：`resolve_columns` 改成**每行无条件调用**，窗口
识别完全靠边框 tile `//`，不依赖任何游戏状态标志。

`ust
// compositor.rs 主循环，每行都跑
self.resolve_columns(rec, &input, y);
// 内部: self.text_box = text_box_span(&self.col_id[..]);
` 

已固化成 `hd_art_gives_way_inside_a_text_window`：构造一个「有边框但
`text_rect = None`」的行，断言窗口内不出 HD、窗口外仍出 HD。

**用户反馈（第四次）**：仍乱码。

**真凶**：`background_pixel` / `paint_slot` 里用 `text_box` 挡 HD 单元格时
**单位搞错了**。`text_box_span` 索引的是 `resolve_columns` 逐**像素列**填的
`col_id` 数组，返回的 `first..last` 是**像素列**；但 `paint_slot` 里拿 slot
`k` 比，`background_pixel` 里又把 `sx >> 3`。两边都错，真实窗口
（列 5..26 = 像素 40..215）从来没命中过。

统一为**像素列**后修复。

**为什么前几轮测不出来**：旧测试只画了**边框行**（首尾 `$CA/$CB` + 中间
`$CB`），而乱码发生在**内容行** —— 那些行只有左右两条 `$CC` 竖边、中间
全是字形。补了 `hd_art_gives_way_on_window_content_rows` 才覆盖真实现象。

**已确认无问题（2026-10-22，用户实机测试）**。

之前记的「暂停菜单乱码」是误判。自动化驱动进暂停菜单后，窗口本身正常
（蓝框、`x2/x6/x0` 计数都对），窗口内部是空的 —— 那是**新存档没有道具**，
道具栏本来就该是空的，不是 bug。用户实测确认暂停菜单显示正常。

**不要再把这条当成待办。** 如果以后又出现类似的「某处中文乱码」报告，
先确认是不是同一个误判（空道具栏 / 空列表），再动手。

（顺带记一个驱动坑：`tap(key, N)` 连按 N 次会**反复开关**菜单，偶数次
等于没开。驱动暂停菜单必须只按一次。这个坑让我一度以为 `$0524` 不是
暂停标志。）

### 2026-10-22：存档不带 MMC5（已修）

`GameState` 只有 `mmc1`，没有 `mmc5`。受影响的不是 F7（`.z2snap` 本来只存
RAM+WRAM，所有 mapper 一致），而是 **`GameState` 那条路径**：

* `netplay.rs` 的 `save_state` / `load_state` —— 联机回滚恢复不了 CHR bank、
  `$5130` 简繁位、IRQ 锁存和 ExRAM；
* `hash_state(&[&state.to_bytes()])` —— 连 MMC5 漂移都看不见，不同步检测不出来。

修法（`crates/z2-core/src/state.rs`）：

1. `GameState` 加 `mmc5: Mmc5` + `mmc5_in_use: bool`（只有 MMC5 才写块，
   MMC1 镜像只多 1 字节）；
2. `save_state_into` 拷 `self.mmc5`，按 `mapper` 置标志；
3. `load_state` 先还原寄存器，**再**还原 PPU，然后调
   `sync_from_mapper_mmc5` 重算 CHR 槽位和 `$5105` 镜像 —— 这一步对
   v4 和 v3 存档都正确（v3 没有寄存器，就让实时值赢，同时把 PPU 拉一致）；
4. `STATE_VERSION` 3 → 4，`to_bytes`/`from_bytes` 加 MMC5 块。

实测（禁用第 3 步就会复现）：

```
存档前: upper2=0 chr1k=[0,1,2,3,104,105,106,107]
切换后: upper2=1 chr1k=[256,257,258,259,360,361,362,363]
读档后: upper2=0 chr1k=[0,1,2,3,104,105,106,107]   ← 修复后
```

固化成 `save_state_restores_mmc5_registers` 和 `state_byte_image_carries_mmc5`。
另外 `state_tests.rs` 里伪造 v2 镜像的用例要跟着改：v2 早于 MMC5 块，
削掉增强块之后还得 `pop()` 掉那个存在标志字节。

### 简繁切换的真实行为（实测，别再写反）

用真实 ROM 逐键实测（`probe_lr`）：

| 按键 | `$5130` | 字形 |
|---|---|---|
| 右键 `0x80` | **1** | **繁体** |
| 左键 `0x40` | **0** | **简体** |
| 不按 | 0 | 简体（启动默认） |

**右键不是 toggle** —— 连按 4 次仍是 `upper2=1`。要回简体必须按左键。

这个选择存在 RAM `$01D4` 和 `$5130` 里，**整局游戏都会保持**。所以用户报
「进游戏默认还是繁体」不是 bug：上次在标题画面按过右键。用户已确认**保留
切换能力**，因此 `Game::reset` 里的清零只负责「每次启动回到简体」，不干预
游戏内按键。已固化成 `left_key_switches_back_to_simplified`。

（我一度按 `$9E70` 的汇编反推出「右键 → 0」，与实测相反；汇编里
`$F7` 是移位后的位序，与 `poll_pad1` 的输出不是同一套编号。**这类结论
必须以实测为准**。）

### 下一步（按价值排序）

1. **继续实测更多场景**：已验证标题、存档菜单、名字注册、宫殿侧视场景。
   大地图、战斗、对话、其他宫殿还没跑过。
2. **ExRAM 扩展属性 / 填充模式**：ROM 都未使用（`$5104` 从未被写），
   实测无影响，优先级最低。
3. **性能**：汉化版走纯解释器，比原版 trap 路径慢。

---

## 七、常用命令

```bash
# 环境
export PATH="/c/Users/ezplayer/.cargo/bin:/c/msys64/ucrt64/bin:$PATH"
export CARGO_TARGET_DIR="D:/z2rs-build"
cd "D:/下载/Compressed/main/z2rs-main"

# 单元测试（不依赖 ROM）
cargo test -j 4 -p z2-core --lib            # 184 条，全部应通过
cargo test -j 4 -p z2-core --lib mmc5       # 只跑 MMC5 的 13 条

# 汉化 ROM 端到端（需要 ROM 在指定路径）
cargo test -j 4 -p z2-native --test mmc5_zh_rom -- --nocapture

# 编译 release
cargo build --release -j 4 -p z2-native -p z2-launcher -p z2-signal

# 用发布版 exe 直接验 ROM（无需窗口）
"D:/z2rs-中文版/z2rs.exe" --headless \
  --rom "D:/z2rs-中文版/Zelda II - The Adventure of Link (U) [20260806_NOKOH].nes" \
  --frames 300 --dump-frame out.png

# 部署（同步全部 exe + 校验 + 清理别名）
bash deploy.sh
```

**注意**

- 只编译 `-p z2-native` 会让 exe 从 59MB 掉到 53MB（feature 解析不同），
  必须带齐三个 `-p`。
- **这台机器上 `git` 和 `bash` 都不可用**（msys64 只装了 coreutils，没有
  `git.exe`；`/usr/bin/env bash` 也解析不到）。`deploy.sh` 需手工照做：
  复制三个 exe → 逐字节校验 → 确认无别名。校验逻辑见文件末尾的 Python heredoc。
- **不要在 PowerShell 里用 `Set-Content` / `Out-File` 写含中文的文件** ——
  会把 UTF‑8 变成乱码（本次已经把 HANDOVER.md 写坏过一次，用
  `[System.IO.File]::WriteAllText($p, $s, (New-Object System.Text.UTF8Encoding($false)))`
  或编辑器写才安全）。

---

## 八、重要提醒

1. **MMC5 版已部署、已打包、实测通过**。`D:/z2rs-中文版/z2rs.exe`
   （59,191,894 / sha256 `2827dacf7f88...`）能直接跑汉化 ROM；
   zip 发布包（29.79 MB / `3112c89916a5...`）已重新打包并验证解压可运行。

2. **原版 MMC1 路径完全没动**。`z2-core` 185 + `z2-ppu` 65 + `z2-render` 全过，共 845 条。

3. **汉化版走纯解释器（无 trap）**，比原版慢很多。这是唯一正确做法 ——
   汉化版固定 bank 有 51% 与原版不同，按原版地址硬编码的 Rust trap 全部会跑错。

4. **不要再试"从汉化 ROM 提取字库"**，见第四节路线 B。

5. **简繁切换是「→ 右方向键」**（标题画面）。手柄位序 NES 标准：
   `A=0x01 B=0x02 Select=0x04 Start=0x08 Up=0x10 Down=0x20 Left=0x40 Right=0x80`。
   早期文档误写成「按 A」，是把手柄掩码 `0x80` 当成了 A。
   切换靠 `$5130` 把 CHR 窗口整体 +256 个 1KB bank。

6. **`$5105`（nametable 映射）必须接进 PPU**。只存值不生效会让某些场景
   从左 nametable 取右半边的图块 → 贴图错乱。另外 `$44` 是**竖**镜像、
   `$50` 是**横**镜像，别记反。

7. **译制 ROM 上只有 ZALiA 增强和随机化器不装**。它们按原版**字节偏移**
   直接 patch 且不做身份校验。
   **宽屏游戏性和双人合作是可用的**：它们挂 CPU 地址的 trap、原样跑 ROM、
   唯一的 PRG 改写先校验目标字节。别再把它们当成「不支持」。
   被拒绝的功能由 `refuse_unsupported_features()` 明确提示，不静默失效。

5. **不要再给 MMC5 加"ROM 专属补丁"**。这次的教训就是：一个看似必要的特例
   （钉死 `$E000`）掩盖了真正的寄存器分配错误，多花了很多时间。遇到诡异
   行为先怀疑自己的实现，别先怀疑 ROM。

6. **exe 命名规则**：每个程序只有一个名字，不造别名。
   `z2rs.exe`（游戏）/ `z2-launcher.exe`（启动器）/ `z2-signal.exe`（辅助）。
   曾经复制过 `z2rs-launcher.exe`、`z2rs-native.exe`，导致同目录两份启动器
   一新一旧，用户点到旧的会以为更新没生效。


---

## 2026-10-22：移植到官方 0.6.0

源码目录 `D:\z2rs-060-zh`，发布目录 `D:\z2rs-中文版-0.6.0`。

### 怎么做的

旧仓库（`D:\下载\Compressed\main\z2rs-main`）没有配置 remote，官方源码
是用户下载的 `z2rs-0.6.0.zip`。用 git 历史做三方合并：

1. 解压 0.6.0，`git init` + 提交为 `736f8e1 upstream z2rs 0.6.0`
2. 旧仓库里把未提交的改动落成提交 `4c0b6dc`
3. 新仓库 `git remote add old <旧仓库>`、`fetch`，再
   `git cherry-pick daa89c2..old/master`

这样 git 能用「父提交」做真正的三方合并，比 `git apply --3way` 靠谱
（后者报 `repository lacks the necessary blob`，因为新仓库没有基线对象）。

### 冲突（6 处，全部手工解）

| 文件 | 冲突内容 | 解法 |
|---|---|---|
| `z2-core/src/cpu.rs` | 0.6.0 在 `$6000-$7FFF` 加了 `wram-probe` | 两边都要：MMC5 分支 + probe |
| `z2-core/src/ppu_bind.rs` | `mapper_key` 类型 + `text_writes` 字段 | 用我们的 `Option<u64>`；`text_writes` **删掉**（0.6.0 已移除该机制） |
| `z2-render/src/compositor.rs` | `ComposeInput` 加 `time`（上游）/ `text_rect`（我们） | 两个字段都留 |
| `z2-render/src/presenter.rs` | 同上，构造处 | 两个字段都传 |
| `z2-render/tests/bench.rs` | 同上 | 同上 |
| `z2-render/tests/compositor.rs` | 同上 + 函数错位 | 见下 |

**`tests/compositor.rs` 的做法**：不要逐块合并。以 0.6.0 的文件为基准，
把我们新增的 5 个测试函数**追加**到末尾即可（它们只依赖上游已有的
helper）。逐块拼会把 `compose_at` 的函数体挤掉、`ComposeInput` 字面量
字段重复或缺失 —— 我踩过，错误很多。

### 移植后必须跟着改的两处

1. **`Enhancements` 新增 `content` 字段**（官方新 crate `z2-content`）。
   任何**完整结构体字面量**都要补 `content: Default::default()`，
   否则 `missing field`。

2. **`ComposeInput` 的 `Copy`**：0.6.0 是 `#[derive(Debug, Clone, Copy)]`，
   我们旧版改成了 `Clone`。测试里大量 `..input` 结构体更新要求 `Copy`，
   恢复成 `Copy`（`text_rect` 是 `Option<TextRect>`，可 Copy）。

3. **`sprite_page` 回退**：我们为 MMC5 把边缘精灵改成读 `sprite_page`，
   但 0.6.0 的测试/调用方只设 `chr_page`，`sprite_page` 停在 `NO_PAGE`
   就画成黑色（`enemies_in_the_margins_replace_their_own_margin_sprites`
   因此失败）。改成：

   ```rust
   let page = match rec.sprite_page[table] {
       NO_PAGE => rec.chr_page[table],   // 未填 = 未拆分，沿用背景页
       page => page,
   };
   ```

   同时 `margin_sprite_row_expansion_8x8_and_8x16` 里「无 CHR 页」的断言
   要**同时**清 `chr_page`，否则回退后不再返回 `None`。

### 验证

```
核心（z2-core + z2-ppu + z2-render）：957 项全过
汉化 ROM 端到端（mmc5_zh_rom）：     24 项全过
美版 headless 冒烟：                 faults 0
汉化 headless 冒烟：                 faults 0
材质包：新包 / 旧包 v0.1.1 / 无包    全部正常
```

### 一个坑：PowerShell 重定向会毁掉 git 输出

`git diff > file`、`git show > file` 用 PowerShell 重定向会写成 UTF-16，
后续 `git apply` 报 `No valid patches in input`。用 `cmd /c "git ... > file"`。

### 发布

```
D:\z2rs-中文版-0.6.0\
  z2rs.exe          59,620,603  233315db30d4
  z2-launcher.exe   25,078,554  2bb0ce4e72a4
  z2-signal.exe      5,263,495  c3eb613d360d
  SHA256SUMS / 版本说明.txt / README-官方.md / HOW-TO-PLAY.txt / LEGAL.md / LICENSE
  z2rs-中文版-0.6.0.zip  29,899,417  3a1ce34f221f
```

与 `D:\z2rs-中文版`（旧版基线）并存，互不影响。
---

## 2026-10-23：作者补档，材质包修复版已验证

作者重发了两个修复版压缩包：

    D:\下载\Compressed\z2rs-hd-pack-2026.10.22-outlined_2.zip
    D:\下载\Compressed\z2rs-hd-pack-2026.10.22-unoutlined_2.zip

首发版缺 4 个水面动画帧（`environment/layers/W_0.png ~ W_3.png`），
修复版已补齐。核对结果：

| 项 | 首发版 | 修复版 |
|---|---|---|
| `pack.json` 引用 PNG | 22,920 | 22,920 |
| 磁盘实际 PNG | 22,916 | 22,920 |
| `RELEASE.json` 声明 | 22,916 | 22,920 |
| 缺失文件 | 4 | **0** |
| `W_0~3.png` | 无 | 有（各 1024x832） |
| SHA256SUMS 含 `W_0.png` | 否 | 是 |

**修复版可以直接加载，不再需要 `-playable` 补丁目录。**

已验证（汉化 ROM + 官方 0.6.0 + 4x）：

* 两个修复版原包均加载正常，`interpreter faults: 0`
* 宫殿场景 HD 差异 603,191（unoutlined）/ 603,473（outlined）像素
* 宽屏 16:9 输出 1728x960，扩展区 HD 正常
* 水面动画：四帧互不相同（各差约 8,000-9,000 像素），
  渲染随时间推进（相邻 12 帧差异 7,232 / 8,704 / 12,432 像素）
   —— 证明移植时保留下来的 0.6.0 `ComposeInput::time` 字段
   确实在驱动动画

交付位置（已从 zip 解压拷贝，非补丁版）：

    D:\z2rs-hd-pack-2026.10.23-outlined
    D:\z2rs-hd-pack-2026.10.23-unoutlined

`D:\z2rs-中文版-0.6.0\版本说明.txt` 的「已知问题」第 1 条
已从「用 -playable 绕过」改为「用作者修复版」。

### 一个 PowerShell 坑

`Expand-Archive` 解这个 200MB+ 的包时静默失败（目标目录为空，
且不报错），`Add-Type System.IO.Compression.FileSystem` 在新
PowerShell 会话里也可能找不到类型。用 `tar.exe -xf` 最可靠：

    tar.exe -xf xxx.zip -C <目标目录>

tar 解出来的顶层是 zip 里那层同名目录，注意路径要多套一层。