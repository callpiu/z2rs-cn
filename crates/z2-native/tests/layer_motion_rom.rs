//! ROM-gated: HD-pack layers that drift (`drift_x`/`drift_y`) and animate
//! (`frames` + `fps`) are presentation only. The same input replayed with
//! and without a presented pack of such layers gives identical RAM and
//! `collision_geometry()` every frame; the layers move with emulated frames
//! and hold still when the same frame is presented again. The pack is
//! synthetic (solid colours in a temporary directory); nothing derived from
//! the ROM is stored.

mod common;

use z2_native::app::{self, Display, DisplaySettings, Emu};

const A: u8 = 0x01;
const SELECT: u8 = 0x04;
const START: u8 = 0x08;
const RIGHT: u8 = 0x80;

/// Palace 1, Horsehead's room.
const ROOM: [(usize, u8); 7] = [
    (0x0706, 0x00),
    (0x0707, 0x03),
    (0x0748, 0x34),
    (0x056C, 0x00),
    (0x075C, 0x00),
    (0x0701, 0x00),
    (0x0561, 0x0D),
];

/// Boot to side-view play in North Castle, then load the room `pokes`
/// names through the game's own loader.
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

/// A band that drifts and repeats, a strip that animates (both front, so
/// they show whatever the room draws).
fn write_pack(dir: &std::path::Path) {
    let _ = std::fs::remove_dir_all(dir);
    std::fs::create_dir_all(dir).unwrap();
    let band: Vec<u8> = (0..8)
        .flat_map(|_| (0..32u8).flat_map(|x| [x * 8, 64, 255, 255]))
        .collect();
    std::fs::write(
        dir.join("band.png"),
        z2_render::encode_png_rgba(32, 8, &band, &[]).unwrap(),
    )
    .unwrap();
    for (name, c) in [("a.png", [255u8, 0, 0, 255]), ("b.png", [0, 255, 0, 255])] {
        let rgba = c.repeat(256 * 8);
        std::fs::write(
            dir.join(name),
            z2_render::encode_png_rgba(256, 8, &rgba, &[]).unwrap(),
        )
        .unwrap();
    }
    let manifest = serde_json::json!({"version": 1, "name": "moving layers", "scale": 1,
    "layers": [
            {"file": "band.png", "depth": "front", "y": 200, "scroll": 30, "repeat_x": true, "drift_x": -24},
        {"frames": ["a.png", "b.png"], "fps": 4, "depth": "front", "scroll": 0, "y": 232}
    ]});
    std::fs::write(dir.join("pack.json"), manifest.to_string()).unwrap();
}

#[test]
fn drifting_and_animated_layers_never_change_the_game() {
    let Some(rom) = common::rom_path("drifting_and_animated_layers") else {
        return;
    };
    let dir = std::env::temp_dir().join(format!("z2-layer-drift-{}", std::process::id()));
    write_pack(&dir);
    let mut display = Display::new(DisplaySettings {
        scale: 1,
        pack_dir: Some(dir.clone()),
        ..DisplaySettings::default()
    })
    .expect("pack display");
    let mut plain = to_scene(&rom, &ROOM);
    let mut packed = to_scene(&rom, &ROOM);
    let (w, _) = display.size();
    let w = w as usize;
    let mut strip = std::collections::BTreeSet::new();
    let mut moved = 0;
    let mut last: Option<Vec<u8>> = None;
    for f in 0..240 {
        let input = if f < 120 { RIGHT } else { 0 };
        app::step_frames(&mut plain, &[input], None);
        app::step_frames(&mut packed, &[input], None);
        let image = display.present(&packed.game).expect("present").to_vec();
        // Presenting the same emulated frame again (a paused game) changes
        // nothing.
        assert_eq!(display.present(&packed.game).expect("present"), &image[..]);
        assert_eq!(packed.game.ram, plain.game.ram, "frame {f}: RAM");
        assert_eq!(
            packed.game.collision_geometry(),
            plain.game.collision_geometry(),
            "frame {f}: collision geometry"
        );
        let at = |x: usize, y: usize| image[(y * w + x) * 4..(y * w + x) * 4 + 4].to_vec();
        strip.insert(at(128, 236));
        if f >= 120 {
            // Link stands still: the drift alone moves the band.
            let band: Vec<u8> = (0..w).flat_map(|x| at(x, 204)).collect();
            if last.as_ref().is_some_and(|l| l != &band) {
                moved += 1;
            }
            last = Some(band);
        }
    }
    let _ = std::fs::remove_dir_all(&dir);
    assert!(
        strip.contains(&vec![255, 0, 0, 255]) && strip.contains(&vec![0, 255, 0, 255]),
        "the front strip animates: {strip:?}"
    );
    assert!(
        moved > 30,
        "the drifting band moved on {moved} still frames"
    );
}
