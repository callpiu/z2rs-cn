> 2026-10-24 菜单修复更新：后四种魔法名称重复已修复，游戏内 O 键菜单已汉化。验证、发布与限制见 [HANDOVER-MENU-FIX.md](HANDOVER-MENU-FIX.md)。下文为历史记录。

> 2026-10-23 大地图修复更新：进入大地图后的长柱/贴图错乱已定位并修复，见 [HANDOVER-OVERWORLD-FIX.md](HANDOVER-OVERWORLD-FIX.md)。下文关于 CHR 无切换和兼容分支的旧结论已被本次真实场景验证推翻。

# MMC5 移植 · 进度与交接

> 目标：让 z2rs 直接运行汉化版 ROM
> `Zelda II - The Adventure of Link (U) [20260806_NOKOH].nes`
> （mapper 5，PRG 256KB，CHR 512KB）
>
> 状态：**✅ 已跑通 —— 能画出标题画面、能按 Start 进存档菜单、汉字正常显示**
> 最后更新：2026-10-21 13:10

## 零、一句话结论

**根因不是"ROM 需要特殊处理"，而是 MMC5 的 PRG 模式寄存器分配写错了。**
之前那段「`$E000-$FFFF` 必须钉死最后一个 bank」的补丁是在绕错，把真正的
bug 藏起来了。改成教科书语义（模式 1/2 用 `$5115` 驱动 `$8000`，模式 2 的
`$E000` 由 `$5117` 驱动）之后，ROM 一次跑通。

## 一、已完成

### 1. MMC5 mapper（`crates/z2-core/src/mmc5.rs`）

| 功能 | 状态 |
|---|---|
| PRG 四种模式（32K / 16+16 / 16+8+8 / 8x4），寄存器分配正确 | ✅ |
| CHR 四种模式（8K / 4K / 2K / 1K） | ✅ |
| `$5128-$512B` 四个 **8 位**背景 bank 寄存器 | ✅ |
| `$5130` CHR 高位（bit 8-9）= 简繁切换 | ✅ |
| `$5105` nametable 映射 → **真正接进 PPU 镜像** | ✅ |
| ExRAM `$5C00-$5FFF` / `$5104` 模式位 | ✅（仅 RAM 读写） |
| 扫描线 IRQ `$5203`/`$5204`（含 bit 6 in-frame 跟踪） | ✅ |
| 硬件乘法器 `$5205`/`$5206` | ✅ |
| PRG-RAM bank `$5113`、保护位 `$5102/$5103` | ✅ |
| **ExRAM 扩展属性（逐格调色板）** | **未实现**（ROM 未用 `$5104`，实测无影响） |
| 填充模式 `$5106/$5107` | 只存值（ROM 未用） |

### 1b. 显示类功能（不 patch ROM，译制版可用）

| 功能 | 状态 |
|---|---|
| 宽屏画面（16:10 / 16:9 / 21:9） | ✅ 需要 `feats.record` |
| 边缘精灵 margin sprites | ✅ |
| HD 材质包 | ⚠️ 页号 ≥32 取不到（见第六节） |
| 显示增强（扫描线/亮度/饱和度/泛光/模糊/抖屏） | ✅ 纯 `DisplayEnh`，走 presenter |

### 2. 接入 Game

- `MapperKind::{Mmc1, Mmc5}` 枚举
- `Game::from_ines` 接受 mapper 1 和 5，其余仍报 `WrongMapper`
- PRG 读、mapper 寄存器读写、PPU 的 CHR 同步，全部按 `game.mapper` 分派
- `capture_frame` 的 frame-end CHR 同步也按 mapper 分派

### 3. 通用加载路径（`crates/z2-native/src/app.rs`）

`emu_from_generic_ines()`：不装 trap、不打补丁，整个 ROM 交给 6502 解释器。
`emu_from_rom_file_with()` 检测到 mapper 5 时自动走这条路。

