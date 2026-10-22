//! Challenge rooms: five standalone one- or two-room trials entered from
//! the overworld, each with one reward, inspired by ZALiA's challenge
//! rooms. Original layouts.
//!
//! | area | world | rooms | trial | needs | reward |
//! |---|---|---|---|---|---|
//! | Skyward Stair | Great Palace (5) | `stair_base` (2 pages), `stair_top` (1) | vertical climb: an elevator up, then two ledges four rows apart | `JUMP` (+ `RAFT` to reach East Hyrule) | magic container |
//! | Ember Run | overworld cave (0), West | `ember_run` (4) | stepping stones over a lava floor, three-column gaps, Aches overhead and Bago Bagos leaping from the lava | `JUMP` | 200 P-bag |
//! | Rubble Maze | palaces 1/2/5 (3) | `rubble_maze` (4) | two tiers of brick, walls that break with the glove, one way up and one way down | `GLOVE` (+ `RAFT`) | 500 P-bag |
//! | Lizalfos Pit | overworld cave (0), East | `lizalfos_pit` (2) | a closed arena: four Lizalfos, a Tektite and a Basilisk around the reward dais | `DOWNSTAB` (+ `RAFT`) | heart container |
//! | Crumbling Causeway | palaces 3/4/6 (4) | `causeway` (4), `sump` (3) | crumbling spans over two pits with three-column gaps; a fall lands in the sump, whose doors lead back out | `JUMP` | 500 P-bag |
//!
//! `DOWNSTAB` is a declared design requirement (the arena is meant to be
//! won by thrusting down onto shielded enemies); the movement check does
//! not model it. The others are checked on the room grids.
//!
//! # Attach points
//!
//! All five are entered from the overworld through location slots that are
//! all-zero in the original game (see README.md), on open
//! ground joined to a vanilla landmark ([`crate::areas::SLOTS`], checked
//! against the ROM by `tests/areas_rom.rs`). Palace-world rooms take slots
//! in the palace range (55-61), caves 33-39:
//!
//! | area | continent | slot | row, col | tile | arrives | near |
//! |---|---|---|---|---|---|---|
//! | Skyward Stair | East Hyrule | 57 | 36, 23 (grass) | palace | `stair_base` page 0 | Nabooru |
//! | Ember Run | West Hyrule | 33 | 14, 32 (grass) | cave | `ember_run` page 0 | North Palace |
//! | Rubble Maze | East Hyrule | 56 | 33, 29 (desert) | palace | `rubble_maze` page 0 | Nabooru |
//! | Lizalfos Pit | East Hyrule | 33 | 26, 12 (grass) | cave | `lizalfos_pit` page 0 | Nabooru |
//! | Crumbling Causeway | West Hyrule | 56 | 17, 42 (grass) | palace | `causeway` page 0 | Rauru |
//!
//! Every exit back out is `Outside`. `first_map` only numbers the rooms in
//! the tool-facing connection bytes (Skyward Stair 55-56, Ember Run 25,
//! Rubble Maze 50, Lizalfos Pit 15, Crumbling Causeway 60-61): the engine
//! runs every virtual room of a table in that table's one reserved slot.
//!
//! The two world-0 rooms (Ember Run, Lizalfos Pit) use the alternate (cave)
//! object tiles and build their ledges from `Object::Block` (two rows of
//! rock in a world-0 cave, the top one walkable): world 0 has no ledge
//! object, and a `Platform` there crashes the cave loader.
//!
//! Items: one "taken" bit per reward in the content engine's per-room item
//! state; nothing else is tracked.

use crate::model::{
    item, Abilities, Area, Dir, EnemySpawn, Entrance, Exit, Object, Region, Room, Target, TileKind,
    World,
};
use crate::StaticArea;

#[rustfmt::skip]
#[path = "challenge_rooms_generated.rs"]
mod generated;

/// Floor pattern used throughout: ceiling on row 0, floor from row 11.
const FLOOR: u8 = 0;
/// Standing row on [`FLOOR`].
const STAND: u8 = 10;
/// Row of an enemy standing on [`FLOOR`]: the list row above Link's
/// standing row (measured: a list entry on row 10 never spawns; one on row
/// 9 spawns and drops onto the floor).
const ENEMY_ROW: u8 = 9;

