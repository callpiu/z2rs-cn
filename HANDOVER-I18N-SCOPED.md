> 历史记录：本文件描述的编译问题已于 2026-10-23 修复，当前状态见 [HANDOVER-I18N-COMPLETE.md](HANDOVER-I18N-COMPLETE.md)。下文保留原始接手现场。

# 交接 · 增强页与随机化页汉化（未完成）

> 记录时间：2026-10-23 19:35
> 工作目录：`D:\z2rs-060-zh`
> **状态：代码处于重构中途，编译失败。** 先说这一条，免得接手时困惑。

---

## 一、现在的状态（重要）

### 编译状态

| 目标 | 状态 |
|---|---|
| `cargo build -p z2-enh-ui` | ✅ 通过 |
| `cargo test -p z2-enh-ui` | ❌ 2 个错误（`Opts` 初始化缺 `lang`） |
| `cargo build -p z2-launcher` | ❌ **98 个错误**（`Lang::t` / `label` / `ALL` 找不到） |
| `cargo build -p z2-native` | 未测（overlay 也用 enh-ui，很可能也有错） |

**工作树是脏的**，这些改动都还没提交：

```
 M Cargo.lock
 M crates/z2-enh-ui/Cargo.toml
 M crates/z2-enh-ui/src/lib.rs
 M crates/z2-launcher/src/enh_ui.rs
 M crates/z2-launcher/src/i18n.rs
?? crates/z2-enh-ui/src/i18n.rs
?? crates/z2-enh-ui/tests/
```

### 最后一个可用状态

提交 **`0b489d7`**（"启动器加载系统中文字体，修复界面中文显示为方块"）。
这是**已发布、可运行**的版本，对应 `D:\z2rs-中文版-0.6.0\`：

```
z2rs.exe         59,616,796  46162ea3aea8
z2-launcher.exe  25,236,891  e3c82921ab0b
z2-signal.exe     5,263,495  c3eb613d360d
zip              29,941,152  f20816c8a5f1
```

那个版本里：字体正常、启动器主界面（开始游戏页）已汉化，
**但"游戏增强"和"ROM 随机化"两个页签仍是英文** —— 这正是本轮要解决的。

### 想立刻恢复可用状态

```powershell
cd D:\z2rs-060-zh
git checkout -- Cargo.lock crates/z2-enh-ui/Cargo.toml crates/z2-enh-ui/src/lib.rs `
                crates/z2-launcher/src/enh_ui.rs crates/z2-launcher/src/i18n.rs
Remove-Item -Recurse -Force crates/z2-enh-ui/src/i18n.rs, crates/z2-enh-ui/tests
```

注意：`crates/z2-enh-ui/src/i18n.rs` 里有 148 条**已经写好的译文**，
删之前先备份 —— 那是本轮的主要成果。

---

## 二、本轮做成了什么

### 1. `z2-enh-ui/src/i18n.rs`（新文件，已完成）

148 条 `(英文, 简体中文)` 对照，覆盖"游戏增强"页签全部文案：

* 道具/魔法名（盾、跳跃、生命、妖精、火焰、反射、魔法、雷电…）
* 分组标题（文本与 HUD、体验优化、引擎修复、敌人与首领、能力、
  随机化与初始状态、作弊、新增区域、画面特效、音频、开发者工具）
* 每个选项的标签与悬停提示（含 ZALiA 来源说明）

术语沿用了**旧版汉化**（`D:\下载\Compressed\main\z2rs-main` 的
`crates/z2-enh-ui/src/lib.rs`，那里有 173 条历史译文），保证前后一致。

同时提供：

```rust
pub enum Lang { En, Zh }        // 默认 En，serde 可序列化
pub fn Lang::tr(self, english: &str) -> &str   // 查不到返回原文
pub fn translated_keys() -> &'static [(&'static str, &'static str)]
```

5 项自测：译文与原文不同、都含汉字、无重复键、英文透传、未知键回退。

### 2. `z2-enh-ui/src/lib.rs` 的接线（已完成，但测试未过）

* `Opts` 新增 `lang: Lang` 字段（默认 `En`）
* 6 个叶子函数加 `lang` 参数并在内部查表：
  `hint` / `check` / `slider_u8` / `slider_pct` / `slider_f32` / `bit_boxes`
* `group()` 加 `lang` 参数（组标题走这里，11 处调用点已批量补 `o.lang`）
* `intro()`、`preset_buttons()` 加 `lang` 参数
* `enhancements_section()` 的 `ui.heading(...)` 改查表
* 7 处 `ui.label(...)` / `CollapsingHeader::new(...)` 改显式 `o.lang.tr(...)`

**调用点是用脚本批量改的**（单行 + 跨行两种形式），共 66 + 11 处。

### 3. 渲染验证测试（已写好，曾通过）

`crates/z2-enh-ui/tests/i18n_render.rs`：开启 egui 的 accesskit，
收集一帧里**实际画出的所有文本**，断言中文模式下没有残留英文。

它在我改 `group()` **之前**跑通过：

```
中文模式: 共 81 条文本，其中含汉字 81 条
仍为英文的 0 条
英文模式: 共 81 条，含汉字 0 条
test result: ok. 2 passed
```

这是本轮最有价值的验证 —— 它不依赖"代码看起来改对了"，
而是直接检查渲染输出。

