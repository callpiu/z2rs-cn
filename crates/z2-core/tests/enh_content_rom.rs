//! New areas (`z2_core::enh::content`) in a real run: the developer rooms
//! (`z2_content::areas::dev_rooms`), one per kind of table.
//!
//! * Great Palace (slot 60): the test room, linked from map 1's right exit
//!   (below).
//! * West Hyrule cave (world 0, slot 28): entered with
//!   `content::enter_from_overworld` and the game's own loader, left
//!   through an outside exit.
//! * Palace group A (map 63, the `$C524` pointer redirect): `Dev Palace`
//!   linked from palace 1 map 1's right exit, a room-to-room exit inside
//!   the slot, out to map 2.
//! * Towns (slot 62): `Dev Inn` through the doorway of town map 1 page 1
//!   (the ROM's own door routine `$CFFC`), and its own doorway back.
//! * Battery persistence: the test room's P-bag stays taken across a SAVE
//!   on the game-over screen, a fresh game on the saved battery RAM and a
//!   load from the file select.
//!
//! ROM-gated (self-skips unless `Z2_ROM` names a file). The GP test starts a
//! new game through the menus, puts Link on the last page of Great Palace
//! map 1 through the game's own loader (scene registers, then game mode 0,
//! as `z2-native/tests/rando_palaces.rs` does) and walks right: with the
//! option on he arrives in the test room instead of map 2. It then checks
//! the room's tiles, its Bot, the P-bag pickup (a crouching stab, the way
//! the game collects P-bags) and that the pickup sticks (save state,
//! re-entry), and walks out both ways.
//!
//! With `Z2_CONTENT_SHOTS=DIR` (outside the work tree: frames come from the
//! ROM) it saves PNGs of the room for eyeballing.

mod common;

use z2_core::enh::content::{self, ContentOpts, TEST_ROOM, WRAM_ENEMY_BUF, WRAM_ROOM_BUF};
use z2_core::enh::Enhancements;
use z2_core::game::{
    Game, INPUT_A, INPUT_B, INPUT_DOWN, INPUT_LEFT, INPUT_RIGHT, INPUT_SELECT, INPUT_START,
};

const A: u8 = 1 << INPUT_A;
const B: u8 = 1 << INPUT_B;
const DOWN: u8 = 1 << INPUT_DOWN;
const SELECT: u8 = 1 << INPUT_SELECT;
const START: u8 = 1 << INPUT_START;
const LEFT: u8 = 1 << INPUT_LEFT;
const RIGHT: u8 = 1 << INPUT_RIGHT;

const MODE: usize = 0x0736;
const MODE_SIDEVIEW: u8 = 0x0B;
const SCENE: usize = 0x0561;
const WORLD: usize = 0x0707;
const PAGE: usize = 0x003B;
const LINK_X: usize = 0x004D;
const LINK_Y: usize = 0x0029;
const XP_HI: usize = 0x0775;
const XP_LO: usize = 0x0776;
/// Great Palace enemy id of the Bot.
const BOT: u8 = 0x04;
/// The Great Palace slot map.
const GP_SLOT: u8 = 60;

fn register_default_groups(game: &mut Game) {
    z2_core::bank7_traps::register_bank7_traps(game);
    z2_core::sideview_traps::register_sideview_traps(game);
    z2_core::sideview_traps::register_overworld_traps(game);
    z2_core::player_traps::register_player_traps(game);
    z2_core::enemy_traps::register_enemy_traps(game);
    z2_core::town_traps::register_town_traps(game);
    z2_core::palace_traps::register_palace_traps(game);
    z2_core::title_traps::register_title_traps(game);
    z2_core::boot_traps::register_boot_traps(game);
}

fn content_on() -> Enhancements {
    Enhancements {
        content: ContentOpts {
            virtual_rooms: true,
            test_room: true,
            ..ContentOpts::default()
        },
        ..Enhancements::default()
    }
}

fn game(rom: &[u8], e: Enhancements) -> Game {
    let mut g = Game::from_ines(rom).expect("ROM");
    register_default_groups(&mut g);
    g.set_enhancements(e);
    g.reset();
    g
}

fn step(g: &mut Game, frames: usize, pad: u8) {
    for _ in 0..frames {
        g.step(pad);
    }
}

/// Title -> file select -> name -> load -> side-view play.
fn reach_gameplay(g: &mut Game) {
    step(g, 30, 0);
    step(g, 5, START);
    step(g, 20, 0);
    step(g, 5, START);
    step(g, 20, 0);
    for _ in 0..8 {
        step(g, 2, A);
        step(g, 8, 0);
    }
    for _ in 0..3 {
        step(g, 2, SELECT);
        step(g, 8, 0);
    }
    step(g, 5, START);
    step(g, 30, 0);
    step(g, 5, START);
    step(g, 170, 0);
    assert_eq!(g.ram[MODE], MODE_SIDEVIEW, "reached side-view play");
}

/// Keep Link alive and topped up.
fn buff_link(g: &mut Game) {
    g.ram[0x0774] = 0xFF;
    g.ram[0x0773] = 0xFF;
    g.ram[0x0784] = 8;
    g.ram[0x0783] = 8;
}

/// Load Great Palace map `map`, entering on `page` facing right.
fn load_gp_room(g: &mut Game, map: u8, page: u8) {
    g.ram[0x0706] = 2;
    g.ram[WORLD] = 5;
    g.ram[0x0748] = 0x36;
    g.ram[0x056C] = 2;
    g.ram[SCENE] = map;
    g.ram[0x075C] = page;
    g.ram[0x0701] = page & 1;
    g.ram[0x0709] = 0;
    g.ram[0x0704] = 0;
    g.ram[MODE] = 0;
    step(g, 150, 0);
    for _ in 0..30 {
        if g.ram[MODE] == MODE_SIDEVIEW {
            break;
        }
        step(g, 20, 0);
    }
    assert_eq!(g.ram[MODE], MODE_SIDEVIEW, "GP map {map} settled");
}

