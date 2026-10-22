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
        ..Default::default()
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
fn decoded_frames_from_before_a_state_load_can_be_evicted() {
    let mut p = Presenter::new(PresentConfig::default()).unwrap();
    let mut pk = pack(true);
    // Room for one 3x3 frame.
    pk.actor_library_mut().set_decode_budget(36);
    p.set_pack(Some(pk)).unwrap();
    p.set_actors(1000, 60.0, &[input(1, "move")]);
    // A save state from frame 10 is loaded: the frame clock goes back.
    p.set_actors(10, 60.0, &[input(0, "move")]);
    let resident = p.pack().unwrap().actor_library().resident_bytes();
    assert_eq!(
        resident, 36,
        "frames last shown at frame 1000 are evicted once the clock goes back"
    );
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
    assert_eq!(pack.actor_clips()[0].frames[0].width, 3);
}

/// Link's body at OAM 2 and an enemy cell at OAM 32 (the third enemy
/// block) on line 30; the enemy cell is a filled procedural tile.
fn link_and_enemy_record() -> (FrameRecord, [u8; 4096], [u8; WIDTH * HEIGHT]) {
    let mut r = FrameRecord::new();
    for y in 0..HEIGHT {
        r.lines[y].valid = true;
        r.lines[y].mask = 0x1e;
        r.lines[y].sprite_start = r.sprites.len() as u16;
        if y == 30 {
            for (slot, x) in [(2u8, 20u8), (32, 100)] {
                r.sprites.push(SpriteRef {
                    oam_index: slot,
                    x,
                    page: 0,
                    tile: 0,
                    ..Default::default()
                });
                r.lines[y].sprite_len += 1;
            }
            r.lines[y].palette[0x11] = 1;
        }
    }
    let mut chr = [0; 4096];
    chr[..8].fill(255);
    let mut indexed = [0u8; WIDTH * HEIGHT];
    indexed[30 * WIDTH + 100..30 * WIDTH + 108].fill(1);
    (r, chr, indexed)
}

fn enemy_pack(with_enemy: bool) -> HdPack {
    let mut clips = vec![json!({"character":"link","name":"idle","fps":60,
        "frames":[{"file":"link.png","pivot":[1,2]}]})];
    if with_enemy {
        // Pivot at the bottom centre of a 4x3 frame, as an exporter writes it.
        clips.push(json!({"character":"enemy_b4_20","name":"walk","fps":60,
            "frames":[{"file":"enemy.png","pivot":[3,3]}]}));
    }
    let manifest = json!({"version":1,"name":"enemies","scale":1,"actor_clips":clips});
    let rgba: Vec<u8> = (0..12)
        .flat_map(|i| {
            if i % 4 == 0 {
                [0, 0, 255, 255]
            } else {
                [255, 0, 255, 255]
            }
        })
        .collect();
    HdPack::from_files(&common::files(vec![
        ("pack.json", manifest.to_string().into_bytes()),
        ("link.png", common::solid_png(3, 3, [255, 0, 0, 255])),
        (
            "enemy.png",
            z2_render::encode_png_rgba(4, 3, &rgba, &[]).unwrap(),
        ),
    ]))
    .unwrap()
}

fn link_input() -> ActorInput<'static> {
    ActorInput {
        id: 0,
        character: "link",
        clip: "idle",
        position: [21.0, 31.0],
        flip_h: false,
        oam_mask: 1 << 2,
        priority_oam: 2,
        ..Default::default()
    }
}

fn enemy_input(flip_h: bool) -> ActorInput<'static> {
    ActorInput {
        id: 10,
        character: "enemy_b4_20",
        clip: "walk",
        // Ground anchor: column centre, last row of the bottom cell.
        position: [104.0, 31.0],
        flip_h,
        oam_mask: 0x3f << 32,
        priority_oam: 32,
        ..Default::default()
    }
}

