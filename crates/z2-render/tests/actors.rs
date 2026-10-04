//! Synthetic actors only; no private graphics or ROM-derived fixtures.
mod common;
use serde_json::json;
use z2_ppu::record::SpriteRef;
use z2_ppu::{FrameRecord, HEIGHT, WIDTH};
use z2_render::{ActorInput, HdPack, PresentConfig, Presenter};

fn pack(looped: bool) -> HdPack {
    let entries = ["hero", "friend"].map(|character| {
        json!({
            "character":character,"name":"move","fps":60,"looped":looped,
            "frames":[{"file":"a.png","pivot":[1,1]},{"file":"b.png","pivot":[1,1]}]
        })
    });
    let manifest = json!({"version":1,"name":"synthetic actors","scale":1,"actor_clips":entries});
    HdPack::from_files(&common::files(vec![
        ("pack.json", manifest.to_string().into_bytes()),
        ("a.png", common::solid_png(3, 3, [255, 0, 0, 255])),
        ("b.png", common::solid_png(3, 3, [0, 255, 0, 255])),
    ]))
    .unwrap()
}

fn record(visible: bool) -> FrameRecord {
    let mut r = FrameRecord::new();
    for y in 0..HEIGHT {
        r.lines[y].valid = true;
        r.lines[y].mask = 0x1e;
        r.lines[y].sprite_start = r.sprites.len() as u16;
        if visible && y == 30 {
            for slot in [2, 56] {
                r.sprites.push(SpriteRef {
                    oam_index: slot,
                    x: 20,
                    page: 0,
                    tile: 0,
                    ..Default::default()
                });
                r.lines[y].sprite_len += 1;
            }
        }
    }
    r
}

fn input(id: u8, clip: &str) -> ActorInput<'_> {
    let slot = if id == 0 { 2 } else { 56 };
    ActorInput {
        id,
        character: if id == 0 { "hero" } else { "friend" },
        clip,
        position: [20.0 + f32::from(id) * 10.0, 30.0],
        flip_h: false,
        oam_mask: 1u64 << slot,
        priority_oam: slot,
    }
}

fn pixel(p: &mut Presenter, r: &FrameRecord, x: usize, y: usize) -> [u8; 4] {
    let frame = [0u8; WIDTH * HEIGHT];
    let rgba = p.present(&frame, r, &[0; 4096]).unwrap();
    rgba[(y * WIDTH + x) * 4..(y * WIDTH + x) * 4 + 4]
        .try_into()
        .unwrap()
}

#[test]
fn clocks_are_independent_pause_safe_looping_and_seek_safe() {
    let mut p = Presenter::new(PresentConfig::default()).unwrap();
    p.set_pack(Some(pack(true))).unwrap();
    let r = record(true);
    p.set_actors(10, 60.0, &[input(0, "move")]);
    assert_eq!(pixel(&mut p, &r, 20, 29), [255, 0, 0, 255]);
    p.set_actors(11, 60.0, &[input(0, "move"), input(1, "move")]);
    assert_eq!(pixel(&mut p, &r, 20, 29), [0, 255, 0, 255]);
    assert_eq!(pixel(&mut p, &r, 30, 29), [255, 0, 0, 255]);
    assert_eq!(pixel(&mut p, &r, 20, 29), [0, 255, 0, 255]);
    p.set_actors(12, 60.0, &[input(0, "move")]);
    assert_eq!(pixel(&mut p, &r, 20, 29), [255, 0, 0, 255]);
    p.set_actors(5, 60.0, &[input(0, "move")]);
    assert_eq!(pixel(&mut p, &r, 20, 29), [255, 0, 0, 255]);
}

#[test]
fn one_shots_hold_and_hidden_oam_never_leaves_a_ghost() {
    let mut p = Presenter::new(PresentConfig::default()).unwrap();
    p.set_pack(Some(pack(false))).unwrap();
    p.set_actors(0, 60.0, &[input(0, "move")]);
    p.set_actors(600, 60.0, &[input(0, "move")]);
    assert_eq!(pixel(&mut p, &record(true), 20, 29), [0, 255, 0, 255]);
    assert_ne!(pixel(&mut p, &record(false), 20, 29), [0, 255, 0, 255]);
}

#[test]
fn malformed_clips_are_rejected_by_the_pack_loader() {
    for entry in [
        json!({"character":"h","name":"m","fps":0,"frames":[]}),
        json!({"character":"h","name":"m","fps":60,"frames":[{"file":"../outside.png","pivot":[0,0]}]}),
    ] {
        let manifest = json!({"version":1,"name":"bad","scale":1,"actor_clips":[entry]});
        let error = HdPack::from_files(&common::files(vec![(
            "pack.json",
            manifest.to_string().into_bytes(),
        )]))
        .unwrap_err();
        assert!(error.to_string().contains("actor_clips"));
    }
}

