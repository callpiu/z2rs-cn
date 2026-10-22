//! Town people in virtual rooms (`z2_core::enh::content_town`) in a real
//! run, on the developer town `Dev Inn` (`z2_content::areas::dev_rooms`):
//!
//! * the talker types each of its three messages from the WRAM text
//!   buffer, letter for letter into the dialog box (the PPU packets the
//!   typewriter builds and the nametable bytes they leave);
//! * the two healers start the game's refill and fill life and magic;
//! * the hint giver says its locked line until the sage in the back room
//!   (reached by pressing Up on the doorway) has set the group flag; leaving
//!   the back room puts Link at that doorway, not at the street's edge;
//! * an original town's dialog is the same with the group on and off.
//!
//! ROM-gated (self-skips unless `Z2_ROM` names a file). Link enters the
//! inn the way the engine's own town test does (town map 1 through the
//! game's loader, then the door routine `$CFFC` on page 1, house depth
//! `$075B` set as the Up press would). With `Z2_CONTENT_SHOTS=DIR`
//! (outside the work tree: frames come from the ROM) it saves PNGs.

mod common;

use z2_content::areas::dev_rooms::{inn_npcs, INN_BACK_DOOR_X, INN_TALKER_LINES};
use z2_content::text;
use z2_core::enh::content::{self, ContentOpts, Group};
use z2_core::enh::content_town::{self, TEXT_BUF, TEXT_BUF_LEN};
use z2_core::enh::Enhancements;
use z2_core::game::{
    Game, INPUT_A, INPUT_B, INPUT_LEFT, INPUT_RIGHT, INPUT_SELECT, INPUT_START, INPUT_UP,
};

const A: u8 = 1 << INPUT_A;
const B: u8 = 1 << INPUT_B;
const SELECT: u8 = 1 << INPUT_SELECT;
const START: u8 = 1 << INPUT_START;
const LEFT: u8 = 1 << INPUT_LEFT;
const RIGHT: u8 = 1 << INPUT_RIGHT;
const UP: u8 = 1 << INPUT_UP;

const MODE: usize = 0x0736;
const MODE_SIDEVIEW: u8 = 0x0B;
const SCENE: usize = 0x0561;
const WORLD: usize = 0x0707;
const PAGE: usize = 0x003B;
const LINK_X: usize = 0x004D;
const DIALOG: usize = 0x074C;
const ROUTINE: usize = 0x0524;
/// `$0524` while the box waits for B.
const WAIT_B: u8 = 9;
const HOUSE_DEPTH: usize = 0x075B;
const LIFE: usize = 0x0774;
const MAGIC: usize = 0x0773;
/// The towns slot map.
const TOWN_SLOT: u8 = 62;

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

fn step_until(g: &mut Game, max: usize, pad: u8, f: impl Fn(&Game) -> bool) -> bool {
    for _ in 0..max {
        if f(g) {
            return true;
        }
        g.step(pad);
    }
    f(g)
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

/// Load map `map` of world `world` through the game's own loader.
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
    step_until(g, 600, 0, |g| g.ram[MODE] == MODE_SIDEVIEW)
}

/// Call the door routine `$CFFC` as the game does for a doorway on `page`.
fn take_door(g: &mut Game, page: u8) {
    g.ram[PAGE] = page;
    let at = 0x0900; // $6900, engine scratch
    g.wram[at..at + 4].copy_from_slice(&[0x20, 0xFC, 0xCF, 0x60]);
    g.call_asm(0x6900);
}

fn shot(g: &Game, name: &str) {
    if let Some(dir) = std::env::var_os("Z2_CONTENT_SHOTS") {
        let png = z2_ppu::encode_indexed_png(g.frame_indexed());
        let path = std::path::Path::new(&dir).join(format!("{name}.png"));
        std::fs::write(path, png).expect("write shot");
    }
}

fn inn_id() -> content::VRoomId {
    content::find_room("Dev Inn", 0).expect("Dev Inn")
}

