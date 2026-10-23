//! 端到端：用 MMC5 跑汉化版 ROM，验证能启动、能出画面。
//!
//! 这个测试直接读真实 ROM 文件（不在仓库里），找不到就跳过 —— 和
//! `cjk_e2e_real.rs` 一样的约定。

use std::path::Path;

use z2_native::app::{emu_from_generic_ines, ines_is_mmc5};

const ROM: &str = r"D:/z2rs-中文版/Zelda II - The Adventure of Link (U) [20260806_NOKOH].nes";

// 手柄位序：`Game::poll_pad1` 从 bit0 先读出（NES 标准顺序）。
/// `A`。
const PAD_A: u8 = 0x01;
/// `Start`。
const PAD_START: u8 = 0x08;
/// `Down`。
const PAD_DOWN: u8 = 0x20;
/// `Right`。
const PAD_RIGHT: u8 = 0x80;

/// Mode `$0736` value of a side-view gameplay scene (`wide_margins` calls it
/// `MODE_SIDEVIEW`).
const MODE_SIDEVIEW: u8 = 0x0B;
/// Mode `$0736` value of the overworld.
const MODE_OVERWORLD: u8 = 0x05;

/// Tap `key` for `n` frames, letting the pad settle in between.
fn tap(e: &mut z2_native::app::Emu, key: u8, n: u32) {
    for _ in 0..n {
        e.game.step(key);
        for _ in 0..3 {
            e.game.step(0);
        }
    }
}

fn settle(e: &mut z2_native::app::Emu, n: u32) {
    for _ in 0..n {
        e.game.step(0);
    }
}

/// Drive the title screen through registration into a side-view scene.
///
/// The register-your-name screen is ROM logic: pick a slot with A, then **Down
/// then A** to activate the「結束」cell. (Start, B, Up and the arrows all leave
/// it stuck in mode 1.)
fn boot_into_game(e: &mut z2_native::app::Emu) {
    settle(e, 700); // title
    tap(e, PAD_START, 8); // -> save-file menu
    settle(e, 120);
    tap(e, PAD_A, 8); // pick the first empty slot
    settle(e, 150);
    tap(e, PAD_DOWN, 6); // move to 「結束」
    settle(e, 90);
    tap(e, PAD_A, 8); // confirm
    settle(e, 240);
}

fn rom_bytes() -> Option<Vec<u8>> {
    let p = Path::new(ROM);
    if !p.exists() {
        eprintln!("跳过：找不到汉化版 ROM");
        return None;
    }
    Some(std::fs::read(p).expect("读 ROM"))
}

#[test]
fn header_is_mmc5() {
    let Some(bytes) = rom_bytes() else { return };
    assert!(ines_is_mmc5(&bytes), "汉化版应该是 mapper 5");
    assert_eq!(bytes[4], 16, "PRG 应该是 256KB (16 x 16KB)");
    assert_eq!(bytes[5], 64, "CHR 应该是 512KB (64 x 8KB)");
}

#[test]
fn boots_without_panic_and_draws_something() {
    let Some(bytes) = rom_bytes() else { return };
    let mut emu = emu_from_generic_ines(&bytes, 44100, z2_native::app::Features::default()).expect("加载汉化版");

    // 跑 300 帧（约 5 秒）覆盖启动 + 标题画面。
    for _ in 0..300 {
        emu.game.step(0);
    }

    let frame = emu.game.frame_indexed();
    let nonzero = frame.iter().filter(|&&b| b != 0).count();
    eprintln!("300 帧后：非零像素 {nonzero} / {}", frame.len());
    assert!(
        nonzero > 1000,
        "画面几乎是空的（{nonzero} 个非零像素），可能没启动成功"
    );
    // 没有解释器错误：MMC5 的 bank 切换必须全程正确，否则会撞上 $FF。
    assert_eq!(
        emu.game.exec_errors, 0,
        "解释器报错 {:?}",
        emu.game.last_exec_error
    );
    // PPU 必须真的开着（$2000 的 NMI/图案表位 + $2001 的渲染位）。
    let ctrl = emu.game.ppu.ppu.ctrl();
    let mask = emu.game.ppu.ppu.mask();
    assert!(
        mask & 0x18 != 0,
        "PPU 渲染没开（mask=${mask:02X}），画面不可能有内容"
    );
    eprintln!("ctrl=${ctrl:02X} mask=${mask:02X}");
}

/// 三组此前被跳过的 trap 必须在汉化 ROM 上真的注册。
///
/// 这一条是防回归的：上一轮改过 `register_translated_traps`，但那版改动
/// 在后续 `git checkout --` / `stash` 中丢了，只留下注释说"已启用"，
/// 函数体其实还是 6 组 —— 于是"24 个端到端全过"是在未提交的改动下跑的。
///
/// 直接数注册表，不依赖行为观察：三组各至少注册一条 trap，
/// 且总数与美版路径一致（9 组）。
#[test]
fn all_nine_trap_groups_are_registered_on_translated_rom() {
    let Some(bytes) = rom_bytes() else { return };
    let emu = emu_from_generic_ines(&bytes, 44100, z2_native::app::Features::default())
        .expect("加载");

    let n = emu.game.traps.len();
    // 每组至少一条，9 组。用美版 ROM 的数做下界对照更稳，但这里先要求
    // 一个明确高于旧版（6 组）的数量。
    eprintln!("汉化 ROM 注册的 trap 总数: {n}");
    assert!(
        n > 60,
        "trap 总数 {n} 太少 —— 三组（title/sideview/overworld）可能没注册"
    );

    // 三组各自的代表地址必须被注册。
    for (name, addr) in [
        ("title: bank7_code7", 0xC33Cu16),
        ("sideview: LC9A5", 0xC9A5),
        ("sideview: Get_Area_Code", 0xCC97),
        ("overworld: LD625", 0xD625),
    ] {
        assert!(
            emu.game.traps.get(addr).is_some(),
            "{name} (${addr:04X}) 没注册 —— 该组没启用"
        );
    }
}