/// Area names.
pub mod names {
    /// Vertical climb.
    pub const SKYWARD_STAIR: &str = "Skyward Stair";
    /// Lava gauntlet.
    pub const EMBER_RUN: &str = "Ember Run";
    /// Block maze.
    pub const RUBBLE_MAZE: &str = "Rubble Maze";
    /// Arena.
    pub const LIZALFOS_PIT: &str = "Lizalfos Pit";
    /// Crumbling-bridge run.
    pub const CRUMBLING_CAUSEWAY: &str = "Crumbling Causeway";
}

fn e(x: u8, y: u8, id: u8) -> EnemySpawn {
    EnemySpawn { x, y, id }
}

fn entrance(region: Region, row: u8, col: u8, tile: TileKind, room: &'static str) -> Entrance {
    Entrance {
        region,
        row,
        col,
        tile,
        room,
        page: 0,
        // East Hyrule is across the sea.
        requires: if region == Region::East {
            Abilities::RAFT
        } else {
            Abilities::NONE
        },
    }
}

fn platform(x: u8, y: u8, len: u8) -> Object {
    Object::Platform { x, y, len }
}

fn block(x: u8, y: u8, len: u8) -> Object {
    Object::Block { x, y, len }
}

// ------------------------------------------------------------ Skyward Stair

/// "Skyward Stair": an elevator up from a hall to a shaft climbed on two
/// ledges, each four rows above the last (the jump spell's reach).
#[must_use]
pub fn skyward_stair() -> Area {
    /// Great Palace IDs.
    const MYU: u8 = 0x03;
    const BOT: u8 = 0x04;
    const SLOW_BUBBLE: u8 = 0x14;

    let mut base = Room::new("stair_base", 2, FLOOR);
    base.objects = vec![
        block(9, 9, 2),
        platform(15, 7, 3),
        block(20, 9, 1),
        // Same column within its page as the shaft's elevator.
        Object::Elevator { x: 28 },
    ];
    base.enemies = vec![e(13, ENEMY_ROW, MYU), e(24, ENEMY_ROW, BOT)];
    base.exits = vec![
        Exit::new(Dir::Left, Target::Outside),
        Exit::new(Dir::Up, Target::Room("stair_top")),
    ];

    let mut top = Room::new("stair_top", 1, FLOOR);
    top.objects = vec![
        platform(1, 3, 4),
        Object::Item {
            x: 2,
            y: 2,
            item: item::MAGIC_CONTAINER,
        },
        platform(6, 7, 3),
        Object::Elevator { x: 12 },
    ];
    top.item_requires = Abilities::JUMP;
    top.enemies = vec![e(8, 4, SLOW_BUBBLE), e(4, ENEMY_ROW, BOT)];
    top.exits = vec![Exit::new(Dir::Down, Target::Room("stair_base"))];

    Area {
        name: names::SKYWARD_STAIR,
        world: World::GreatPalace,
        region: Region::East,
        first_map: 55,
        rooms: vec![base, top],
        entrances: vec![entrance(
            Region::East,
            36,
            23,
            TileKind::Palace,
            "stair_base",
        )],
        links: vec![],
        terrain: vec![],
        npcs: vec![],
        needs: Abilities::JUMP.with(Abilities::RAFT),
    }
}

// ------------------------------------------------------------ Ember Run

/// "Ember Run": a cave whose floor is lava, crossed on stepping stones with
/// three-column gaps (the jump spell's reach), some raised, one floating.
#[must_use]
pub fn ember_run() -> Area {
    /// West Hyrule IDs.
    const ACHE: u8 = 0x07;
    const BAGO_BAGO: u8 = 0x0F;

    let mut r = Room::new("ember_run", 4, FLOOR);
    // The cave object tiles: `Object::Block` is rock there.
    r.alt_objects = true;
    r.objects = vec![
        // Each stretch of lava is drawn before the stones on it. World-0
        // lava fills rows 10-12 (measured), so a stone's top row is 10 (9
        // for the raised ones): one at row 11 would sit under the lava.
        Object::Lava { x: 6, width: 16 },
        block(9, 10, 2),
        block(14, 10, 2),
        block(19, 10, 2),
        Object::Lava { x: 22, width: 16 },
        block(24, 9, 2),
        block(29, 10, 2),
        // Floating, three rows above the lava.
        block(34, 8, 2),
        Object::Lava { x: 38, width: 16 },
        block(39, 10, 2),
        block(44, 9, 2),
        block(49, 10, 2),
        Object::Lava { x: 54, width: 4 },
        block(54, 10, 2),
        Object::Item {
            x: 61,
            y: STAND,
            item: item::BAG_200,
        },
    ];
    r.requires = Abilities::JUMP;
    r.enemies = vec![
        e(17, 11, BAGO_BAGO),
        e(20, 1, ACHE),
        e(37, 11, BAGO_BAGO),
        e(42, 1, ACHE),
        e(52, 11, BAGO_BAGO),
    ];
    r.exits = vec![
        Exit::new(Dir::Left, Target::Outside),
        Exit::new(Dir::Right, Target::Outside),
    ];

    Area {
        name: names::EMBER_RUN,
        world: World::Overworld,
        region: Region::West,
        first_map: 25,
        rooms: vec![r],
        entrances: vec![entrance(Region::West, 14, 32, TileKind::Cave, "ember_run")],
        links: vec![],
        terrain: vec![],
        npcs: vec![],
        needs: Abilities::JUMP,
    }
}

