//! ROM-gated: the camera zoom (`DisplayEnh::camera_zoom`) is presentation
//! only. The same input replayed with and without a zoomed display gives
//! identical RAM and `collision_geometry()` on every frame; the status bar
//! and the boss life meter are drawn exactly as unzoomed; the zoomed play
//! area is magnified around Link. No pack (original graphics); nothing
//! derived from the ROM is stored.

mod common;

use z2_native::app::{self, Display, DisplaySettings, Emu};

const A: u8 = 0x01;
const SELECT: u8 = 0x04;
const START: u8 = 0x08;
const RIGHT: u8 = 0x80;
const B: u8 = 0x02;

/// Palace 1, Horsehead's room (the boss life meter is on screen).
const HORSEHEAD_ROOM: [(usize, u8); 7] = [
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

fn display(zoom: f32, wide_tiles: u8, scale: u32) -> Display {
    Display::new(DisplaySettings {
        scale,
        wide_tiles,
        fill_left_clip: true,
        fill_right_clip: true,
        display_enh: z2_core::enh::DisplayEnh {
            camera_zoom: zoom,
            ..Default::default()
        },
        ..DisplaySettings::default()
    })
    .expect("display")
}

fn px(img: &[u8], w: usize, x: usize, y: usize) -> [u8; 4] {
    img[(y * w + x) * 4..(y * w + x) * 4 + 4]
        .try_into()
        .unwrap()
}

#[test]
fn camera_zoom_never_changes_the_game_and_keeps_the_hud_unzoomed() {
    let Some(rom) = common::rom_path("camera_zoom_never_changes_the_game") else {
        return;
    };
    let mut plain = to_scene(&rom, &HORSEHEAD_ROOM);
    let mut zoomed = to_scene(&rom, &HORSEHEAD_ROOM);
    let mut flat = display(1.0, 0, 1);
    let mut zoom = display(2.0, 0, 1);
    let (w, h) = zoom.size();
    assert_eq!((w, h), flat.size(), "zoom keeps the presented size");
    let (w, h) = (w as usize, h as usize);
    let (mut meter_frames, mut magnified) = (0, 0);
    for f in 0..500 {
        let input = match f {
            0..250 => RIGHT,
            _ if f % 30 < 4 => B,
            _ => 0,
        };
        app::step_frames(&mut plain, &[input], None);
        app::step_frames(&mut zoomed, &[input], None);
        flat.update_actor_animations(&zoomed.game);
        zoom.update_actor_animations(&zoomed.game);
        let one = flat.present(&zoomed.game).expect("present").to_vec();
        let two = zoom.present(&zoomed.game).expect("present").to_vec();
        assert_eq!(zoomed.game.ram, plain.game.ram, "frame {f}: RAM");
        assert_eq!(
            zoomed.game.collision_geometry(),
            plain.game.collision_geometry(),
            "frame {f}: collision geometry"
        );
        if zoomed.game.ram[0x0736] != 0x0B {
            continue;
        }
        // The status bar is drawn unzoomed, pixel for pixel.
        for y in 0..32 {
            for x in 0..w {
                assert_eq!(
                    px(&two, w, x, y),
                    px(&one, w, x, y),
                    "frame {f}: HUD ({x}, {y})"
                );
            }
        }
        // So is every opaque pixel of the boss life meter (X 16).
        let cells = z2_native::camera_zoom::life_meter_cells(&zoomed.game, 0);
        for c in &cells {
            for (r, bits) in c.rows.iter().enumerate() {
                for b in 0..8 {
                    if bits & (0x80 >> b) != 0 {
                        let (x, y) = ((c.x + b) as usize, (c.y + r as i32) as usize);
                        assert_eq!(px(&two, w, x, y), px(&one, w, x, y), "frame {f}: meter");
                    }
                }
            }
        }
        meter_frames += usize::from(!cells.is_empty());
        // The play area is magnified: each 2x2 block below the HUD is one
        // colour.
        let uniform = (32..h - 1).step_by(2).all(|y| {
            (0..w - 1).step_by(2).all(|x| {
                [(1, 0), (0, 1), (1, 1)]
                    .iter()
                    .all(|&(dx, dy)| px(&two, w, x + dx, y + dy) == px(&two, w, x, y))
            })
        });
        magnified += usize::from(uniform && one != two);
    }
    assert!(meter_frames > 100, "boss meter seen {meter_frames} frames");
    assert!(magnified > 100, "magnified frames {magnified}");
}

#[test]
fn present_size_maps_each_source_pixel_to_an_exact_block() {
    let Some(rom) = common::rom_path("present_size_exact_blocks") else {
        return;
    };
    let mut emu = to_scene(&rom, &HORSEHEAD_ROOM);
    // 4x output, 16:9 widescreen (11 tiles), zoom 2, 2560x1440: 3x3 blocks.
    let mut d = Display::new(DisplaySettings {
        scale: 4,
        wide_tiles: 11,
        fill_left_clip: true,
        fill_right_clip: true,
        display_enh: z2_core::enh::DisplayEnh {
            camera_zoom: 2.0,
            ..Default::default()
        },
        output_size: Some((2560, 1440)),
        ..DisplaySettings::default()
    })
    .expect("display");
    assert_eq!(d.size(), (2560, 1440));
    for f in 0..200 {
        app::step_frames(&mut emu, &[if f < 150 { RIGHT } else { 0 }], None);
        d.update_actor_animations(&emu.game);
    }
    let img = d.present(&emu.game).expect("present").to_vec();
    assert_eq!(img.len(), 2560 * 1440 * 4);
    let hud = 32 * 6;
    for y in (hud..1440).step_by(3) {
        for x in (0..2559).step_by(3) {
            let c = px(&img, 2560, x, y);
            for (dx, dy) in [(1, 0), (2, 0), (0, 1), (1, 2), (2, 2)] {
                if x + dx < 2560 && y + dy < 1440 {
                    assert_eq!(px(&img, 2560, x + dx, y + dy), c, "block at ({x}, {y})");
                }
            }
        }
    }
}