/// Hold `pad` until the scene changes (at most `max` frames), then let the
/// next room settle.
fn walk_out(g: &mut Game, pad: u8, max: usize) -> bool {
    let from = g.ram[SCENE];
    for _ in 0..max {
        buff_link(g);
        g.step(pad);
        if g.ram[SCENE] != from {
            step(g, 120, 0);
            return true;
        }
    }
    false
}

fn shot(g: &Game, name: &str) {
    if let Some(dir) = std::env::var_os("Z2_CONTENT_SHOTS") {
        let png = z2_ppu::encode_indexed_png(g.frame_indexed());
        let path = std::path::Path::new(&dir).join(format!("{name}.png"));
        std::fs::write(path, png).expect("write shot");
    }
}

fn xp(g: &Game) -> u16 {
    u16::from_be_bytes([g.ram[XP_HI], g.ram[XP_LO]])
}

/// Level-RAM tile at room column `x`, row `y` (`$6000 + page * $D0`,
/// row-major 13 x 16 per page).
fn tile(g: &Game, x: usize, y: usize) -> u8 {
    g.wram()[(x / 16) * 0xD0 + y * 16 + x % 16]
}

fn bot_active(g: &Game) -> bool {
    (0..6).any(|s| g.ram[0xB6 + s] != 0 && g.ram[0xA1 + s] == BOT)
}

/// Walk right through the test room until Link reaches page `page`,
/// crouch-stabbing a few times once he stands just left of the P-bag
/// (page 1, column 8) when `stab`. Returns whether a Bot was ever active.
fn walk_to_page(g: &mut Game, page: u8, max: usize, stab: bool) -> bool {
    let mut bot = false;
    let mut stabbed = !stab;
    for _ in 0..max {
        buff_link(g);
        if !stabbed && g.ram[PAGE] == 1 && g.ram[LINK_X] >= 0x6C {
            stabbed = true;
            for _ in 0..4 {
                g.step(B | DOWN);
                step(g, 20, 0);
            }
        }
        g.step(RIGHT);
        bot |= bot_active(g);
        if g.ram[PAGE] >= page || g.ram[SCENE] != GP_SLOT {
            break;
        }
    }
    bot
}

/// From map 1's last page, walk right into the test room.
fn enter_test_room(g: &mut Game) {
    buff_link(g);
    load_gp_room(g, content::TEST_ROOM_FROM_MAP, 3);
    assert!(walk_out(g, RIGHT, 400), "left map 1");
    assert_eq!(g.exec_errors, 0);
    assert_eq!(g.ram[WORLD], 5);
    assert_eq!(g.ram[SCENE], GP_SLOT, "in the slot map");
    assert_eq!(g.ram[MODE], MODE_SIDEVIEW);
    assert_eq!(content::current_room(g), Some(TEST_ROOM));
}

#[test]
fn test_room_is_entered_drawn_and_keeps_its_item() {
    let t = "test_room_is_entered_drawn_and_keeps_its_item";
    let Some(rom) = common::rom_bytes(t) else {
        return;
    };

    let r = content::room(TEST_ROOM).unwrap().room;

    // Off: map 1's right exit leads to map 2.
    let mut og = game(&rom, Enhancements::default());
    reach_gameplay(&mut og);
    buff_link(&mut og);
    load_gp_room(&mut og, content::TEST_ROOM_FROM_MAP, 3);
    assert!(walk_out(&mut og, RIGHT, 400));
    assert_eq!(og.ram[SCENE], 2, "original: map 1 -> map 2");

    let mut g = game(&rom, content_on());
    reach_gameplay(&mut g);
    enter_test_room(&mut g);
    shot(&g, "test-room-page0");
    let entry = g.save_state();

    // The buffers hold the room, the mirror its exits.
    let rb = usize::from(WRAM_ROOM_BUF - 0x6000);
    assert_eq!(&g.wram()[rb..rb + r.sideview.len()], r.sideview);
    let eb = usize::from(WRAM_ENEMY_BUF - 0x6000);
    assert_eq!(&g.wram()[eb..eb + r.enemies.len()], r.enemies);
    let cb = 0x0AFC + 4 * usize::from(GP_SLOT);
    assert_eq!(&g.wram()[cb..cb + 4], &[0x07, 0xFC, 0xFC, 0x08]);

    // Tiles: the platforms (x 6-9 row 7, x 20-23 row 5) are drawn over the
    // open background; the floor (pattern 1, rows 10-12) runs the whole
    // room and Link stands on it.
    let open = tile(&g, 3, 7);
    let brick = tile(&g, 6, 7);
    assert_ne!(brick, open);
    for x in 6..10 {
        assert_eq!(tile(&g, x, 7), brick, "platform 1 col {x}");
        assert_eq!(tile(&g, x, 6), open);
        assert_eq!(tile(&g, x, 8), open);
    }
    assert_eq!(tile(&g, 10, 7), open);
    for x in 20..24 {
        assert_eq!(tile(&g, x, 5), brick, "platform 2 col {x}");
    }
    let floor = tile(&g, 0, 10);
    assert_ne!(floor, open);
    for x in [0, 15, 31, 47, 63] {
        assert_eq!(tile(&g, x, 10), floor);
        assert_eq!(tile(&g, x, 9), open);
    }
    assert_eq!(g.ram[LINK_Y], 0xA0, "Link stands on the floor");

    // Item: the P-bag on page 1 is collected (next level moved far away so
    // the 50 XP do not open the level-up menu).
    g.ram[0x0770] = 0x0F;
    g.ram[0x0771] = 0xFF;
    let xp0 = xp(&g);
    assert_eq!(content::taken_pages(&g, TEST_ROOM), 0);
    let mut bot = walk_to_page(&mut g, 2, 600, true);
    shot(&g, "test-room-page2");
    assert_eq!(content::taken_pages(&g, TEST_ROOM), 0b0010, "P-bag taken");
    assert_eq!(xp(&g), xp0 + 50, "P-bag XP");
    // Enemy: the Bot of page 2 spawned.
    bot |= walk_to_page(&mut g, 3, 200, false);
    shot(&g, "test-room-page3");
    assert!(bot, "Bot active in the test room");

    // A save state carries the virtual room and its taken item.
    let snap = g.save_state();
    let mut h = game(&rom, content_on());
    h.load_state(&snap);
    assert_eq!(content::current_room(&h), Some(TEST_ROOM));
    assert_eq!(content::taken_pages(&h, TEST_ROOM), 0b0010);

    // Right exit: on to map 2, out of the virtual room.
    assert!(walk_out(&mut g, RIGHT, 600), "left the test room");
    assert_eq!(g.ram[SCENE], 2, "test room -> map 2");
    assert_eq!(content::current_room(&g), None);

    // Back in: the P-bag stays taken (no item object, no XP).
    enter_test_room(&mut g);
    assert!(
        !(0..6).any(|s| g.ram[0xB6 + s] != 0 && g.ram[0xA1 + s] == 1),
        "no item object on re-entry"
    );
    let xp2 = xp(&g);
    walk_to_page(&mut g, 2, 600, true);
    assert_eq!(xp(&g), xp2, "no second P-bag");
    assert_eq!(content::taken_pages(&g, TEST_ROOM), 0b0010);

    // Left exit (from the first entry): back to map 1.
    let mut l = game(&rom, content_on());
    l.load_state(&entry);
    assert!(walk_out(&mut l, LEFT, 600), "left the test room leftwards");
    assert_eq!(
        l.ram[SCENE],
        content::TEST_ROOM_FROM_MAP,
        "test room -> map 1"
    );
    assert_eq!(content::current_room(&l), None);
    assert_eq!(g.exec_errors + l.exec_errors, 0);
}

