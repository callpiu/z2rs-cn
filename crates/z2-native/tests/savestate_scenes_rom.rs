//! ROM-gated: a quick save (`F5`) loaded (`F7`) while the game shows a
//! different scene resumes exactly as the saved run would have. Before save
//! states carried the whole `Game`, only RAM and WRAM came back, so the
//! nametables, both CHR slots, the MMC1 selects and the CPU of the scene on
//! screen stayed behind: the loaded scene drew with the wrong tiles (an HD
//! pack then matched the wrong cells) and the CPU could resume on a stack
//! that was not its own.

mod common;

use z2_native::app::{self, Emu};

const A: u8 = 0x01;
const SELECT: u8 = 0x04;
const START: u8 = 0x08;

/// Boot to side-view play in North Castle.
fn boot_to_play(rom: &std::path::Path) -> Emu {
    let mut emu = app::emu_from_rom_file(rom, 44_100).expect("rom loads");
    app::arm_record(&mut emu, true);
    let mut track = vec![0u8; 30];
    let mut push = |n: usize, b: u8| track.extend(std::iter::repeat_n(b, n));
    push(5, START);
    push(20, 0);
    push(5, START);
    push(20, 0);
    for _ in 0..8 {
        push(2, A);
        push(8, 0);
    }
    for _ in 0..3 {
        push(2, SELECT);
        push(8, 0);
    }
    push(5, START);
    push(30, 0);
    push(5, START);
    push(1160, 0);
    app::step_frames(&mut emu, &track, None);
    assert_eq!(emu.game.ram[0x0736], 0x0B, "side-view gameplay");
    emu
}

/// Load palace 3's Iron Knuckle room through the game's own loader.
fn to_palace(emu: &mut Emu) {
    for (a, v) in [
        (0x0706, 0x00),
        (0x0707, 0x04),
        (0x0748, 0x36),
        (0x056C, 0x02),
        (0x075C, 0x00),
        (0x0701, 0x00),
        (0x0561, 0x05),
        (0x0736, 0x00),
    ] {
        emu.game.ram[a] = v;
    }
    app::step_frames(emu, &[0; 150], None);
    assert_eq!(emu.game.ram[0x0736], 0x0B, "room loaded");
}

/// Walk out of North Castle onto the overworld.
fn to_overworld(emu: &mut Emu) {
    // Walking left off the first screen leaves the castle.
    let mut track = vec![0x40u8; 400];
    track.extend(std::iter::repeat_n(0, 200));
    app::step_frames(emu, &track, None);
    assert_eq!(emu.game.ram[0x0736], 0x05, "overworld");
}

fn save_load_case(name: &str, leave: fn(&mut Emu)) {
    let Some(rom) = common::rom_path(name) else {
        return;
    };
    let dir = std::env::temp_dir().join(format!("z2rs-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);

    let mut emu = boot_to_play(&rom);
    app::save_savestate(&emu.game, &dir, 1).expect("save");
    let saved_scene = (emu.game.ram[0x0736], emu.game.ram[0x0707], emu.game.mmc1);

    // Reference: the same boot, played on from the save point.
    let after = [0u8; 90];
    let mut want = boot_to_play(&rom);
    app::step_frames(&mut want, &after, None);

    // Go somewhere else, then quick-load.
    leave(&mut emu);
    assert_ne!(
        (emu.game.ram[0x0736], emu.game.ram[0x0707], emu.game.mmc1),
        saved_scene,
        "left the saved scene"
    );
    app::load_savestate(&mut emu.game, &dir, 1).expect("load");
    app::step_frames(&mut emu, &after, None);

    let ram_diff: Vec<String> = (0..0x800)
        .filter(|&a| emu.game.ram[a] != want.game.ram[a])
        .map(|a| format!("${a:04X}:{:02X}/{:02X}", emu.game.ram[a], want.game.ram[a]))
        .collect();
    let px_diff = (0..emu.game.frame.len())
        .filter(|&i| emu.game.frame[i] != want.game.frame[i])
        .count();
    assert!(
        ram_diff.is_empty() && px_diff == 0 && emu.game.mmc1 == want.game.mmc1,
        "after load: {} RAM bytes differ ({}), {px_diff} pixels differ, MMC1 {:?} vs {:?}",
        ram_diff.len(),
        ram_diff
            .iter()
            .take(24)
            .cloned()
            .collect::<Vec<_>>()
            .join(" "),
        emu.game.mmc1,
        want.game.mmc1,
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn quick_load_from_another_palace_restores_the_scene() {
    save_load_case(
        "quick_load_from_another_palace_restores_the_scene",
        to_palace,
    );
}

#[test]
fn quick_load_from_the_overworld_restores_the_scene() {
    save_load_case(
        "quick_load_from_the_overworld_restores_the_scene",
        to_overworld,
    );
}
