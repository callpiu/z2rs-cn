//! `test_room`: developer rooms, one per kind of table, that the engine's
//! ROM-gated tests walk through (`z2-core/tests/enh_content_rom.rs`). All
//! original and deliberately plain.
//!
//! * `Test Room` (Great Palace): four pages of floor, four brick ledges, a
//!   50-XP P-bag on page 1 and a Bot on page 2, between Great Palace maps 1
//!   and 2 (map 1's right exit is linked to it).
//! * `Dev Cave` (West Hyrule, world 0): two pages, a two-row block (large
//!   object kind 1, the ledge, crashes the world-0 loader: caves have no
//!   such object), a P-bag on page 1. Two cave mouths on the road east of
//!   the North Palace ([`DEV_CAVE_ROW`], [`DEV_CAVE_COLS`]): the left one
//!   enters on page 0 and the left exit leads back out to it, the right
//!   one enters on page 1 and the right exit (`OutsideAt(1)`) leads out to
//!   it. Its terrain turns two moat cells south of the palace into desert
//!   and the two below them into shallow water.
//! * `Dev Palace` (palace group A, the map-63 slot): `hall` and `end`, two
//!   pages each, between palace 1's maps 1 and 2 (map 1's right exit is
//!   linked to `hall`); a P-bag on `end`'s page 1.
//! * `Dev Inn` (towns): two pages entered through the doorway on page 1 of
//!   town map 1, whose own doorway on page 0 leads back there. Five town
//!   people ([`NPC_SCRIPTS`]): a talker with three messages, a life and a
//!   magic healer, a hint giver gated on flag 0, and a sage in `back` (the
//!   room behind the doorway on page 1, left exit back to the inn) who sets
//!   flag 0; also entered from a town tile on the overworld
//!   ([`DEV_INN_TILE`]; a world load, like any town).
//!
//! Regenerate `dev_rooms_generated.rs` with
//! `Z2_CONTENT_BLESS=1 cargo test -p z2-content areas`.

use crate::model::{
    item, Abilities, Area, Dir, EnemySpawn, Entrance, Exit, MapTile, Message, Object, Region, Room,
    Target, TerrainEdit, TileKind, TownNpc, VanillaLink, World,
};
use crate::npc::{Gift, NpcRole, NpcScript};
use crate::StaticArea;

#[rustfmt::skip]
#[path = "dev_rooms_generated.rs"]
mod generated;

/// Great Palace map the test room is linked from (its right exit).
pub const TEST_ROOM_FROM_MAP: u8 = 1;
/// Palace-group-A map `Dev Palace` is linked from (its right exit).
pub const DEV_PALACE_FROM_MAP: u8 = 1;
/// Town map whose page-1 doorway leads to `Dev Inn`.
pub const DEV_INN_FROM_MAP: u8 = 1;
/// West Hyrule row of `Dev Cave`'s two mouths: the road east of the North
/// Palace (row 22, column 23).
pub const DEV_CAVE_ROW: u8 = 22;
/// Columns of `Dev Cave`'s left mouth (page 0; its left exit comes back
/// out here) and right mouth (page 1; its right exit, `OutsideAt(1)`).
pub const DEV_CAVE_COLS: [u8; 2] = [27, 31];
/// West Hyrule (row, column) of `Dev Inn`'s town tile, on the grass
/// north-east of the cave's right mouth.
pub const DEV_INN_TILE: (u8, u8) = (21, 33);

fn ledge(x: u8, y: u8, len: u8) -> Object {
    Object::Platform { x, y, len }
}

