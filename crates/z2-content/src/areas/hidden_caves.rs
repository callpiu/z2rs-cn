//! `hidden_caves`: six small secret caves on the overworld, in the style
//! of the game's own world-0 caves, each holding a container piece, a
//! P-bag or a 1-up doll. One of them, the Old Barrow, is a three-room set
//! inspired by ZALiA's "Zelda 1 area" idea: a sunken pocket on the
//! cemetery side of the Death Mountain range that needs the hammer to open
//! and the candle to explore.
//!
//! All six layouts are original (no ZALiA maps, no rooms of the game).
//!
//! # Caves
//!
//! | area | region | rooms | reward | needs |
//! |---|---|---|---|---|
//! | Lakeside Hollow | West Hyrule | 1 (2 pages) | heart piece | - |
//! | Sandcrawl Burrow | East Hyrule | 2 (fall through a pit into the den) | magic piece | - |
//! | Reed Grotto | East Hyrule | 1 (1 page) | 1-up doll | - |
//! | Ember Niche | Death Mountain | 1 (2 pages, lava with stepping ledges) | heart piece | - |
//! | Tidepool Cave | Maze Island | 1 (1 page, a pool with stepping stones) | magic piece | - |
//! | Old Barrow | West Hyrule (cemetery side of Death Mountain) | 3: gate, hall, vault (fall from the hall) | 200 P-bag, 2 magic pieces | hammer (boulder on the mouth), candle (dark) |
//!
//! # Attach points
//!
//! Location slots are handed out in [`crate::areas::SLOTS`] (following
//! README.md, "Overworld location slots"). The ROM-gated
//! `tests/areas_rom.rs` checks every claim against the player's ROM: the
//! slot is unused (all four bytes zero on West and East; on Death Mountain
//! and Maze Island no vanilla use in either bank's copy, so all four bytes
//! are overwritten), the tile is plain passable ground with no vanilla
//! location on it.
//!
//! | area | region | location slot | tile (row, col) | terrain |
//! |---|---|---|---|---|
//! | Lakeside Hollow | West | 34 | (26, 24) | grass by the lake south of the start |
//! | Old Barrow | West | 35 | (54, 46) | graveyard edge under the mountain spur |
//! | Sandcrawl Burrow | East | 34 | (31, 30) | desert south of Nabooru |
//! | Reed Grotto | East | 35 | (55, 28) | swamp in the south-west |
//! | Ember Niche | Death Mountain | 53 (bank 1 copy) | (38, 3) | the small graveyard in the south of Death Mountain |
//! | Tidepool Cave | Maze Island | 54 (bank 2 copy) | (26, 47) | a dead-end road in the maze |
//!
//! Notes on the picks:
//!
//! * `first_map` only numbers the rooms in the tool-facing connection
//!   bytes (the engine runs every virtual room of a table in that table's
//!   reserved slot); the numbers are still the authors' picks from each
//!   table's free list.
//! * Slots 53 and 54 of the shared Death Mountain / Maze Island layout
//!   hold `80 00 00 00` in both banks (off the map) and have no vanilla
//!   use on either continent.
//! * World 0 has no ledge object (a `Platform` crashes the cave loader),
//!   so every ledge here is an `Object::Block`: in a world-0 room with the
//!   alternate (cave) object tiles, two rows of rock whose top row Link
//!   stands on (measured; without the alternate tiles the same object is
//!   a background tile Link falls through).
//! * The entrance tiles look plain ([`TileKind::Hidden`], found by walking
//!   onto them) except the Old Barrow's cave mouth, which the engine
//!   covers with a boulder the hammer breaks.
//!
//! # Pieces
//!
//! Four pieces make one container. [`HEART_PIECE`] and [`MAGIC_PIECE`] are
//! placeholder item bytes that no vanilla item uses; the engine has to
//! define them (count the piece on pickup, and on every fourth piece give
//! a container). Across the content group:
//!
//! | piece | where |
//! |---|---|
//! | heart 1 | Lakeside Hollow |
//! | heart 2 | Ember Niche |
//! | heart 3 | target gallery (`minigames`) |
//! | heart 4 | pogo tower (`minigames`) |
//! | magic 1 | Sandcrawl Burrow, den |
//! | magic 2 | Tidepool Cave |
//! | magic 3 | Old Barrow, hall |
//! | magic 4 | Old Barrow, vault |
//!
//! So the group holds exactly one extra heart container and one extra
//! magic container, in pieces, plus a 200 P-bag and a 1-up doll.
//!
//! # Engine features these caves need
//!
//! * Entering a virtual room straight from the overworld (a new location
//!   slot whose target is a virtual room), and leaving it back to that
//!   tile.
//! * The two piece item bytes and their pickup.
//! * The Old Barrow's boulder on the cave mouth (hammer) and darkness
//!   without the candle; the model only records them as requirements
//!   ([`Entrance::requires`], [`Room::requires`]), the movement check
//!   does not see them.
//! * Falls (the Sandcrawl Burrow's pit and the Old Barrow hall's shaft)
//!   from one virtual room into another.