/// A game in the inn (page 1, by the door from town map 1).
fn in_the_inn(rom: &[u8]) -> Game {
    let mut g = game(rom, content_on());
    reach_gameplay(&mut g);
    assert!(
        load_scene(&mut g, 0, 1, 0x2D, content::DEV_INN_FROM_MAP, 1),
        "town map"
    );
    g.ram[HOUSE_DEPTH] = 1;
    take_door(&mut g, 1);
    assert!(
        step_until(&mut g, 800, 0, |g| g.ram[MODE] == MODE_SIDEVIEW
            && content::current_room(g) == Some(inn_id())),
        "inn loaded"
    );
    step(&mut g, 60, 0);
    assert_eq!(g.ram[SCENE], TOWN_SLOT);
    g
}

/// Object slot of person `npc` of the `Dev Inn` area (in Link's room),
/// once spawned: its enemy-list offset `$BC,x` is `1 + 2 * index`.
fn npc_slot(g: &Game, npc: u8) -> Option<usize> {
    let id = content::current_room(g).unwrap_or_else(inn_id);
    let r = content::room(id).unwrap();
    let off = 1 + 2 * r.area.npcs[usize::from(npc)].enemy_index;
    (0..6).find(|&s| g.ram[0xB6 + s] != 0 && g.ram[0xBC + s] == off)
}

/// Each letter one talk typed: nametable address and tile, from the
/// typewriter's PPU packets (`$0301 = 5`, address `$0302/$0303`, tile
/// `$0306`).
type Typed = Vec<(u16, u8)>;

/// Record the typing until the box waits for B, check the letters are in
/// the nametable, close the box.
fn read_box(g: &mut Game, name: &str) -> Typed {
    let mut typed = Typed::new();
    let mut last = None;
    for _ in 0..1200 {
        g.step(0);
        if g.ram[0x0301] == 5 && g.ram[0x0307] == 0xFF {
            let addr = u16::from_be_bytes([g.ram[0x0302], g.ram[0x0303]]);
            if last != Some(addr) {
                last = Some(addr);
                typed.push((addr, g.ram[0x0306]));
            }
        }
        if g.ram[ROUTINE] == WAIT_B {
            break;
        }
    }
    assert_eq!(g.ram[ROUTINE], WAIT_B, "box waits for B");
    step(g, 4, 0);
    // The packet writes two tiles down a column (`$0304 = $82`): the
    // accent row, then the letter one row below.
    for &(addr, tile) in &typed {
        let at = addr.wrapping_add(32);
        assert_eq!(
            g.ppu.model().nt_read(at),
            tile,
            "nametable ${at:04X} holds the typed letter"
        );
    }
    shot(g, name);
    step(g, 2, B);
    assert!(
        step_until(g, 600, 0, |g| g.ram[DIALOG] == 0 && g.ram[ROUTINE] == 0),
        "box closed"
    );
    step(g, 30, 0);
    typed
}

/// Link's position in pixels from the room's left edge.
fn link_pos(g: &Game) -> i32 {
    i32::from(g.ram[PAGE]) * 256 + i32::from(g.ram[LINK_X])
}

/// Right edge of the current virtual room, in pixels.
fn room_right(g: &Game) -> i32 {
    let pages = content::current_room(g)
        .and_then(content::room)
        .map_or(1, |r| ((r.room.sideview[1] >> 5) & 3) + 1);
    i32::from(pages) * 256
}

/// Walk Link to room pixel `x`.
fn walk_to(g: &mut Game, x: i32) -> bool {
    for _ in 0..600 {
        let dx = x - link_pos(g);
        if dx.abs() <= 8 {
            step(g, 10, 0);
            // Settle on the exact column (same page: a nudge, not a jump).
            if i32::from(g.ram[PAGE]) == x / 256 {
                g.ram[LINK_X] = (x % 256) as u8;
            }
            step(g, 2, 0);
            return (x - link_pos(g)).abs() <= 2;
        }
        g.step(if dx > 0 { RIGHT } else { LEFT });
        if dx.abs() < 24 {
            g.step(0);
        }
    }
    false
}