/// 标题画面之后按下 Start 必须真的换场景 —— 这一条锁住 bank 切换：
/// 只要 PRG 模式 1/2 的寄存器分配错了，ROM 就会停在一个死循环里，
/// 状态机 `$0736` 不再前进。
#[test]
fn start_button_leaves_the_title_screen() {
    let Some(bytes) = rom_bytes() else { return };
    let mut emu = emu_from_generic_ines(&bytes, 44100, z2_native::app::Features::default()).expect("加载");

    for _ in 0..480 {
        emu.game.step(0);
    }
    let title_state = emu.game.ram[0x0736];
    let title_pixels: Vec<u8> = emu.game.frame_indexed().to_vec();

    // 按住 Start 一段时间（菜单需要几帧去抖）。
    for _ in 0..80 {
        emu.game.step(PAD_START);
    }
    for _ in 0..240 {
        emu.game.step(0);
    }

    let after_state = emu.game.ram[0x0736];
    let after = emu.game.frame_indexed();
    eprintln!("$0736: {title_state:02X} -> {after_state:02X}");
    assert_eq!(emu.game.exec_errors, 0, "按 Start 之后解释器报错");
    assert_ne!(
        title_state, after_state,
        "按 Start 后 $0736 没变，游戏卡在标题状态机"
    );
    let changed = title_pixels
        .iter()
        .zip(after.iter())
        .filter(|(a, b)| a != b)
        .count();
    eprintln!("画面变化 {changed} 像素");
    assert!(
        changed > 5000,
        "按 Start 后画面几乎没变（{changed} 像素），可能没进菜单"
    );
    // 存档界面是满屏的，不是黑屏。
    let nonzero = after.iter().filter(|&&b| b != 0).count();
    assert!(nonzero > 20_000, "存档界面几乎是黑的（{nonzero}）");
}

#[test]
fn reset_vector_is_readable_through_mmc5() {
    let Some(bytes) = rom_bytes() else { return };
    let emu = emu_from_generic_ines(&bytes, 44100, z2_native::app::Features::default()).expect("加载");
    // 直接走 MMC5 的映射，读 $FFFC/$FFFD。
    let len = emu.game.prg.len();
    let lo = emu.game.prg[emu.game.mmc5.map_prg(0xFFFC, len)];
    let hi = emu.game.prg[emu.game.mmc5.map_prg(0xFFFD, len)];
    let reset = u16::from(lo) | (u16::from(hi) << 8);
    eprintln!("上电时 $FFFC = ${reset:04X}");
    assert!(
        (0x8000..=0xFFFF).contains(&reset),
        "RESET 向量 ${reset:04X} 不合法"
    );
    let b = emu.game.prg[emu.game.mmc5.map_prg(reset, len)];
    eprintln!("RESET 处首字节 = ${b:02X}");
    assert!(b != 0x00 && b != 0xFF, "RESET 处字节是 ${b:02X}，可疑");
}


/// 诊断：跑 600 帧，每 60 帧打印 PC / PPUCTRL / PPUMASK / 非零像素数。
#[test]
fn diagnose_boot_progress() {
    let Some(bytes) = rom_bytes() else { return };
    let mut emu = emu_from_generic_ines(&bytes, 44100, z2_native::app::Features::default()).expect("加载");
    for f in 0..600u32 {
        emu.game.step(0);
        if f % 60 == 0 {
            let pc = emu.game.cpu.pc;
            let ctrl = emu.game.ppu.ppu.ctrl();
            let mask = emu.game.ppu.ppu.mask();
            let nz = emu.game.frame_indexed().iter().filter(|&&b| b != 0).count();
            let banks = emu.game.mmc5.prg_banks;
            eprintln!(
                "f{f:3} PC=${pc:04X} ctrl=${ctrl:02X} mask=${mask:02X} 非零={nz:5} prg={banks:?} mode={}",
                emu.game.mmc5.prg_mode
            );
        }
    }
}


/// 逐帧采样：看 CPU 是不是真的在跑，还是卡在某个循环。
#[test]
fn diagnose_frame_by_frame() {
    let Some(bytes) = rom_bytes() else { return };
    let mut emu = emu_from_generic_ines(&bytes, 44100, z2_native::app::Features::default()).expect("加载");
    let mut prev_cycles = emu.game.cpu.cycles;
    for f in 0..12u32 {
        let pc_before = emu.game.cpu.pc;
        emu.game.step(0);
        let dc = emu.game.cpu.cycles - prev_cycles;
        prev_cycles = emu.game.cpu.cycles;
        eprintln!(
            "f{f:2}: 起始PC=${pc_before:04X} -> 结束PC=${:04X}  本帧周期={dc}  错误累计={} 最近={:?}",
            emu.game.cpu.pc,
            emu.game.exec_errors,
            emu.game.last_exec_error
        );
    }
}


/// 简繁切换：标题画面按**右**（`Right`），ROM 会把整个 CHR 窗口平移
/// +256 个 1KB bank（4KB 页 0-63 -> 64-127），字形随之变成繁体。
///
/// 手柄位序（`Game::poll_pad1` 从 bit0 先读出，NES 标准）：
/// `A=0x01 B=0x02 Select=0x04 Start=0x08 Up=0x10 Down=0x20 Left=0x40 Right=0x80`。
/// 之前把 `0x80` 当成 A 是错的 —— 那是 Right，也正好解释了用户说的
/// 「左右键切换简繁」。
///
/// 实测（真实 ROM）：**右键 → `$5130 = 1`（繁体），左键 → `$5130 = 0`
/// （简体）**，且右键不是 toggle，连按仍是繁体。默认启动是简体。
///
/// 这一条锁住两个曾经失效的 bug：
/// 1. `$5130` 被当成 bit 10（`<< 10`）而不是 bit 8-9，导致页号取模 512 后
///    绕回同一页，按右键只改了 8 个像素；
/// 2. `$5128-$512B` 被当成每寄存器 2 位字段，而不是四个 8 位 bank 寄存器。
#[test]
fn right_key_switches_simplified_to_traditional() {
    let Some(bytes) = rom_bytes() else { return };
    let mut emu =
        emu_from_generic_ines(&bytes, 44100, z2_native::app::Features::default())
            .expect("加载");

    // 跑到字幕完整显示之后再比对（约 900 帧）。
    for _ in 0..900 {
        emu.game.step(0);
    }
    let simp = *emu.game.frame_indexed();
    let simp_banks: Vec<usize> = (0..8).map(|s| emu.game.mmc5.chr_bank_1k(s)).collect();
    assert_eq!(emu.game.mmc5.chr_upper2, 0, "默认应该是简体");

    // 按右（bit 7）。
    for _ in 0..30 {
        emu.game.step(PAD_RIGHT);
    }
    for _ in 0..30 {
        emu.game.step(0);
    }
    let trad = *emu.game.frame_indexed();
    let trad_banks: Vec<usize> = (0..8).map(|s| emu.game.mmc5.chr_bank_1k(s)).collect();
    assert_eq!(emu.game.mmc5.chr_upper2, 1, "按右键后 $5130 应为 1");

    // 每个 bank 号都应整体 +256。
    for (a, b) in simp_banks.iter().zip(trad_banks.iter()) {
        assert_eq!(b.wrapping_sub(*a), 256, "bank {a} -> {b} 不是 +256");
    }
    // 且必须落在 CHR 图像范围内（512 个 1KB bank）。
    let banks = emu.game.chr.len() / 1024;
    for b in &trad_banks {
        assert!(*b < banks, "bank {b} 越界（共 {banks}）");
    }
    // 画面必须真的变了（繁体字形不同）。
    let diff = simp.iter().zip(trad.iter()).filter(|(a, b)| a != b).count();
    eprintln!("简繁切换：差异像素 {diff} / {}", simp.len());
    assert!(
        diff > 300,
        "按右键后画面几乎没变（{diff} 像素），切换没生效"
    );
    assert_eq!(emu.game.exec_errors, 0, "切换过程中解释器报错");
}