// ------------------------------------------------------------ other tables

const STAGE: usize = 0x076C;
const MENU_SEL: usize = 0x0488;
const LIVES: usize = 0x0700;

/// Load map `map` of world `world` (region `region`, key area `key`)
/// through the game's own loader, entering on `page`; true once side-view
/// play runs.
fn load_scene(g: &mut Game, region: u8, world: u8, key: u8, map: u8, page: u8) -> bool {
    g.ram[0x0706] = region;
    g.ram[WORLD] = world;
    g.ram[0x070A] = 0;
    g.ram[0x0748] = key;
    g.ram[0x056C] = key.wrapping_sub(0x34);
    g.ram[SCENE] = map;
    g.ram[0x075C] = page;
    g.ram[0x0701] = page & 1;
    g.ram[0x0709] = 0;
    g.ram[0x0704] = 0;
    g.ram[MODE] = 0;
    step(g, 150, 0);
    for _ in 0..30 {
        if g.ram[MODE] == MODE_SIDEVIEW {
            return true;
        }
        step(g, 20, 0);
    }
    g.ram[MODE] == MODE_SIDEVIEW
}

/// Step until `f` holds (at most `max` frames).
fn step_until(g: &mut Game, max: usize, f: impl Fn(&Game) -> bool) -> bool {
    for _ in 0..max {
        if f(g) {
            return true;
        }
        buff_link(g);
        g.step(0);
    }
    f(g)
}

fn room_id(name: &str, index: u8) -> content::VRoomId {
    content::find_room(name, index).expect(name)
}

fn assert_buffers_hold(g: &Game, id: content::VRoomId) {
    let r = content::room(id).unwrap().room;
    let rb = usize::from(WRAM_ROOM_BUF - 0x6000);
    assert_eq!(&g.wram()[rb..rb + r.sideview.len()], r.sideview);
    let eb = usize::from(WRAM_ENEMY_BUF - 0x6000);
    // The game flags a spawned entry in place (bit 7 of its first byte,
    // `$9A11`), so compare without that bit.
    let strip = |l: &[u8]| -> Vec<u8> {
        l.iter()
            .enumerate()
            .map(|(i, &b)| if i % 2 == 1 { b & 0x7F } else { b })
            .collect()
    };
    assert_eq!(strip(&g.wram()[eb..eb + r.enemies.len()]), strip(r.enemies));
}

/// The ledge at columns `x0..x1`, row 7 (`row`), is drawn over open background and
/// the floor (pattern 1, row 10) runs under it.
fn assert_ledge(g: &Game, x0: usize, x1: usize) {
    assert_ledge_at(g, x0, x1, 7);
}

fn assert_ledge_at(g: &Game, x0: usize, x1: usize, row: usize) {
    let open = tile(g, x1 + 1, row);
    let ledge = tile(g, x0, row);
    let grid: Vec<String> = (0..13)
        .map(|y| {
            (0..32)
                .map(|x| format!("{:02X}", tile(g, x, y)))
                .collect::<Vec<_>>()
                .join(" ")
        })
        .collect();
    assert_ne!(ledge, open, "ledge drawn:\n{}", grid.join("\n"));
    for x in x0..x1 {
        assert_eq!(tile(g, x, row), ledge, "ledge col {x}");
    }
    assert_ne!(tile(g, x0, 10), open, "floor drawn");
}

#[test]
fn cave_is_entered_from_the_overworld_and_left_outside() {
    let t = "cave_is_entered_from_the_overworld_and_left_outside";
    let Some(rom) = common::rom_bytes(t) else {
        return;
    };
    let cave = room_id("Dev Cave", 0);
    // Off: nothing to enter.
    let mut off = game(&rom, Enhancements::default());
    assert!(!content::enter_from_overworld(&mut off, cave));

    let mut g = game(&rom, content_on());
    reach_gameplay(&mut g);
    buff_link(&mut g);
    let target = content::location_target(&g, cave).expect("West slot live");
    assert_eq!((target.world, target.table, target.map), (0, 0, 28));
    // A world-0 load takes the map from the location entry (`$0748`), so
    // point West location $10's map byte (bank 1 `$861F + 2 * 63 + slot`)
    // at the slot, the way an injected entrance does.
    const KEY: u16 = 0x10;
    let loc_map = 0x4000 + usize::from(0x861F - 0x8000 + 126 + KEY);
    g.prg[loc_map] = (g.prg[loc_map] & 0xC0) | target.map;
    assert!(content::enter_from_overworld(&mut g, cave));
    assert_eq!(content::pending_room(&g), Some(cave));
    assert!(
        load_scene(&mut g, 0, 0, KEY as u8, target.map, 0),
        "cave loaded"
    );
    shot(&g, "dev-cave");
    assert_eq!(g.exec_errors, 0);
    assert_eq!(g.ram[SCENE], 28);
    assert_eq!(content::current_room(&g), Some(cave));
    assert_eq!(content::pending_room(&g), None);
    assert_buffers_hold(&g, cave);
    // A world-0 cave (alternate object tiles) draws `Object::Block` as
    // rock (object kind 2: a walkable top row over a solid one).
    assert_ledge_at(&g, 10, 13, 6);
    let cb = 0x0AFC + 4 * 28;
    assert_eq!(
        &g.wram()[cb..cb + 4],
        &[0xFC, 0xFD, 0xFC, 0xFC],
        "left to mouth 0, right to mouth 1"
    );
    // Left walks out to the overworld.
    let out = (0..600).any(|_| {
        buff_link(&mut g);
        g.step(LEFT);
        g.ram[MODE] != MODE_SIDEVIEW
    });
    assert!(out, "left the cave");
    step(&mut g, 200, 0);
    assert_eq!(content::current_room(&g), None);
    assert_eq!(g.ram[WORLD], 0);
    assert_eq!(g.exec_errors, 0);
}