fn test_room() -> Area {
    let mut r = Room::new("test_room", 4, 1);
    r.objects = vec![
        ledge(6, 7, 4),
        ledge(20, 5, 4),
        Object::Item {
            x: 24,
            y: 7,
            item: item::BAG_50,
        },
        ledge(36, 7, 6),
        ledge(50, 6, 3),
    ];
    r.enemies = vec![EnemySpawn { x: 40, y: 9, id: 4 }];
    r.exits = vec![
        Exit::new(
            Dir::Left,
            Target::Map {
                map: TEST_ROOM_FROM_MAP,
                page: 3,
            },
        ),
        Exit::new(Dir::Right, Target::Map { map: 2, page: 0 }),
    ];
    Area {
        name: "Test Room",
        world: World::GreatPalace,
        region: Region::West,
        first_map: 60,
        rooms: vec![r],
        entrances: vec![],
        links: vec![VanillaLink::byte(TEST_ROOM_FROM_MAP, 3, "test_room", 0)],
        terrain: vec![],
        npcs: vec![],
        needs: Abilities::NONE,
    }
}

fn dev_cave() -> Area {
    let mut r = Room::new("cave", 2, 1);
    // The cave object tiles: `Object::Block` is rock there.
    r.alt_objects = true;
    r.objects = vec![
        Object::Block {
            x: 10,
            y: 6,
            len: 3,
        },
        Object::Item {
            x: 20,
            y: 8,
            item: item::BAG_50,
        },
    ];
    r.exits = vec![
        Exit::new(Dir::Left, Target::Outside),
        Exit::new(Dir::Right, Target::OutsideAt(1)),
    ];
    let mouth = |col: u8, page: u8| Entrance {
        region: Region::West,
        row: DEV_CAVE_ROW,
        col,
        tile: TileKind::Cave,
        room: "cave",
        page,
        requires: Abilities::NONE,
    };
    let mut terrain = TerrainEdit::rect(Region::West, 24..=24, 20..=21, MapTile::Desert);
    terrain.extend(TerrainEdit::rect(
        Region::West,
        25..=25,
        20..=21,
        MapTile::ShallowWater,
    ));
    Area {
        name: "Dev Cave",
        world: World::Overworld,
        region: Region::West,
        first_map: 28,
        rooms: vec![r],
        entrances: vec![mouth(DEV_CAVE_COLS[0], 0), mouth(DEV_CAVE_COLS[1], 1)],
        links: vec![],
        terrain,
        npcs: vec![],
        needs: Abilities::NONE,
    }
}

fn dev_palace() -> Area {
    let mut hall = Room::new("hall", 2, 1);
    hall.objects = vec![ledge(12, 7, 4)];
    hall.exits = vec![
        Exit::new(
            Dir::Left,
            Target::Map {
                map: DEV_PALACE_FROM_MAP,
                page: 3,
            },
        ),
        Exit::new(Dir::Right, Target::Room("end")),
    ];
    let mut end = Room::new("end", 2, 1);
    end.objects = vec![
        ledge(4, 7, 3),
        Object::Item {
            x: 20,
            y: 8,
            item: item::BAG_50,
        },
    ];
    end.exits = vec![
        Exit::new(Dir::Left, Target::Room("hall")),
        Exit::new(Dir::Right, Target::Map { map: 2, page: 0 }),
    ];
    Area {
        name: "Dev Palace",
        world: World::PalaceA,
        region: Region::West,
        first_map: 61,
        rooms: vec![hall, end],
        entrances: vec![],
        links: vec![VanillaLink::byte(DEV_PALACE_FROM_MAP, 3, "hall", 0)],
        terrain: vec![],
        npcs: vec![],
        needs: Abilities::NONE,
    }
}

/// Town people of `Dev Inn` (index in `Area::npcs`, see
/// [`NPC_SCRIPTS`]).
pub mod inn_npcs {
    /// Talks; three messages over three talks.
    pub const TALKER: u8 = 0;
    /// Refills life when the box closes.
    pub const HEALER: u8 = 1;
    /// Refills magic when the box closes.
    pub const MAGE: u8 = 2;
    /// Gives a hint once the sage's flag is set.
    pub const HINT: u8 = 3;
    /// The sage in the back room: sets flag 0.
    pub const SAGE: u8 = 4;
}

/// Plain talking townsperson id (world 1/2; default dialog conditions).
pub const INN_TALKER_ID: u8 = 0x22;
/// Townsperson id that stands still (the original "immobile" kinds
/// `$10-$12`); the healers are `$10` and `$11`.
pub const INN_STANDING_ID: u8 = 0x10;
/// Column of the doorway into the back room (page 1).
pub const INN_BACK_DOOR_X: u8 = 24;