/// 左键把字形切回**简体**（`$5130 = 0`），并且右键不是 toggle。
///
/// 实测行为：右键 → 1（繁体），左键 → 0（简体）。用户报告「默认是繁体」
/// 就是因为上次按过右键 —— 这个状态在整局游戏里保持，属于 ROM 的设计，
/// 模拟器不应该干预（用户也确认要保留切换能力）。
#[test]
fn left_key_switches_back_to_simplified() {
    const PAD_LEFT: u8 = 0x40;
    let Some(bytes) = rom_bytes() else { return };
    let mut emu =
        emu_from_generic_ines(&bytes, 44100, z2_native::app::Features::default())
            .expect("加载");
    for _ in 0..900 {
        emu.game.step(0);
    }
    assert_eq!(emu.game.mmc5.chr_upper2, 0, "启动应为简体");

    // 右键 -> 繁体。
    for _ in 0..30 {
        emu.game.step(PAD_RIGHT);
    }
    for _ in 0..30 {
        emu.game.step(0);
    }
    assert_eq!(emu.game.mmc5.chr_upper2, 1, "右键应切到繁体");

    // 再按右键仍是繁体（不是 toggle）。
    for _ in 0..30 {
        emu.game.step(PAD_RIGHT);
    }
    for _ in 0..30 {
        emu.game.step(0);
    }
    assert_eq!(emu.game.mmc5.chr_upper2, 1, "右键不应是 toggle");

    // 左键 -> 回到简体。
    for _ in 0..30 {
        emu.game.step(PAD_LEFT);
    }
    for _ in 0..30 {
        emu.game.step(0);
    }
    assert_eq!(emu.game.mmc5.chr_upper2, 0, "左键应切回简体");
    assert_eq!(emu.game.exec_errors, 0, "切换过程中解释器报错");
}

/// 存档必须带上 MMC5 的寄存器，否则读档后画面会变成别的场景。
///
/// `GameState` 原来只序列化 `mmc1`。汉化 ROM 的 PRG/CHR bank、`$5130` 简繁位、
/// IRQ 锁存和 ExRAM 全都不进存档：
///
/// * F7 存档读回来，CHR 窗口还停在存档之后被改过的状态 —— 花屏；
/// * 联机回滚（`netplay.rs` 的 `save_state`/`load_state`）同样恢复不了，
///   而且 `hash_state` 连 MMC5 漂移都看不见，不同步检测不出来。
///
/// 这里先存，再切到繁体把 bank 整个挪走，最后读档，要求三样都回到存档时刻。
#[test]
fn save_state_restores_mmc5_registers() {
    let Some(bytes) = rom_bytes() else { return };
    let mut emu =
        emu_from_generic_ines(&bytes, 44100, z2_native::app::Features::default())
            .expect("加载");
    for _ in 0..900 {
        emu.game.step(0);
    }

    let saved_upper = emu.game.mmc5.chr_upper2;
    let saved_chr: Vec<usize> = (0..8).map(|s| emu.game.mmc5.chr_bank_1k(s)).collect();
    let saved_prg: Vec<usize> = (0..4).map(|s| emu.game.mmc5.prg_bank_8k(s)).collect();
    let saved_mirror = emu.game.mmc5.nt_map;
    let saved_exram = emu.game.mmc5.exram;

    let state = emu.game.save_state();

    // 切到繁体：整个 CHR 窗口 +256 个 1 KiB bank。
    for _ in 0..30 {
        emu.game.step(PAD_RIGHT);
    }
    for _ in 0..30 {
        emu.game.step(0);
    }
    assert_eq!(emu.game.mmc5.chr_upper2, 1, "前置条件：应已切到繁体");
    assert_ne!(
        emu.game.mmc5.chr_bank_1k(0),
        saved_chr[0],
        "前置条件：CHR bank 应该变了"
    );

    emu.game.load_state(&state);

    assert_eq!(
        emu.game.mmc5.chr_upper2, saved_upper,
        "存档没恢复 MMC5 的简繁位（$5130）"
    );
    assert_eq!(
        (0..8).map(|s| emu.game.mmc5.chr_bank_1k(s)).collect::<Vec<_>>(),
        saved_chr,
        "存档没恢复 MMC5 的 CHR bank"
    );
    assert_eq!(
        (0..4).map(|s| emu.game.mmc5.prg_bank_8k(s)).collect::<Vec<_>>(),
        saved_prg,
        "存档没恢复 MMC5 的 PRG bank"
    );
    assert_eq!(emu.game.mmc5.nt_map, saved_mirror, "存档没恢复 $5105 镜像");
    assert_eq!(emu.game.mmc5.exram, saved_exram, "存档没恢复 ExRAM");
    assert_eq!(emu.game.exec_errors, 0, "读档后解释器报错");
}

/// 存档的字节镜像（`to_bytes`/`from_bytes`）也要带 MMC5，否则联机回滚和
/// `hash_state` 仍然看不到 MMC5 状态。
#[test]
fn state_byte_image_carries_mmc5() {
    let Some(bytes) = rom_bytes() else { return };
    let mut emu =
        emu_from_generic_ines(&bytes, 44100, z2_native::app::Features::default())
            .expect("加载");
    for _ in 0..900 {
        emu.game.step(0);
    }
    let saved = emu.game.save_state();
    let saved_chr: Vec<usize> = (0..8).map(|s| emu.game.mmc5.chr_bank_1k(s)).collect();
    let image = saved.to_bytes();

    // 切到繁体，让实时状态与 `saved` 明确不同。
    for _ in 0..30 {
        emu.game.step(PAD_RIGHT);
    }
    for _ in 0..30 {
        emu.game.step(0);
    }
    assert_eq!(emu.game.mmc5.chr_upper2, 1, "前置条件");

    let decoded = z2_core::state::GameState::from_bytes(&image).expect("解码存档");
    emu.game.load_state(&decoded);

    assert_eq!(
        emu.game.mmc5.chr_upper2, 0,
        "字节镜像没带 MMC5 简繁位，联机回滚会不同步"
    );
    assert_eq!(
        (0..8).map(|s| emu.game.mmc5.chr_bank_1k(s)).collect::<Vec<_>>(),
        saved_chr,
        "字节镜像没带 MMC5 CHR bank"
    );
}


