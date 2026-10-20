# MMC5 移植 · 进度与交接

> 目标：让 z2rs 直接运行汉化版 ROM
> `Zelda II - The Adventure of Link (U) [20260806_NOKOH].nes`
> （mapper 5，PRG 256KB，CHR 512KB）
>
> 状态：**ROM 能复位、能进入初始化代码，但还没画出第一帧**
> 最后更新：2026-10-21 01:32

## 一、已完成

### 1. MMC5 mapper（`crates/z2-core/src/mmc5.rs`，11 条测试全过）

| 功能 | 状态 |
|---|---|
| PRG 四种模式（32K / 16+16 / 16+8+8 / 8x4） | 已实现 |
| CHR 四种模式（8K / 4K / 2K / 1K）+ `$5128-$512B` 高位 | 已实现 |
| `$5130` CHR 高位 | 已实现 |
| `$5105` nametable 映射（解码成 source） | 已实现 |
| ExRAM `$5C00-$5FFF` / `$5104` 模式位 | 已实现（仅 RAM 读写） |
| 扫描线 IRQ `$5203`/`$5204` | 已实现（`on_scanline`） |
| 硬件乘法器 `$5205`/`$5206` | 已实现 |
| PRG-RAM bank `$5113`、保护位 `$5102/$5103` | 已实现 |
| **ExRAM 扩展属性（逐格调色板）** | **未实现**（只影响颜色，不影响流程） |
| 填充模式 `$5106/$5107` | 只存值，渲染未用 |

### 2. 接入 Game

- `MapperKind::{Mmc1, Mmc5}` 枚举
- `Game::from_ines` 接受 mapper 1 和 5，其余仍报 `WrongMapper`
- PRG 读、mapper 寄存器读写、PPU 的 CHR 同步，全部按 `game.mapper` 分派
- `capture_frame` 的 frame-end CHR 同步也按 mapper 分派

### 3. 通用加载路径（`crates/z2-native/src/app.rs`）

`emu_from_generic_ines()`：不装 trap、不打补丁，整个 ROM 交给 6502 解释器。
`emu_from_rom_file_with()` 检测到 mapper 5 时自动走这条路。

## 二、踩过的三个真 bug（都已修）

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

修法：`Mmc5::init_fix_last()`，上电固定 `$8000`=bank 0、`$E000`=最后一个。

### ③ `$E000-$FFFF` 必须固定为最后一个 bank

这不是教科书 MMC5（那里 `$5116` 驱动 `$E000`），但这个 ROM 需要。
证据：

- `STA $5117` 在 256KB PRG 里**一次都没出现** → 只用 PRG 模式 2
- 启动代码 `$FDCB` 执行 `STA $5116` 之后**仍在 `$E000-$FFFF` 继续跑**
  （`$FDDF`），若真的换了 `$E000` 的 bank，下一条取指就落到 bank 14
  的数据区（`00 00 …`）
- 它写的 `$8E`（=142，取低 5 位得 14）指向的 bank 14，其 `$C000`
  确实是初始化代码（`LDA #0 / STA $2001 / STA $2000 / LDA #5 / JSR $FFCC`）

修法：`map_prg` 里 `addr >= 0xE000` 直接返回最后一个 bank。

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
$FDDA: LDA #$8E / STA $5116 ; $C000 用 bank 14
$FDDF: LDA #$03 / STA $5101 ; CHR 模式 3（1KB）
$FDE4: LDA #$00 / STA $5204 ; 关 IRQ
$FDE9: JSR $FEB0
$FDEC: LDA #$8E / STA $5115
$FDF1: LDA #$00 / STA $5113
$FDF6: CLI / JMP $C000      ; 进 bank 14 初始化
```

bank 14 的 `$C000`：

```
$C000: LDA #$00 / STA $2001 / STA $2000
$C008: LDA #$05 / JSR $FFCC
$C00D: JSR $A6A0
$C010: LDA $0736 / CMP #$08 / BEQ $C904
$C018: CMP #$14 / BNE $C010   ; ← 当前卡在这个等待循环
$C01E: LDA $076C / CMP #$01 / BNE $C010
```

## 四、当前卡点

诊断输出（`mmc5_zh_rom.rs` 的 `diagnose_main_loop`）：

```
f 3 PC=$C024 $0736=00 $076C=00 ctrl=$00 mask=$00
f 4 PC=$C025 ... （每帧只前进 1 字节，本帧周期 = 0）
```

两个现象：

1. **`ctrl=$00 mask=$00`** —— PPU 从未被启用
2. **每帧周期 = 0** —— 解释器在某处 `ExecError` 后冻结
   （`run_budget` 遇错立即 return false）

`$0736`/`$076C` 是游戏状态机变量，应该由中断服务程序更新，但没更新。

### 下一步排查建议

1. **先看 `exec_errors` / `last_exec_error`**：确认是否真的报错，
   报在哪个地址。如果是 `IllegalOpcode`，对照 6502 指令表找漏掉的 opcode。
2. **单步跟踪 `$C000..$C024`**：确认 `JSR $FFCC`、`JSR $A6A0`、
   `JSR $C4CB` 都正确返回。这几个子程序在固定 bank，需要确认
   `$FFCC` 的实现在 MMC5 下也正常（它可能又写 MMC5 寄存器）。
3. **检查 `$5204` 读取**：IRQ 状态寄存器读会清 pending，
   如果 ROM 依赖这个行为判断扫描线，返回值不对会走错分支。
4. **确认 `$0736` 的写入者**：在 ROM 里搜 `STA $0736`，
   看是主循环自己写还是 NMI/IRQ 写。
5. **PPU 为何没启用**：`$C000` 明明写了 `LDA #0 / STA $2001`
   （关渲染），但后续应该再打开。看 `$FFCC` 做什么。

## 五、怎么继续

```bash
export PATH="/c/Users/ezplayer/.cargo/bin:/c/msys64/ucrt64/bin:$PATH"
export CARGO_TARGET_DIR="D:/z2rs-build"

# 单元测试（不依赖 ROM）
cargo test -j 4 -p z2-core --lib mmc5

# 端到端诊断（需要汉化 ROM 在指定路径）
cargo test -j 4 -p z2-native --test mmc5_zh_rom -- --nocapture
```

ROM 路径写在 `crates/z2-native/tests/mmc5_zh_rom.rs` 的 `ROM` 常量里。

## 六、重要提醒

- **原版 MMC1 路径没动**：`z2-core` 的 182 条单元测试全过
- 汉化版走的是**纯解释器**（无 trap），比原版路径慢很多，
  但这是唯一正确的做法 —— 汉化版固定 bank 有 51% 字节与原版不同，
  按原版地址硬编码的 Rust trap 全部会跑错代码
- 已经放弃的路线（不要再试）：从汉化 ROM 提取字库。
  它的中文是 38 页「已排版的连排文本图」，字距不规整、无空列可切分，
  另有 20KB 自定义压缩数据
