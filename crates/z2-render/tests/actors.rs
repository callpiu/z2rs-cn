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

/// One CHR tile, one pack holding both a sheet replacement for it and a whole
/// actor clip: the shape of every pack that ships the animated Link next to
/// the older tile-by-tile Link in its sheets.
fn sheet_and_actor_pack(frame_file: &str, files: Vec<(&str, Vec<u8>)>) -> HdPack {
    let manifest = json!({"version":1,"name":"sheets + actors","scale":1,
        "sheets":[{"file":"sheets/s.png","layout":"free"}],
        "tiles":[{"page":0,"tile":0,"sheet":0,"x":0,"y":0}],
        "actor_clips":[{"character":"hero","name":"move","fps":60,
            "frames":[{"file":frame_file,"pivot":[1,1]}]}]});
    let mut list = vec![
        ("pack.json", manifest.to_string().into_bytes()),
        ("sheets/s.png", common::solid_png(8, 8, [0, 0, 255, 255])),
    ];
    list.extend(files);
    HdPack::from_files(&common::files(list)).unwrap()
}

/// The original sprite row the sheet tile replaces, under the actor's origin.
fn sheet_scene() -> (FrameRecord, [u8; 4096], [u8; WIDTH * HEIGHT]) {
    let mut r = record(true);
    r.sprites.truncate(1);
    r.lines[30].sprite_len = 1;
    r.lines[30].palette[0x11] = 1;
    let mut chr = [0; 4096];
    chr[..8].fill(255);
    let mut indexed = [0u8; WIDTH * HEIGHT];
    indexed[30 * WIDTH + 20..30 * WIDTH + 28].fill(1);
    (r, chr, indexed)
}

fn rgba_at(image: &[u8], x: usize, y: usize) -> [u8; 4] {
    image[(y * WIDTH + x) * 4..(y * WIDTH + x) * 4 + 4]
        .try_into()
        .unwrap()
}

#[test]
fn actor_frames_take_precedence_over_sheet_tiles_of_the_same_sprite() {
    let pack = sheet_and_actor_pack(
        "actors/hero/move/0000.png",
        vec![(
            "actors/hero/move/0000.png",
            common::solid_png(3, 3, [255, 0, 0, 255]),
        )],
    );
    assert_eq!(pack.tile_count(), 1);
    assert_eq!(pack.actor_clips().len(), 1);
    let mut p = Presenter::new(PresentConfig::default()).unwrap();
    p.set_pack(Some(pack)).unwrap();
    let (r, chr, indexed) = sheet_scene();
    let backdrop = z2_ppu::NES_PALETTE_RGB[usize::from(r.lines[30].backdrop)];
    let backdrop = [backdrop[0], backdrop[1], backdrop[2], 255];

    // No actor this frame: the sheet tile draws the sprite.
    p.set_actors(0, 60.0, &[]);
    let image = p.present(&indexed, &r, &chr).unwrap().to_vec();
    assert_eq!(rgba_at(&image, 20, 30), [0, 0, 255, 255]);
    assert_eq!(rgba_at(&image, 25, 30), [0, 0, 255, 255]);

    // The actor clip plays: its frame is drawn and the sheet tile of a sprite
    // it replaces is not, even outside the frame's own pixels.
    p.set_actors(1, 60.0, &[input(0, "move")]);
    let image = p.present(&indexed, &r, &chr).unwrap().to_vec();
    assert_eq!(rgba_at(&image, 20, 30), [255, 0, 0, 255]);
    assert_eq!(rgba_at(&image, 25, 30), backdrop);

    // A pose the pack has no clip for falls back to the sheet tile.
    p.set_actors(2, 60.0, &[input(0, "not_in_pack")]);
    let image = p.present(&indexed, &r, &chr).unwrap().to_vec();
    assert_eq!(rgba_at(&image, 20, 30), [0, 0, 255, 255]);
}

#[test]
fn windows_style_frame_paths_name_the_same_files() {
    // A manifest written on Windows, handed over with backslash-separated
    // picker paths under the pack's own folder.
    for (frame_file, stored) in [
        (
            "actors\\hero\\move\\0000.png",
            "My Pack\\actors\\hero\\move\\0000.png",
        ),
        (
            "./actors/hero/move/0000.png",
            "My Pack/actors/hero/move/0000.png",
        ),
        (
            "actors/hero//move/0000.png",
            "My Pack\\actors/hero\\move/0000.png",
        ),
    ] {
        let manifest = json!({"version":1,"name":"win","scale":1,"actor_clips":[{
            "character":"hero","name":"move","fps":60,
            "frames":[{"file":frame_file,"pivot":[1,1]}]}]});
        let pack = HdPack::from_files(&[
            (
                "My Pack\\pack.json".to_string(),
                manifest.to_string().into_bytes(),
            ),
            (
                stored.to_string(),
                common::solid_png(3, 3, [255, 0, 0, 255]),
            ),
        ])
        .unwrap_or_else(|e| panic!("{frame_file}: {e}"));
        assert_eq!(pack.actor_clips()[0].frames.len(), 1, "{frame_file}");
    }
}

#[test]
fn frame_paths_may_not_leave_the_pack() {
    for bad in [
        "C:\\packs\\a.png",
        "C:/packs/a.png",
        "\\a.png",
        "/a.png",
        "actors\\..\\..\\a.png",
        "actors/link:stream.png",
        "",
    ] {
        let manifest = json!({"version":1,"name":"bad","scale":1,"actor_clips":[{
            "character":"hero","name":"move","fps":60,
            "frames":[{"file":bad,"pivot":[1,1]}]}]});
        let error = HdPack::from_files(&common::files(vec![
            ("pack.json", manifest.to_string().into_bytes()),
            ("a.png", common::solid_png(1, 1, [255, 0, 0, 255])),
        ]))
        .unwrap_err();
        assert!(
            error.to_string().contains("invalid relative frame path"),
            "{bad}: {error}"
        );
    }
}

#[test]
fn native_pack_directories_load_actor_frames_from_nested_folders() {
    let dir = std::env::temp_dir().join(format!(
        "z2-render-actor-dir-{}-{}",
        std::process::id(),
        line!()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    let frames = dir.join("actors").join("link").join("attack");
    std::fs::create_dir_all(&frames).unwrap();
    std::fs::create_dir_all(dir.join("sheets")).unwrap();
    std::fs::write(
        frames.join("0000.png"),
        common::solid_png(3, 3, [255, 0, 0, 255]),
    )
    .unwrap();
    std::fs::write(
        dir.join("sheets").join("s.png"),
        common::solid_png(8, 8, [0, 0, 255, 255]),
    )
    .unwrap();
    // Backslash separators, as a manifest written on Windows has them.
    let manifest = json!({"version":1,"name":"dir","scale":1,
        "sheets":[{"file":"sheets\\s.png","layout":"free"}],
        "tiles":[{"page":0,"tile":0,"sheet":"sheets/s.png","x":0,"y":0}],
        "actor_clips":[{"character":"link","name":"attack","fps":60,
            "frames":[{"file":"actors\\link\\attack\\0000.png","pivot":[1,1]}]}]});
    std::fs::write(dir.join("pack.json"), manifest.to_string()).unwrap();
    let pack = z2_render::fs::load_pack_dir(&dir);
    let _ = std::fs::remove_dir_all(&dir);
    let pack = pack.unwrap();
    assert_eq!(pack.tile_count(), 1);
    assert_eq!(pack.actor_clips().len(), 1);
    assert_eq!(pack.actor_clips()[0].frames[0].image.width, 3);
}