use crate::model::{
    item, Abilities, Area, Dir, EnemySpawn, Entrance, Exit, Object, Region, Room, Target, TileKind,
    World,
};
use crate::StaticArea;

#[rustfmt::skip]
#[path = "hidden_caves_generated.rs"]
mod generated;

/// Placeholder item byte for a heart container piece (four make a
/// container). Not a vanilla item; the engine must define it.
pub const HEART_PIECE: u8 = 0x13;
/// Placeholder item byte for a magic container piece (four make a
/// container). Not a vanilla item; the engine must define it.
pub const MAGIC_PIECE: u8 = 0x14;
/// Pieces per container.
pub const PIECES_PER_CONTAINER: u8 = 4;

/// Overworld enemy IDs used here. West Hyrule and Death Mountain share one
/// enemy group, East Hyrule and Maze Island the other; the small enemies
/// below have the same ID in both.
mod ids {
    /// Myu (both groups).
    pub const MYU: u8 = 0x03;
    /// Bot (both groups).
    pub const BOT: u8 = 0x04;
    /// Bit (both groups).
    pub const BIT: u8 = 0x05;
    /// Lowder (West / Death Mountain group).
    pub const LOWDER: u8 = 0x1C;
    /// Megmet (West / Death Mountain group).
    pub const MEGMET: u8 = 0x1F;
    /// Leever (East / Maze Island group).
    pub const LEEVER: u8 = 0x16;
}

/// Palette selection for the cave rooms (the cave palette of world 0).
const CAVE_PALETTE: u8 = 9;

/// A cave room: the cave palette and the alternate (cave) object tiles,
/// whose kind-2 object is the rock block `Object::Block` draws.
fn cave(id: &'static str, pages: u8) -> Room {
    let mut r = Room::new(id, pages, 1);
    r.palette = CAVE_PALETTE;
    r.alt_objects = true;
    r
}

/// A cave room with the plain world-0 object tiles: the only ones whose
/// pit command is a hole (with the cave tiles it is a solid black wall,
/// measured), but whose blocks are background Link falls through, so its
/// ledges are floor steps ([`step`]).
fn pit_cave(id: &'static str, pages: u8) -> Room {
    let mut r = Room::new(id, pages, 1);
    r.palette = CAVE_PALETTE;
    r
}

/// From column `x` on, floor pattern `pattern` (0-7: Link stands on row
/// `10 - pattern`), ceiling kept.
const fn step(x: u8, pattern: u8) -> Object {
    Object::Floor {
        x,
        pattern,
        no_ceiling: false,
    }
}

fn spawn(x: u8, y: u8, id: u8) -> EnemySpawn {
    EnemySpawn { x, y, id }
}

