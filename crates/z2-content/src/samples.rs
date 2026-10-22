//! Original sample areas: small, complete examples of the authoring model.
//!
//! Every room here was laid out for this crate; nothing is copied from the
//! game or from any other hack. Floor pattern 1 (used throughout) is the
//! game's own "ceiling plus a three-row floor" pattern: Link stands on
//! row 9.

use crate::model::{
    item, Abilities, Area, Dir, EnemySpawn, Entrance, Exit, Message, Object, Region, Room, Target,
    TileKind, TownNpc, World,
};

/// Palace enemy IDs (palaces 1/2/5 group) used by the samples.
mod palace_ids {
    /// Myu.
    pub const MYU: u8 = 0x03;
    /// Bot.
    pub const BOT: u8 = 0x04;
}

/// "Hollow Annex": a three-room palace annex.
///
/// * `hall` (2 pages): in from the overworld on the left; a ledge with a
///   small key; a locked door before the right exit.
/// * `shaft` (2 pages): an elevator down; a Bot guards the floor.
/// * `vault` (1 page): the elevator back up; a heart container behind a
///   breakable wall (glove).
#[must_use]
pub fn hollow_annex() -> Area {
    let mut hall = Room::new("hall", 2, 1);
    hall.objects = vec![
        Object::Platform {
            x: 10,
            y: 8,
            len: 3,
        },
        Object::Item {
            x: 11,
            y: 7,
            item: item::SMALL_KEY,
        },
        Object::Block {
            x: 18,
            y: 8,
            len: 2,
        },
        Object::LockedDoor { x: 28, y: 7 },
    ];
    hall.enemies = vec![
        EnemySpawn {
            x: 14,
            y: 9,
            id: palace_ids::MYU,
        },
        EnemySpawn {
            x: 22,
            y: 9,
            id: palace_ids::BOT,
        },
    ];
    hall.exits = vec![
        Exit::new(Dir::Left, Target::Outside),
        Exit::new(Dir::Right, Target::Room("shaft")).needs(Abilities::KEY),
    ];

    let mut shaft = Room::new("shaft", 2, 1);
    shaft.objects = vec![
        Object::Platform { x: 3, y: 6, len: 2 },
        Object::Elevator { x: 22 },
    ];
    shaft.enemies = vec![EnemySpawn {
        x: 12,
        y: 9,
        id: palace_ids::BOT,
    }];
    shaft.exits = vec![
        Exit::new(Dir::Left, Target::Room("hall")),
        Exit::new(Dir::Down, Target::Room("vault")),
    ];

    let mut vault = Room::new("vault", 1, 1);
    vault.objects = vec![
        Object::Elevator { x: 3 },
        Object::BreakableBlock {
            x: 10,
            y: 2,
            len: 1,
        },
        Object::BreakableBlock {
            x: 10,
            y: 4,
            len: 1,
        },
        Object::BreakableBlock {
            x: 10,
            y: 6,
            len: 1,
        },
        Object::BreakableBlock {
            x: 10,
            y: 8,
            len: 1,
        },
        Object::Item {
            x: 13,
            y: 9,
            item: item::HEART_CONTAINER,
        },
    ];
    vault.item_requires = Abilities::GLOVE;
    vault.exits = vec![Exit::new(Dir::Up, Target::Room("shaft"))];

    Area {
        name: "Hollow Annex",
        world: World::PalaceA,
        region: Region::East,
        first_map: 60,
        rooms: vec![hall, shaft, vault],
        links: vec![],
        entrances: vec![Entrance {
            region: Region::East,
            row: 40,
            col: 30,
            tile: TileKind::Cave,
            room: "hall",
            page: 0,
            requires: Abilities::NONE,
        }],
        terrain: vec![],
        npcs: vec![],
        needs: Abilities::KEY.with(Abilities::GLOVE),
    }
}

/// "Wayside Hut": a one-room town stop with a person who talks.
#[must_use]
pub fn wayside_hut() -> Area {
    let mut hut = Room::new("hut", 1, 1);
    hut.no_ceiling = true;
    hut.objects = vec![Object::Platform { x: 6, y: 8, len: 4 }];
    hut.exits = vec![
        Exit::new(Dir::Left, Target::Outside),
        Exit::new(Dir::Right, Target::Outside),
    ];
    Area {
        name: "Wayside Hut",
        world: World::WestTowns,
        region: Region::West,
        first_map: 40,
        rooms: vec![hut],
        links: vec![],
        entrances: vec![Entrance {
            region: Region::West,
            row: 20,
            col: 12,
            tile: TileKind::Town,
            room: "hut",
            page: 0,
            requires: Abilities::NONE,
        }],
        terrain: vec![],
        npcs: vec![TownNpc {
            room: "hut",
            spawn: EnemySpawn {
                x: 12,
                y: 9,
                id: 0x05,
            },
            message: Message {
                text: "REST HERE,\nTRAVELER.\nTHE ROAD IS\nLONG.",
            },
        }],
        needs: Abilities::NONE,
    }
}

/// Every sample area with the identifier its static table gets in
/// `src/generated.rs`.
#[must_use]
pub fn all() -> Vec<(&'static str, Area)> {
    vec![
        ("HOLLOW_ANNEX", hollow_annex()),
        ("WAYSIDE_HUT", wayside_hut()),
    ]
}