/// The first talker's messages, in order.
pub const INN_TALKER_LINES: [&str; 3] = [
    "WELCOME TO\nDEV INN.",
    "TALK AGAIN\nFOR NEWS.",
    "NO MORE\nNEWS TODAY.",
];

fn dev_inn() -> Area {
    let mut r = Room::new("inn", 2, 1);
    r.objects = vec![
        ledge(3, 7, 2),
        Object::Raw(crate::sideview::Command::new(INN_BACK_DOOR_X, 7, 0x01)),
    ];
    r.exits = vec![
        Exit::new(Dir::Left, Target::Outside),
        Exit::new(
            Dir::Door { x: 8 },
            Target::Map {
                map: DEV_INN_FROM_MAP,
                page: 1,
            },
        ),
        Exit::new(Dir::Door { x: INN_BACK_DOOR_X }, Target::Room("back")),
    ];
    let mut back = Room::new("back", 1, 1);
    // At least one object: the loader draws the first command slot even
    // when the length byte says there is none.
    back.objects = vec![ledge(12, 6, 2)];
    back.exits = vec![Exit::new(Dir::Left, Target::Room("inn"))];
    let npc = |room, x, id, text| TownNpc {
        room,
        spawn: EnemySpawn { x, y: 9, id },
        message: Message { text },
    };
    Area {
        name: "Dev Inn",
        world: World::WestTowns,
        region: Region::West,
        first_map: 61,
        rooms: vec![r, back],
        entrances: vec![Entrance {
            region: Region::West,
            row: DEV_INN_TILE.0,
            col: DEV_INN_TILE.1,
            tile: TileKind::Town,
            room: "inn",
            page: 0,
            requires: Abilities::NONE,
        }],
        links: vec![VanillaLink::door(DEV_INN_FROM_MAP, 1, "inn", 0)],
        terrain: vec![],
        npcs: vec![
            npc("inn", 12, INN_TALKER_ID, INN_TALKER_LINES[0]),
            npc("inn", 18, INN_STANDING_ID, "REST HERE.\nI WILL MEND\nYOU."),
            npc("inn", 21, INN_STANDING_ID + 1, "DRINK THIS\nFOR MAGIC."),
            npc("inn", 28, INN_TALKER_ID, "THE SAGE\nBLESSED\nYOU. GO ON!"),
            npc("back", 8, crate::npc::ID_WISE_MAN, "I BLESS\nYOUR PATH."),
        ],
        needs: Abilities::NONE,
    }
}

/// Scripts of the developer people (`Dev Inn`).
pub const NPC_SCRIPTS: &[NpcScript] = &[
    NpcScript {
        more: &[
            Message {
                text: INN_TALKER_LINES[1],
            },
            Message {
                text: INN_TALKER_LINES[2],
            },
        ],
        ..NpcScript::new("Dev Inn", inn_npcs::TALKER, NpcRole::Talk)
    },
    NpcScript::new("Dev Inn", inn_npcs::HEALER, NpcRole::HealLife),
    NpcScript::new("Dev Inn", inn_npcs::MAGE, NpcRole::HealMagic),
    NpcScript {
        requires: Some(0),
        locked: Some(Message {
            text: "SEE THE\nSAGE IN THE\nBACK ROOM.",
        }),
        ..NpcScript::new("Dev Inn", inn_npcs::HINT, NpcRole::Talk)
    },
    NpcScript::new(
        "Dev Inn",
        inn_npcs::SAGE,
        NpcRole::Sage {
            flag: Some(0),
            gift: Gift::None,
        },
    ),
];

/// The developer areas in model form.
#[must_use]
pub fn areas() -> Vec<Area> {
    vec![test_room(), dev_cave(), dev_palace(), dev_inn()]
}

/// The developer areas in static form.
#[must_use]
pub fn statics() -> &'static [StaticArea] {
    generated::AREAS
}