fn both_ways_out(r: &mut Room) {
    r.exits = vec![
        Exit::new(Dir::Left, Target::Outside),
        Exit::new(Dir::Right, Target::Outside),
    ];
}

fn hidden(region: Region, row: u8, col: u8, room: &'static str) -> Entrance {
    Entrance {
        region,
        row,
        col,
        tile: TileKind::Hidden,
        room,
        page: 0,
        requires: Abilities::NONE,
    }
}

/// Lakeside Hollow (West Hyrule): a two-page hollow whose ledges climb to
/// a heart piece. Floor pattern 1 throughout (Link stands on row 9).
#[must_use]
pub fn lakeside_hollow() -> Area {
    let mut r = cave("hollow", 2);
    r.objects = vec![
        Object::Block { x: 9, y: 8, len: 3 },
        Object::Block {
            x: 13,
            y: 6,
            len: 2,
        },
        Object::Block {
            x: 16,
            y: 4,
            len: 4,
        },
        Object::Item {
            x: 18,
            y: 3,
            item: HEART_PIECE,
        },
        Object::Block {
            x: 25,
            y: 8,
            len: 2,
        },
    ];
    r.enemies = vec![
        spawn(5, 9, ids::BOT),
        spawn(22, 9, ids::MEGMET),
        spawn(29, 9, ids::BIT),
    ];
    both_ways_out(&mut r);
    Area {
        name: "Lakeside Hollow",
        world: World::Overworld,
        region: Region::West,
        first_map: 32,
        rooms: vec![r],
        entrances: vec![hidden(Region::West, 26, 24, "hollow")],
        links: vec![],
        terrain: vec![],
        npcs: vec![],
        needs: Abilities::NONE,
    }
}

/// Sandcrawl Burrow (East Hyrule): a three-page crawl with a two-column
/// pit; jump it to walk on through, or drop down it into the den, where a
/// stair of blocks leads up to a magic piece.
///
/// The crawl uses the plain world-0 object tiles (`pit_cave`), the only
/// ones whose pit is a hole, so its humps are floor steps, not blocks.
#[must_use]
pub fn sandcrawl_burrow() -> Area {
    let mut crawl = pit_cave("crawl", 3);
    crawl.objects = vec![
        // A hump to hop (top row 8).
        step(9, 3),
        step(11, 1),
        // Two one-column pit commands (a world-0 pit command clears one
        // column, see `Object::Pit`): Link walks over a one-column hole
        // without falling in, so the den needs two.
        Object::Pit { x: 22, width: 1 },
        Object::Pit { x: 23, width: 1 },
        step(30, 3),
        step(33, 1),
        // A two-step rise (top rows 8 and 6) and back down.
        step(37, 3),
        step(38, 5),
        step(41, 3),
        step(42, 1),
    ];
    crawl.enemies = vec![
        spawn(14, 9, ids::LEEVER),
        spawn(27, 9, ids::MYU),
        spawn(43, 9, ids::LEEVER),
    ];
    crawl.exits = vec![
        Exit::new(Dir::Left, Target::Outside),
        // Past the hole: the movement check's plain jump clears one
        // column, so it only proves the far side with the jump spell (in
        // play a running jump clears the two-column hole). The den is the
        // other way on.
        Exit::new(Dir::Right, Target::Outside).needs(Abilities::JUMP),
        Exit::new(Dir::Drop { page: 1 }, Target::Room("den")),
    ];

    let mut den = cave("den", 2);
    den.objects = vec![
        Object::Block {
            x: 14,
            y: 8,
            len: 2,
        },
        Object::Block {
            x: 17,
            y: 6,
            len: 2,
        },
        Object::Block {
            x: 20,
            y: 4,
            len: 3,
        },
        Object::Item {
            x: 21,
            y: 3,
            item: MAGIC_PIECE,
        },
    ];
    den.enemies = vec![spawn(4, 9, ids::MYU), spawn(26, 9, ids::BOT)];
    both_ways_out(&mut den);

    Area {
        name: "Sandcrawl Burrow",
        world: World::Overworld,
        region: Region::East,
        first_map: 17,
        rooms: vec![crawl, den],
        entrances: vec![hidden(Region::East, 31, 30, "crawl")],
        links: vec![],
        terrain: vec![],
        npcs: vec![],
        needs: Abilities::NONE,
    }
}