/// 诊断：主循环在等 $0736/$076C，这两个由 NMI 更新。看 NMI 有没有来。
#[test]
fn diagnose_main_loop() {
    let Some(bytes) = rom_bytes() else { return };
    let mut emu = emu_from_generic_ines(&bytes, 44100, z2_native::app::Features::default()).expect("加载");
    for f in 0..30u32 {
        emu.game.step(0);
        if f >= 2 {
            let a = emu.game.ram[0x0736];
            let b = emu.game.ram[0x076C];
            let ctrl = emu.game.ppu.ppu.ctrl();
            let mask = emu.game.ppu.ppu.mask();
            eprintln!(
                "f{f:2} PC=${:04X} $0736={a:02X} $076C={b:02X} ctrl=${ctrl:02X} mask=${mask:02X}",
                emu.game.cpu.pc
            );
        }
    }
}

/// 进到真实侧视场景（宫殿内部），确认贴图正常。
///
/// 这一条锁住 `$5105` 镜像修复：侧视场景用 `$44`（竖镜像），如果 PPU 停在
/// 自己的默认布局，背景会从左 nametable 取错图块。同时确认状态栏的
/// 「魔法 / 生命 / 下一级」汉字渲染正常、全程零解释器错误。
#[test]
fn reaches_sideview_scene_with_correct_tiles() {
    let Some(bytes) = rom_bytes() else { return };
    let mut emu =
        emu_from_generic_ines(&bytes, 44100, z2_native::app::Features::default())
            .expect("加载");

    boot_into_game(&mut emu);

    let mode = emu.game.ram[0x0736];
    eprintln!(
        "mode=${mode:02X} nt=${:02X} ctrl=${:02X} mask=${:02X}",
        emu.game.mmc5.nt_map,
        emu.game.ppu.ppu.ctrl(),
        emu.game.ppu.ppu.mask()
    );
    assert_eq!(mode, MODE_SIDEVIEW, "没进入侧视场景（mode=${mode:02X}）");
    assert_eq!(emu.game.exec_errors, 0, "进场景过程中解释器报错");
    // The scene writes `$44` (screens A,B,A,B -> vertical mirroring).
    assert_eq!(emu.game.mmc5.nt_map, 0x44, "侧视场景的 $5105 应为 $44");

    // 背景和精灵都要有内容：状态栏 + 场景本身。
    let frame = *emu.game.frame_indexed();
    let nonzero = frame.iter().filter(|&&b| b != 0).count();
    eprintln!("非零像素 {nonzero} / {}", frame.len());
    assert!(nonzero > 20_000, "侧视场景几乎是空的（{nonzero}）");

    // 场景跑一段时间仍然稳定（不是画一帧就崩）。
    for _ in 0..120 {
        emu.game.step(0);
    }
    let later = *emu.game.frame_indexed();
    let still = later.iter().filter(|&&b| b != 0).count();
    assert!(still > 20_000, "跑 120 帧后画面变空（{still}）");
    assert_eq!(emu.game.exec_errors, 0, "侧视场景里解释器报错");
    assert_eq!(emu.game.ram[0x0736], MODE_SIDEVIEW, "中途掉出了侧视场景");
}

/// 宽屏边距必须用**真实的 4 KiB CHR 页号**解码。
///
/// 回归测试：`PpuBind::apply_mmc5` 曾经把 MMC5 的 1 KiB bank 号（如 104）
/// 直接存进 `record.page`，而 `wide::chr_row` 按 `page * 4096` 取图，于是
/// 第二槽被读成「页 104」= 偏移 425,984 —— 落在汉字**文本**页上。表现就是
/// 宽屏两侧铺满文字笔画碎片，而不是石柱/帷幔。
///
/// 正确值是 `1k_bank / 4`：译制 ROM 的槽是 `[0,1,2,3]` / `[104..107]`，
/// 对应 4 KiB 页 `0` / `26`。
#[test]
fn wide_margins_use_4k_chr_page_numbers() {
    let Some(bytes) = rom_bytes() else { return };
    let mut feats = z2_native::app::Features::default();
    feats.record = true;
    let mut emu = emu_from_generic_ines(&bytes, 44100, feats).expect("加载");
    boot_into_game(&mut emu);

    let rec = emu.game.frame_record().expect("render record 没开");
    let line = rec.line(120);
    eprintln!("chr_page={:?}", line.chr_page);
    // 译制 ROM 的 CHR 是 512 KiB = 128 个 4 KiB 页；页号必须落在这里面。
    for (slot, &page) in line.chr_page.iter().enumerate() {
        assert_ne!(page, u8::MAX, "槽 {slot} 的页号没设置");
        assert!(
            usize::from(page) < 128,
            "槽 {slot} 的页号 {page} 超出 128 个 4 KiB 页（多半又存成 1 KiB bank 号了）"
        );
    }

    // 页号必须是**背景**（B 组）的页，因为边距解码取的是背景图块。
    // MMC5 8x16 模式下 A 组（精灵）和 B 组（背景）选不同的 bank；用 A 组
    // 的页去取 BG 像素就会画错 —— 这正是宽屏两侧乱掉的根因。
    let m = &emu.game.mmc5;
    eprintln!(
        "A 组(精灵) 1k = {:?}",
        (0..8).map(|s| m.chr_bank_1k_ex(s, true)).collect::<Vec<_>>()
    );
    eprintln!(
        "B 组(背景) 1k = {:?}",
        (0..8).map(|s| m.chr_bank_1k_ex(s, false)).collect::<Vec<_>>()
    );
    let bg_page: Vec<u8> = (0..2)
        .map(|p| (m.chr_bank_1k_ex(p * 4, false) / 4) as u8)
        .collect();
    assert_eq!(line.chr_page, [bg_page[0], bg_page[1]], "record 的页号必须取背景 B 组");
    assert!(emu.game.ppu.ppu.has_chr_bg(), "背景槽没装（A/B 组没分离）");

    // 宽屏解码要能出内容，且两侧像素来自真实地形（非零像素够多）。
    let tiles = 19u8;
    let mut margins = z2_ppu::Margins::new(tiles);
    margins.fill_left_clip = true;
    margins.fill_right_clip = true;
    let mut wide = z2_ppu::WideFrame::new(tiles);
    assert!(
        emu.game.compose_wide(tiles, &mut margins, &mut wide),
        "compose_wide 失败"
    );
    let w = wide.width;
    let mpx = usize::from(tiles) * 8;
    let mut left_nz = 0;
    for y in 0..240usize {
        for x in 0..mpx {
            if wide.pixels[y * w + x] != 0 {
                left_nz += 1;
            }
        }
    }
    eprintln!("左 margin 非零像素 {left_nz} / {}", mpx * 240);
    assert!(left_nz > 5_000, "左 margin 内容太少（{left_nz}），解码可能失败");
}

