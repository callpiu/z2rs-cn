//! ROM-gated checks of every authored area of every option
//! (`z2_content::areas::authored`): the movement check on the game's real
//! floor patterns, that every declared ability is needed by the geometry,
//! and that every attach point (vanilla palace links, the overworld slots
//! of `areas::SLOTS`) is sound in the vanilla game read from the player's
//! ROM. Nothing read here is stored. Area-specific checks are in
//! `caves_minigames_rom.rs` and `dark_isle_rom.rs`.
//!
//! `Z2_ROM=/path/to/zelda2.nes cargo test -p z2-content --test areas_rom`
//! (skips when `Z2_ROM` is not set).

mod common;

use std::collections::{BTreeSet, VecDeque};

use z2_content::areas::{self, dark_isle};
use z2_content::model::{Abilities, Area, Dir, LinkVia, Region, Target, World};
use z2_content::sideview::{FloorTable, ObjectSet, Sideview, PAGE_COLS, ROWS};
use z2_content::validate_geometry;
use z2_rando::enemies::data::{self, EnemySets};
use z2_rando::enemies::shuffle::{self, BLOB_ADDR, BLOB_WRAM, OVERWORLD, PALACES};
use z2_rando::overworld::continent::Continent;
use z2_rando::overworld::loc;
use z2_rando::overworld::map::Cont;
use z2_rando::overworld::terrain::Terrain;
use z2_rando::rom::Rom;

use common::body;

fn issues(r: Result<(), Vec<z2_content::Issue>>) -> String {
    match r {
        Ok(()) => String::new(),
        Err(v) => v
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join("\n"),
    }
}

#[test]
fn areas_pass_geometry_on_the_real_floor_patterns() {
    let Some(body) = body() else { return };
    let floors = FloorTable::from_body(&body).unwrap();
    for (g, a) in areas::authored() {
        assert_eq!(
            issues(validate_geometry(&a, &floors)),
            "",
            "{g}::{}",
            a.name
        );
    }
}

/// The abilities the movement check models.
const GEOMETRIC: [Abilities; 3] = [Abilities::JUMP, Abilities::GLOVE, Abilities::KEY];

fn without(a: Abilities, b: Abilities) -> Abilities {
    Abilities(a.0 & !b.0)
}

/// Every geometric requirement a room, an exit or a room's items declare
/// is really needed: taking it away makes the movement check fail.
#[test]
fn every_declared_requirement_is_needed_by_the_geometry() {
    let Some(body) = body() else { return };
    let floors = FloorTable::from_body(&body).unwrap();
    let mut checked = 0;
    for (_, a) in areas::authored() {
        let name = a.name;
        for ri in 0..a.rooms.len() {
            for b in GEOMETRIC {
                // Room-wide.
                if a.rooms[ri].requires.has(b) {
                    let mut c = a.clone();
                    c.rooms[ri].requires = without(c.rooms[ri].requires, b);
                    assert!(
                        validate_geometry(&c, &floors).is_err(),
                        "{name}/{}: passes without {b:?}",
                        a.rooms[ri].id
                    );
                    checked += 1;
                }
                // Items.
                if a.rooms[ri].item_requires.has(b) {
                    let mut c = a.clone();
                    c.rooms[ri].item_requires = without(c.rooms[ri].item_requires, b);
                    assert!(
                        validate_geometry(&c, &floors).is_err(),
                        "{name}/{}: items reachable without {b:?}",
                        a.rooms[ri].id
                    );
                    checked += 1;
                }
                // Exits.
                for ei in 0..a.rooms[ri].exits.len() {
                    if !a.rooms[ri].exits[ei].requires.has(b) {
                        continue;
                    }
                    let mut c = a.clone();
                    let room = &mut c.rooms[ri];
                    room.exits[ei].requires = without(room.exits[ei].requires, b);
                    // Keep KEY declared somewhere so only the geometry can
                    // object (a locked door must name a key).
                    if b == Abilities::KEY {
                        room.item_requires = room.item_requires.with(b);
                    }
                    assert!(
                        validate_geometry(&c, &floors).is_err(),
                        "{name}/{}: {:?} exit open without {b:?}",
                        a.rooms[ri].id,
                        a.rooms[ri].exits[ei].dir
                    );
                    checked += 1;
                }
            }
        }
    }
    // Annex: KEY (door) and JUMP (sluice); challenges: JUMP x3, GLOVE;
    // the Dark Isle: JUMP x2, GLOVE, KEY.
    assert!(checked >= 10, "only {checked} requirements checked");
}