// ------------------------------------------------------------ overworld

const OW_MODE: u8 = 5;
const OW_Y: usize = 0x0073;
const OW_X: usize = 0x0074;
/// Raw row of map row 0 (`$73` = row + 30).
const ROW_BASE: u8 = 30;

fn on_overworld(g: &Game) -> bool {
    g.ram[WORLD] == 0 && g.ram[MODE] == OW_MODE
}

/// Terrain at map (`row`, `col`) as the game reads it: the `$6000` row
/// pointer, then the runs of that row.
fn ow_tile(g: &Game, row: u8, col: u8) -> u8 {
    let w = g.wram();
    let r = 2 * usize::from(row);
    let p = usize::from(u16::from_le_bytes([w[r], w[r + 1]]));
    let mut off = p - 0x6000;
    let mut acc = 0usize;
    loop {
        let b = w[off];
        acc += usize::from(b >> 4) + 1;
        if acc > usize::from(col) {
            return b & 0x0F;
        }
        off += 1;
    }
}

/// The original map of West Hyrule, decoded from the ROM (bank 1, map
/// pointer at `$8508`), for comparison.
fn rom_west(rom: &[u8]) -> Vec<[u8; 64]> {
    let b1 = 16 + 0x4000;
    let ptr = usize::from(u16::from_le_bytes([rom[b1 + 0x508], rom[b1 + 0x509]]));
    let mut off = b1 + (ptr - 0x8000);
    (0..75)
        .map(|_| {
            let mut row = [0u8; 64];
            let mut c = 0;
            while c < 64 {
                let b = rom[off];
                for _ in 0..=(b >> 4) {
                    row[c] = b & 0x0F;
                    c += 1;
                }
                off += 1;
            }
            row
        })
        .collect()
}

/// Hold `pad` on the overworld until Link leaves it (an entry); false
/// after `max` frames.
fn walk_into(g: &mut Game, pad: u8, max: usize) -> bool {
    for _ in 0..max {
        buff_link(g);
        g.step(pad);
        if !on_overworld(g) {
            return true;
        }
    }
    false
}

/// Hold `pad` in a side-view until Link is back on the overworld.
fn walk_outside(g: &mut Game, pad: u8, max: usize) -> bool {
    (0..max).any(|_| {
        buff_link(g);
        g.step(pad);
        on_overworld(g)
    })
}

/// Map cells `Dev Cave` paints: both mouths, two desert cells in the moat
/// south of the North Palace and two shallow-water cells under them.
fn dev_cave_cell(row: u8, col: u8) -> Option<u8> {
    use z2_content::areas::dev_rooms::{DEV_CAVE_COLS, DEV_CAVE_ROW, DEV_INN_TILE};
    match (row, col) {
        (r, c) if r == DEV_CAVE_ROW && DEV_CAVE_COLS.contains(&c) => Some(0x1),
        (r, c) if (r, c) == DEV_INN_TILE => Some(0x0),
        (24, 20 | 21) => Some(0x4),
        (25, 20 | 21) => Some(0xD),
        _ => None,
    }
}

/// The West Hyrule cells the palace-to-stone routine turns into
/// mountains (bank 1 `$878F`: three absolute addresses in the `$7C00` map
/// window, each a one-tile palace run of the original layout).
fn stone_cells(rom: &[u8]) -> Vec<(u8, u8)> {
    let b1 = 16 + 0x4000;
    let ptr = usize::from(u16::from_le_bytes([rom[b1 + 0x508], rom[b1 + 0x509]]));
    let blob = b1 + (ptr - 0x8000);
    (0..3)
        .map(|x| {
            let at = usize::from(u16::from_le_bytes([
                rom[b1 + 0x78F + 2 * x],
                rom[b1 + 0x790 + 2 * x],
            ]));
            let want = at - 0x7C00;
            let (mut i, mut row, mut col) = (0usize, 0u8, 0usize);
            while i < want {
                col += usize::from(rom[blob + i] >> 4) + 1;
                if col == 64 {
                    row += 1;
                    col = 0;
                }
                i += 1;
            }
            assert_eq!(rom[blob + i], 0x02, "a one-tile palace run");
            (row, col as u8)
        })
        .collect()
}

/// Every cell of the WRAM map: the painted ones (`painted`), mountains on
/// `stone`, and every other cell as the ROM has it (the rows after a
/// painted one shifted whole).
fn check_map(g: &Game, west: &[[u8; 64]], painted: bool, stone: &[(u8, u8)], when: &str) {
    for row in 0..75u8 {
        for col in 0..64u8 {
            let rom = if stone.contains(&(row, col)) {
                0x0B
            } else {
                west[usize::from(row)][usize::from(col)]
            };
            let want = if painted {
                dev_cave_cell(row, col).unwrap_or(rom)
            } else {
                rom
            };
            assert_eq!(ow_tile(g, row, col), want, "{when}: tile ({row}, {col})");
        }
    }
}