// ------------------------------------------------------------ Rubble Maze

/// "Rubble Maze": a brick shelf splits the room into two tiers. The upper
/// tier is walled at column 34 by bricks the glove breaks; a gap at 40-42
/// leads down and back up past a solid wall; the reward sits below the
/// shelf's open end. The lower tier's left part is a dead end behind more
/// breakable bricks.
#[must_use]
pub fn rubble_maze() -> Area {
    /// Palace 1/2/5 IDs.
    const BOT: u8 = 0x04;
    const BUBBLE: u8 = 0x06;
    const ROPE: u8 = 0x11;
    const IRON_KNUCKLE: u8 = 0x19;

    let mut r = Room::new("rubble_maze", 4, FLOOR);
    r.objects = vec![
        // Step up from the entry to the upper tier.
        platform(10, 9, 1),
        // The shelf (rows 7-8, columns 12-39 and 43-57): the upper tier
        // stands on row 6, the lower on row 10. Lower-tier walls:
        // breakable at 12, solid at 22, breakable at 32, solid at 43.
        block(12, 7, 16),
        Object::BreakableBlock {
            x: 12,
            y: 9,
            len: 1,
        },
        block(22, 9, 1),
        block(28, 7, 12),
        Object::BreakableBlock {
            x: 32,
            y: 9,
            len: 1,
        },
        // Upper tier: breakable wall at 34 (rows 1-6).
        Object::BreakableBlock {
            x: 34,
            y: 1,
            len: 1,
        },
        Object::BreakableBlock {
            x: 34,
            y: 3,
            len: 1,
        },
        Object::BreakableBlock {
            x: 34,
            y: 5,
            len: 1,
        },
        // Step back up at the gap, then a solid wall under the shelf.
        platform(41, 9, 1),
        block(43, 7, 15),
        block(43, 9, 1),
        // Below the shelf's open end: the reward and a step back up.
        platform(59, 9, 1),
        Object::Item {
            x: 61,
            y: STAND,
            item: item::BAG_500,
        },
    ];
    r.requires = Abilities::GLOVE;
    r.enemies = vec![
        e(5, ENEMY_ROW, BOT),
        e(17, ENEMY_ROW, ROPE),
        e(26, 6, BOT),
        e(38, 6, IRON_KNUCKLE),
        e(50, ENEMY_ROW, BOT),
        e(52, 6, BOT),
        e(60, 3, BUBBLE),
    ];
    r.exits = vec![Exit::new(Dir::Left, Target::Outside)];

    Area {
        name: names::RUBBLE_MAZE,
        world: World::PalaceA,
        region: Region::East,
        first_map: 50,
        rooms: vec![r],
        entrances: vec![entrance(
            Region::East,
            33,
            29,
            TileKind::Palace,
            "rubble_maze",
        )],
        links: vec![],
        terrain: vec![],
        npcs: vec![],
        needs: Abilities::GLOVE.with(Abilities::RAFT),
    }
}

// ------------------------------------------------------------ Lizalfos Pit