/// Walk Link toward the object in slot `s` (or right while `None`), and
/// press B once he touches it. True once a box opens.
fn approach_and_press_b(g: &mut Game, s: Option<usize>) -> bool {
    let Some(s) = s else {
        // Not spawned yet: walk right (not out of the room) to bring it in.
        let pad = if link_pos(g) < room_right(g) - 48 {
            RIGHT
        } else {
            0
        };
        g.step(pad);
        return false;
    };
    let pos = |g: &Game, s: usize| i32::from(g.ram[0x3C + s]) * 256 + i32::from(g.ram[0x4E + s]);
    let dx = pos(g, s) - link_pos(g);
    if dx.abs() > 6 {
        // Follow, but never off either edge of the room (its exits).
        let pad = if dx > 0 { RIGHT } else { LEFT };
        let blocked = (dx < 0 && link_pos(g) < 32) || (dx > 0 && link_pos(g) > room_right(g) - 32);
        let before = (link_pos(g), g.ram[SCENE], dx);
        g.step(if blocked { 0 } else { pad });
        assert_eq!(g.ram[SCENE], before.1, "left the room from {before:X?}");
        return false;
    }
    // Only the one person in reach (B talks to whoever Link touches).
    let crowded =
        (0..6).any(|o| o != s && g.ram[0xB6 + o] != 0 && (pos(g, o) - link_pos(g)).abs() < 24);
    g.step(0);
    if crowded {
        return false;
    }
    g.step(B);
    g.ram[DIALOG] != 0
}

/// Walk Link up to person `npc`, press B, read the box.
fn talk(g: &mut Game, npc: u8) -> Typed {
    assert_eq!(g.ram[DIALOG], 0, "no box open");
    for _ in 0..600 {
        let s = npc_slot(g, npc);
        // The wise man starts talking by himself when Link comes close.
        if approach_and_press_b(g, s) || g.ram[DIALOG] != 0 {
            assert_eq!(s, Some(usize::from(g.ram[0x048B])), "talking to {npc}");
            return read_box(g, &format!("inn-talk-{npc}"));
        }
    }
    let slots: Vec<String> = (0..6)
        .filter(|&s| g.ram[0xB6 + s] != 0)
        .map(|s| {
            format!(
                "id {:02X} off {:02X} at {:02X}/{:02X} af {:02X} bd {:02X} a8 {:02X}",
                g.ram[0xA1 + s],
                g.ram[0xBC + s],
                g.ram[0x3C + s],
                g.ram[0x4E + s],
                g.ram[0xAF + s],
                g.ram[0x5BD + s],
                g.ram[0xA8 + s]
            )
        })
        .collect();
    panic!(
        "talk to person {npc} never started (map {}, room {:?}, link {:04X}): {slots:#?}",
        g.ram[SCENE],
        content::current_room(g),
        link_pos(g)
    );
}

/// The tiles `message` types (letters and spaces; line breaks and the end
/// byte type nothing).
fn tiles_of(message: &[u8]) -> Vec<u8> {
    message
        .iter()
        .take_while(|&&b| b != text::END)
        .filter(|&&b| b < text::NEWLINE)
        .map(|&b| z2_core::town_dialog::decode_tile(b).0)
        .collect()
}

fn assert_said(g: &Game, typed: &Typed, line: &str) {
    let bytes = text::encode_message(line).unwrap();
    let tiles: Vec<u8> = typed.iter().map(|&(_, t)| t).collect();
    assert_eq!(tiles, tiles_of(&bytes), "typed {line:?}");
    let at = usize::from(TEXT_BUF - 0x6000);
    assert_eq!(
        &g.wram()[at..at + bytes.len()],
        &bytes[..],
        "staged in the WRAM text buffer"
    );
    assert!(bytes.len() <= TEXT_BUF_LEN);
}

fn message(npc: u8) -> String {
    message_of("Dev Inn", npc)
}

fn message_of(area: &str, npc: u8) -> String {
    let r = content::room(content::find_room(area, 0).unwrap()).unwrap();
    let n = r.area.npcs[usize::from(npc)];
    text::decode(r.area.messages[usize::from(n.message)])
}