#[test]
fn overworld_entrances_lead_into_the_cave_and_back_out() {
    let t = "overworld_entrances_lead_into_the_cave_and_back_out";
    let Some(rom) = common::rom_bytes(t) else {
        return;
    };
    use z2_content::areas::dev_rooms::{DEV_CAVE_COLS, DEV_CAVE_ROW};
    use z2_core::enh::content_overworld as ow;
    let cave = room_id("Dev Cave", 0);
    let mut g = game(&rom, content_on());
    // Both mouths on the first free run of West slots, consecutive.
    let placed: Vec<_> = ow::placements(&g)
        .iter()
        .filter(|p| p.room == cave)
        .copied()
        .collect();
    assert_eq!(placed.len(), 2, "{placed:?}");
    // (The first free run past the slots the authored areas claim.)
    assert_eq!(placed[0].region, 0);
    assert_eq!(placed[1].slot, placed[0].slot + 1);
    assert!(z2_content::areas::SLOTS.iter().all(
        |c| !(c.region == z2_content::Region::West && placed.iter().any(|p| p.slot == c.slot))
    ));
    assert_eq!(
        placed[0].bytes,
        [DEV_CAVE_ROW + ROW_BASE, DEV_CAVE_COLS[0], 28, 0x00]
    );
    // Mouth 1: entrance index 1, page 1, and (page 1 being the room's
    // last) the enter-from-the-right flag.
    assert_eq!(
        placed[1].bytes,
        [
            DEV_CAVE_ROW + ROW_BASE,
            DEV_CAVE_COLS[1] | 0x40,
            28 | 0x40,
            0x20
        ]
    );
    let west = rom_west(&rom);

    reach_gameplay(&mut g);
    assert!(walk_outside(&mut g, LEFT, 1500), "reached the overworld");
    step(&mut g, 60, 0);
    assert!(on_overworld(&g));
    assert_eq!((g.ram[OW_Y], g.ram[OW_X]), (DEV_CAVE_ROW + ROW_BASE, 23));
    for p in &placed {
        let got: Vec<u8> = (0..4)
            .map(|k| g.wram()[0x0A00 + 63 * k + usize::from(p.slot)])
            .collect();
        assert_eq!(got, p.bytes, "slot {} written", p.slot);
    }
    check_map(&g, &west, true, &[], "first arrival");
    shot(&g, "ow-mouths");

    // Right along the road onto mouth 0.
    assert!(walk_into(&mut g, RIGHT, 400), "stepped on mouth 0");
    assert_eq!(g.ram[0x0748], placed[0].slot);
    assert!(
        step_until(&mut g, 600, |g| g.ram[MODE] == MODE_SIDEVIEW),
        "cave loaded"
    );
    shot(&g, "ow-cave-left");
    assert_eq!(g.exec_errors, 0);
    assert_eq!((g.ram[SCENE], g.ram[WORLD]), (28, 0));
    assert_eq!(content::current_room(&g), Some(cave));
    assert_buffers_hold(&g, cave);
    assert_eq!(g.ram[PAGE], 0, "entered on page 0");
    // Through the cave and out of its right edge: mouth 1.
    assert!(walk_outside(&mut g, RIGHT, 1500), "out of the right edge");
    assert_eq!(
        (g.ram[OW_Y], g.ram[OW_X]),
        (DEV_CAVE_ROW + ROW_BASE, DEV_CAVE_COLS[1]),
        "came out at mouth 1"
    );
    assert_eq!(content::current_room(&g), None);
    // Standing on the mouth does not enter it again.
    step(&mut g, 120, 0);
    assert!(on_overworld(&g), "no re-entry while standing");
    check_map(&g, &west, true, &[], "after the cave");
    shot(&g, "ow-out-right");

    // Off the mouth and back onto it from the east: page 1, right edge.
    assert!(!walk_into(&mut g, RIGHT, 4));
    step(&mut g, 40, 0);
    assert!(on_overworld(&g));
    assert_eq!(g.ram[OW_X], DEV_CAVE_COLS[1] + 1);
    assert!(walk_into(&mut g, LEFT, 400), "stepped back on mouth 1");
    assert_eq!(g.ram[0x0748], placed[1].slot, "stepped on mouth 1");
    assert!(
        step_until(&mut g, 600, |g| g.ram[MODE] == MODE_SIDEVIEW),
        "cave loaded again"
    );
    assert_eq!(g.ram[0x0748], placed[0].slot, "rebased to the first mouth");
    assert_eq!(content::current_room(&g), Some(cave));
    // Link is placed once the room has scrolled in.
    step(&mut g, 30, 0);
    assert_eq!(g.ram[PAGE], 1, "entered on page 1");
    assert!(
        g.ram[LINK_X] >= 0xC0,
        "at the right edge: {:#x}",
        g.ram[LINK_X]
    );
    shot(&g, "ow-cave-right");
    assert!(walk_outside(&mut g, LEFT, 1500), "out of the left edge");
    assert_eq!(
        (g.ram[OW_Y], g.ram[OW_X]),
        (DEV_CAVE_ROW + ROW_BASE, DEV_CAVE_COLS[0]),
        "came out at mouth 0"
    );
    step(&mut g, 60, 0);
    check_map(&g, &west, true, &[], "after the second visit");
    assert_eq!(g.exec_errors, 0);

    // The three West palaces done: the stone routine (mode 2, run on every
    // return to the overworld) writes absolute map addresses, so the
    // painted rows go back to their original bytes around it. A cave
    // visit (no world load) shows its mountains in the right cells, the
    // rows after them included.
    let stone = stone_cells(&rom);
    assert!(
        stone.iter().any(|&(r, _)| r > 25),
        "one after the painted rows"
    );
    for y in 0..3 {
        g.ram[0x0785 + y] = 1;
        g.ram[0x078D + y] = 1;
    }
    assert!(!walk_into(&mut g, RIGHT, 4));
    step(&mut g, 40, 0);
    assert!(walk_into(&mut g, LEFT, 400), "back on mouth 0");
    assert!(
        step_until(&mut g, 600, |g| g.ram[MODE] == MODE_SIDEVIEW),
        "cave loaded a third time"
    );
    assert!(walk_outside(&mut g, LEFT, 1500), "out again");
    step(&mut g, 60, 0);
    check_map(&g, &west, true, &stone, "palaces turned to stone");
    assert_eq!(g.exec_errors, 0);

    // Turning the option off puts the original slots and map back.
    g.set_enhancements(Enhancements::default());
    assert_eq!(
        g.wram()[0x0A00 + usize::from(placed[0].slot)],
        0,
        "slot restored"
    );
    check_map(&g, &west, false, &stone, "option off");
    // A state from without the option, loaded with it on: the end of the
    // next overworld frame writes the slots and paints the map.
    let vanilla = g.save_state();
    let mut h = game(&rom, content_on());
    h.load_state(&vanilla);
    step(&mut h, 1, 0);
    assert_eq!(
        h.wram()[0x0A00 + usize::from(placed[0].slot)],
        placed[0].bytes[0]
    );
    check_map(&h, &west, true, &stone, "state loaded with the option on");
    assert_eq!(h.exec_errors, 0);
}