### 4. 端到端验证（`crates/z2-native/tests/mmc5_zh_rom.rs`，7 条全过）

```
300 帧后：非零像素 55201 / 61440     ctrl=$B2 mask=$1E
按 Start：$0736 03 -> 00，画面变化 18121 像素
exec_errors = 0（全程零解释器错误）
```

标题画面（亚瑟王之剑插在石头上）和存档菜单（「選擇紀錄 / 名字 / 等級 /
註冊你的名字 / 刪除模式」）都正确渲染，汉字清晰。

## 二、踩过的四个真 bug（都已修）

### ① `bus_write` 把 MMC5 寄存器写入全吞了

```rust
// 原来
0x4020..=0x5FFF => {} // Expansion stub: ignore.
```

MMC5 的全部寄存器（`$5100-$5FFF`）都落在这个区间。这一行导致
**所有 bank 切换都无效** —— ROM 复位后跳进 `$C000`，读到未初始化的 `$FF`。

### ② MMC5 上电时所有 bank 寄存器为 0

导致 `$FFFC` 从 bank 0 读，而 bank 0 的 `$1FFC` 是 `$9658`，
`$9658` 处是 `$FF` —— 直接跑飞。

实测：只有 **最后一个 8KB bank（bank 31）** 有合法向量表
（`NMI=$C07B`、`RESET=$FF4C`、`IRQ=$FF9C`）。

修法：`Mmc5::init_fix_last()`，上电固定 `$8000`=bank 0、`$C000`=倒数第二、
`$E000`=最后一个。

### ③ ★ 真凶：PRG 模式 1/2 的寄存器分配写错了

**这是让 ROM 一直不出画面的原因。** 原来写成模式 1/2 用 `$5114` 驱动
`$8000`，并且把 `$E000-$FFFF` 硬钉在最后一个 bank。两处都错。

正确的 NESdev 分配：

| 模式 | `$8000` | `$A000` | `$C000` | `$E000` |
|---|---|---|---|---|
| 0（32K） | `$5114` | `$5114`+1 | `$5114`+2 | `$5114`+3 |
| 1（16+16） | `$5115` | `$5115`+1 | `$5116` | `$5116`+1 |
| 2（16+8+8） | `$5115` | `$5115`+1 | `$5116` | **`$5117`** |
| 3（8x4） | `$5114` | `$5115` | `$5116` | `$5117` |

**字节级证据**（都在这个 ROM 里）：

- 全 PRG 只有 `8D 15 51`（`STA $5115`）和 `8D 16 51`（`STA $5116`），
  **`$5117` 一次都没写过** → 顶窗保持 `init_fix_last` 上电状态（bank 31），
  向量表和共享子程序（`$FEB0`、`$FFCC`）才一直可调用。
- `$FDEB: LDA #$8E / STA $5115`（bank 14 进 `$8000`）、
  `$FDD9: STA $5116`（bank 14 进 `$C000`）、
  `$FDF6: JMP $C000` → 落在 bank 14 的 `$C000`，正是初始化代码
  （`LDA #0 / STA $2001 / STA $2000 / LDA #5 / JSR $FFCC`）。
- 运行时切 bank 在 `$C0CC`：`LDA #$8F / STA $5116 / 9×NOP / JSR $D2EC`，
  目标是 bank 15 的 `$C000` 区（bank 15 的 `$C000-$C1FF` 是 `$FF` 填充，
  代码从 `$C200` 开始，那 9 个 `NOP` 就是跨过填充用的）。

**症状**：修之前 PC 停在 `$FF63`，从 `$FF62` 开始全是 `$FF`（非法 opcode）。
因为 `STA $5116` 在错误语义下把 `$E000` 换成了 bank 30（全 `$FF`），
下一条取指直接撞墙。

### ④ `$5204` 的 bit 6 是"在帧内"，不是"NMI 使能"