#[test]
fn enemy_characters_replace_only_their_own_oam_and_keep_link_identical() {
    let (r, chr, indexed) = link_and_enemy_record();
    let backdrop = z2_ppu::NES_PALETTE_RGB[usize::from(r.lines[30].backdrop)];
    let backdrop = [backdrop[0], backdrop[1], backdrop[2], 255];
    let mut only_link = Presenter::new(PresentConfig::default()).unwrap();
    only_link.set_pack(Some(enemy_pack(true))).unwrap();
    only_link.set_actors(0, 60.0, &[link_input()]);
    let base = only_link.present(&indexed, &r, &chr).unwrap().to_vec();
    // The enemy keeps its tile art while it has no actor input.
    assert_ne!(rgba_at(&base, 100, 30), backdrop);

    let mut both = Presenter::new(PresentConfig::default()).unwrap();
    both.set_pack(Some(enemy_pack(true))).unwrap();
    both.set_actors(0, 60.0, &[link_input(), enemy_input(false)]);
    let image = both.present(&indexed, &r, &chr).unwrap().to_vec();
    // Link's pixels are unchanged by the enemy actor.
    for y in 26..32 {
        for x in 0..60 {
            assert_eq!(rgba_at(&image, x, y), rgba_at(&base, x, y), "({x}, {y})");
        }
    }
    // The enemy frame sits on its anchor (pivot 3,3 -> top left 101,28)...
    assert_eq!(rgba_at(&image, 101, 28), [0, 0, 255, 255]);
    assert_eq!(rgba_at(&image, 104, 30), [255, 0, 255, 255]);
    // ... and its original cell is gone, also outside the frame.
    assert_eq!(rgba_at(&image, 106, 30), backdrop);
    // Mirroring about the anchor: pixel 104 - 3 + c moves to 104 + 3 - 1 - c.
    both.set_actors(1, 60.0, &[link_input(), enemy_input(true)]);
    let image = both.present(&indexed, &r, &chr).unwrap().to_vec();
    assert_eq!(rgba_at(&image, 106, 28), [0, 0, 255, 255]);
    assert_eq!(rgba_at(&image, 103, 28), [255, 0, 255, 255]);
    assert_ne!(rgba_at(&image, 102, 28), [255, 0, 255, 255]);
}

#[test]
fn enemies_without_clips_fall_back_to_tile_art() {
    let (r, chr, indexed) = link_and_enemy_record();
    let mut p = Presenter::new(PresentConfig::default()).unwrap();
    p.set_pack(Some(enemy_pack(false))).unwrap();
    p.set_actors(0, 60.0, &[link_input()]);
    let base = p.present(&indexed, &r, &chr).unwrap().to_vec();
    p.set_actors(1, 60.0, &[link_input(), enemy_input(false)]);
    assert_eq!(p.present(&indexed, &r, &chr).unwrap(), &base[..]);
    // A known character with an unknown clip falls back too.
    let mut p = Presenter::new(PresentConfig::default()).unwrap();
    p.set_pack(Some(enemy_pack(true))).unwrap();
    p.set_actors(0, 60.0, &[link_input()]);
    let base = p.present(&indexed, &r, &chr).unwrap().to_vec();
    let mut attack = enemy_input(false);
    attack.clip = "attack";
    p.set_actors(1, 60.0, &[link_input(), attack]);
    assert_eq!(p.present(&indexed, &r, &chr).unwrap(), &base[..]);
}