#[test]
fn talker_types_each_of_its_messages_from_wram() {
    let t = "talker_types_each_of_its_messages_from_wram";
    let Some(rom) = common::rom_bytes(t) else {
        return;
    };
    let mut g = in_the_inn(&rom);
    // Three talks, three messages; the fourth repeats the last.
    for line in INN_TALKER_LINES.iter().chain([&INN_TALKER_LINES[2]]) {
        let typed = talk(&mut g, inn_npcs::TALKER);
        assert_said(&g, &typed, line);
        // The text pointer ran through the buffer, not a ROM message.
        let ptr = u16::from_le_bytes([g.ram[0x0569], g.ram[0x056A]]);
        assert!((TEXT_BUF..TEXT_BUF + TEXT_BUF_LEN as u16).contains(&ptr));
    }
    assert_eq!(g.exec_errors, 0);
}

#[test]
fn healers_refill_life_and_magic() {
    let t = "healers_refill_life_and_magic";
    let Some(rom) = common::rom_bytes(t) else {
        return;
    };
    let mut g = in_the_inn(&rom);
    let (life, magic) = (g.ram[LIFE], g.ram[MAGIC]);
    assert!(life > 0x10 && magic > 0x10, "full meters at the start");
    g.ram[LIFE] = 0x08;
    g.ram[MAGIC] = 0x08;
    let typed = talk(&mut g, inn_npcs::HEALER);
    assert_said(&g, &typed, &message(inn_npcs::HEALER));
    assert!(
        step_until(&mut g, 600, 0, |g| g.ram[LIFE] == life),
        "life refilled ({:02X})",
        g.ram[LIFE]
    );
    assert_eq!(g.ram[MAGIC], 0x08, "the life healer leaves magic alone");
    let typed = talk(&mut g, inn_npcs::MAGE);
    assert_said(&g, &typed, &message(inn_npcs::MAGE));
    assert!(
        step_until(&mut g, 600, 0, |g| g.ram[MAGIC] == magic),
        "magic refilled ({:02X})",
        g.ram[MAGIC]
    );
    assert_eq!(g.ram[LIFE], life);
    // The plain talker heals nothing.
    g.ram[LIFE] = 0x08;
    talk(&mut g, inn_npcs::TALKER);
    step(&mut g, 120, 0);
    assert_eq!(g.ram[LIFE], 0x08);
    assert_eq!(g.exec_errors, 0);
}

#[test]
fn sage_unlocks_the_hint_and_the_house_door_returns_link() {
    let t = "sage_unlocks_the_hint_and_the_house_door_returns_link";
    let Some(rom) = common::rom_bytes(t) else {
        return;
    };
    let mut g = in_the_inn(&rom);
    let hint = z2_content::areas::npc_scripts()
        .find(|s| s.area == "Dev Inn" && s.npc == inn_npcs::HINT)
        .unwrap();
    assert!(!content::flag(&g, Group::TestRoom, 0));
    let typed = talk(&mut g, inn_npcs::HINT);
    assert_said(&g, &typed, hint.locked.unwrap().text);

    // Up on the doorway (page 1) into the back room.
    // Link's x is 8 px left of the column the door check reads.
    let door = i32::from(INN_BACK_DOOR_X) * 16 - 8;
    assert!(
        walk_to(&mut g, door),
        "at the doorway ({:04X})",
        link_pos(&g)
    );
    shot(&g, "inn-at-door");
    let back = content::find_room("Dev Inn", 1).unwrap();
    assert!(
        step_until(&mut g, 120, UP, |g| g.ram[HOUSE_DEPTH] == 2),
        "Up on the doorway enters it"
    );
    assert!(
        step_until(&mut g, 800, 0, |g| g.ram[MODE] == MODE_SIDEVIEW
            && content::current_room(g) == Some(back)),
        "back room loaded"
    );
    step(&mut g, 60, 0);
    shot(&g, "inn-back");
    let typed = talk(&mut g, inn_npcs::SAGE);
    assert_said(&g, &typed, &message(inn_npcs::SAGE));
    assert!(content::flag(&g, Group::TestRoom, 0), "the sage set flag 0");
    // The wise man's own condition never ran: no spell learned.
    assert!(g.ram[0x077B..0x0783].iter().all(|&s| s == 0), "no spell");

    // Out to the left: back at the doorway, not at the inn's right edge.
    assert!(
        step_until(&mut g, 600, LEFT, |g| content::current_room(g)
            != Some(back)),
        "left the back room"
    );
    assert!(
        step_until(&mut g, 800, 0, |g| g.ram[MODE] == MODE_SIDEVIEW
            && content::current_room(g) == Some(inn_id())),
        "inn reloaded"
    );
    step(&mut g, 30, 0);
    shot(&g, "inn-door-return");
    assert_eq!(g.ram[HOUSE_DEPTH], 1, "one house level left");
    assert!(
        (link_pos(&g) - door).abs() <= 16,
        "back at the doorway: {:04X}, door {door:04X}",
        link_pos(&g)
    );

    // The hint now says its message.
    let typed = talk(&mut g, inn_npcs::HINT);
    assert_said(&g, &typed, &message(inn_npcs::HINT));
    assert_eq!(g.exec_errors, 0);
}