原来读 `$5204` 返回 `irq_enabled << 6`。硬件上 bit 6 表示束流是否在可见帧
内（vblank 带 240-260 为 0），bit 7 才是 pending。修法：`on_scanline`
里跟踪 `in_frame`，读时返回 `pending<<7 | in_frame<<6`。

### ⑤ 语音 ROM 的 `$5105` 没接进 PPU → 部分场景贴图错乱

`$5105` 决定 4 个逻辑屏幕各取哪块 nametable。汉化 ROM 启动写 `$44`
（屏幕 A,B,A,B = 竖镜像），某些场景改写成 `$50`（A,A,B,B = 横镜像）。

原来 `Mmc5::write` 只是把值**存下来**（`nt_map` 字段），
`decode` 成 `nametable_source()` 也没有任何调用者 —— PPU 一直用自己
的默认 `Mirroring::Vertical`。后果是某些场景**从左边的 nametable 取右
半边的图块**，表现就是贴图错乱。

修法三处：

1. `PpuBind::apply_mmc5()` 里调 `ppu.set_mirroring(from_mmc5_nt_map(nt_map))`；
2. `cpu.rs` 的 bus_write 触发范围加上 `addr == 0x5105`（原来只有
   `$5120-$5130`，改映射不会重新同步）；
3. 缓存 key 里也并入 `nt_map`，否则改了映射但 key 没变会被跳过。

注意位序：`$44` = 屏幕 A,B,A,B = **竖镜像**；`$50` = A,A,B,B = **横镜像**。
别想当然（我第一次就写反了，被单元测试抓住）。

### ⑥ `$5130` 位宽错（简繁切换失效）+ 手柄位序误标

**简繁切换键是 →（右方向键），不是 A。** 手柄位序是 NES 标准：
`A=0x01 B=0x02 Select=0x04 Start=0x08 Up=0x10 Down=0x20 Left=0x40 Right=0x80`
（`Game::poll_pad1` 从 bit0 先读出）。早期文档把 `0x80` 误当成 A 了。

切换机制：按右键后 ROM 把 `$5130` 置 1，整个 CHR 窗口平移
**+256 个 1KB bank**（4KB 页 0-63 简体 → 64-127 繁体）。

两个 bug：

1. `chr_bank_1k` 里写成 `hi2 << 10`（应为 bit 8-9，即 `<< 8`）。页号
   变成 `原值+1024`，取模 512 个 1KB bank 后**绕回同一页** —— 按右键
   只改了 8 个像素，看起来就像功能没了。
2. `$5128-$512B` 被当成"每寄存器 2 位高位字段"，实际是**四个 8 位
   bank 寄存器**（背景用）。ROM 往里面写的是 `104,105,106,107`。

修好后：按右键，8 个 bank 号整体 +256，画面差异 1452 像素，
「塞尔达Ⅱ 林克的冒险」→「塞爾達Ⅱ 林克的冒險」。

## 三、现在的启动链路（已验证走通）

```
$FF4C: SEI / CLD
$FF4E: LDX #$00
$FF50: STX $2000            ; 关 NMI
$FF54: LDA $2002 / BPL      ; 等 vblank（两次）
$FF5C: TXS
$FF5D: LDA #$02 / STA $5100 ; PRG 模式 2
$FF62: LDA #$40 / STA $4017
$FF67: LDA #$01 / STA $5103
$FF6C: LDA #$02 / JMP $FDCB

$FDCB: STA $5102
$FDCE: LDA $01D4 / CMP #$01
$FDD7: STA $5130
$FDD9: LDA #$8E / STA $5116 ; $C000 用 bank 14（$5115 仍是 bank 0）
$FDDF: LDA #$03 / STA $5101 ; CHR 模式 3（1KB）
$FDE4: LDA #$00 / STA $5204 ; 关 IRQ
$FDE9: JSR $FEB0            ; ← 在 $E000-$FFFF，靠 $5117 仍指向 bank 31
$FDEB: LDA #$8E / STA $5115 ; $8000 换 bank 14
$FDF1: LDA #$00 / STA $5113
$FDF6: CLI / JMP $C000      ; 进 bank 14 初始化
```