/// Reed Grotto (East Hyrule): one page; a two-block pillar with a 1-up
/// doll on top and a step on either side of it.
#[must_use]
pub fn reed_grotto() -> Area {
    let mut r = cave("grotto", 1);
    r.objects = vec![
        Object::Block { x: 5, y: 8, len: 1 },
        Object::Block { x: 7, y: 6, len: 2 },
        Object::Block { x: 7, y: 8, len: 2 },
        Object::Block {
            x: 10,
            y: 8,
            len: 1,
        },
        Object::Item {
            x: 8,
            y: 5,
            item: item::ONE_UP,
        },
    ];
    // Nothing within reach of the left edge: Link walks in there, and an
    // enemy at column 2 hits him on arrival and knocks him straight back
    // out of the one-page room (measured).
    r.enemies = vec![spawn(12, 9, ids::MYU), spawn(14, 9, ids::BIT)];
    both_ways_out(&mut r);
    Area {
        name: "Reed Grotto",
        world: World::Overworld,
        region: Region::East,
        first_map: 21,
        rooms: vec![r],
        entrances: vec![hidden(Region::East, 55, 28, "grotto")],
        links: vec![],
        terrain: vec![],
        npcs: vec![],
        needs: Abilities::NONE,
    }
}

/// Ember Niche (Death Mountain): a lava channel crossed on five short
/// ledges (one has a Megmet on it), then an alcove with a heart piece.
#[must_use]
pub fn ember_niche() -> Area {
    let mut r = cave("niche", 2);
    let mut objects = vec![
        // Columns 7-22: floor only on the bottom two rows, which the lava
        // replaces.
        Object::Floor {
            x: 7,
            pattern: 0,
            no_ceiling: false,
        },
        Object::Lava { x: 7, width: 16 },
    ];
    for x in [8, 11, 14, 17, 20] {
        objects.push(Object::Block { x, y: 9, len: 2 });
    }
    objects.extend([
        Object::Floor {
            x: 23,
            pattern: 1,
            no_ceiling: false,
        },
        Object::Block {
            x: 27,
            y: 8,
            len: 3,
        },
        Object::Item {
            x: 28,
            y: 7,
            item: HEART_PIECE,
        },
    ]);
    r.objects = objects;
    r.enemies = vec![
        spawn(14, 8, ids::MEGMET),
        spawn(25, 9, ids::LOWDER),
        spawn(31, 9, ids::BIT),
    ];
    both_ways_out(&mut r);
    Area {
        name: "Ember Niche",
        world: World::Overworld,
        region: Region::DeathMountain,
        first_map: 38,
        rooms: vec![r],
        entrances: vec![hidden(Region::DeathMountain, 38, 3, "niche")],
        links: vec![],
        terrain: vec![],
        npcs: vec![],
        needs: Abilities::NONE,
    }
}