#[test]
fn palace_room_in_map_63_is_linked_and_chains() {
    let t = "palace_room_in_map_63_is_linked_and_chains";
    let Some(rom) = common::rom_bytes(t) else {
        return;
    };
    let hall = room_id("Dev Palace", 0);
    let end = room_id("Dev Palace", 1);

    // Off: palace 1 map 1's right exit leads to map 2.
    let mut og = game(&rom, Enhancements::default());
    reach_gameplay(&mut og);
    buff_link(&mut og);
    assert!(load_scene(
        &mut og,
        0,
        3,
        0x34,
        content::DEV_PALACE_FROM_MAP,
        3
    ));
    assert!(walk_out(&mut og, RIGHT, 600));
    assert_eq!(og.ram[SCENE], 2, "original: map 1 -> map 2");

    let mut g = game(&rom, content_on());
    reach_gameplay(&mut g);
    buff_link(&mut g);
    assert_eq!(
        content::slot_map(&g, 5),
        Some(63),
        "palace group A uses map 63"
    );
    assert!(load_scene(
        &mut g,
        0,
        3,
        0x34,
        content::DEV_PALACE_FROM_MAP,
        3
    ));
    assert!(walk_out(&mut g, RIGHT, 600), "left map 1");
    shot(&g, "dev-palace-hall");
    assert_eq!(g.exec_errors, 0);
    assert_eq!(g.ram[SCENE], 63, "in map 63");
    assert_eq!(g.ram[MODE], MODE_SIDEVIEW);
    assert_eq!(content::current_room(&g), Some(hall));
    assert_buffers_hold(&g, hall);
    assert_ledge(&g, 12, 16);
    // Room to room inside the slot (proxy byte, then back to 63).
    assert!(walk_out_scene_or_room(&mut g, RIGHT, 900), "left the hall");
    shot(&g, "dev-palace-end");
    assert_eq!(g.ram[SCENE], 63);
    assert_eq!(content::current_room(&g), Some(end));
    assert_buffers_hold(&g, end);
    assert_ledge(&g, 4, 7);
    // And out to the original map 2.
    assert!(walk_out(&mut g, RIGHT, 900), "left the end room");
    assert_eq!(g.ram[SCENE], 2, "end -> map 2");
    assert_eq!(content::current_room(&g), None);
    assert_eq!(g.exec_errors, 0);
}

/// Like [`walk_out`], but also done when the virtual room changes inside
/// the same slot map.
fn walk_out_scene_or_room(g: &mut Game, pad: u8, max: usize) -> bool {
    let from = (g.ram[SCENE], content::current_room(g));
    for _ in 0..max {
        buff_link(g);
        g.step(pad);
        if g.ram[SCENE] != from.0 || content::pending_room(g).is_some() {
            step(g, 150, 0);
            return (g.ram[SCENE], content::current_room(g)) != from;
        }
    }
    false
}

/// Call the ROM's door routine (`$CFFC`) the way the game does when Link
/// walks into a doorway on page `page`: a `JSR` from scratch WRAM, so the
/// hooked trap fires.
fn take_door(g: &mut Game, page: u8) {
    g.ram[PAGE] = page;
    let at = 0x0900; // $6900, engine scratch (content_mem::VROOM_SCRATCH)
    g.wram[at..at + 4].copy_from_slice(&[0x20, 0xFC, 0xCF, 0x60]);
    g.call_asm(0x6900);
}

#[test]
fn town_doorway_leads_into_a_virtual_room_and_back() {
    let t = "town_doorway_leads_into_a_virtual_room_and_back";
    let Some(rom) = common::rom_bytes(t) else {
        return;
    };
    let inn = room_id("Dev Inn", 0);
    let from = content::DEV_INN_FROM_MAP;
    let mut g = game(&rom, content_on());
    reach_gameplay(&mut g);
    buff_link(&mut g);
    assert!(load_scene(&mut g, 0, 1, 0x2D, from, 1), "town map loaded");
    assert_eq!(content::slot_map(&g, 4), Some(62));
    take_door(&mut g, 1);
    assert_eq!(g.ram[SCENE], 62, "door led to the slot");
    assert_eq!(content::pending_room(&g), Some(inn));
    assert!(
        step_until(&mut g, 600, |g| g.ram[MODE] == MODE_SIDEVIEW
            && content::current_room(g) == Some(inn)),
        "inn loaded"
    );
    step(&mut g, 60, 0);
    shot(&g, "dev-inn");
    assert_buffers_hold(&g, inn);
    assert_ledge(&g, 3, 5);
    // The inn's own doorway (page 0) leads back to the street, page 1.
    take_door(&mut g, 0);
    assert_eq!(g.ram[SCENE], from, "inn door -> street");
    assert_eq!(g.ram[0x075C], 1);
    assert!(step_until(&mut g, 600, |g| g.ram[MODE] == MODE_SIDEVIEW));
    assert_eq!(content::current_room(&g), None);
    assert_eq!(g.exec_errors, 0);
}