/// A pack with the plain enemy key's `walk` and `attack` and a qualified
/// key's `walk`, each a solid 4x3 colour.
fn fallback_pack() -> HdPack {
    let clip = |character: &str, name: &str, file: &str| {
        json!({"character":character,"name":name,"fps":60,"looped":true,
            "frames":[{"file":file,"pivot":[3,3]}]})
    };
    let clips = vec![
        clip("link", "idle", "link.png"),
        clip("enemy_b4_21", "walk", "plain_walk.png"),
        clip("enemy_b4_21", "attack", "plain_attack.png"),
        clip("enemy_b4_21_p5", "walk", "gooma_walk.png"),
    ];
    let manifest = json!({"version":1,"name":"fallbacks","scale":1,"actor_clips":clips});
    HdPack::from_files(&common::files(vec![
        ("pack.json", manifest.to_string().into_bytes()),
        ("link.png", common::solid_png(3, 3, [255, 0, 0, 255])),
        ("plain_walk.png", common::solid_png(4, 3, [0, 255, 0, 255])),
        (
            "plain_attack.png",
            common::solid_png(4, 3, [0, 0, 255, 255]),
        ),
        (
            "gooma_walk.png",
            common::solid_png(4, 3, [255, 255, 0, 255]),
        ),
    ]))
    .unwrap()
}

#[test]
fn qualified_keys_and_new_clips_fall_back_to_older_names() {
    let (r, chr, indexed) = link_and_enemy_record();
    let mut p = Presenter::new(PresentConfig::default()).unwrap();
    p.set_pack(Some(fallback_pack())).unwrap();
    let enemy = |character, clip, clip_fallback| ActorInput {
        character,
        clip,
        character_fallback: "enemy_b4_21",
        clip_fallback,
        ..enemy_input(false)
    };
    let mut colour = |frame, input: ActorInput<'_>| {
        p.set_actors(frame, 60.0, &[link_input(), input]);
        rgba_at(p.present(&indexed, &r, &chr).unwrap(), 101, 28)
    };
    // The qualified key's own clip.
    assert_eq!(
        colour(0, enemy("enemy_b4_21_p5", "walk", "")),
        [255, 255, 0, 255]
    );
    // A qualified key without clips uses the plain key.
    assert_eq!(
        colour(1, enemy("enemy_b4_21_p2", "walk", "")),
        [0, 255, 0, 255]
    );
    // A new clip name falls back to the older one, on the plain key too.
    assert_eq!(
        colour(2, enemy("enemy_b4_21_p2", "attack_high", "attack")),
        [0, 0, 255, 255]
    );
    // Nothing matches: the original tile art stays.
    let mut tiles = Presenter::new(PresentConfig::default()).unwrap();
    tiles.set_pack(Some(fallback_pack())).unwrap();
    tiles.set_actors(0, 60.0, &[link_input()]);
    let base = tiles.present(&indexed, &r, &chr).unwrap().to_vec();
    tiles.set_actors(
        1,
        60.0,
        &[link_input(), enemy("enemy_b4_21_p2", "perch", "fly")],
    );
    assert_eq!(tiles.present(&indexed, &r, &chr).unwrap(), &base[..]);
}

#[test]
fn restart_events_restart_the_same_clip() {
    let mut p = Presenter::new(PresentConfig::default()).unwrap();
    p.set_pack(Some(pack(true))).unwrap();
    let r = record(true);
    // Frames alternate red, green at 60 fps.
    p.set_actors(10, 60.0, &[input(0, "move")]);
    p.set_actors(11, 60.0, &[input(0, "move")]);
    assert_eq!(pixel(&mut p, &r, 20, 29), [0, 255, 0, 255]);
    let restart = ActorInput {
        restart: true,
        ..input(0, "move")
    };
    p.set_actors(12, 60.0, &[restart]);
    assert_eq!(pixel(&mut p, &r, 20, 29), [255, 0, 0, 255], "restarted");
    // Repainting the same frame does not restart again.
    p.set_actors(12, 60.0, &[restart]);
    p.set_actors(13, 60.0, &[input(0, "move")]);
    assert_eq!(pixel(&mut p, &r, 20, 29), [0, 255, 0, 255]);
}