/// Tidepool Cave (Maze Island): one page; a pool crossed on three
/// stepping stones, then a rock with a magic piece on it.
#[must_use]
pub fn tidepool_cave() -> Area {
    let mut r = cave("pool", 1);
    r.objects = vec![
        Object::Floor {
            x: 4,
            pattern: 0,
            no_ceiling: false,
        },
        Object::Lava { x: 4, width: 7 },
        Object::Block { x: 5, y: 9, len: 1 },
        Object::Block { x: 7, y: 9, len: 1 },
        Object::Block { x: 9, y: 9, len: 1 },
        Object::Floor {
            x: 11,
            pattern: 1,
            no_ceiling: false,
        },
        Object::Block {
            x: 12,
            y: 8,
            len: 1,
        },
        Object::Item {
            x: 12,
            y: 7,
            item: MAGIC_PIECE,
        },
    ];
    // Both past the pool: an enemy at column 2 hits Link as he walks in
    // and knocks him back out of the one-page room (measured).
    r.enemies = vec![spawn(13, 9, ids::BOT), spawn(14, 9, ids::MYU)];
    both_ways_out(&mut r);
    Area {
        name: "Tidepool Cave",
        world: World::Overworld,
        region: Region::MazeIsland,
        first_map: 31,
        rooms: vec![r],
        entrances: vec![hidden(Region::MazeIsland, 26, 47, "pool")],
        links: vec![],
        terrain: vec![],
        npcs: vec![],
        needs: Abilities::NONE,
    }
}

/// The Old Barrow (West Hyrule, under the southern spurs of the Death
/// Mountain range by the graveyard): a sunken crypt in three dark rooms.
/// The mouth is behind a boulder (hammer); every room is dark (candle).
///
/// * `gate` (2 pages): in from the overworld on the left; a long tomb
///   block with a 200 P-bag on it; on to the hall on the right.
/// * `hall` (4 pages): a stair of floor steps up to a shelf with a magic piece;
///   a two-column shaft at the far end drops into the vault; a solid wall
///   closes the right end.
/// * `vault` (3 pages): Link lands in it from the shaft, on page 2 (a
///   fall keeps its page); a dais with the second magic piece; out to the
///   overworld on either side.
#[must_use]
pub fn old_barrow() -> Area {
    let mut gate = cave("gate", 2);
    gate.requires = Abilities::CANDLE;
    gate.objects = vec![
        Object::Block {
            x: 12,
            y: 8,
            len: 4,
        },
        Object::Item {
            x: 14,
            y: 7,
            item: item::BAG_200,
        },
        Object::Block {
            x: 22,
            y: 8,
            len: 1,
        },
        Object::Block {
            x: 26,
            y: 8,
            len: 1,
        },
    ];
    gate.enemies = vec![
        spawn(8, 9, ids::MYU),
        spawn(18, 9, ids::BOT),
        spawn(29, 9, ids::MYU),
    ];
    gate.exits = vec![
        Exit::new(Dir::Left, Target::Outside),
        Exit::new(Dir::Right, Target::Room("hall")),
    ];

    // The plain object tiles (`pit_cave`) for the shaft: the stair is
    // floor steps (top rows 8, 6 and 4).
    let mut hall = pit_cave("hall", 4);
    hall.requires = Abilities::CANDLE;
    hall.objects = vec![
        step(20, 3),
        step(23, 5),
        step(26, 7),
        step(29, 1),
        Object::Item {
            x: 27,
            y: 3,
            item: MAGIC_PIECE,
        },
        Object::Pit { x: 44, width: 1 },
        Object::Pit { x: 45, width: 1 },
        // Solid from column 46 to the end of the room.
        Object::Floor {
            x: 46,
            pattern: 15,
            no_ceiling: false,
        },
    ];
    hall.enemies = vec![
        spawn(10, 9, ids::LOWDER),
        spawn(33, 9, ids::MEGMET),
        spawn(38, 9, ids::BOT),
        spawn(41, 9, ids::MYU),
    ];
    hall.exits = vec![
        Exit::new(Dir::Left, Target::Room("gate")),
        Exit::new(Dir::Drop { page: 2 }, Target::Room("vault")),
    ];

    // Three pages: a fall keeps the page Link fell on (the hall's page 2),
    // so the dais is on page 2 and a passage leads back west.
    let mut vault = cave("vault", 3);
    vault.requires = Abilities::CANDLE;
    vault.objects = vec![
        Object::Block {
            x: 18,
            y: 8,
            len: 1,
        },
        Object::Block {
            x: 43,
            y: 8,
            len: 2,
        },
        Object::Item {
            x: 44,
            y: 7,
            item: MAGIC_PIECE,
        },
    ];
    vault.enemies = vec![spawn(24, 9, ids::BOT), spawn(36, 9, ids::MYU)];
    both_ways_out(&mut vault);

    Area {
        name: "Old Barrow",
        world: World::Overworld,
        region: Region::West,
        first_map: 25,
        rooms: vec![gate, hall, vault],
        entrances: vec![Entrance {
            region: Region::West,
            row: 54,
            col: 46,
            tile: TileKind::Cave,
            room: "gate",
            page: 0,
            requires: Abilities::HAMMER,
        }],
        links: vec![],
        terrain: vec![],
        npcs: vec![],
        needs: Abilities::HAMMER.with(Abilities::CANDLE),
    }
}