// ------------------------------------------------------------ vanilla links

struct Table {
    bank: u8,
    sideview: u16,
    conn: u16,
    set: ObjectSet,
}

fn table(w: World) -> Table {
    match w {
        World::PalaceA => Table {
            bank: 4,
            sideview: 0x8523,
            conn: 0x871B,
            set: ObjectSet::Palace,
        },
        World::PalaceB => Table {
            bank: 4,
            sideview: 0xA000,
            conn: 0xA1F8,
            set: ObjectSet::Palace,
        },
        World::GreatPalace => Table {
            bank: 5,
            sideview: 0x8523,
            conn: 0x871B,
            set: ObjectSet::GreatPalace,
        },
        other => panic!("no vanilla link table for {other:?}"),
    }
}

fn vanilla_room(rom: &Rom, t: &Table, map: u8) -> (Sideview, [u8; 4]) {
    let ptr = rom
        .read_cpu_word(t.bank, t.sideview + 2 * u16::from(map))
        .unwrap();
    let off = rom.cpu_offset(t.bank, ptr).unwrap();
    let len = usize::from(rom.read(off).unwrap());
    let sv = Sideview::parse(rom.read_slice(off, len).unwrap()).unwrap();
    let mut conn = [0u8; 4];
    for (i, c) in conn.iter_mut().enumerate() {
        *c = rom
            .read_cpu(t.bank, t.conn + 4 * u16::from(map) + i as u16)
            .unwrap();
    }
    (sv, conn)
}

/// Rows Link can stand on in column `x` of `g`.
fn stand_rows(g: &z2_content::sideview::Grid, x: usize) -> BTreeSet<usize> {
    (1..ROWS - 1)
        .filter(|&y| !g.is_solid(x, y) && !g.is_solid(x, y - 1) && g.is_solid(x, y + 1))
        .collect()
}

/// Each palace link re-points a side exit of a vanilla room that walks out
/// to the overworld in the original game (so the room keeps another way
/// out), the floor heights on both sides of the seam line up, and the
/// authored room leads back onto the vanilla room's edge page. (Town door
/// links are checked in `caves_minigames_rom.rs`.)
#[test]
fn vanilla_links_are_sound() {
    let Some(body) = body() else { return };
    let rom = Rom::from_body(&body).unwrap();
    let floors = FloorTable::from_body(&body).unwrap();
    let mut n = 0;
    for (_, a) in areas::authored() {
        if !matches!(
            a.world,
            World::PalaceA | World::PalaceB | World::GreatPalace
        ) {
            continue;
        }
        for &l in &a.links {
            let LinkVia::Byte(exit) = l.via else {
                panic!("{}: door link outside the towns", a.name);
            };
            n += 1;
            check_link(&rom, &floors, &a, l, exit);
        }
    }
    assert!(n >= 1, "no palace links");
}

fn check_link(rom: &Rom, floors: &FloorTable, a: &Area, l: z2_content::VanillaLink, exit: u8) {
    let t = table(a.world);
    let (sv, conn) = vanilla_room(rom, &t, l.map);
    let pages = sv.pages();
    let g = sv.grid(t.set, floors);
    // Only side exits are linked so far.
    let side = match usize::from(exit) {
        0 => Dir::Left,
        i if i + 1 == usize::from(pages) => Dir::Right,
        i => panic!("link on byte {i} is not a side exit"),
    };
    assert!(
        conn[usize::from(exit)] >= 0xFC,
        "map {} byte {} is in use in the original",
        l.map,
        exit
    );
    let other = if side == Dir::Right {
        0
    } else {
        usize::from(pages) - 1
    };
    assert!(
        conn[other] >= 0xFC,
        "map {} would lose its way out to the overworld",
        l.map
    );
    let vcol = if side == Dir::Right { g.width() - 1 } else { 0 };
    let vrows = stand_rows(&g, vcol);
    assert!(!vrows.is_empty(), "map {} edge is walled", l.map);

    let r = a.room(l.room).unwrap();
    let ag = z2_content::validate::room_grid(a, r, floors).unwrap();
    // Link walks right out of the vanilla room onto page 0 of ours
    // (or left onto our last page).
    let (acol, back_dir) = if side == Dir::Right {
        assert_eq!(l.page, 0);
        (0, Dir::Left)
    } else {
        assert_eq!(l.page + 1, r.pages);
        (ag.width() - 1, Dir::Right)
    };
    let arows = stand_rows(&ag, acol);
    assert!(
        !vrows.is_disjoint(&arows),
        "standing rows differ across the seam: vanilla {vrows:?}, authored {arows:?}"
    );
    let back = r.exits.iter().find(|e| e.dir == back_dir).unwrap();
    let edge_page = if side == Dir::Right { pages - 1 } else { 0 };
    assert_eq!(
        back.target,
        Target::Map {
            map: l.map,
            page: edge_page
        }
    );
    assert_eq!(acol / PAGE_COLS, usize::from(l.page));
}

