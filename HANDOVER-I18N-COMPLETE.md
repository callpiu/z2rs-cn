# 启动器增强页与随机化页汉化 · 2026-10-23

本轮已修复未完成重构造成的编译错误，并完成启动器“游戏增强”和“ROM 随机化”页的中文接线。

- 增强界面词表 205 条，随机化词表 529 条（含标签、说明、枚举、预设及文件对话框）；支持切回 English。
- 启动器保留自己的 `Lang` API，在共享增强控件边界显式转换，避免循环依赖。
- 随机化选项的值、配置码和底层枚举不变。错误详情按语言切换，并保留字段名和数值供排错。
- `z2-native` 的 O 键菜单仅适配共享 API，仍为既有英文界面；本轮范围为启动器两页。
- 中文版 ROM、HD 包、双人和宽屏相关代码未改动。

验证：核心 957 项、增强与启动器 57 项、汉化 ROM 端到端 25 项、native 单元 198 项，共 **1237 项通过**。
源码审计无漏译、格式占位符无漂移；微软雅黑 TTC 第 0 个 face 的 Unicode cmap 覆盖全部 950 个所需字符。字体排版宽度测试通过。
发布版实际运行指定汉化 ROM，300 帧标题和 800 帧存档菜单均为 0 interpreter faults，存档菜单汉字已目视确认。
桌面截图工具连续两次超时，因此启动器实机窗口的裁切、滚动、提示框布局尚未目视确认。渲染文本检查与字体检查已通过；二者不代替实机布局检查。

构建环境：rustc 1.99.1，x86_64-pc-windows-gnu（本机当前工具链）；PE 导入表仅发现 Windows 系统 DLL，无额外 MinGW/VC 运行库文件。
本机缺少 rustfmt 组件，未执行自动格式化。`git diff --check` 通过。

## 复现

在 `D:\z2rs-060-zh` 运行，按需把 cargo 的 bin 目录加入 PATH：

```powershell
$env:PATH = 'C:\Users\ezplayer\.cargo\bin;' + $env:PATH
$env:CARGO_TARGET_DIR = 'D:\z2rs-build-060'
cargo test -p z2-enh-ui -p z2-launcher
cargo test -p z2-core -p z2-ppu -p z2-render
cargo test -p z2-native --lib
cargo test -p z2-native --test mmc5_zh_rom
cargo build --release -p z2-launcher -p z2-native -p z2-signal
python tools/audit_ui_localization.py --out analysis/ui-localization.json
```

便携目录中的 `z2rs.exe` 直接复制自构建产物 `z2-native.exe`。不附带 ROM、HD 包或系统字体。旧版目录保留。
既有限制：ROM 随机化仍只支持美版 ROM；汉化 ROM 的文本钩子仍不支持中文字符表。

术语：Vanilla=原版，Flags=配置码，Overworld=大地图，Great Palace=大神殿，Downstab/Upstab=下刺/上刺。魔法名称沿用现有增强页；技术字段名、ZALiA 模块标识保留以便定位。