/// Every hidden cave.
#[must_use]
pub fn areas() -> Vec<Area> {
    vec![
        lakeside_hollow(),
        sandcrawl_burrow(),
        reed_grotto(),
        ember_niche(),
        tidepool_cave(),
        old_barrow(),
    ]
}

/// The group's areas in static form (what the engine registers).
#[must_use]
pub fn statics() -> &'static [StaticArea] {
    generated::AREAS
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{compile, validate};

    fn issues(a: &Area) -> String {
        match validate(a) {
            Ok(()) => String::new(),
            Err(v) => v
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join("\n"),
        }
    }

    #[test]
    fn every_cave_validates() {
        for a in areas() {
            assert_eq!(issues(&a), "", "{}", a.name);
            assert!(a.rooms.len() <= 3, "{}", a.name);
            assert_eq!(a.world, World::Overworld);
        }
        assert!((5..=7).contains(&areas().len()));
    }

    #[test]
    fn rewards_add_up_to_whole_containers() {
        let mut heart = 0;
        let mut magic = 0;
        for a in areas()
            .iter()
            .chain(super::super::minigames::areas().iter())
        {
            for r in &a.rooms {
                for o in &r.objects {
                    if let Object::Item { item, .. } = o {
                        heart += u8::from(*item == HEART_PIECE);
                        magic += u8::from(*item == MAGIC_PIECE);
                    }
                }
            }
        }
        assert_eq!(heart, PIECES_PER_CONTAINER);
        assert_eq!(magic, PIECES_PER_CONTAINER);
    }

    #[test]
    fn the_barrow_needs_the_hammer_and_the_candle() {
        let mut a = old_barrow();
        assert_eq!(issues(&a), "");
        a.needs = Abilities::CANDLE;
        assert!(issues(&a).contains("not reachable"), "{}", issues(&a));
        a.needs = Abilities::HAMMER;
        let s = issues(&a);
        assert!(s.contains("item not reachable"), "{s}");
        assert!(s.contains("no way back out"), "{s}");
        let reach = validate::reachable(&old_barrow(), Abilities::HAMMER);
        assert_eq!(reach.items, vec![false; 3]);
    }

    #[test]
    fn the_burrow_and_the_barrow_fall_into_their_lower_rooms() {
        let c = compile(&sandcrawl_burrow()).unwrap();
        // crawl: left out, drop on page 1 to den page 1 (a fall keeps its
        // page), right (byte 2) out.
        assert_eq!(c.rooms[0].connections, [0xFC, (18 << 2) | 1, 0xFC, 0xFC]);
        let c = compile(&old_barrow()).unwrap();
        // gate: right (byte 1) into the hall's page 0.
        assert_eq!(c.rooms[0].connections, [0xFC, 26 << 2, 0xFC, 0xFC]);
        // hall: left into the gate's last page; fall on page 2 to the
        // vault's page 2.
        assert_eq!(
            c.rooms[1].connections,
            [(25 << 2) | 1, 0xFC, (27 << 2) | 2, 0xFC]
        );
        assert_eq!(c.rooms[2].item, Some(MAGIC_PIECE));
    }
}