#[test]
fn taken_item_survives_a_battery_save_and_reload() {
    let t = "taken_item_survives_a_battery_save_and_reload";
    let Some(rom) = common::rom_bytes(t) else {
        return;
    };
    let mut g = game(&rom, content_on());
    reach_gameplay(&mut g);
    enter_test_room(&mut g);
    g.ram[0x0770] = 0x0F;
    g.ram[0x0771] = 0xFF;
    walk_to_page(&mut g, 2, 600, true);
    assert_eq!(content::taken_pages(&g, TEST_ROOM), 0b0010, "P-bag taken");
    // Pool bit 0: map 25's nibble in the West block ($060C, low, bit $08).
    assert_eq!(g.ram[0x060C] & 0x08, 0, "stored in the pool nibble");

    // SAVE on the game-over screen.
    g.ram[LIVES] = 1;
    g.ram[STAGE] = 2;
    assert!(
        step_until(&mut g, 3000, |g| g.ram[STAGE] == 2 && g.ram[MODE] == 9),
        "game-over screen"
    );
    g.ram[MENU_SEL] = 1;
    step(&mut g, 2, START);
    step(&mut g, 300, 0);
    let sram = *g.wram();
    assert_eq!(
        sram[0x749A - 0x6000 + 0x0C] & 0x08,
        0,
        "saved in slot 0 part 2"
    );

    // A fresh game on that battery RAM, file 1 loaded from the menu.
    let mut h = Game::from_ines(&rom).expect("ROM");
    register_default_groups(&mut h);
    h.set_enhancements(content_on());
    h.wram = sram;
    h.reset();
    step(&mut h, 30, 0);
    step(&mut h, 5, START);
    step(&mut h, 20, 0);
    step(&mut h, 5, START);
    assert!(
        step_until(&mut h, 600, |g| g.ram[MODE] == MODE_SIDEVIEW),
        "file loaded"
    );
    assert_eq!(content::taken_pages(&h, TEST_ROOM), 0b0010, "still taken");
    step(&mut h, 200, 0);
    enter_test_room(&mut h);
    assert!(
        !(0..6).any(|s| h.ram[0xB6 + s] != 0 && h.ram[0xA1 + s] == 1),
        "no item object after reload"
    );
    assert_eq!(h.exec_errors + g.exec_errors, 0);
}

/// The runtime map walk agrees with the survey on the original ROM: every
/// survey-free map (so every slot and pool map) is unreachable, every pool
/// nibble starts all set, and the game's item-bit pointer table (`$C265`,
/// `region * 5 + world`) names the blocks `content_mem` uses.
#[test]
fn runtime_scan_matches_the_survey_on_the_original_rom() {
    use z2_core::enh::content_mem::{PROGRESS_POOL, SETS};
    let t = "runtime_scan_matches_the_survey_on_the_original_rom";
    let Some(rom) = common::rom_bytes(t) else {
        return;
    };
    let g = Game::from_ines(&rom).expect("ROM");
    let used = content::reachable_maps(&g);
    for (s, set) in SETS.iter().enumerate() {
        for &m in set.free_maps {
            assert_eq!(used[s] & (1 << m), 0, "{} map {m} reachable", set.name);
        }
        assert_eq!(used[s] & (1 << set.reserved_map), 0, "{} slot", set.name);
        assert_ne!(used[s] & 1, 0, "{} map 0 reached", set.name);
    }
    let all = ContentOpts {
        virtual_rooms: true,
        test_room: true,
        parapa_annex: true,
        challenge_rooms: true,
        minigames: true,
        hidden_caves: true,
        dark_isle: true,
    };
    let rt = content::scan(&g, &all);
    assert_eq!(
        rt.pool_ok,
        (1u64 << PROGRESS_POOL.len()) - 1,
        "every pool map"
    );
    // The developer rooms use West, towns, palace group A and the GP.
    assert_eq!(rt.slots[0], Some(28));
    assert_eq!(rt.slots[4], Some(62));
    assert_eq!(rt.slots[5], Some(63));
    assert_eq!(rt.slots[7], Some(60));
    let ptr = |region: u8, world: u8| {
        let at =
            z2_core::enh::bank_offset(g.prg.len(), 7, 0xC265 + 2 * u16::from(region * 5 + world));
        u16::from_le_bytes([g.prg[at], g.prg[at + 1]])
    };
    for (region, world, set) in [
        (0, 0, 0),
        (1, 0, 1),
        (2, 0, 2),
        (0, 1, 4),
        (2, 2, 4),
        (0, 3, 5),
        (2, 3, 5),
        (0, 4, 6),
        (1, 4, 6),
        (2, 4, 6),
        (2, 5, 7),
    ] {
        assert_eq!(
            ptr(region, world),
            SETS[set].item_block,
            "region {region} world {world}"
        );
    }
}

