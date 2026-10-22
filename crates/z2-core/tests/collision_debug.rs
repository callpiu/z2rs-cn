//! Collision overlay observations must agree with the executable collision
//! routines, including inclusive endpoints, wrap/clipping and patched tables.
#![cfg(feature = "interp")]
mod common;
use z2_core::{collision_debug::Kind, game::Game};

#[test]
fn synthetic_tables_coop_probes_and_gates() {
    let mut g = Game::with_test_program(0x8000, &vec![0; 0x8000]);
    let mut put = |addr, bytes: &[u8]| {
        let at = g.mmc1.map_prg(addr, g.prg.len());
        g.prg[at..at + bytes.len()].copy_from_slice(bytes);
    };
    // Authored test values, deliberately unlike the cartridge's geometry.
    put(0xe9d4, &[3, 5, 9, 11]);
    put(0xe971, &[7, 13, 17, 19]);
    put(0xe99c, &[2, 4, 6, 8, 10, 12]);
    put(0xe9b5, &[21]);
    put(0xeaa0, &[0, 1, 2, 3, 4, 5, 6, 7]);
    put(0xeac0, &[8, 9, 10, 11, 12, 13, 14, 15]);
    assert!(g.collision_geometry().boxes.is_empty());
    g.ram[0x736] = 0x0b;
    g.ram[0xcc] = 50;
    g.ram[0x29] = 90;
    g.ram[0x9f] = 1;
    g.ram[0x480] = 0xf8;
    g.coop.enabled = true;
    g.coop.active = true;
    for (addr, value) in [(0xcc, 100), (0x29, 80), (0x9f, 2), (0x480, 0xf8)] {
        g.coop.block[z2_core::coop::idx(addr)] = value;
    }
    let before = g.ram;
    let geometry = g.collision_geometry();
    assert_eq!(g.ram, before);
    assert_eq!(geometry.boxes.len(), 4, "two players, retracted swords");
    let shield = geometry
        .boxes
        .iter()
        .find(|b| b.player == 0 && b.kind == Kind::Shield)
        .unwrap();
    assert_eq!([shield.x, shield.y, shield.w, shield.h], [61, 99, 6, 13]);
    let p2 = geometry
        .boxes
        .iter()
        .find(|b| b.player == 1 && b.kind == Kind::Shield)
        .unwrap();
    assert_eq!(p2.x, 113, "P2 uses its own position and facing");
    assert!(
        geometry.terrain.contains(&[54, 134]),
        "level row zero is below the HUD"
    );
    g.ram[0x480] = 100;
    assert_eq!(
        g.collision_geometry()
            .boxes
            .iter()
            .find(|b| b.kind == Kind::Sword)
            .unwrap()
            .w,
        22
    );
}

#[test]
fn observed_player_boxes_match_cartridge_routines() {
    let Some(raw) = common::rom_bytes("collision_debug") else {
        return;
    };
    let mut g = Game::from_ines(&raw).unwrap();
    g.ram[0x736] = 0x0b;
    for facing in [1, 2] {
        for shield in [0, 1] {
            for anim in [3, 5, 7, 8, 9] {
                for sx in [0, 80, 240, 250] {
                    for ly in [90, 250] {
                        g.ram[0xcc] = sx;
                        g.ram[0x29] = ly;
                        g.ram[0x9f] = facing;
                        g.ram[0x17] = shield;
                        g.ram[0x80] = anim;
                        g.ram[0x47e] = sx.wrapping_add(if facing == 1 { 24 } else { 0 });
                        g.ram[0x480] = ly.wrapping_sub(20);
                        let before = (g.ram, g.wram, g.cpu, g.mmc1);
                        let geometry = g.collision_geometry();
                        assert_eq!(g.ram, before.0);
                        assert_eq!(g.wram, before.1);
                        assert_eq!(g.cpu, before.2);
                        assert_eq!(g.mmc1, before.3);
                        for (kind, routine) in [
                            (Kind::Shield, 0xe9d8),
                            (Kind::Body, 0xe975),
                            (Kind::Sword, 0xe9a2),
                        ] {
                            g.try_call_asm(routine, 1000).unwrap();
                            let b = geometry.boxes.iter().find(|b| b.kind == kind).unwrap();
                            assert_eq!(
                                [b.x, b.y, b.w, b.h],
                                [
                                    i32::from(g.ram[0]),
                                    i32::from(g.ram[1]),
                                    i32::from(g.ram[2]) + 1,
                                    i32::from(g.ram[3]) + 1
                                ],
                                "{kind:?}, facing={facing}, shield={shield}, anim={anim}"
                            );
                        }
                    }
                }
            }
        }
    }
    // Sword reach is patched in the cartridge; the observer must read it.
    let at = g.mmc1.map_prg(0xe9b5, g.prg.len());
    g.prg[at] = 23;
    assert_eq!(
        g.collision_geometry()
            .boxes
            .iter()
            .find(|b| b.kind == Kind::Sword)
            .unwrap()
            .w,
        24
    );
    let mut options = z2_core::enh::Enhancements::default();
    options.fixes.shield_hitbox_symmetry = true;
    g.set_enhancements(options);
    g.ram[0x9f] = 1;
    let shield = *g
        .collision_geometry()
        .boxes
        .iter()
        .find(|b| b.kind == Kind::Shield)
        .unwrap();
    assert!(matches!(
        g.fire_trap(0xe9d8),
        z2_core::traps::TrapExit::Return
    ));
    assert_eq!(
        shield.x,
        i32::from(g.ram[0]),
        "enhanced shield follows its hook"
    );
}

#[test]
fn observed_enemy_boxes_match_cartridge_routine() {
    let Some(raw) = common::rom_bytes("collision_debug_enemy") else {
        return;
    };
    let mut g = Game::from_ines(&raw).unwrap();
    g.ram[0x736] = 0x0b;
    g.ram[0xcc] = 100;
    g.ram[0x4d] = 100;
    g.ram[0xb6] = 1;
    for code in 0..=35 {
        g.ram[0xa1] = code;
        // Distinct synthetic type mapping, kept in runtime WRAM only.
        g.wram[0xe1d + usize::from(code)] = code % 10;
        for sx in [0, 80, 240, 250] {
            for ly in [90, 250] {
                g.ram[0x4e] = sx;
                g.ram[0x2a] = ly;
                let b = *g
                    .collision_geometry()
                    .boxes
                    .iter()
                    .find(|b| b.kind == Kind::Enemy(0))
                    .unwrap();
                g.ram[0xcd] = sx;
                g.cpu.x = 0;
                g.try_call_asm(0xe942, 1000).unwrap();
                assert_eq!(
                    [b.x, b.y, b.w, b.h],
                    [
                        i32::from(g.ram[4]),
                        i32::from(g.ram[5]),
                        i32::from(g.ram[6]) + 1,
                        i32::from(g.ram[7]) + 1
                    ]
                );
            }
        }
    }
}