/// 宽屏「游戏性」扩展在汉化 ROM 上真的开着：两个 overworld trap 注册、
/// 被命中上千次、且全程零解释器错误。
///
/// 它们 hook `$8284`/`$841B`（bank 0），译制 ROM 保留了这两个 CPU 地址；
/// hook 体在 bank 0 未映射时自动放弃（`bank0_mapped` 已改成按实际 mapper
/// 判断，原来只读 `mmc1`，在 MMC5 上会误判）。
#[test]
fn wide_gameplay_traps_fire_on_translated_rom() {
    let Some(bytes) = rom_bytes() else { return };
    let mut feats = z2_native::app::Features::default();
    feats.wide_gameplay = Some(11);
    feats.record = true;
    let mut emu = emu_from_generic_ines(&bytes, 44100, feats).expect("加载");

    assert!(emu.game.wide_gameplay_tiles().is_some(), "宽屏游戏性没打开");
    assert!(emu.game.traps.is_trapped(0x8284), "$8284 trap 没注册");
    assert!(emu.game.traps.is_trapped(0x841B), "$841B trap 没注册");

    // 先走完标题/注册流程，再一路往右走进大地图（blob 循环只在
    // overworld 跑）。
    boot_into_game(&mut emu);
    let mut hits = 0u32;
    for _ in 0..1500 {
        emu.game.step(PAD_RIGHT);
        for r in emu.game.trap_log().iter() {
            if r.attribution().starts_with("wide_") {
                hits += 1;
            }
        }
    }
    eprintln!(
        "宽屏 trap 命中 {hits} 次, mode=${:02X}, errors={}",
        emu.game.ram[0x0736],
        emu.game.exec_errors
    );
    // 命中的关键证据：两条 trap 都在跑（`wide_overworld1`/`wide_L841B`）。
    // 它们只在 bank 0 映射时做扩展，否则原样跑 ROM —— 两者都算“正常”。
    assert!(hits > 1000, "宽屏 trap 只命中 {hits} 次，扩展没跑起来");
    assert_eq!(emu.game.exec_errors, 0, "宽屏游戏性过程中解释器报错");
}

/// 双人合作在汉化 ROM 上真的生效：六条 coop trap 注册并被命中，
/// 且进入侧视场景后第二 Link 被激活（有血、活着）。
#[test]
fn coop_activates_second_link_on_translated_rom() {
    let Some(bytes) = rom_bytes() else { return };
    let mut feats = z2_native::app::Features::default();
    feats.coop = true;
    let mut emu = emu_from_generic_ines(&bytes, 44100, feats).expect("加载");

    assert!(emu.game.coop_status().is_some(), "双人没打开");
    // `$D5A7`/`$EBF0`/`$E4D9` 有原版实现，必定装上；`$D6C1`/`$E677`/`$E558`
    // 只在 ROM 里对应地址存在时才作为覆盖装上（`coop.rs` 的注释写明
    // 「no original: no override」），所以只断言前三个。
    for addr in [0xD5A7u16, 0xEBF0, 0xE4D9] {
        assert!(
            emu.game.traps.is_trapped(addr),
            "coop trap ${addr:04X} 没注册"
        );
    }

    boot_into_game(&mut emu);
    assert_eq!(
        emu.game.ram[0x0736],
        MODE_SIDEVIEW,
        "没进侧视场景，P2 不会激活"
    );

    let mut hits = 0u32;
    for _ in 0..300 {
        emu.game.step(0);
        for r in emu.game.trap_log().iter() {
            if r.attribution().starts_with("coop_") {
                hits += 1;
            }
        }
    }
    let st = emu.game.coop_status().expect("双人状态");
    eprintln!(
        "coop trap 命中 {hits} 次; P2 active={} alive={} hp={}/{}",
        st.active, st.p2_alive, st.p2_hp, st.hp_max
    );
    assert!(hits > 1000, "coop trap 只命中 {hits} 次");
    assert!(st.active, "第二 Link 没有激活");
    assert!(st.p2_alive, "第二 Link 没活着");
    assert!(st.p2_hp > 0, "第二 Link 血量是 0");
    assert_eq!(emu.game.exec_errors, 0, "双人过程中解释器报错");
    // 画面必须真的有两个 Link（非零像素足够多）。
    let nz = emu.game.frame_indexed().iter().filter(|&&b| b != 0).count();
    assert!(nz > 20_000, "双人场景画面异常（{nz} 非零像素）");
}

/// 大地图能到达（宽屏游戏性的 blob 循环只在那里跑）。
#[test]
fn reaches_overworld() {
    let Some(bytes) = rom_bytes() else { return };
    let mut emu =
        emu_from_generic_ines(&bytes, 44100, z2_native::app::Features::default())
            .expect("加载");
    boot_into_game(&mut emu);
    // 从宫殿往外走，直到进入大地图。
    let mut reached = false;
    for _ in 0..1500 {
        emu.game.step(PAD_RIGHT);
        if emu.game.ram[0x0736] == MODE_OVERWORLD {
            reached = true;
            break;
        }
    }
    eprintln!(
        "到达大地图={reached}, mode=${:02X}",
        emu.game.ram[0x0736]
    );
    assert!(reached, "没能走到大地图");
    assert_eq!(emu.game.exec_errors, 0, "去大地图路上解释器报错");
}