#[test]
fn hit_flash_recolours_enemies_along_the_drawn_palette_only() {
    let (mut r, chr, indexed) = link_and_enemy_record();
    // Sprite palette 2: dark, mid, light NES colours.
    for line in r.lines.iter_mut() {
        line.palette[0x19] = 0x0F;
        line.palette[0x1A] = 0x16;
        line.palette[0x1B] = 0x30;
    }
    let mut p = Presenter::new(PresentConfig::default()).unwrap();
    p.set_pack(Some(enemy_pack(true))).unwrap();
    p.set_actors(0, 60.0, &[link_input(), enemy_input(false)]);
    let plain = p.present(&indexed, &r, &chr).unwrap().to_vec();
    let flashing = ActorInput {
        flash_palette: Some(2),
        ..enemy_input(false)
    };
    p.set_actors(1, 60.0, &[link_input(), flashing]);
    let image = p.present(&indexed, &r, &chr).unwrap().to_vec();
    let ramp: Vec<[u8; 4]> = [0x0F, 0x16, 0x30]
        .iter()
        .map(|&i| {
            let [r, g, b] = z2_ppu::NES_PALETTE_RGB[i];
            [r, g, b, 255]
        })
        .collect();
    let tinted = rgba_at(&image, 101, 28);
    assert_ne!(tinted, rgba_at(&plain, 101, 28));
    // The blue pixel (dark) lands between black and the palette's red.
    assert!(tinted[0] <= ramp[1][0] && tinted[2] <= 64, "{tinted:?}");
    // Link is never tinted by an enemy's flash.
    for y in 26..32 {
        for x in 0..60 {
            assert_eq!(rgba_at(&image, x, y), rgba_at(&plain, x, y), "({x}, {y})");
        }
    }
}

#[test]
fn packs_may_hold_more_pairs_than_fit_decoded_and_decode_on_first_use() {
    // 600 character/clip pairs (over the old 512 limit).
    let clips: Vec<_> = (0..600)
        .map(|i| {
            json!({"character": format!("enemy_b1_{:02x}_{}", i % 256, i / 256),
                "name":"walk","fps":60,"frames":[{"file":"a.png","pivot":[1,1]}]})
        })
        .chain(std::iter::once(
            json!({"character":"hero","name":"move","fps":60,
            "frames":[{"file":"a.png","pivot":[1,1]},{"file":"b.png","pivot":[1,1]}]}),
        ))
        .collect();
    let manifest = json!({"version":1,"name":"many","scale":1,"actor_clips":clips});
    let mut pack = HdPack::from_files(&common::files(vec![
        ("pack.json", manifest.to_string().into_bytes()),
        ("a.png", common::solid_png(3, 3, [255, 0, 0, 255])),
        ("b.png", common::solid_png(3, 3, [0, 255, 0, 255])),
    ]))
    .unwrap();
    assert_eq!(pack.actor_clips().len(), 601);
    assert_eq!(
        pack.actor_library().resident_bytes(),
        0,
        "nothing decoded yet"
    );
    assert!(pack.actor_library().verify().is_ok());
    // A budget of one frame: showing a second character evicts the first.
    pack.actor_library_mut().set_decode_budget(36);
    let mut p = Presenter::new(PresentConfig::default()).unwrap();
    p.set_pack(Some(pack)).unwrap();
    let r = record(true);
    p.set_actors(1, 60.0, &[input(0, "move")]);
    assert_eq!(pixel(&mut p, &r, 20, 29), [255, 0, 0, 255]);
    let other = ActorInput {
        character: "enemy_b1_05_1",
        clip: "walk",
        ..input(0, "move")
    };
    p.set_actors(2, 60.0, &[other]);
    assert_eq!(pixel(&mut p, &r, 20, 29), [255, 0, 0, 255]);
    let library = p.pack().unwrap().actor_library();
    assert_eq!(
        library.resident_bytes(),
        36,
        "the hero's frame was given back"
    );
    assert!(p.pack().unwrap().actor_clips()[600].frames[0]
        .image()
        .is_none());
    // Shown again, it decodes again.
    p.set_actors(3, 60.0, &[input(0, "move")]);
    assert_eq!(pixel(&mut p, &r, 20, 29), [255, 0, 0, 255]);
}