**注意**：这个数字（81 条）是 `Group::Text` 单组展开的量，
不是全部 148 条，因为 `enhancements_section` 只有标题/说明常开。

### 4. `Cargo.toml`

`z2-enh-ui` 加了 `serde = { version = "1", features = ["derive"] }`
（`Lang` 要能被启动器的 settings 序列化）。

---

## 三、剩下的错误与怎么修

### 错误 1：`z2-enh-ui` 测试（2 个）

`lib.rs` 的 `#[cfg(test)]` 里有两处 `Opts { ... }` 字面量缺 `lang` 字段。

修法：给这两处补 `lang: Lang::En,`。

### 错误 2：`z2-launcher`（98 个，当前阻塞点）

我把 `crates/z2-launcher/src/i18n.rs` 里的 `Lang` 枚举**改成
`pub use z2_enh_ui::Lang;`**（想让两边共用一个类型），
但 `Lang::t()`、`Lang::label()`、`Lang::ALL` 这三个方法/常量
**只在我删掉的那份定义里有**，`z2_enh_ui::Lang` 没有。

于是 `app.rs` / `ui.rs` / `settings.rs` 里所有 `lang.t(...)` 全部报错。

**两条修法，推荐第一条：**

**A. 恢复 launcher 自己的 `Lang` 枚举**（最小改动，回到能编译）

把 `crates/z2-launcher/src/i18n.rs` 里的 `pub use z2_enh_ui::Lang;`
换回原来的定义（`git show 0b489d7:crates/z2-launcher/src/i18n.rs`
能拿到完整原文），然后在 `enh_ui.rs` 里做一次类型转换：

```rust
fn to_enh(l: crate::i18n::Lang) -> z2_enh_ui::Lang {
    match l {
        crate::i18n::Lang::En => z2_enh_ui::Lang::En,
        crate::i18n::Lang::Zh => z2_enh_ui::Lang::Zh,
    }
}
```

**B. 给 `z2_enh_ui::Lang` 补上 `t` / `label` / `ALL`**

但 `t()` 要查 launcher 自己的两张表，逻辑上应该留在 launcher 里，
放进去会造成循环依赖的观感问题。**不推荐。**

### 错误 3：`z2-native` 的 overlay

`crates/z2-native/src/overlay.rs` 调用 `z2_enh_ui::intro(ui, &m.enhancements)`
和 `preset_buttons(...)`，这两个函数的签名本轮加了 `lang` 参数，
所以 overlay 那边必然要跟着改。还没动。

---

## 四、还没开始的部分

### 1. 随机化页签（`crates/z2-launcher/src/rando_ui/`）

13 个文件，**约 190 处英文**，一行都没改。

结构：每个文件是一组控件（`overworld.rs` 8912 字节最大，`towns.rs` 841 最小），
文字都是内联字面量，例如 `"West size"` / `"West biome"` / `"Vanilla"`。

`z2-rando/src/flags.rs` 第 186-208 行有个宏统一生成 `label()`，
那里是集中改的好位置；但 `rando_ui` 里的内联字面量要另外处理。

### 2. 启动器「游戏增强」页签的接线

`crates/z2-launcher/src/enh_ui.rs` 已经改成传 `s.language.into()`，
但因为错误 2，这行现在编译不过。修完错误 2 后要确认它真的传对了。

### 3. 重新验证 + 发布

修完编译后必须重新跑：

```
cargo test -p z2-enh-ui        （含 i18n_render 的 81/81 断言）
cargo test -p z2-launcher      （40 项）
cargo test -p z2-core -p z2-ppu -p z2-render   （957 项）
cargo test -p z2-native --test mmc5_zh_rom      （25 项）
```

然后重新构建三件套、更新 `SHA256SUMS`、重打包 zip。

---

## 五、踩过的坑（本轮新记录的）

* **egui 的 `Context::run` 在 0.36 叫 `run_ui`**，闭包参数是 `ui` 不是 `ctx`。
* **`run_ui` 返回的 `FullOutput` 带未应用的 `textures_delta`**，
  直接 drop 会触发 epaint 断言
  （`Dropped TexturesDelta with 1 unapplied deltas`）。
  headless 测试里必须手动 `out.textures_delta.clear()`。
* **`accesskit` 的开关是 `ctx.enable_accesskit()`**（不是
  `enable_accessibility(Role, bool)`），结果在
  `platform_output.accesskit_update: Option<TreeUpdate>`，
  节点在 `update.nodes`（`Vec<(NodeId, Node)>`）。
* **批量改调用点时，单行和跨行两种形式都要处理**：
  `check(ui, ...)` 和 `check(\n    ui,\n    ...)` 是两种写法，
  只处理一种会漏掉大半（我第一次只命中 5 处，补上跨行后变 66 处）。
* **PowerShell 的 `"$var.method()"` 会被当子表达式解析**，
  拼路径时用 `-f` 格式化或先赋值给变量。

---

## 六、一句话总结

增强页的 **148 条译文和接线代码都写完了，
且用渲染测试验证过 81/81 全中文**；
但我在最后一步想让启动器和 enh-ui 共用 `Lang` 类型时
把 launcher 的枚举删了，导致 **98 个编译错误**。

接手第一件事：按第三节「错误 2 · 修法 A」恢复 launcher 的 `Lang`，
就能回到可编译状态，然后跑测试确认 81/81 仍然成立。
之后再做随机化页的 190 条。