/// One talk in an original town (Rauru): the text pointer, the typed
/// letters and the quest bytes `$0796-$079F` afterwards.
fn vanilla_talk(rom: &[u8], e: Enhancements) -> (u16, Typed, Vec<u8>) {
    let mut g = game(rom, e);
    reach_gameplay(&mut g);
    assert!(load_scene(&mut g, 0, 1, 0x2D, 0, 0), "Rauru");
    step(&mut g, 60, 0);
    let mut typed = None;
    for _ in 0..900 {
        let s = (0..6).find(|&s| g.ram[0xB6 + s] != 0 && g.ram[0xA1 + s] == 0x22);
        if approach_and_press_b(&mut g, s) {
            typed = Some(read_box(&mut g, "rauru-talk"));
            break;
        }
    }
    if typed.is_none() {
        for s in 0..6 {
            eprintln!(
                "id {:02X} ex {:02X} at {:02X}/{:02X} af {:02X} bd {:02X} a8 {:02X}",
                g.ram[0xA1 + s],
                g.ram[0xB6 + s],
                g.ram[0x3C + s],
                g.ram[0x4E + s],
                g.ram[0xAF + s],
                g.ram[0x5BD + s],
                g.ram[0xA8 + s]
            );
        }
    }
    let typed = typed.expect("talk started");
    let ptr = u16::from_le_bytes([g.ram[0x0569], g.ram[0x056A]]);
    step(&mut g, 200, 0);
    assert_eq!(g.exec_errors, 0);
    (ptr, typed, g.ram[0x0796..0x07A0].to_vec())
}

#[test]
fn original_town_dialog_is_unchanged_with_the_group_on() {
    let t = "original_town_dialog_is_unchanged_with_the_group_on";
    let Some(rom) = common::rom_bytes(t) else {
        return;
    };
    let off = vanilla_talk(&rom, Enhancements::default());
    let on = vanilla_talk(&rom, content_on());
    assert!(!off.1.is_empty(), "typed something");
    assert!(off.0 >= 0x8000, "ROM text, not the WRAM buffer");
    assert_eq!(on, off, "same pointer, letters and quest bits");
}

/// The column of the doorway threshold on `page` of the level buffer.
fn door_column(g: &Game, page: usize) -> Option<usize> {
    (0..13 * 16)
        .find(|&i| g.wram()[page * 0xD0 + i] == content_town::DOOR_TILES[3])
        .map(|i| i % 16)
}

