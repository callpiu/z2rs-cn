# z2rs 中文增强版 · z2rs Chinese Edition

[中文](#中文) · [English](#english) · [Releases](https://github.com/callpiu/z2rs-cn/releases) · [问题反馈](https://github.com/callpiu/z2rs-cn/issues)

> 基于 [z2rs 0.6.0](https://github.com/troyedwardsjr/z2rs) 的中文增强分叉。
> A Chinese-focused enhancement fork of [z2rs 0.6.0](https://github.com/troyedwardsjr/z2rs).

## 中文

本仓库让 z2rs 原生运行采用 **MMC5** 的《塞尔达传说 II：林克的冒险》汉化版 ROM，并保留上游的宽屏、HD 材质包、本地/在线双人、游戏增强、随机化和存档状态等能力。启动器和游戏内设置菜单均提供中文界面。

本仓库不提供游戏 ROM、用户存档或 HD 材质包。请使用自己合法持有的 ROM；ROM 不会由程序上传，也不会提交到仓库。

### 主要改动

- **MMC5 支持**：实现 PRG/CHR 银行切换、nametable 映射、扫描线 IRQ、硬件乘法器和存档状态，修复汉化 ROM 的启动、大地图贴图错乱与暂停菜单字形问题。
- **完整中文界面**：启动器、增强页、随机化页和游戏内 `O` 键设置菜单支持简体中文与英文切换；使用 Windows 系统字体显示中文，不打包字体。
- **HD 材质与中文兼容**：修复 MMC5 CHR 页解析、宽屏边缘和文字窗口覆盖，避免材质包遮挡暂停菜单及对话中的中文字形；优化大包合成路径，减少逐帧开销。
- **宽屏与双人修复**：修正 MMC5 下的 CHR 组选择、地形银行映射、宽屏边距和玩家 2 攻击路径。
- **回归验证**：真实汉化 ROM 覆盖标题、存档菜单、魔法列表、`O` 菜单、大地图和 HD 渲染；最新一轮修复记录为 581 项自动测试通过。详见 [PROJECT-SUMMARY.md](PROJECT-SUMMARY.md) 与各 `HANDOVER-*.md` 文档。

### 下载与使用

1. 打开 [Releases](https://github.com/callpiu/z2rs-cn/releases)，下载最新的 Windows 便携包。
2. 解压，保持 `z2rs.exe`、`z2-launcher.exe`、`z2-signal.exe` 在同一目录。
3. 运行 `z2-launcher.exe`，选择自己的汉化版 `.nes` ROM，然后开始游戏。
4. 启动器中可选择中文或英文界面、宽屏比例、HD 材质包、本地双人和游戏增强选项。

配置、普通存档与即时存档位于 `%APPDATA%\z2rs`。更新程序时保留此目录即可沿用设置和存档。首次运行未签名的 Windows 程序时，SmartScreen 可能显示提示。

### 操作

| 动作 | 玩家 1 | 玩家 2（本地双人） |
|---|---|---|
| A | `Z` | `G` |
| B | `X` | `F` |
| Start | `Enter` | `T` |
| Select | `Shift` | `R` |
| 方向 | 方向键 | `W` `A` `S` `D` |

`O` 打开游戏内设置菜单；`Tab` 快进；`F5` 保存即时存档；`F7` 读取；`F6` 切换存档槽；`P` 暂停；`.` 暂停时单步；`F11` 或 `Alt+Enter` 切换全屏。

### 构建

需要 Rust stable：

```powershell
cargo test -p z2-core -p z2-ppu -p z2-render
cargo test -p z2-native --lib
cargo build --release -p z2-native -p z2-launcher -p z2-signal
```

ROM 集成测试使用环境变量 `Z2_ROM` 指向本机合法 ROM；不设置时相关测试会跳过。

### 已知限制

- 随机化功能仍只支持美版 ROM；汉化版文本钩子也不支持中文字符表。
- HD 材质包、ROM 和存档不包含在发布包中；第三方材质包需自行取得并确认其许可。
- 汉化版兼容性已覆盖标题、存档菜单、大地图、魔法菜单和 `O` 菜单等关键路径，但未覆盖完整游戏流程及所有系统/字体环境。

### 来源与许可

上游项目为 [troyedwardsjr/z2rs](https://github.com/troyedwardsjr/z2rs)，源代码沿用 [MIT License](LICENSE)。上游的 [PROVENANCE.md](PROVENANCE.md) 与 [LEGAL.md](LEGAL.md) 同样适用。本仓库不代表任天堂或 ROM 汉化作者的官方发行。

---

## English

This fork adds native support for the **MMC5 Chinese-localized** ROM of *Zelda II: The Adventure of Link* while preserving upstream z2rs features: widescreen, HD graphics packs, local and online co-op, enhancements, the randomizer, save states, and gamepad support. Both the launcher and the in-game options overlay are available in Chinese and English.

No game ROM, save file, or HD pack is distributed here. Use a ROM you legally own. The program does not upload the ROM, and the repository does not accept ROMs.

### Changes in this fork

- **MMC5 support:** PRG/CHR bank switching, nametable mapping, scanline IRQs, hardware multiplication, and mapper state in snapshots. Fixes startup, overworld tile corruption, and missing spell-name glyphs in the localized ROM.
- **Complete Chinese UI:** Chinese and English for the launcher, enhancement and randomizer pages, and the in-game `O` options menu. Chinese text uses a system font supplied by Windows; no font is bundled.
- **HD-pack compatibility:** Correct MMC5 CHR-page handling and protect text windows from full-screen layers. The compositor also reuses resolved columns and batches pixel copies for lower per-frame overhead.
- **Widescreen and co-op fixes:** Correct MMC5 CHR-group selection, terrain-bank mapping, margin rendering, and Player 2 attack handling.
- **Regression coverage:** Real localized-ROM tests cover the title screen, save menu, spell list, `O` menu, overworld, and HD rendering. The latest menu-fix pass reports 581 automated tests passing; see [PROJECT-SUMMARY.md](PROJECT-SUMMARY.md) and the `HANDOVER-*.md` files.

### Download and run

1. Open [Releases](https://github.com/callpiu/z2rs-cn/releases) and download the latest Windows portable archive.
2. Extract it, keeping `z2rs.exe`, `z2-launcher.exe`, and `z2-signal.exe` together.
3. Run `z2-launcher.exe`, choose your own localized `.nes` ROM, and start the game.
4. Configure language, widescreen, HD pack, local co-op, and enhancements in the launcher.

Settings and saves live under `%APPDATA%\z2rs`; keep that directory when updating the program. Windows SmartScreen may warn because the executables are not code signed.

### Controls

| Action | Player 1 | Player 2 (local co-op) |
|---|---|---|
| A | `Z` | `G` |
| B | `X` | `F` |
| Start | `Enter` | `T` |
| Select | `Shift` | `R` |
| D-pad | arrow keys | `W` `A` `S` `D` |

`O` opens the in-game options menu. `Tab` fast-forwards, `F5` saves a state, `F7` loads one, `F6` cycles slots, `P` pauses, `.` advances one frame while paused, and `F11` or `Alt+Enter` toggles fullscreen.

### Build

Rust stable is required:

```powershell
cargo test -p z2-core -p z2-ppu -p z2-render
cargo test -p z2-native --lib
cargo build --release -p z2-native -p z2-launcher -p z2-signal
```

ROM integration tests read a legally obtained ROM from the `Z2_ROM` environment variable and skip when it is unset.

### Known limitations

- The randomizer still supports the USA ROM only, and localized-ROM text hooks do not support its Chinese character table.
- ROMs, saves, and HD packs are not included. Supply your own HD pack and verify its license.
- Compatibility testing covers key localized-ROM paths but not a complete playthrough or every OS/font configuration.

### Upstream and license

The upstream project is [troyedwardsjr/z2rs](https://github.com/troyedwardsjr/z2rs). The source code remains under the [MIT License](LICENSE); [PROVENANCE.md](PROVENANCE.md) and [LEGAL.md](LEGAL.md) also apply. This fork is not an official release by Nintendo or the ROM localization's authors.