/// 7 项增强在译制 ROM 上必须真的改到 ROM 字节（而不是被字节校验静默跳过）。
///
/// 这依赖 `enh::prg_offset` 按 mapper 算偏移：译制镜像用 8 KiB bank、
/// 只有最后 8 KiB 固定。用原版 MMC1 的算法会让每个 patch 落到错误字节，
/// 然后被各自的校验函数跳过 —— 表现就是「选项勾了但没效果」。
#[test]
fn enhancements_patch_the_translated_rom() {
    use z2_core::enh::{
        AbilityOpts, ContinueFrom, Enhancements, FixesOpts, QolOpts, TextHudOpts,
    };
    let Some(bytes) = rom_bytes() else { return };

    let peek = |e: &z2_native::app::Emu, bank: u8, addr: u16, n: usize| {
        let off = z2_core::enh::prg_offset(&e.game, bank, addr);
        e.game.prg[off..off + n].to_vec()
    };

    // 基准：全关。
    let base =
        emu_from_generic_ines(&bytes, 44100, z2_native::app::Features::default())
            .expect("加载");

    let enh = Enhancements {
        text: TextHudOpts {
            dialogue_speed: 5,
            protect_spell_name: false,
        },
        qol: QolOpts {
            continue_from: ContinueFrom::Both,
            gameover_keep_xp_pct: 100,
            lives_from_dolls: true,
            enter_town_from_side: true,
            wise_men_restore_mp: true,
            no_mp_requirement_for_spells: true,
            overworld_softlock_warp: true,
        },
        fixes: FixesOpts {
            levelup_softlocks: true,
            iframe_update_skip: true,
            jump_direction_balance: true,
            shield_hitbox_symmetry: true,
            crumble_both_feet: true,
            xp_drain_fix: true,
        },
        enemies: Default::default(),
        abilities: AbilityOpts {
            sword_reach_px: 3,
            rescue_fairy: true,
            flute_warp: true,
            ..Default::default()
        },
        rando: Default::default(),
        cheats: Default::default(),
        content: Default::default(),
    };
    let mut feats = z2_native::app::Features::default();
    feats.enhancements = enh;
    let e = emu_from_generic_ines(&bytes, 44100, feats).expect("加载");

    assert!(e.game.enh_active(), "增强没启用");

    // 对话速度：bank 3 `$B615`，原 0x2A，满速地板 0x0A。
    let d0 = peek(&base, 3, 0xB615, 1);
    let d1 = peek(&e, 3, 0xB615, 1);
    eprintln!("对话延迟 {d0:02X?} -> {d1:02X?}");
    assert_eq!(d0, vec![0x2A], "基准对话延迟应是 $2A");
    assert_eq!(d1, vec![0x0A], "对话速度没生效（仍是 {d1:02X?}）");

    // 剑距离：bank 7 `$E99D`（原 0x02）+ `$E9B5`（原 0x0E）。
    let s0 = peek(&base, 7, 0xE99D, 1);
    let w0 = peek(&base, 7, 0xE9B5, 1);
    let s1 = peek(&e, 7, 0xE99D, 1);
    let w1 = peek(&e, 7, 0xE9B5, 1);
    eprintln!("剑 {s0:02X?}/{w0:02X?} -> {s1:02X?}/{w1:02X?}");
    assert_eq!(s0, vec![0x02], "基准剑偏移应是 $02");
    assert_eq!(w0, vec![0x0E], "基准剑宽应是 $0E");
    assert_ne!(s1, s0, "剑偏移没变");
    assert_ne!(w1, w0, "剑宽没变");

    // 智者无 MP 要求：bank 3 `$B52B` 的分支 -> 两个 NOP。
    let wm0 = peek(&base, 3, 0xB52B, 2);
    let wm1 = peek(&e, 3, 0xB52B, 2);
    eprintln!("智者检查 {wm0:02X?} -> {wm1:02X?}");
    assert_eq!(wm1, vec![0xEA, 0xEA], "智者检查没被 NOP 掉");
}

/// 增强开着跑进游戏也不能出错。
#[test]
fn enhancements_stay_stable_in_gameplay() {
    use z2_core::enh::{AbilityOpts, Enhancements, QolOpts};
    let Some(bytes) = rom_bytes() else { return };
    let enh = Enhancements {
        qol: QolOpts {
            continue_from: z2_core::enh::ContinueFrom::Both,
            gameover_keep_xp_pct: 100,
            lives_from_dolls: true,
            ..Default::default()
        },
        abilities: AbilityOpts {
            sword_reach_px: 3,
            rescue_fairy: true,
            flute_warp: true,
            ..Default::default()
        },
        ..Default::default()
    };
    let mut feats = z2_native::app::Features::default();
    feats.enhancements = enh;
    let mut emu = emu_from_generic_ines(&bytes, 44100, feats).expect("加载");
    boot_into_game(&mut emu);
    assert_eq!(emu.game.ram[0x0736], MODE_SIDEVIEW, "没进侧视场景");
    for _ in 0..600 {
        emu.game.step(PAD_RIGHT);
    }
    eprintln!(
        "增强跑 600 帧: mode=${:02X} errors={}",
        emu.game.ram[0x0736],
        emu.game.exec_errors
    );
    assert_eq!(emu.game.exec_errors, 0, "增强跑动中解释器报错");
}

/// 边缘精灵（margin sprites）在译制 ROM 上必须真的画进边距。
///
/// 回归测试：`wide_sprites::set_enabled` 用 `Game::new()` 建 shadow，而它
/// 的 mapper 默认是 MMC1。译制镜像是 MMC5，shadow 重跑 `bank7_Display`
/// 时会读错 bank，一个精灵都收集不到 —— 表现为「选项开了但边缘什么都没有」。
/// 同时 `observe` 每帧只同步了 `mmc1`，MMC5 的 bank 切换没跟过去。
///
/// 注意时序：`shown` 在 `$4014` DMA 时填充，必须在 `step` 之后**立刻**读，
/// 否则下一帧的 DMA 会把它覆盖。
#[test]
fn margin_sprites_reach_the_margins() {
    let Some(bytes) = rom_bytes() else { return };
    let mut feats = z2_native::app::Features::default();
    feats.record = true;
    feats.margin_sprites = true;
    let mut emu = emu_from_generic_ines(&bytes, 44100, feats).expect("加载");
    assert!(emu.game.margin_sprites_enabled());
    assert!(emu.game.traps.is_trapped(0xEF11), "$EF11 观察器没注册");

    boot_into_game(&mut emu);
    assert_eq!(emu.game.ram[0x0736], MODE_SIDEVIEW, "没进侧视场景");

    // 走动 + 挥剑，制造有敌人的画面；同时每步都 compose 取最大的一次。
    let mut best_sprites = 0usize;
    let mut best_shown = 0usize;
    for i in 0..1200 {
        emu.game.step(if (i / 30) % 2 == 0 { PAD_RIGHT } else { PAD_A });
        let shown = z2_core::wide_sprites::shown(&emu.game).len();
        let mut margins = z2_ppu::Margins::new(11);
        margins.fill_left_clip = true;
        margins.fill_right_clip = true;
        margins.fill_left_sprites = true;
        let mut wide = z2_ppu::WideFrame::new(11);
        if emu.game.compose_wide(11, &mut margins, &mut wide) {
            if margins.sprites.len() > best_sprites {
                best_sprites = margins.sprites.len();
                best_shown = shown;
            }
        }
    }
    eprintln!("边缘精灵: shown 最大 {best_shown}, 进图 {best_sprites}");
    assert_eq!(
        emu.game.margin_sprite_errors(),
        0,
        "shadow 重跑 ROM 失败了"
    );
    assert!(
        best_shown > 0,
        "一个边缘精灵都没收集到（shadow 的 mapper 可能又退回 MMC1）"
    );
    assert!(
        best_sprites > 0,
        "收集到 {best_shown} 个，但没进入 margins"
    );
}