#[test]
fn town_entrance_loads_the_virtual_town_through_a_world_load() {
    let t = "town_entrance_loads_the_virtual_town_through_a_world_load";
    let Some(rom) = common::rom_bytes(t) else {
        return;
    };
    use z2_content::areas::dev_rooms::{DEV_CAVE_COLS, DEV_CAVE_ROW, DEV_INN_TILE};
    use z2_core::enh::content_overworld as ow;
    const UP: u8 = 1 << z2_core::game::INPUT_UP;
    let inn = room_id("Dev Inn", 0);
    let mut g = game(&rom, content_on());
    let p = *ow::placements(&g)
        .iter()
        .find(|p| p.room == inn)
        .expect("inn placed");
    // A town takes the highest free West slot; external entry into world 1
    // (West towns), region 0; Rauru's town code (the first West town).
    assert_eq!(p.slot, 61);
    assert_eq!(
        p.bytes,
        [(DEV_INN_TILE.0 + ROW_BASE) | 0x80, DEV_INN_TILE.1, 62, 0x04]
    );
    assert_eq!(p.codes, Some((0, 0)));

    reach_gameplay(&mut g);
    assert!(walk_outside(&mut g, LEFT, 1500), "reached the overworld");
    step(&mut g, 60, 0);
    // Through the cave to its right mouth, then east and north onto the
    // town tile.
    assert!(walk_into(&mut g, RIGHT, 400), "mouth 0");
    assert!(step_until(&mut g, 600, |g| g.ram[MODE] == MODE_SIDEVIEW));
    assert!(walk_outside(&mut g, RIGHT, 1500), "out at mouth 1");
    step(&mut g, 40, 0);
    assert_eq!(g.ram[OW_X], DEV_CAVE_COLS[1]);
    for _ in 0..2 {
        assert!(!walk_into(&mut g, RIGHT, 4));
        step(&mut g, 40, 0);
    }
    assert_eq!(
        (g.ram[OW_Y], g.ram[OW_X]),
        (DEV_CAVE_ROW + ROW_BASE, DEV_INN_TILE.1)
    );
    assert_eq!(
        ow_tile(&g, DEV_INN_TILE.0, DEV_INN_TILE.1),
        0x0,
        "town tile"
    );
    assert!(walk_into(&mut g, UP, 400), "stepped on the town");
    assert!(
        step_until(&mut g, 900, |g| g.ram[MODE] == MODE_SIDEVIEW),
        "town loaded"
    );
    step(&mut g, 30, 0);
    shot(&g, "ow-town");
    assert_eq!(g.exec_errors, 0);
    assert_eq!((g.ram[WORLD], g.ram[SCENE]), (1, 62));
    assert_eq!(content::current_room(&g), Some(inn));
    assert_buffers_hold(&g, inn);
    assert_eq!(g.ram[0x056B], 0, "town code supplied");
    assert_eq!(g.ram[0x0748], p.slot);
    // Out of its left edge: a world load back to West Hyrule, on the tile.
    assert!(walk_outside(&mut g, LEFT, 1500), "out of the town");
    step(&mut g, 60, 0);
    assert_eq!(
        (g.ram[0x0706], g.ram[OW_Y], g.ram[OW_X]),
        (0, DEV_INN_TILE.0 + ROW_BASE, DEV_INN_TILE.1)
    );
    assert_eq!(ow_tile(&g, DEV_INN_TILE.0, DEV_INN_TILE.1), 0x0);
    assert!(on_overworld(&g));
    assert_eq!(g.exec_errors, 0);
}

#[test]
fn authored_entrances_take_their_claimed_slots_and_fit_the_map() {
    let t = "authored_entrances_take_their_claimed_slots_and_fit_the_map";
    let Some(rom) = common::rom_bytes(t) else {
        return;
    };
    use z2_core::enh::content_overworld as ow;
    let all = Enhancements {
        content: ContentOpts {
            virtual_rooms: true,
            test_room: true,
            parapa_annex: true,
            challenge_rooms: true,
            minigames: true,
            hidden_caves: true,
            dark_isle: true,
        },
        ..Enhancements::default()
    };
    let g = game(&rom, all);
    let placed = ow::placements(&g);
    // Every claim of `z2_content::areas::SLOTS` is placed on its slot.
    for c in z2_content::areas::SLOTS {
        let (_, area) = content::all_areas()
            .find(|(_, a)| a.name == c.area)
            .unwrap_or_else(|| panic!("area {}", c.area));
        let e = area.entrances[usize::from(c.entrance)];
        let p = placed
            .iter()
            .find(|p| {
                content::room(p.room).is_some_and(|r| r.area.name == c.area)
                    && (p.region, p.row, p.col) == (e.region, e.row, e.col)
            })
            .unwrap_or_else(|| panic!("{} entrance {} placed", c.area, c.entrance));
        assert_eq!((p.region, p.slot), (c.region.id(), c.slot), "{}", c.area);
    }
    // No two placements share a slot of one table.
    for (i, a) in placed.iter().enumerate() {
        for b in &placed[i + 1..] {
            assert!((a.region, a.slot) != (b.region, b.slot), "{a:?} / {b:?}");
        }
    }
    // Every painted cell fits each region's window and the backup scratch.
    let plan = ow::plan_of(&g);
    let window = usize::from(ow::WINDOW_END - 0x7C00);
    for (region, bank, ptr_at, height) in [
        (0u8, 1usize, 0x508usize, 75usize),
        (1, 1, 0x50A, 60),
        (2, 2, 0x508, 75),
        (3, 2, 0x50A, 60),
    ] {
        let b = 16 + bank * 0x4000;
        let ptr = usize::from(u16::from_le_bytes([rom[b + ptr_at], rom[b + ptr_at + 1]]));
        let win = &rom[b + ptr - 0x8000..b + ptr - 0x8000 + window];
        let cells = ow::edits(plan, region);
        let rows: std::collections::BTreeSet<u8> = cells.iter().map(|c| c.0).collect();
        let (painted, backup) = ow::paint_window(win, height, &cells, usize::from(ow::SCRATCH.len));
        let spans = ow::row_spans(&painted, height);
        let changed = rows
            .iter()
            .filter(|&&r| {
                let s = ow::row_spans(win, height)[usize::from(r)];
                cells
                    .iter()
                    .filter(|c| c.0 == r)
                    .any(|c| ow::decode_row(&win[s.0..s.1]).unwrap()[usize::from(c.1)] != c.2)
            })
            .count();
        assert_eq!(
            usize::from(backup[0]),
            changed,
            "region {region}: every row painted"
        );
        for &(r, c, tile) in &cells {
            let (s, e) = spans[usize::from(r)];
            assert_eq!(
                ow::decode_row(&painted[s..e]).unwrap()[usize::from(c)],
                tile,
                "region {region} cell ({r}, {c})"
            );
        }
        let back = ow::unpaint_window(&painted, height, &backup).unwrap();
        let used = ow::row_spans(win, height)[height - 1].1;
        assert_eq!(&back[..used], &win[..used], "region {region} round trip");
    }
    // The Dark Isle: land and shallows in East Hyrule's south-east corner
    // (an entrance tile on it shows the entrance).
    let east = ow::edits(plan, 2);
    for &(row, col, shallow) in &z2_content::areas::dark_isle::ISLE_TERRAIN {
        let want = placed
            .iter()
            .find(|p| (p.region, p.row, p.col) == (2, row, col))
            .and_then(|p| p.tile)
            .unwrap_or(if shallow { 0xD } else { 0x4 });
        assert!(east.contains(&(row, col, want)), "isle cell ({row}, {col})");
    }
}