/// "Lizalfos Pit": a closed two-page arena. The reward sits on a dais in
/// the middle of the pack; low ledges along the walls give room to come
/// down on shields from above.
#[must_use]
pub fn lizalfos_pit() -> Area {
    /// East Hyrule IDs.
    const TEKTITE: u8 = 0x14;
    const BASILISK: u8 = 0x18;
    const LIZALFOS_A: u8 = 0x1A;
    const LIZALFOS_B: u8 = 0x1B;

    let mut r = Room::new("lizalfos_pit", 2, FLOOR);
    // The cave object tiles: `Object::Block` is rock there.
    r.alt_objects = true;
    r.objects = vec![
        block(4, 7, 3),
        block(13, 9, 4),
        Object::Item {
            x: 14,
            y: 8,
            item: item::HEART_CONTAINER,
        },
        block(22, 7, 3),
        block(27, 5, 3),
    ];
    r.item_requires = Abilities::DOWNSTAB;
    r.enemies = vec![
        e(9, ENEMY_ROW, LIZALFOS_B),
        e(11, ENEMY_ROW, LIZALFOS_A),
        e(19, ENEMY_ROW, LIZALFOS_B),
        e(21, ENEMY_ROW, LIZALFOS_A),
        e(25, ENEMY_ROW, TEKTITE),
        e(29, ENEMY_ROW, BASILISK),
    ];
    r.exits = vec![Exit::new(Dir::Left, Target::Outside)];

    Area {
        name: names::LIZALFOS_PIT,
        world: World::Overworld,
        region: Region::East,
        first_map: 15,
        rooms: vec![r],
        entrances: vec![entrance(
            Region::East,
            26,
            12,
            TileKind::Cave,
            "lizalfos_pit",
        )],
        links: vec![],
        terrain: vec![],
        npcs: vec![],
        needs: Abilities::DOWNSTAB.with(Abilities::RAFT),
    }
}

// ------------------------------------------------------------ Crumbling Causeway

/// "Crumbling Causeway": two pits crossed on crumbling spans with
/// three-column gaps (the jump spell's reach). A fall lands in the sump,
/// whose two doors lead back out.
#[must_use]
pub fn crumbling_causeway() -> Area {
    /// Palace 3/4/6 IDs.
    const MYU: u8 = 0x03;
    const MOA: u8 = 0x07;
    const FAST_BUBBLE: u8 = 0x0E;

    let mut c = Room::new("causeway", 4, FLOOR);
    c.objects = vec![
        Object::Pit { x: 18, width: 12 },
        Object::Crumble {
            x: 18,
            y: 11,
            len: 3,
        },
        Object::Crumble {
            x: 24,
            y: 11,
            len: 3,
        },
        Object::Pit { x: 34, width: 12 },
        Object::Crumble {
            x: 37,
            y: 11,
            len: 3,
        },
        Object::Crumble {
            x: 43,
            y: 11,
            len: 3,
        },
        block(56, 9, 2),
        Object::Item {
            x: 57,
            y: 8,
            item: item::BAG_500,
        },
    ];
    c.requires = Abilities::JUMP;
    c.enemies = vec![
        e(10, ENEMY_ROW, MYU),
        e(24, 3, MOA),
        e(41, 4, FAST_BUBBLE),
        e(51, ENEMY_ROW, MYU),
    ];
    c.exits = vec![
        Exit::new(Dir::Left, Target::Outside),
        Exit::new(Dir::Right, Target::Outside),
        Exit::new(Dir::Drop { page: 1 }, Target::Room("sump")),
        Exit::new(Dir::Drop { page: 2 }, Target::Room("sump")),
    ];

    // Three pages: a fall keeps its page (1 or 2 here).
    let mut s = Room::new("sump", 3, FLOOR);
    s.objects = vec![
        block(3, 9, 1),
        block(12, 9, 1),
        block(30, 9, 1),
        block(44, 9, 1),
    ];
    s.enemies = vec![e(7, ENEMY_ROW, MYU), e(38, ENEMY_ROW, MYU)];
    s.exits = vec![
        Exit::new(Dir::Left, Target::Outside),
        Exit::new(Dir::Right, Target::Outside),
    ];

    Area {
        name: names::CRUMBLING_CAUSEWAY,
        world: World::PalaceB,
        region: Region::West,
        first_map: 60,
        rooms: vec![c, s],
        entrances: vec![entrance(Region::West, 17, 42, TileKind::Palace, "causeway")],
        links: vec![],
        terrain: vec![],
        npcs: vec![],
        needs: Abilities::JUMP,
    }
}

/// Every area of this module.
#[must_use]
pub fn areas() -> Vec<Area> {
    vec![
        skyward_stair(),
        ember_run(),
        rubble_maze(),
        lizalfos_pit(),
        crumbling_causeway(),
    ]
}

/// The group's areas in static form (what the engine registers).
#[must_use]
pub fn statics() -> &'static [StaticArea] {
    generated::AREAS
}