// ------------------------------------------------------------ overworld

fn cont(r: Region) -> Cont {
    match r {
        Region::West => Cont::West,
        Region::DeathMountain => Cont::DeathMountain,
        Region::East => Cont::East,
        Region::MazeIsland => Cont::Maze,
    }
}

fn open_ground(t: Terrain) -> bool {
    matches!(
        t,
        Terrain::Desert
            | Terrain::Grass
            | Terrain::Forest
            | Terrain::Swamp
            | Terrain::Grave
            | Terrain::Road
            | Terrain::Bridge
            | Terrain::Town
            | Terrain::Cave
            | Terrain::Palace
    )
}

/// Continents whose location tables a slot in `c` must stay clear of
/// (Death Mountain and Maze Island share a layout).
fn sharing(c: Cont) -> Vec<Cont> {
    match c {
        Cont::DeathMountain | Cont::Maze => vec![Cont::DeathMountain, Cont::Maze],
        _ => vec![c],
    }
}

/// Every claimed slot is unused in the original (all four bytes zero on
/// West and East; on Death Mountain and Maze Island no vanilla use in
/// either copy of the shared layout), every entrance tile is on the map
/// and holds no vanilla location (of either shared copy), and entrances
/// placed by a landmark sit on open ground joined to it (no boulders,
/// water or mountains). The Dark Isle's isle-side tiles are painted over
/// the sea by the engine (`dark_isle_rom.rs` checks those).
#[test]
fn overworld_entrances_are_sound() {
    let Some(body) = body() else { return };
    let rom = Rom::from_body(&body).unwrap();
    let all = areas::authored();
    let isle: Vec<&str> = dark_isle::areas().iter().map(|a| a.name).collect();
    for s in areas::SLOTS {
        let k = cont(s.region);
        let slot = usize::from(s.slot);
        assert!(slot < 0x3E, "{}: slot {slot} is not scanned", s.area);
        let c = Continent::vanilla(&rom, k).unwrap();
        match k {
            Cont::West | Cont::East => assert_eq!(
                c.slot_bytes(slot),
                [0; 4],
                "{} slot {} is in use",
                s.area,
                s.slot
            ),
            _ => {
                for o in sharing(k) {
                    assert!(loc::info(o, slot).is_none(), "{}: {o:?}", s.area);
                    let l = loc::read_table(&rom, o).unwrap()[slot];
                    assert!(l.pos().is_none(), "{}: on the map in {o:?}", s.area);
                }
            }
        }
        let (_, a) = all.iter().find(|(_, a)| a.name == s.area).unwrap();
        let e = a.entrances[usize::from(s.entrance)];
        let (row, col) = (usize::from(e.row), usize::from(e.col));
        assert!(row < c.rows && col < c.cols, "{}: off the map", s.area);
        for o in sharing(k) {
            for (i, l) in loc::read_table(&rom, o).unwrap().iter().enumerate() {
                assert_ne!(l.pos(), Some((row, col)), "{}: slot {i} of {o:?}", s.area);
            }
        }
        assert!(c.slot_at(row, col).is_none(), "{}: tile is taken", s.area);
        let t = c.grid.get(row, col);
        let painted = isle.contains(&s.area) && t == Terrain::Water;
        if !painted {
            assert!(
                open_ground(t) && !matches!(t, Terrain::Town | Terrain::Cave | Terrain::Palace),
                "{}: tile is {t:?}",
                s.area
            );
        }
        let Some(near) = s.near else { continue };
        let start = c.pos(usize::from(near)).unwrap();
        let mut seen = vec![false; 75 * 64];
        let mut q = VecDeque::from([start]);
        seen[start.0 * 64 + start.1] = true;
        let mut found = false;
        while let Some((r, k)) = q.pop_front() {
            if (r, k) == (row, col) {
                found = true;
                break;
            }
            for (dr, dk) in [(-1i32, 0i32), (1, 0), (0, -1), (0, 1)] {
                let (nr, nk) = (r as i32 + dr, k as i32 + dk);
                if nr < 0 || nk < 0 || nr as usize >= c.rows || nk as usize >= c.cols {
                    continue;
                }
                let (nr, nk) = (nr as usize, nk as usize);
                if !seen[nr * 64 + nk] && open_ground(c.grid.get(nr, nk)) {
                    seen[nr * 64 + nk] = true;
                    q.push_back((nr, nk));
                }
            }
        }
        assert!(
            found,
            "{}: ({row}, {col}) is not joined to slot {near}",
            s.area
        );
    }
}

