//! 端到端：用 MMC5 跑汉化版 ROM，验证能启动、能出画面。
//!
//! 这个测试直接读真实 ROM 文件（不在仓库里），找不到就跳过 —— 和
//! `cjk_e2e_real.rs` 一样的约定。

use std::path::Path;

use z2_native::app::{emu_from_generic_ines, ines_is_mmc5};

const ROM: &str = r"D:/z2rs-中文版/Zelda II - The Adventure of Link (U) [20260806_NOKOH].nes";

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
    let mut emu = emu_from_generic_ines(&bytes, 44100).expect("加载汉化版");

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
}

#[test]
fn reset_vector_is_readable_through_mmc5() {
    let Some(bytes) = rom_bytes() else { return };
    let emu = emu_from_generic_ines(&bytes, 44100).expect("加载");
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
    let mut emu = emu_from_generic_ines(&bytes, 44100).expect("加载");
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
    let mut emu = emu_from_generic_ines(&bytes, 44100).expect("加载");
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


/// 诊断：主循环在等 $0736/$076C，这两个由 NMI 更新。看 NMI 有没有来。
#[test]
fn diagnose_main_loop() {
    let Some(bytes) = rom_bytes() else { return };
    let mut emu = emu_from_generic_ines(&bytes, 44100).expect("加载");
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