#[test]
fn trimmed_pivots_mirror_about_the_actor_origin() {
    let manifest = json!({"version":1,"name":"trimmed pivot","scale":1,"actor_clips":[{
        "character":"hero","name":"move","fps":60,
        "frames":[{"file":"stripe.png","pivot":[1,1]}]
    }]});
    let rgba = [255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255];
    let pack = HdPack::from_files(&common::files(vec![
        ("pack.json", manifest.to_string().into_bytes()),
        (
            "stripe.png",
            z2_render::encode_png_rgba(3, 1, &rgba, &[]).unwrap(),
        ),
    ]))
    .unwrap();
    let mut p = Presenter::new(PresentConfig::default()).unwrap();
    p.set_pack(Some(pack)).unwrap();
    let mut actor = input(0, "move");
    let r = record(true);
    p.set_actors(0, 60.0, &[actor]);
    assert_eq!(pixel(&mut p, &r, 19, 29), [255, 0, 0, 255]);
    assert_eq!(pixel(&mut p, &r, 21, 29), [0, 0, 255, 255]);
    actor.flip_h = true;
    p.set_actors(0, 60.0, &[actor]);
    assert_eq!(pixel(&mut p, &r, 18, 29), [0, 0, 255, 255]);
    assert_eq!(pixel(&mut p, &r, 20, 29), [255, 0, 0, 255]);
}

#[test]
fn transparent_actor_replaces_the_entire_original_sprite_and_respects_oam_priority() {
    let mut p = Presenter::new(PresentConfig::default()).unwrap();
    p.set_pack(Some(pack(false))).unwrap();
    let mut r = record(true);
    r.sprites.truncate(1);
    r.lines[30].sprite_len = 1;
    r.lines[30].palette[0x11] = 1;
    // A filled procedural NES tile. No cartridge data is involved.
    let mut chr = [0; 4096];
    chr[..8].fill(255);
    let mut indexed = [0u8; WIDTH * HEIGHT];
    indexed[30 * WIDTH + 20..30 * WIDTH + 28].fill(1);
    let backdrop = z2_ppu::NES_PALETTE_RGB[usize::from(r.lines[30].backdrop)];
    p.set_actors(0, 60.0, &[input(0, "move")]);
    let image = p.present(&indexed, &r, &chr).unwrap();
    assert_eq!(
        &image[(30 * WIDTH + 27) * 4..(30 * WIDTH + 27) * 4 + 4],
        &[backdrop[0], backdrop[1], backdrop[2], 255]
    );
    assert_eq!(
        &image[(30 * WIDTH + 20) * 4..(30 * WIDTH + 20) * 4 + 4],
        &[255, 0, 0, 255]
    );
    // Another actor overlaps P1. Lower OAM index wins independently of input order.
    r.sprites.push(SpriteRef {
        oam_index: 56,
        x: 20,
        page: 0,
        tile: 0,
        ..Default::default()
    });
    r.lines[30].sprite_len = 2;
    let mut other = input(1, "move");
    other.position = [20.0, 30.0];
    p.set_actors(0, 60.0, &[input(0, "move")]);
    p.set_actors(1, 60.0, &[other, input(0, "move")]);
    assert_eq!(pixel(&mut p, &r, 20, 29), [0, 255, 0, 255]);
    // OAM overflow can drop the first body cell while other body cells remain.
    r.sprites[0].oam_index = 3;
    let mut partial = input(0, "move");
    partial.oam_mask |= 1 << 3;
    p.set_actors(1, 60.0, &[partial]);
    assert_eq!(pixel(&mut p, &r, 20, 29), [0, 255, 0, 255]);
}

#[test]
fn background_priority_applies_beyond_the_original_body_rows() {
    let mut p = Presenter::new(PresentConfig::default()).unwrap();
    p.set_pack(Some(pack(false))).unwrap();
    let mut r = record(true);
    for line in &mut r.lines[..] {
        line.tiles.fill(z2_ppu::BgTileId {
            page: 0,
            tile: 0,
            fetched: true,
            ..z2_ppu::BgTileId::NONE
        });
    }
    r.sprites[0].behind = true;
    let mut chr = [0; 4096];
    chr[..8].fill(255);
    let indexed = [0u8; WIDTH * HEIGHT];
    p.set_actors(0, 60.0, &[input(0, "move")]);
    let image = p.present(&indexed, &r, &chr).unwrap();
    assert_ne!(
        &image[(29 * WIDTH + 20) * 4..(29 * WIDTH + 20) * 4 + 4],
        &[255, 0, 0, 255]
    );
    r.sprites[0].behind = false;
    p.set_actors(0, 60.0, &[input(0, "move")]);
    let image = p.present(&indexed, &r, &chr).unwrap();
    assert_eq!(
        &image[(29 * WIDTH + 20) * 4..(29 * WIDTH + 20) * 4 + 4],
        &[255, 0, 0, 255]
    );
}