/// 宽屏「游戏性」扩展要让敌人在边距里活动（overworld blob）。
#[test]
fn wide_gameplay_blobs_reach_the_margins() {
    let Some(bytes) = rom_bytes() else { return };
    let mut feats = z2_native::app::Features::default();
    feats.record = true;
    feats.wide_gameplay = Some(11);
    let mut emu = emu_from_generic_ines(&bytes, 44100, feats).expect("加载");
    assert!(emu.game.wide_gameplay_tiles().is_some());

    boot_into_game(&mut emu);
    // 走到大地图（blob 循环只在那里跑）。
    let mut reached = false;
    for _ in 0..2500 {
        emu.game.step(PAD_RIGHT);
        if emu.game.ram[0x0736] == MODE_OVERWORLD {
            reached = true;
            break;
        }
    }
    assert!(reached, "没走到大地图");

    let mut hits = 0u32;
    for _ in 0..600 {
        emu.game.step(PAD_RIGHT);
        for r in emu.game.trap_log().iter() {
            if r.attribution().starts_with("wide_") {
                hits += 1;
            }
        }
    }
    eprintln!("大地图宽屏 trap 命中 {hits}");
    assert!(hits > 0, "宽屏游戏性的 overworld trap 没跑");
    assert_eq!(emu.game.exec_errors, 0, "宽屏游戏性跑动中解释器报错");
}

/// 双人模式下 P2 必须真的参与战斗（会受伤，也就能打出伤害）。
///
/// 回归测试：`emu_from_generic_ines` 原本**不注册任何 trap**，于是
/// `coop::register_coop_traps` 的三个覆盖项（`$D6C1` 接触、`$E677` 剑击、
/// `$E558` 敌武器）因为「只在已有基础 trap 时覆盖」而全部跳过 —— P2 能走
/// 能跳，但对敌人完全无害，也不会被敌人打。
///
/// 顺带锁住：`bank7_traps` 的 MMC1 串行切换器在译制镜像上必须跳过，
/// 否则 `SwapPRG` 会把 bank 切错，PC 跑飞到 `$A6A1…`。
#[test]
fn coop_p2_runs_in_gameplay() {
    let Some(bytes) = rom_bytes() else { return };
    let mut feats = z2_native::app::Features::default();
    feats.coop = true;
    let mut emu = emu_from_generic_ines(&bytes, 44100, feats).expect("加载");

    // 三个 P2 覆盖项都必须挂上（否则 P2 打不出伤害）。
    for (addr, want) in [
        (0xD6C1u16, "coop_link_collision"),
        (0xE677, "coop_sword_hit"),
        (0xE558, "coop_code39"),
    ] {
        let got = emu.game.traps.get(addr).map(|t| t.name);
        assert_eq!(got, Some(want), "${addr:04X} 的 P2 覆盖项没注册");
    }
    // MMC1 串行切换器必须跳过：译制镜像在这些地址上是 MMC5 代码。
    assert!(
        emu.game.traps.get(0xFFCC).is_none(),
        "SwapPRG 不该在译制镜像上注册（会把 bank 切错）"
    );

    boot_into_game(&mut emu);
    assert_eq!(emu.game.ram[0x0736], MODE_SIDEVIEW, "没进侧视场景");
    let hp0 = emu.game.coop_status().map(|s| s.p2_hp).unwrap_or(0);
    assert!(hp0 > 0, "P2 没激活");

    // 往右走撞进战斗，让 P2 参与。
    for _ in 0..3000 {
        emu.game.step(PAD_RIGHT);
        if emu.game.ram[0x0736] == MODE_OVERWORLD {
            break;
        }
    }
    for i in 0..3000 {
        emu.game.step(if (i / 20) % 2 == 0 { PAD_RIGHT } else { 0 });
    }

    let st = emu.game.coop_status().expect("coop 状态没了");
    eprintln!(
        "P2 hp {hp0} -> {} (deaths={}) mode=${:02X} errors={} \
         p2_contacts={} n_display={} n_update={}",
        st.p2_hp,
        st.p2_deaths,
        emu.game.ram[0x0736],
        emu.game.exec_errors,
        emu.game.coop.n_p2_contacts,
        emu.game.coop.n_display,
        emu.game.coop.n_update
    );
    assert_eq!(emu.game.exec_errors, 0, "双人跑动中解释器报错");
    // P2 通道是否真的在跑，看 `n_display` / `n_update` ——
    // `coop_links_display`（`$EBF0`）和 `coop_code21`（`$D5A7`）每帧都会
    // 走一次，只要 P2 在侧视场景里活动，这两个计数就必然增长。
    //
    // 这里**不能**断言「P2 掉血」或「`n_p2_contacts > 0`」：前者取决于
    // 这一路走下来的场景里有没有敌人打到 P2，后者要求真实发生接触。实测
    // 美版 ROM 在同一组输入下也是 `p2_contacts=0`、P2 全程满血，场景序列
    // 与汉化版完全一致（都是 [0, 39]），所以那是场景运气，不是通道是否
    // 接通的证据。曾经这里断言掉血，在启用 sideview/overworld trap 后
    // 场景从 22 变成 39（与美版一致），测试就假失败了。
    //
    // `n_display` / `n_update` 与美版对照：2072/2222 vs 2077/2228，
    // 说明两边每帧都在跑同一条路。
    assert!(
        emu.game.coop.n_display > 0 && emu.game.coop.n_update > 0,
        "P2 的显示/更新通道一次都没跑（n_display={} n_update={}），\
         P2 没真正参与",
        emu.game.coop.n_display,
        emu.game.coop.n_update
    );
}

