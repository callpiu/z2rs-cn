//! ROM-gated: an HD pack with enemy actor clips changes the picture only.
//! The same input replayed with and without a presented pack gives identical
//! RAM and `collision_geometry()` every frame, and the enemy's clip frame is
//! drawn on its ground anchor. The pack is synthetic (solid colours written
//! to a temporary directory); nothing derived from the ROM is stored.

mod common;

use z2_native::app::{self, Display, DisplaySettings, Emu};

const A: u8 = 0x01;
const SELECT: u8 = 0x04;
const START: u8 = 0x08;
const RIGHT: u8 = 0x80;

/// Boot to side-view play in North Castle, then load palace 3's Iron
/// Knuckle room through the game's own loader.
fn to_room(rom: &std::path::Path) -> Emu {
    to_scene(rom, &IRON_KNUCKLE_ROOM)
}

/// Palace 3, a red Iron Knuckle and Myus.
const IRON_KNUCKLE_ROOM: [(usize, u8); 7] = [
    (0x0706, 0x00),
    (0x0707, 0x04),
    (0x0748, 0x36),
    (0x056C, 0x02),
    (0x075C, 0x00),
    (0x0701, 0x00),
    (0x0561, 0x05),
];

/// Palace 1, Tinsuits.
const TINSUIT_ROOM: [(usize, u8); 7] = [
    (0x0706, 0x00),
    (0x0707, 0x03),
    (0x0748, 0x34),
    (0x056C, 0x00),
    (0x075C, 0x02),
    (0x0701, 0x00),
    (0x0561, 0x01),
];

/// Boot as [`to_room`], then load the room `pokes` names.
fn to_scene(rom: &std::path::Path, pokes: &[(usize, u8)]) -> Emu {
    let mut emu = app::emu_from_rom_file(rom, 44_100).expect("rom loads");
    // HD packs compose from the PPU render record; arm it in both runs.
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
    for &(a, v) in pokes.iter().chain(&[(0x0736, 0x00)]) {
        emu.game.ram[a] = v;
    }
    app::step_frames(&mut emu, &[0; 150], None);
    assert_eq!(emu.game.ram[0x0736], 0x0B, "room loaded");
    emu
}

/// 16x32 magenta frames for every clip the room's enemies select.
fn write_pack(dir: &std::path::Path) {
    let _ = std::fs::remove_dir_all(dir);
    std::fs::create_dir_all(dir).unwrap();
    let rgba = [255u8, 0, 255, 255].repeat(16 * 32);
    std::fs::write(
        dir.join("f.png"),
        z2_render::encode_png_rgba(16, 32, &rgba, &[]).unwrap(),
    )
    .unwrap();
    let mut clips = Vec::new();
    for character in ["enemy_b4b_19", "enemy_b4b_03", "enemy_b4_0c"] {
        for name in ["idle", "walk", "attack", "jump", "hurt", "death"] {
            clips.push(serde_json::json!({"character": character, "name": name,
                "fps": 60, "looped": true, "frames": [{"file": "f.png", "pivot": [8, 32]}]}));
        }
    }
    let manifest = serde_json::json!({"version": 1, "name": "synthetic enemies",
        "scale": 1, "actor_clips": clips});
    std::fs::write(dir.join("pack.json"), manifest.to_string()).unwrap();
}

#[test]
fn enemy_actor_packs_never_change_collisions() {
    let Some(rom) = common::rom_path("enemy_actor_packs_never_change_collisions") else {
        return;
    };
    let dir = std::env::temp_dir().join(format!("z2-actor-pack-{}", std::process::id()));
    write_pack(&dir);
    let mut display = Display::new(DisplaySettings {
        scale: 1,
        pack_dir: Some(dir.clone()),
        ..DisplaySettings::default()
    })
    .expect("pack display");
    let mut plain = to_room(&rom);
    let mut packed = to_room(&rom);
    let mut replaced = 0;
    for f in 0..240 {
        let input = if f < 100 { RIGHT } else { 0 };
        app::step_frames(&mut plain, &[input], None);
        app::step_frames(&mut packed, &[input], None);
        display.update_actor_animations(&packed.game);
        let image = display.present(&packed.game).expect("present").to_vec();
        assert_eq!(packed.game.ram, plain.game.ram, "frame {f}: RAM");
        assert_eq!(
            packed.game.collision_geometry(),
            plain.game.collision_geometry(),
            "frame {f}: collision geometry"
        );
        let (w, _) = display.size();
        for v in packed.game.actor_visuals() {
            if v.character != "enemy_b4b_19" {
                continue;
            }
            // The frame's bottom centre (pivot 8,32) is at the anchor.
            let [x, y] = v.position.map(|p| p as usize);
            if (1..255).contains(&x) && (1..240).contains(&y) {
                let at = ((y - 1) * w as usize + x) * 4;
                assert_eq!(&image[at..at + 4], &[255, 0, 255, 255], "frame {f}");
                replaced += 1;
            }
        }
    }
    let _ = std::fs::remove_dir_all(&dir);
    assert!(
        replaced > 100,
        "the Iron Knuckle was replaced: {replaced} frames"
    );
}

/// Killing enemies with a pack that replaces them (hurt flash, death clip,
/// the dying body's spilled cells, score popups left as tiles) still leaves
/// RAM and collision geometry identical to the run without the pack.
#[test]
fn deaths_and_hit_flashes_never_change_collisions() {
    let Some(rom) = common::rom_path("deaths_and_hit_flashes_never_change_collisions") else {
        return;
    };
    let dir = std::env::temp_dir().join(format!("z2-actor-death-{}", std::process::id()));
    write_pack(&dir);
    let mut display = Display::new(DisplaySettings {
        scale: 1,
        pack_dir: Some(dir.clone()),
        ..DisplaySettings::default()
    })
    .expect("pack display");
    let mut plain = to_scene(&rom, &TINSUIT_ROOM);
    let mut packed = to_scene(&rom, &TINSUIT_ROOM);
    let (mut deaths, mut flashes) = (0, 0);
    for f in 0..640 {
        // Face left and stab, as the art gate's kill runs do.
        let input = match f % 16 {
            0..6 => 0x40,
            6..10 => 0x02,
            _ => 0,
        };
        if f == 40 {
            // Hit stun on every live enemy (both runs), as the art gate's
            // hurt runs poke it: the cartridge flashes their palettes.
            for emu in [&mut plain, &mut packed] {
                for s in 0..6 {
                    if emu.game.ram[0xB6 + s] == 1 {
                        emu.game.ram[0x040E + s] = 0x10;
                    }
                }
            }
        }
        app::step_frames(&mut plain, &[input], None);
        app::step_frames(&mut packed, &[input], None);
        display.update_actor_animations(&packed.game);
        display.present(&packed.game).expect("present");
        assert_eq!(packed.game.ram, plain.game.ram, "frame {f}: RAM");
        assert_eq!(
            packed.game.collision_geometry(),
            plain.game.collision_geometry(),
            "frame {f}: collision geometry"
        );
        for v in packed.game.actor_visuals() {
            deaths += usize::from(v.clip == "death");
            flashes += usize::from(v.flash_palette.is_some());
        }
    }
    let _ = std::fs::remove_dir_all(&dir);
    assert!(
        deaths > 20 && flashes > 0,
        "deaths {deaths}, flashes {flashes}"
    );
}
