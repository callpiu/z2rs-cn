//! ROM-gated check that the HD player pose follows the cartridge's own
//! crouch state (`player_visual`): a standing stab leaves `$0495` set
//! (it latches the shield row `$17` at each stab), which once made the HD
//! Link keep the crouch pose while standing and walking.
//!
//! Skips (never fails) without `$Z2_ROM`.

#![cfg(feature = "interp")]

mod common;

use z2_core::game::Game;

// Input bytes on the shared contract (bit0 = A … bit7 = Right).
const A: u8 = 0x01;
const B: u8 = 0x02;
const SELECT: u8 = 0x04;
const START: u8 = 0x08;
const DOWN: u8 = 0x20;
const RIGHT: u8 = 0x80;

const MODE: usize = 0x0736;
const ANIM: usize = 0x0080;
const SHIELD_ROW: usize = 0x0017;
const STAB_ROW: usize = 0x0495;
const LAND_CROUCH: usize = 0x0497;
const MIDAIR: usize = 0x0479;

fn register_all(g: &mut Game) {
    z2_core::bank7_traps::register_bank7_traps(g);
    z2_core::sideview_traps::register_sideview_traps(g);
    z2_core::sideview_traps::register_overworld_traps(g);
    z2_core::player_traps::register_player_traps(g);
    z2_core::enemy_traps::register_enemy_traps(g);
    z2_core::town_traps::register_town_traps(g);
    z2_core::palace_traps::register_palace_traps(g);
    z2_core::title_traps::register_title_traps(g);
    z2_core::boot_traps::register_boot_traps(g);
}

fn step_n(g: &mut Game, n: usize, input: u8) {
    for _ in 0..n {
        g.step(input);
    }
}

/// Fresh cartridge → named slot 0 → side-view gameplay in North Castle
/// (the `enh_abilities_rom` route).
fn to_gameplay(test: &str) -> Option<Game> {
    let raw = common::rom_bytes(test)?;
    let mut g = Game::from_ines(&raw).ok()?;
    register_all(&mut g);
    g.reset();
    step_n(&mut g, 30, 0);
    step_n(&mut g, 5, START);
    step_n(&mut g, 20, 0);
    step_n(&mut g, 5, START);
    step_n(&mut g, 20, 0);
    for _ in 0..8 {
        step_n(&mut g, 2, A);
        step_n(&mut g, 8, 0);
    }
    for _ in 0..3 {
        step_n(&mut g, 2, SELECT);
        step_n(&mut g, 8, 0);
    }
    step_n(&mut g, 5, START);
    step_n(&mut g, 30, 0);
    step_n(&mut g, 5, START);
    step_n(&mut g, 60, 0);
    step_n(&mut g, 1100, 0);
    assert_eq!(g.ram[MODE], 0x0B, "side-view gameplay mode");
    Some(g)
}

fn clip(g: &Game) -> &'static str {
    g.player_visuals()[0].expect("side-view player").clip
}

/// Step with `input`, collecting every clip shown.
fn clips(g: &mut Game, n: usize, input: u8) -> Vec<&'static str> {
    (0..n)
        .map(|_| {
            g.step(input);
            clip(g)
        })
        .collect()
}

#[test]
fn standing_stab_then_crouch_lets_link_stand_up() {
    let Some(mut g) = to_gameplay("standing_stab_then_crouch") else {
        return;
    };
    assert_eq!(clip(&g), "idle");

    // Standing stab: the cartridge latches the high shield row into `$0495`.
    let mut stab = clips(&mut g, 4, B);
    // Windup, thrust, recovery, then the 32-frame `$0400 == 4` re-stab window.
    stab.extend(clips(&mut g, 60, 0));
    assert!(stab.contains(&"attack"), "standing stab shown: {stab:?}");
    assert_eq!(g.ram[STAB_ROW], 1, "standing stab leaves $0495 set");
    assert_eq!(g.ram[SHIELD_ROW], 1, "shield high: Link stands");
    assert!(g.ram[ANIM] < 4, "walk-cycle frame: {}", g.ram[ANIM]);
    assert_eq!(clip(&g), "idle", "no crouch pose after a standing stab");
    let walk = clips(&mut g, 20, RIGHT);
    assert!(walk.iter().all(|&c| c == "walk"), "walks upright: {walk:?}");
    step_n(&mut g, 30, 0);
    assert_eq!(clip(&g), "idle");

    // Crouch, then let go of Down: Link stands up in RAM and on screen.
    let duck = clips(&mut g, 10, DOWN);
    assert_eq!(g.ram[ANIM], 6, "ducked");
    assert!(duck.iter().all(|&c| c == "crouch_idle"), "{duck:?}");
    let up = clips(&mut g, 2, 0);
    assert_ne!(g.ram[ANIM], 6, "the cartridge stands Link up");
    assert_eq!(up[1], "idle", "and so does the HD pose");

    // Crouch stab clears `$0495`; standing up still works afterwards.
    let mut low = clips(&mut g, 4, DOWN | B);
    low.extend(clips(&mut g, 30, DOWN));
    assert!(low.contains(&"crouch_attack"), "crouch stab shown: {low:?}");
    assert_eq!(g.ram[STAB_ROW], 0, "crouch stab clears $0495");
    step_n(&mut g, 2, 0);
    assert_eq!(clip(&g), "idle");

    // Another standing stab, then a jump: rising, apex and fall, no crouch.
    step_n(&mut g, 4, B);
    step_n(&mut g, 60, 0);
    assert_eq!(g.ram[STAB_ROW], 1);
    let mut air = Vec::new();
    for i in 0..120 {
        g.step(if i < 20 { A } else { 0 });
        if g.ram[MIDAIR] != 0 {
            air.push(clip(&g));
        }
    }
    assert!(!air.is_empty(), "Link jumped");
    assert!(air.contains(&"jump_rise"), "rising pose shown: {air:?}");
    assert!(air.contains(&"fall"), "falling pose shown: {air:?}");
    assert!(
        !air.contains(&"crouch_idle"),
        "no crouch pose in the air: {air:?}"
    );
    // A short hop lands without the `$0497` landing crouch.
    assert_eq!(g.ram[LAND_CROUCH], 0);
    assert_eq!(clip(&g), "idle");
}