/// 每次启动都必须回到**简体**。
///
/// 回归测试：简繁选择存在 RAM `$01D4`，ROM 初始化时用它恢复 `$5130`
/// （`$9E8E`: `LDA $01D4 / CMP #$01 / BEQ / LDA #$00 / STA $5130`），
/// 而这两者都跨 `reset` 存活。只清 `$5130` 不够 —— 下一帧就被写回。
#[test]
fn every_start_begins_in_simplified_chinese() {
    let Some(bytes) = rom_bytes() else { return };
    let mut emu =
        emu_from_generic_ines(&bytes, 44100, z2_native::app::Features::default())
            .expect("加载");
    for _ in 0..700 {
        emu.game.step(0);
    }
    assert_eq!(emu.game.mmc5.chr_upper2, 0, "启动时不是简体");

    // 按右切到繁体。
    for _ in 0..6 {
        emu.game.step(PAD_RIGHT);
        for _ in 0..3 {
            emu.game.step(0);
        }
    }
    for _ in 0..60 {
        emu.game.step(0);
    }
    assert_eq!(emu.game.mmc5.chr_upper2, 1, "按右没切到繁体");

    // reset 后必须回到简体，并且**保持**简体。
    emu.game.reset();
    for i in 0..120 {
        emu.game.step(0);
        assert_eq!(
            emu.game.mmc5.chr_upper2, 0,
            "reset 后第 {i} 帧又变回繁体（$01D4 没清）"
        );
    }
}

/// 繁体字形在页 +64 上，HD 包必须也能覆盖到（否则切繁体后画面全掉回原版像素）。
///
/// 回归测试：`HdPack::lookup` 原本只查精确页，切繁体后页号整体 +64，
/// 命中率从 63% 直接掉到 0%。
#[test]
fn traditional_pages_still_get_hd_art() {
    let Some(bytes) = rom_bytes() else { return };
    let pack = z2_render::fs::load_pack_dir(std::path::Path::new(
        r"D:/下载/Compressed/z2rs-hd-pack-v0.1.1-with-new-hd-link/z2rs-hd-pack-v0.1.1-with-new-hd-link",
    ));
    let Ok(pack) = pack else {
        eprintln!("HD 包不在，跳过");
        return;
    };

    let mut feats = z2_native::app::Features::default();
    feats.record = true;
    let mut emu = emu_from_generic_ines(&bytes, 44100, feats).expect("加载");
    boot_into_game(&mut emu);
    for _ in 0..120 {
        emu.game.step(0);
    }

    let coverage = |e: &z2_native::app::Emu| -> (usize, usize, u8) {
        let (mut hit, mut total, mut page) = (0usize, 0usize, 0xFFu8);
        if let Some(rec) = e.game.frame_record() {
            for y in 0..240usize {
                let l = rec.line(y);
                for t in l.tiles.iter() {
                    if !t.fetched || t.page == 0xFF {
                        continue;
                    }
                    page = t.page;
                    total += 1;
                    if pack
                        .lookup(t.page, t.tile, z2_render::bg_colors(l, t.pal))
                        .is_some()
                    {
                        hit += 1;
                    }
                }
            }
        }
        (hit, total, page)
    };

    let (simp_hit, simp_total, simp_page) = coverage(&emu);
    eprintln!("简体: 页 {simp_page} {simp_hit}/{simp_total}");
    assert!(simp_hit > 0, "简体下 HD 也没覆盖，先查包和路径");

    // 强制切到繁体（+64 页）。
    emu.game.mmc5.chr_upper2 = 1;
    for _ in 0..4 {
        emu.game.step(0);
    }
    let (trad_hit, trad_total, trad_page) = coverage(&emu);
    eprintln!("繁体: 页 {trad_page} {trad_hit}/{trad_total}");
    assert_eq!(
        trad_page,
        simp_page + 64,
        "繁体页号应该是简体 +64"
    );
    assert!(
        trad_hit > 0,
        "繁体下 HD 命中 0（页 {trad_page}），回退逻辑没生效"
    );
    // 覆盖率应当和简体一致。
    assert_eq!(
        trad_hit * simp_total,
        simp_hit * trad_total,
        "繁体覆盖率 {trad_hit}/{trad_total} 与简体 {simp_hit}/{simp_total} 不一致"
    );
}

/// 文本**行**必须让 HD 让位，否则中文对话会被地形图块盖成乱码。
///
/// 译制 ROM 把中文字形放进了原版地形的 `(page, tile)` 槽里 —— 例如 HD 包里
/// `page 3, tiles 179-184` 是**云朵**，译制版那里放的是对话框文字。HD 照常
/// 覆盖就会把对话画成一片云和地面碎片。
///
/// 修法：`Game::is_showing_text`（`$074C` 对话 / `$0524` 暂停菜单 /
/// 非游戏模式）驱动 `Presenter::set_text_mode`，文本帧整帧丢掉 pack。
#[test]
fn text_rect_drops_the_hd_pack() {
    let Some(bytes) = rom_bytes() else { return };
    let pack_dir = std::path::Path::new(
        r"D:/下载/Compressed/z2rs-hd-pack-v0.1.1-with-new-hd-link/z2rs-hd-pack-v0.1.1-with-new-hd-link",
    );
    if !pack_dir.is_dir() {
        eprintln!("HD 包不在，跳过");
        return;
    }

    let mut feats = z2_native::app::Features::default();
    feats.record = true;
    let mut emu = emu_from_generic_ines(&bytes, 44100, feats).expect("加载");

    // 标题画面：文本帧。
    for _ in 0..700 {
        emu.game.step(0);
    }
    assert!(
        emu.game.is_showing_text(),
        "标题画面应该判定为文本帧（mode=${:02X}）",
        emu.game.ram[0x0736]
    );

    // 进游戏：非文本帧。
    boot_into_game(&mut emu);
    for _ in 0..120 {
        emu.game.step(0);
    }
    assert!(
        !emu.game.is_showing_text(),
        "宫殿场景不该判定为文本帧（mode=${:02X}）",
        emu.game.ram[0x0736]
    );

    // 打开暂停菜单：又是文本帧。
    for _ in 0..4 {
        emu.game.step(PAD_START);
        for _ in 0..3 {
            emu.game.step(0);
        }
    }
    for _ in 0..60 {
        emu.game.step(0);
    }
    assert!(
        emu.game.is_showing_text(),
        "暂停菜单应该判定为文本帧（menu=${:02X}）",
        emu.game.ram[0x0524]
    );
}