// ------------------------------------------------------------ enemies

/// IDs used in the vanilla enemy lists of `group`'s maps `maps`.
fn vanilla_ids(rom: &Rom, group: &shuffle::Area, maps: std::ops::Range<u16>) -> BTreeSet<u8> {
    let mut ids = BTreeSet::new();
    for m in maps {
        let Ok(wram) = rom.read_cpu_word(group.bank, group.enemies + 2 * m) else {
            continue;
        };
        if !(BLOB_WRAM..BLOB_WRAM + 0x400).contains(&wram) {
            continue;
        }
        let addr = wram - BLOB_WRAM + BLOB_ADDR;
        if let Ok(list) = shuffle::read_list(rom, group.bank, addr) {
            ids.extend(list.iter().map(|e| e.id));
        }
    }
    ids
}

/// Every enemy an area places is one the original game places in the same
/// enemy group, or one the randomizer's catalog lists for it (for the
/// annex: only what palace 1 itself places, maps 0-13).
#[test]
fn enemies_belong_to_their_world() {
    let Some(body) = body() else { return };
    let rom = Rom::from_body(&body).unwrap();
    let mut bad = Vec::new();
    for (_, a) in areas::authored() {
        let Some((group, maps)) = group_of(&a) else {
            continue; // towns: people, not enemies
        };
        let mut ids = vanilla_ids(&rom, group, maps);
        if a.name != areas::parapa_annex::NAME {
            ids.extend(catalog(&a).all());
        }
        for r in &a.rooms {
            for e in r.enemies.iter().filter(|e| !ids.contains(&e.id)) {
                bad.push(format!(
                    "{}/{}: enemy {:#04X} is not used in {}",
                    a.name, r.id, e.id, group.name
                ));
            }
        }
    }
    assert!(bad.is_empty(), "{}", bad.join("\n"));
}

/// The randomizer's catalog of IDs that work in a world's enemy group.
fn catalog(a: &Area) -> &'static EnemySets {
    match a.world {
        World::Overworld => match a.region {
            Region::West | Region::DeathMountain => &data::WEST,
            Region::East | Region::MazeIsland => &data::EAST,
        },
        World::PalaceA => &data::PALACE125,
        World::PalaceB => &data::PALACE346,
        World::GreatPalace => &data::GREAT_PALACE,
        World::WestTowns | World::EastTowns => unreachable!("towns have no enemy catalog"),
    }
}

fn group_of(a: &Area) -> Option<(&'static shuffle::Area, std::ops::Range<u16>)> {
    Some(match a.world {
        World::PalaceA if a.name == areas::parapa_annex::NAME => (&PALACES[0], 0..14),
        World::PalaceA => (&PALACES[0], 0..63),
        World::PalaceB => (&PALACES[1], 0..63),
        World::GreatPalace => (&PALACES[2], 0..63),
        World::Overworld => match a.region {
            Region::West => (&OVERWORLD[0], 0..63),
            Region::DeathMountain => (&OVERWORLD[1], 0..63),
            Region::East => (&OVERWORLD[2], 0..63),
            Region::MazeIsland => (&OVERWORLD[3], 0..63),
        },
        World::WestTowns | World::EastTowns => return None,
    })
}