bank 14 的 `$C000`：

```
$C000: LDA #$00 / STA $2001 / STA $2000
$C008: LDA #$05 / JSR $FFCC ; ← 又在固定顶窗 bank 31
$C00D: JSR $A6A0
$C010: LDA $0736 / CMP #$08 / BEQ $C904
$C018: CMP #$14 / BNE $C010 ; 等状态机
$C01E: LDA $076C / CMP #$01 / BNE $C010
```

实测状态机 `$0736` 正常推进 0→1→2→3，PPU 在 f4 打开（`ctrl=$30`），
f5 起 `ctrl=$B0`，f11 起 `mask=$1E`（背景+精灵都开）。

## 四、怎么继续

```bash
export PATH="/c/Users/ezplayer/.cargo/bin:/c/msys64/ucrt64/bin:$PATH"
export CARGO_TARGET_DIR="D:/z2rs-build"

# 单元测试（不依赖 ROM）
cargo test -j 4 -p z2-core --lib            # 185 条全过
cargo test -j 4 -p z2-ppu  --lib            # 65 条全过
cargo test -j 4 -p z2-core --lib mmc5       # 只跑 MMC5 的 14 条

# 端到端（需要汉化 ROM 在指定路径）
cargo test -j 4 -p z2-native --test mmc5_zh_rom -- --nocapture
```

ROM 路径写在 `crates/z2-native/tests/mmc5_zh_rom.rs` 的 `ROM` 常量里。

### 下一步可做（按价值排序）

1. **进游戏实测**：目前验证到标题、存档菜单、名字输入界面。
   大地图/侧视场景、战斗、对话都还没实机跑过。
2. **HD 材质包页号上限**：`z2-render` 的 `CHR_PAGES` 固定 32（原版
   128 KiB CHR），汉化版有 128 页。页号 ≥32 的素材（大部分中文字形）
   在 `Pack::lookup` 里 `get()` 越界返回 `None`，静默退回原始画面。
   要支持需把 `CHR_PAGES` 提高（`record.page` 是 `u8`，最多 254 页）。
3. **A/B 两组 CHR bank**：`ctrl=$B2`（8x16 精灵）时硬件是「精灵用 A 组、
   背景用 B 组」，实测两组值不同（A=[0,1,2,3,104..107]，
   B=[104,105,106,107]）。当前 PPU 只有 2 个 4KB 槽，两组共用 A 组，
   这对背景来说是错的 —— 可能是某些场景贴图问题的残余来源。
4. **性能**：汉化版走纯解释器，比原版 trap 路径慢很多。

## 五、重要提醒

- **原版 MMC1 路径没动**：`z2-core` 185 条单元测试全过，`z2-ppu` 65 条全过。
- 汉化版走的是**纯解释器**（无 trap），因为汉化版固定 bank 有 51% 字节
  与原版不同，按原版地址硬编码的 Rust trap 全部会跑错代码。
- **不要再给 MMC5 加"ROM 专属补丁"**。这次的教训就是：一个看似必要
  的特例（钉死 `$E000`）掩盖了真正的寄存器分配错误，导致多花了很多时间。
  遇到诡异行为先怀疑自己的实现，别先怀疑 ROM。
- **译制 ROM 上，游戏性增强一律不装**：ZALiA 增强、宽屏游戏性、随机化器、
  双人合作都按原版地址 patch PRG 或装 trap，译制版代码已整体移位，套用
  会破坏游戏。现在由 `refuse_unsupported_features()` 明确提示，不静默失效。
- 已经放弃的路线（不要再试）：从汉化 ROM 提取字库。
  它的中文是 38 页「已排版的连排文本图」，字距不规整、无空列可切分，
  另有 20KB 自定义压缩数据。