#[test]
fn broken_frame_pngs_are_found_by_verify_and_keep_tile_art() {
    let manifest = json!({"version":1,"name":"broken","scale":1,"actor_clips":[{
        "character":"hero","name":"move","fps":60,"frames":[{"file":"a.png","pivot":[1,1]}]}]});
    let mut png = common::solid_png(3, 3, [255, 0, 0, 255]);
    let len = png.len();
    png[len - 20..len - 12].fill(0); // corrupt the image data, keep the header
    let pack = HdPack::from_files(&common::files(vec![
        ("pack.json", manifest.to_string().into_bytes()),
        ("a.png", png),
    ]))
    .unwrap();
    assert!(pack.actor_library().verify().is_err());
    let mut p = Presenter::new(PresentConfig::default()).unwrap();
    p.set_pack(Some(pack)).unwrap();
    let r = record(true);
    p.set_actors(1, 60.0, &[input(0, "move")]);
    assert_ne!(pixel(&mut p, &r, 20, 29), [255, 0, 0, 255]);
}

/// An enemy past the window edge has no visible OAM: the cartridge hid its
/// columns, and widescreen draws them as margin sprites. Its actor replaces
/// the margin sprites in its `margin_mask`; other margin sprites keep their
/// tile art.
#[test]
fn enemies_in_the_margins_replace_their_own_margin_sprites() {
    use z2_ppu::wide::MarginFill;
    use z2_ppu::MarginSprite;
    let (mut r, chr, indexed) = link_and_enemy_record();
    for l in r.lines.iter_mut() {
        l.chr_page = [0; 2];
    }
    let tiles = 4u8;
    let wide = WIDTH + 16 * usize::from(tiles);
    let mp = 8 * i32::from(tiles);
    let at = |img: &[u8], wx: i32, y: usize| -> [u8; 4] {
        let i = (y * wide + (wx + mp) as usize) * 4;
        img[i..i + 4].try_into().unwrap()
    };
    let present = |actors: &[ActorInput<'_>]| {
        let mut p = Presenter::new(PresentConfig {
            margin_tiles: tiles,
            ..PresentConfig::default()
        })
        .unwrap();
        p.set_pack(Some(enemy_pack(true))).unwrap();
        let m = p.margins_mut();
        for l in m.lines.iter_mut() {
            l.fill = MarginFill::Tiles;
        }
        // Line 30 is the top row of both cells (OAM Y 29).
        let cell = |x| MarginSprite {
            x,
            y: 29,
            tile: 0,
            attr: 0,
        };
        m.sprites = vec![cell(268), cell(280)];
        p.set_actors(0, 60.0, actors);
        p.present(&indexed, &r, &chr).unwrap().to_vec()
    };
    let enemy = ActorInput {
        position: [272.0, 31.0],
        oam_mask: 0,
        margin_mask: 1,
        priority_oam: 20,
        ..enemy_input(false)
    };
    let base = present(&[link_input()]);
    let image = present(&[link_input(), enemy]);
    let sprite = at(&base, 274, 30);
    assert_ne!(sprite, at(&base, 277, 30), "the first cell is drawn");
    // The frame sits on its anchor in the margin (pivot 3,3 -> 269,28)...
    assert_eq!(at(&image, 269, 28), [0, 0, 255, 255]);
    assert_eq!(at(&image, 272, 30), [255, 0, 255, 255]);
    // ... its own margin cell is gone where the frame does not cover it...
    assert_eq!(at(&image, 274, 30), at(&base, 277, 30));
    // ... and the other margin sprite keeps its art.
    assert_eq!(at(&image, 282, 30), at(&base, 282, 30));
    assert_eq!(at(&image, 282, 30), sprite);
    // Link is unchanged.
    for y in 26..32 {
        for x in 0..60 {
            assert_eq!(at(&image, x, y), at(&base, x, y), "({x}, {y})");
        }
    }
}