/// A minigame's street doorway (`VanillaLink::door` on a page with no
/// original door): painted at load, entered with Up into the link's room.
fn link_doorway_on_a_bare_street_page(rom: &[u8], area: &str) {
    use z2_content::areas::minigames::ATTACH;
    let at = ATTACH.iter().find(|a| a.area == area).unwrap();
    let e = Enhancements {
        content: ContentOpts {
            virtual_rooms: true,
            minigames: true,
            ..ContentOpts::default()
        },
        ..Enhancements::default()
    };
    // Off: the page has no doorway.
    let mut og = game(rom, Enhancements::default());
    reach_gameplay(&mut og);
    let (region, world) = (at.region.id(), at.world.id());
    assert!(load_scene(
        &mut og,
        region,
        world,
        at.town_slot,
        at.street,
        at.door_slot
    ));
    assert_eq!(
        door_column(&og, usize::from(at.door_slot)),
        None,
        "{area}: no original door"
    );

    let mut g = game(rom, e);
    reach_gameplay(&mut g);
    assert!(load_scene(
        &mut g,
        region,
        world,
        at.town_slot,
        at.street,
        at.door_slot
    ));
    step(&mut g, 30, 0);
    let col = door_column(&g, usize::from(at.door_slot)).expect("doorway painted");
    shot(&g, &format!("link-door-{}", at.street));
    let door = i32::from(at.door_slot) * 256 + col as i32 * 16 - 8;
    assert!(
        walk_to(&mut g, door),
        "{area}: at the doorway ({:04X})",
        link_pos(&g)
    );
    let want = content::find_room(area, 0).unwrap();
    let depth = g.ram[HOUSE_DEPTH];
    assert!(
        step_until(&mut g, 120, UP, |g| g.ram[HOUSE_DEPTH] == depth + 1),
        "{area}: Up on the painted doorway enters it"
    );
    assert!(
        step_until(&mut g, 800, 0, |g| g.ram[MODE] == MODE_SIDEVIEW
            && content::current_room(g) == Some(want)),
        "{area}: the link's room loaded"
    );
    step(&mut g, 30, 0);
    shot(&g, &format!("link-door-{}-in", at.street));
    assert_eq!(g.exec_errors, 0);
}

#[test]
fn minigame_doorways_appear_on_bare_street_pages() {
    let t = "minigame_doorways_appear_on_bare_street_pages";
    let Some(rom) = common::rom_bytes(t) else {
        return;
    };
    link_doorway_on_a_bare_street_page(&rom, "Target Gallery");
    link_doorway_on_a_bare_street_page(&rom, "Pogo Tower");
}

/// Grimhollow (`dark_isle`): its people talk with their own text, the
/// healer lady keeps her original walk to her door, the wise man (no
/// script) only talks.
#[test]
fn grimhollow_people_say_their_lines() {
    let t = "grimhollow_people_say_their_lines";
    let Some(rom) = common::rom_bytes(t) else {
        return;
    };
    let e = Enhancements {
        content: ContentOpts {
            virtual_rooms: true,
            dark_isle: true,
            ..ContentOpts::default()
        },
        ..Enhancements::default()
    };
    let enter = |room: &'static str| {
        let mut g = game(&rom, e);
        reach_gameplay(&mut g);
        let want = content::find_room("Grimhollow", grim_room(room)).unwrap();
        assert!(content::enter_from_overworld(&mut g, want));
        assert!(
            load_scene(&mut g, 2, 2, 0x2E, TOWN_SLOT, 0),
            "{room} loaded"
        );
        step(&mut g, 60, 0);
        assert_eq!(content::current_room(&g), Some(want));
        g
    };
    let said = |typed: &Typed, npc: u8| {
        let bytes = text::encode_message(&message_of("Grimhollow", npc)).unwrap();
        let tiles: Vec<u8> = typed.iter().map(|&(_, t)| t).collect();
        assert_eq!(tiles, tiles_of(&bytes), "Grimhollow person {npc}");
    };
    let mut g = enter("gate_street");
    shot(&g, "grim-gate");
    for npc in [0u8, 1] {
        let typed = talk(&mut g, npc);
        said(&typed, npc);
    }
    assert_eq!(g.exec_errors, 0);

    let mut g = enter("market_street");
    let typed = talk(&mut g, 3);
    said(&typed, 3);
    let s = npc_slot(&g, 3).expect("healer lady still here");
    // `$9A50` (her condition on the street) sets her door timer to $FF;
    // it counts down a frame at a time from there (spawn value $90).
    assert!(g.ram[0x05BD + s] > 0xC0, "her original door walk started");
    assert_eq!(g.exec_errors, 0);

    let mut g = enter("sage_house");
    let typed = talk(&mut g, 6);
    said(&typed, 6);
    assert!(g.ram[0x077B..0x0783].iter().all(|&s| s == 0), "no spell");
    shot(&g, "grim-sage");
    assert_eq!(g.exec_errors, 0);
}

/// Index of Grimhollow's room `id` in its area.
fn grim_room(id: &'static str) -> u8 {
    z2_content::areas::dark_isle::areas()
        .iter()
        .find(|a| a.name == "Grimhollow")
        .and_then(|a| a.room_index(id))
        .unwrap() as u8
}
