//! The Dark Isle: an optional late-game area set in the south-east of East
//! Hyrule, in the spirit of the southern continent of HoverBat's ZALiA (a
//! late island with an eighth palace and a town of monsters). Every room,
//! name and line of dialog here is original: no ZALiA map or text and no
//! room of the original game was copied.
//!
//! Three areas, one per world (an [`Area`] has a single world):
//!
//! * [`caves`], **Dark Isle Caves** (world 0, East Hyrule table): four dark
//!   cave rooms under the strait, from a cave mouth on the mainland's south
//!   coast to a cave mouth on the isle. Invisible Moas, lava pools that only
//!   the jump spell clears, and a 500-experience P-bag on a high ledge.
//! * [`town`], **Grimhollow** (world 2, East towns table): three streets
//!   and two houses where monsters live quietly. An Ache and two Bits talk,
//!   the two healer ladies restore life and magic, and a sage hands over a
//!   magic container.
//! * [`tower`], **Thornspire** (world 4, palace group 3/4/6): the eighth
//!   palace, a ten-room climb by four elevators. A small key waits in a side
//!   wing off the atrium and opens a locked door two floors up (a crumbling
//!   bridge drops careless climbers back into the atrium); two blue Iron
//!   Knuckles guard the top; the crown vault holds a heart container and a
//!   500-experience P-bag.
//!
//! # Attach points
//!
//! Overworld slots are East Hyrule location-table slots that are all zero
//! in the original (the free list in README.md); the
//! ROM-gated test `tests/dark_isle_rom.rs` re-checks that, and that no
//! original location sits on these tiles, at test time. Rows and columns
//! are overworld tile coordinates (row 0-74, column 0-63).
//!
//! | area | world / table | maps (virtual) | entrance | slot | row, col | tile | needs to reach |
//! |---|---|---|---|---|---|---|---|
//! | caves | 0, East Hyrule (bank 2) | 25-28 | mainland mouth, `shore_cave` page 0 | East 36 | 66, 60 (desert by the south coast) | cave | RAFT |
//! | caves | | | isle mouth, `isle_stair` page 1 | East 37 | 72, 61 | cave | the cave route |
//! | town | 2, East towns (bank 3) | 58-62 | `gate_street` page 0 | East 38 | 71, 63 | town | the cave route |
//! | tower | 4, palaces 3/4/6 (bank 4) | 53-62 | `gate` page 0 | East 55 | 74, 62 | palace | the cave route + BOOTS |
//!
//! Slots are handed out in [`crate::areas::SLOTS`]; the two cave mouths
//! are on consecutive slots (mainland first), so `shore_cave`'s left edge
//! (`Target::Outside`) leaves by the mainland mouth and `isle_stair`'s
//! right edge (`Target::OutsideAt(1)`) by the isle mouth, whichever mouth
//! Link came in by.
//!
//! "The cave route" is [`ISLE`]: RAFT (East Hyrule), CANDLE and CROSS (the
//! dark caves and their invisible Moas) and JUMP (the lava pools).
//!
//! **Isle terrain (engine).** The isle is not on the original map. The
//! engine paints it over open sea in the south-east corner, cells that are
//! deep water in the original (checked by the ROM test): rows 71-72,
//! columns 61-63 become land (desert), row 73 columns 61-63 become water the
//! boots walk on, and the four entrance tiles above are drawn with their
//! tile look. Columns 57-60 stay deep water, so the isle is reached only
//! through the caves, and the tower (row 74) only across the shallows.
//! Leaving the caves through `isle_stair`'s right edge comes out at the isle
//! mouth; through `shore_cave`'s left edge, at the mainland mouth.
//!
//! The town's slot is below `$2C`, so the original entry code derives no
//! town code for it; the engine supplies the town identity for a virtual
//! town entry. The tower's slot (`$37`) is in the palace range.
//!
//! **Map numbers.** Room `i` of an area is written as map `first_map + i`
//! in the tool-facing connection bytes only; the engine runs every virtual
//! room of a table in that table's one reserved slot and follows the typed
//! exits.
//!
//! `ghost_gallery`'s ledge is an `Object::Block` (two rows of rock in a
//! world-0 room with the cave object tiles): world 0 has no ledge object, and a `Platform` there crashes the cave
//! loader.
//!
//! # Rewards and progress
//!
//! | room | page | item | note |
//! |---|---|---|---|
//! | caves `ghost_gallery` | 1 | 500 P-bag | high ledge, plain jumps |
//! | town `sage_house` | 0 | magic container | the sage's gift |
//! | tower `key_wing` | 1 | small key | opens `bridge_hall`'s door |
//! | tower `crown_vault` | 0 | heart container | |
//! | tower `crown_vault` | 1 | 500 P-bag | |
//!
//! The sage's reward is a **magic container lying in his house** rather
//! than a spell: there is no ninth spell, and an ordinary item needs no new
//! code (the pickup clears its page bit and the engine records it in
//! `vroom_items`). Like the heart container it is a ninth-or-later
//! container on top of the original eight; a randomizer can swap either
//! for another item. Progress needs one "taken" bit per item page (five
//! bits) plus, if opened locked doors are to persist across visits, one bit
//! for the `bridge_hall` door.

use crate::model::{
    item, Abilities, Area, Dir, EnemySpawn, Entrance, Exit, MapTile, Message, Object, Region, Room,
    Target, TerrainEdit, TileKind, TownNpc, World,
};
use crate::sideview::Command;
use crate::StaticArea;

#[rustfmt::skip]
#[path = "dark_isle_generated.rs"]
mod generated;

/// Abilities the cave route takes: RAFT to be in East Hyrule at all,
/// CANDLE and CROSS for the dark caves and their invisible Moas, JUMP for
/// the lava pools. Everything on the isle needs at least these.
pub const ISLE: Abilities = Abilities::RAFT
    .with(Abilities::CANDLE)
    .with(Abilities::CROSS)
    .with(Abilities::JUMP);

/// Enemy IDs of the East Hyrule group (world 0), from the `z2-rando`
/// enemy catalog.
mod east_ids {
    /// Myu.
    pub const MYU: u8 = 0x03;
    /// Bot.
    pub const BOT: u8 = 0x04;
    /// Moa (invisible without the cross).
    pub const MOA: u8 = 0x06;
    /// Ache (ceiling bat).
    pub const ACHE: u8 = 0x07;
    /// Bago Bago (jumps out of lava and water).
    pub const BAGO_BAGO: u8 = 0x0F;
    /// Lizalfos.
    pub const LIZALFOS: u8 = 0x1A;
}

/// Town person codes (world 1/2 enemy IDs), as `z2-core::town_npc` names
/// them.
mod town_ids {
    /// The "Ache" talker: a walking townsperson (`$22`, the plain dialog
    /// condition). Ids `$00`/`$01` are no town people (nothing spawns);
    /// town talkers are `$0A-$22`, and no talker has a monster sprite.
    pub const ACHE_TALKER: u8 = 0x22;
    /// The "Bit" talker: a standing townsperson (`$10`).
    pub const BIT_TALKER: u8 = 0x10;
    /// Wise man (sage).
    pub const WISE_MAN: u8 = 0x0F;
    /// Healer lady (restores life).
    pub const HEALER: u8 = 0x17;
    /// Magic lady (restores magic).
    pub const MAGIC_LADY: u8 = 0x18;
}

/// Enemy IDs of the palace 3/4/6 group, from the `z2-rando` enemy catalog.
mod palace_ids {
    /// Myu.
    pub const MYU: u8 = 0x03;
    /// Bot.
    pub const BOT: u8 = 0x04;
    /// Orange Moa (flies).
    pub const ORANGE_MOA: u8 = 0x07;
    /// Orange Iron Knuckle.
    pub const IRON_KNUCKLE: u8 = 0x18;
    /// Blue Iron Knuckle (throws swords): the tower's guardians.
    pub const BLUE_IRON_KNUCKLE: u8 = 0x1A;
    /// Wizard (casts; reflect sends it back). Stands on row 9 only.
    pub const WIZARD: u8 = 0x1D;
    /// Doomknocker (swings a mace; needs room on both sides).
    pub const DOOMKNOCKER: u8 = 0x1E;
    /// Stalfos (drops from the ceiling).
    pub const STALFOS: u8 = 0x1F;
    /// The other Stalfos.
    pub const STALFOS_2: u8 = 0x23;
}

const fn floor(x: u8, pattern: u8) -> Object {
    Object::Floor {
        x,
        pattern,
        no_ceiling: false,
    }
}

const fn raw(x: u8, y: u8, b: u8) -> Object {
    Object::Raw(Command::new(x, y, b))
}

const fn spawn(x: u8, y: u8, id: u8) -> EnemySpawn {
    EnemySpawn { x, y, id }
}

const fn room(id: &'static str) -> Target {
    Target::Room(id)
}

/// A cave room: alternate (cave) object tiles, floor `pattern`.
fn cave_room(id: &'static str, pages: u8, pattern: u8) -> Room {
    let mut r = Room::new(id, pages, pattern);
    r.alt_objects = true;
    r
}

// ---------------------------------------------------------------------------
// Dark Isle Caves (world 0)
// ---------------------------------------------------------------------------

/// **Dark Isle Caves**: the route under the strait to the isle.
///
/// Floor patterns used: 8-10 (ceiling of 2-4 rows, Link stands on row 10)
/// and 0-5 (one-row ceiling, a floor that rises a row each, Link stands on
/// row 10 down to row 5).
///
/// * `shore_cave` (4 pages, CANDLE): in from the mainland mouth on the
///   left. A rolling floor, two steps up and back down; Aches hang where
///   the ceiling is thin, a Lizalfos and Bots on the ground.
/// * `ghost_gallery` (3 pages, CROSS): invisible Moas drift over a long
///   hall. A block step and a ledge above it hold the 500 P-bag.
/// * `lava_run` (4 pages, JUMP): three lava pools, each three columns
///   wide, that only the jump spell clears; Bago Bagos leap out of them.
/// * `isle_stair` (2 pages): a staircase climbing five rows to the isle
///   mouth on the right.
#[must_use]
pub fn caves() -> Area {
    use east_ids::*;

    let mut shore = cave_room("shore_cave", 4, 8);
    shore.objects = vec![
        floor(12, 1),
        floor(20, 2),
        floor(28, 9),
        floor(36, 2),
        floor(46, 10),
        floor(56, 8),
    ];
    shore.enemies = vec![
        spawn(15, 1, ACHE),
        spawn(22, 7, LIZALFOS),
        spawn(31, 9, BOT),
        spawn(40, 1, ACHE),
        spawn(52, 9, BOT),
    ];
    shore.exits = vec![
        Exit::new(Dir::Left, Target::Outside),
        Exit::new(Dir::Right, room("ghost_gallery")),
    ];
    shore.requires = Abilities::CANDLE;

    let mut gallery = cave_room("ghost_gallery", 3, 8);
    gallery.objects = vec![
        Object::Block {
            x: 16,
            y: 9,
            len: 2,
        },
        Object::Block {
            x: 19,
            y: 7,
            len: 4,
        },
        Object::Item {
            x: 21,
            y: 6,
            item: item::BAG_500,
        },
        floor(32, 9),
        floor(40, 8),
    ];
    gallery.enemies = vec![
        spawn(10, 5, MOA),
        spawn(26, 4, MOA),
        spawn(36, 5, MOA),
        spawn(44, 9, MYU),
    ];
    gallery.exits = vec![
        Exit::new(Dir::Left, room("shore_cave")),
        Exit::new(Dir::Right, room("lava_run")),
    ];
    gallery.requires = Abilities::CROSS;

    let mut lava = cave_room("lava_run", 4, 0);
    lava.objects = vec![
        Object::Lava { x: 14, width: 3 },
        Object::Lava { x: 30, width: 3 },
        Object::Lava { x: 46, width: 3 },
    ];
    lava.enemies = vec![
        spawn(15, 9, BAGO_BAGO),
        spawn(24, 1, ACHE),
        spawn(31, 9, BAGO_BAGO),
        spawn(47, 9, BAGO_BAGO),
        spawn(56, 9, LIZALFOS),
    ];
    lava.exits = vec![
        Exit::new(Dir::Left, room("ghost_gallery")),
        Exit::new(Dir::Right, room("isle_stair")),
    ];
    lava.requires = Abilities::JUMP;

    let mut stair = cave_room("isle_stair", 2, 8);
    stair.objects = vec![
        floor(8, 1),
        floor(12, 2),
        floor(16, 3),
        floor(20, 4),
        floor(24, 5),
    ];
    stair.enemies = vec![spawn(10, 9, MYU), spawn(18, 7, MYU)];
    stair.exits = vec![
        Exit::new(Dir::Left, room("lava_run")),
        Exit::new(Dir::Right, Target::OutsideAt(1)),
    ];

    Area {
        name: "Dark Isle Caves",
        world: World::Overworld,
        region: Region::East,
        first_map: 25,
        rooms: vec![shore, gallery, lava, stair],
        entrances: vec![
            Entrance {
                region: Region::East,
                row: 66,
                col: 60,
                tile: TileKind::Cave,
                room: "shore_cave",
                page: 0,
                requires: Abilities::RAFT,
            },
            Entrance {
                region: Region::East,
                row: 72,
                col: 61,
                tile: TileKind::Cave,
                room: "isle_stair",
                page: 1,
                requires: ISLE,
            },
        ],
        links: vec![],
        terrain: isle_terrain(),
        npcs: vec![],
        needs: ISLE,
    }
}

// ---------------------------------------------------------------------------
// Grimhollow (world 2)
// ---------------------------------------------------------------------------

/// A house front `w` columns wide at column `x` (roof, wall, two windows,
/// and a door at column `x + w / 2` when `door`), drawn above Link's rows
/// so he walks in front of it.
fn house(x: u8, w: u8, door: bool) -> Vec<Object> {
    let n = w - 1;
    let mut v = vec![
        raw(x, 2, 0x20 | n),
        raw(x, 4, 0x50 | n),
        raw(x + 1, 5, 0xC1),
        raw(x + w - 3, 5, 0xC1),
    ];
    if door {
        v.push(raw(x + w / 2, 8, 0x01));
    }
    v
}

/// An outdoor street: open sky, ground type 2, floor pattern 0 (Link
/// stands on row 10), the town's shared background.
fn street(id: &'static str, pages: u8) -> Room {
    let mut r = Room::new(id, pages, 0);
    r.no_ceiling = true;
    r.ground = 2;
    r.palette = 3;
    r.background = 1;
    r
}

/// A house inside: one page, ceiling, floor pattern 1 (Link stands on
/// row 9), left exit back to `street`.
fn interior(id: &'static str, street: &'static str) -> Room {
    let mut r = Room::new(id, 1, 1);
    r.palette = 3;
    // At least one command: the loader reads the first command slot even
    // when the length byte says there is none (it read past the four
    // header bytes of an empty room, measured). A floor command
    // restating the room's pattern draws nothing new.
    r.objects = vec![floor(0, 1)];
    r.exits = vec![Exit::new(Dir::Left, room(street))];
    r
}

/// The town people of Grimhollow and what they say.
#[must_use]
pub fn town_messages() -> Vec<(&'static str, Message)> {
    let m = |text| Message { text };
    vec![
        (
            "gate Ache",
            m("WE MONSTERS\nLIKE QUIET.\nNO SWORDS\nIN TOWN!"),
        ),
        (
            "gate Bit (hint)",
            m("THE SPIRE\nSTANDS PAST\nTHE SHOALS.\nWEAR BOOTS!"),
        ),
        (
            "market Bit (hint)",
            m("A LOCK SITS\nHIGH ABOVE.\nITS KEY\nHIDES LOW."),
        ),
        (
            "healer (life)",
            m("I PATCH UP\nHEROES AND\nGOBLINS.\nCOME IN."),
        ),
        ("magic lady", m("MY BREW\nFILLS YOUR\nMAGIC. IN\nYOU GO!")),
        (
            "far Ache (hint)",
            m("BEWARE THE\nIRON TWINS\nNEAR THE\nSPIRE TOP."),
        ),
        ("sage", m("MY MAGIC\nVESSEL IS\nYOURS NOW.\nTAKE IT.")),
    ]
}

/// **Grimhollow**: the town of monsters.
///
/// * `gate_street` (4 pages): in from the overworld on the left. Two
///   houses without doors; an Ache asks for quiet, a Bit points at the
///   shallows before the tower.
/// * `market_street` (4 pages): the healer lady (page 1) and the magic
///   lady (page 3) stand by the doors of the mending house; a Bit hints at
///   the tower's key-and-lock loop.
/// * `far_street` (3 pages): an Ache warns of the tower's guardians; the
///   sage's door on page 1; out to the overworld on the right.
/// * `mend_house` (1 page): where the two ladies restore Link.
/// * `sage_house` (1 page): the sage and his magic container.
#[must_use]
pub fn town() -> Area {
    use town_ids::*;
    let msg = town_messages();
    let say = |i: usize| msg[i].1;

    let mut gate = street("gate_street", 4);
    gate.objects = [house(6, 8, false), house(36, 10, false)].concat();
    gate.exits = vec![
        Exit::new(Dir::Left, Target::Outside),
        Exit::new(Dir::Right, room("market_street")),
    ];

    let mut market = street("market_street", 4);
    market.objects = [house(14, 8, true), house(30, 6, false), house(46, 8, true)].concat();
    market.exits = vec![
        Exit::new(Dir::Left, room("gate_street")),
        Exit::new(Dir::Right, room("far_street")),
        Exit::new(Dir::Door { x: 18 }, room("mend_house")),
        Exit::new(Dir::Door { x: 50 }, room("mend_house")),
    ];

    let mut far = street("far_street", 3);
    far.objects = [house(4, 6, false), house(20, 8, true)].concat();
    far.exits = vec![
        Exit::new(Dir::Left, room("market_street")),
        Exit::new(Dir::Right, Target::Outside),
        Exit::new(Dir::Door { x: 24 }, room("sage_house")),
    ];

    let mend = interior("mend_house", "market_street");

    let mut sage = interior("sage_house", "far_street");
    sage.objects = vec![Object::Item {
        x: 12,
        y: 9,
        item: item::MAGIC_CONTAINER,
    }];

    let npc = |room, spawn, i| TownNpc {
        room,
        spawn,
        message: say(i),
    };
    Area {
        name: "Grimhollow",
        world: World::EastTowns,
        region: Region::East,
        first_map: 58,
        rooms: vec![gate, market, far, mend, sage],
        entrances: vec![Entrance {
            region: Region::East,
            row: 71,
            col: 63,
            tile: TileKind::Town,
            room: "gate_street",
            page: 0,
            requires: ISLE,
        }],
        links: vec![],
        terrain: vec![],
        npcs: vec![
            npc("gate_street", spawn(22, 9, ACHE_TALKER), 0),
            npc("gate_street", spawn(50, 9, BIT_TALKER), 1),
            npc("market_street", spawn(26, 9, BIT_TALKER), 2),
            npc("market_street", spawn(15, 9, HEALER), 3),
            npc("market_street", spawn(47, 9, MAGIC_LADY), 4),
            npc("far_street", spawn(12, 9, ACHE_TALKER), 5),
            npc("sage_house", spawn(8, 9, WISE_MAN), 6),
        ],
        needs: ISLE,
    }
}

// ---------------------------------------------------------------------------
// Thornspire (world 4)
// ---------------------------------------------------------------------------

/// Elevator at column `x` (two columns). The compiler opens its shaft
/// through the ceiling and the floor for the room's `Up` / `Down` exits
/// (`compile::elevator_shaft`); `up` / `down` only say which ways this one
/// goes, for the reader.
fn elevator(x: u8, up: bool, down: bool) -> Vec<Object> {
    let _ = (up, down);
    vec![Object::Elevator { x }]
}

/// **Thornspire**: the eighth palace, a climb.
///
/// Floor pattern 1 throughout (Link stands on row 9) unless noted.
///
/// * `gate` (2 pages): in from the overworld on the left, statues, a Bot.
/// * `atrium` (4 pages): the elevator up on page 2; the side wing on the
///   right. Falling through `bridge_hall`'s bridge lands here (page 2).
/// * `key_wing` (2 pages): a block step and a high ledge with the small
///   key; a Stalfos drops in. Dead end.
/// * `lift_a` (1 page): a shaft between the atrium and `bridge_hall`;
///   orange Moas.
/// * `bridge_hall` (4 pages): off the elevator on page 0; a crumbling
///   bridge over a pit on page 2 (a one-column gap to jump, or fall back to
///   the atrium); the locked door before the right exit.
/// * `upper_lift` (2 pages, GLOVE): a breakable wall across the hall; a
///   Doomknocker; the elevator up on page 1.
/// * `watch_gallery` (4 pages, JUMP and REFLECT): Wizards on the lower
///   floor; from column 32 the floor rises three rows (pattern 4, Link
///   stands on row 6), a climb only the jump spell makes; Stalfos above.
/// * `knight_hall` (2 pages): the mini-boss, two blue Iron Knuckles.
/// * `crown_lift` (1 page): the last elevator.
/// * `crown_vault` (2 pages): the heart container and a 500 P-bag.
#[must_use]
pub fn tower() -> Area {
    use palace_ids::*;

    let mut gate = Room::new("gate", 2, 1);
    gate.objects = vec![
        Object::Statue { x: 6, y: 8 },
        Object::Statue { x: 25, y: 8 },
    ];
    gate.enemies = vec![spawn(18, 9, BOT)];
    gate.exits = vec![
        Exit::new(Dir::Left, Target::Outside),
        Exit::new(Dir::Right, room("atrium")),
    ];

    let mut atrium = Room::new("atrium", 4, 1);
    atrium.objects = [
        vec![
            Object::Column {
                x: 12,
                y: 1,
                height: 9,
            },
            Object::Column {
                x: 28,
                y: 1,
                height: 9,
            },
        ],
        elevator(40, true, false),
        vec![Object::Statue { x: 52, y: 8 }],
    ]
    .concat();
    atrium.enemies = vec![
        spawn(20, 8, STALFOS),
        spawn(34, 9, MYU),
        spawn(56, 8, IRON_KNUCKLE),
    ];
    atrium.exits = vec![
        Exit::new(Dir::Left, room("gate")),
        Exit::new(Dir::Up, room("lift_a")),
        Exit::new(Dir::Right, room("key_wing")),
    ];

    let mut wing = Room::new("key_wing", 2, 1);
    wing.objects = vec![
        Object::Block {
            x: 18,
            y: 8,
            len: 2,
        },
        Object::Platform {
            x: 21,
            y: 6,
            len: 3,
        },
        Object::Item {
            x: 22,
            y: 5,
            item: item::SMALL_KEY,
        },
    ];
    wing.enemies = vec![spawn(12, 9, MYU), spawn(27, 8, STALFOS_2)];
    wing.exits = vec![Exit::new(Dir::Left, room("atrium"))];

    let mut lift = Room::new("lift_a", 1, 1);
    lift.objects = [
        elevator(7, true, true),
        vec![
            Object::Platform { x: 1, y: 6, len: 3 },
            Object::Platform {
                x: 12,
                y: 6,
                len: 3,
            },
        ],
    ]
    .concat();
    lift.enemies = vec![spawn(3, 3, ORANGE_MOA), spawn(13, 3, ORANGE_MOA)];
    lift.exits = vec![
        Exit::new(Dir::Down, room("atrium")),
        Exit::new(Dir::Up, room("bridge_hall")),
    ];

    let mut bridge = Room::new("bridge_hall", 4, 1);
    bridge.objects = [
        elevator(7, false, true),
        vec![
            Object::Pit { x: 36, width: 4 },
            Object::Crumble {
                x: 36,
                y: 10,
                len: 2,
            },
            Object::Crumble {
                x: 39,
                y: 10,
                len: 1,
            },
            Object::LockedDoor { x: 58, y: 7 },
        ],
    ]
    .concat();
    bridge.enemies = vec![
        spawn(20, 9, WIZARD),
        spawn(30, 4, ORANGE_MOA),
        spawn(48, 8, IRON_KNUCKLE),
    ];
    bridge.exits = vec![
        Exit::new(Dir::Down, room("lift_a")),
        Exit::new(Dir::Drop { page: 2 }, room("atrium")),
        Exit::new(Dir::Right, room("upper_lift")).needs(Abilities::KEY),
    ];

    let mut upper = Room::new("upper_lift", 2, 1);
    upper.objects = [
        (2..=8)
            .step_by(2)
            .map(|y| Object::BreakableBlock { x: 14, y, len: 1 })
            .collect::<Vec<_>>(),
        elevator(23, true, false),
    ]
    .concat();
    upper.enemies = vec![spawn(7, 8, DOOMKNOCKER)];
    upper.exits = vec![
        Exit::new(Dir::Left, room("bridge_hall")),
        Exit::new(Dir::Up, room("watch_gallery")),
    ];
    upper.requires = Abilities::GLOVE;

    let mut gallery = Room::new("watch_gallery", 4, 1);
    gallery.objects = [
        elevator(7, false, true),
        vec![
            Object::Column {
                x: 16,
                y: 1,
                height: 9,
            },
            floor(32, 4),
            Object::Column {
                x: 48,
                y: 1,
                height: 6,
            },
        ],
    ]
    .concat();
    gallery.enemies = vec![
        spawn(20, 9, WIZARD),
        spawn(27, 9, WIZARD),
        spawn(40, 3, ORANGE_MOA),
        spawn(44, 5, STALFOS),
        spawn(56, 5, STALFOS_2),
    ];
    gallery.exits = vec![
        Exit::new(Dir::Down, room("upper_lift")),
        Exit::new(Dir::Right, room("knight_hall")),
    ];
    gallery.requires = Abilities::JUMP.with(Abilities::REFLECT);

    let mut knights = Room::new("knight_hall", 2, 1);
    knights.objects = vec![
        Object::Statue { x: 3, y: 8 },
        Object::Statue { x: 28, y: 8 },
    ];
    knights.enemies = vec![
        spawn(16, 8, BLUE_IRON_KNUCKLE),
        spawn(24, 8, BLUE_IRON_KNUCKLE),
    ];
    knights.exits = vec![
        Exit::new(Dir::Left, room("watch_gallery")),
        Exit::new(Dir::Right, room("crown_lift")),
    ];

    let mut crown_lift = Room::new("crown_lift", 1, 1);
    crown_lift.objects = elevator(7, true, false);
    crown_lift.exits = vec![
        Exit::new(Dir::Left, room("knight_hall")),
        Exit::new(Dir::Up, room("crown_vault")),
    ];

    let mut vault = Room::new("crown_vault", 2, 1);
    vault.objects = [
        elevator(7, false, true),
        vec![
            Object::Item {
                x: 12,
                y: 9,
                item: item::HEART_CONTAINER,
            },
            Object::Platform {
                x: 22,
                y: 8,
                len: 4,
            },
            Object::Item {
                x: 24,
                y: 7,
                item: item::BAG_500,
            },
        ],
    ]
    .concat();
    vault.exits = vec![Exit::new(Dir::Down, room("crown_lift"))];

    Area {
        name: "Thornspire",
        world: World::PalaceB,
        region: Region::East,
        first_map: 53,
        rooms: vec![
            gate, atrium, wing, lift, bridge, upper, gallery, knights, crown_lift, vault,
        ],
        entrances: vec![Entrance {
            region: Region::East,
            row: 74,
            col: 62,
            tile: TileKind::Palace,
            room: "gate",
            page: 0,
            requires: ISLE.with(Abilities::BOOTS),
        }],
        links: vec![],
        terrain: vec![],
        npcs: vec![],
        needs: ISLE
            .with(Abilities::BOOTS)
            .with(Abilities::GLOVE)
            .with(Abilities::KEY)
            .with(Abilities::REFLECT),
    }
}

/// [`ISLE_TERRAIN`] as the caves area's terrain declaration (desert land,
/// shallow water the boots walk on).
fn isle_terrain() -> Vec<TerrainEdit> {
    ISLE_TERRAIN
        .iter()
        .map(|&(row, col, shallow)| TerrainEdit {
            region: Region::East,
            row,
            col,
            tile: if shallow {
                MapTile::ShallowWater
            } else {
                MapTile::Desert
            },
        })
        .collect()
}

/// The isle the engine paints over open sea: `(row, col, walkable water)`.
/// `false` is land. Every cell is deep water in the original map. The
/// caves area declares it as its terrain ([`crate::TerrainEdit`]).
pub const ISLE_TERRAIN: [(u8, u8, bool); 9] = [
    (71, 61, false),
    (71, 62, false),
    (71, 63, false),
    (72, 61, false),
    (72, 62, false),
    (72, 63, false),
    (73, 61, true),
    (73, 62, true),
    (73, 63, true),
];

/// The Dark Isle areas: caves, town, tower.
#[must_use]
pub fn areas() -> Vec<Area> {
    vec![caves(), town(), tower()]
}

/// The group's areas in static form (what the engine registers).
#[must_use]
pub fn statics() -> &'static [StaticArea] {
    generated::AREAS
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::areas::slot_of;
    use crate::compile::{compile, Dest, OUTSIDE};
    use crate::text;

    #[test]
    fn dialog_fits_the_box() {
        let msgs = town_messages();
        assert_eq!(msgs.len(), town().npcs.len());
        for (who, m) in &msgs {
            text::validate_dialog(m.text, text::MAX_LINES).unwrap_or_else(|e| panic!("{who}: {e}"));
            let bytes = text::encode_message(m.text).unwrap();
            assert_eq!(text::decode(&bytes), m.text, "{who}");
        }
    }

    #[test]
    fn shape_of_the_isle() {
        let caves = caves();
        let town = town();
        let tower = tower();
        assert_eq!(caves.rooms.len(), 4);
        assert_eq!(town.rooms.len(), 5);
        assert!((8..=12).contains(&tower.rooms.len()));
        let elevators = tower
            .rooms
            .iter()
            .filter(|r| r.elevator_page().is_some())
            .count();
        assert!(elevators >= 4);
        // The healers, the sage and the hint-givers are there.
        let ids: Vec<u8> = town.npcs.iter().map(|n| n.spawn.id).collect();
        for id in [
            town_ids::HEALER,
            town_ids::MAGIC_LADY,
            town_ids::WISE_MAN,
            town_ids::BIT_TALKER,
            town_ids::ACHE_TALKER,
        ] {
            assert!(ids.contains(&id), "{id:#04x}");
        }
        // The mini-boss pair.
        let knights = tower.room("knight_hall").unwrap();
        assert_eq!(
            knights
                .enemies
                .iter()
                .filter(|e| e.id == palace_ids::BLUE_IRON_KNUCKLE)
                .count(),
            2
        );
        // Slots and entrances line up; the cave mouths are consecutive.
        let entrances: Vec<(&str, Entrance)> = areas()
            .iter()
            .flat_map(|a| a.entrances.iter().map(|e| (a.name, *e)))
            .collect();
        let mut seen: Vec<(&str, usize)> = Vec::new();
        for (n, _) in &entrances {
            let i = seen.iter().filter(|(m, _)| m == n).count();
            seen.push((n, i));
            assert!(slot_of(n, i).is_some(), "{n} entrance {i}");
        }
        assert_eq!(
            slot_of("Dark Isle Caves", 1).unwrap().slot,
            slot_of("Dark Isle Caves", 0).unwrap().slot + 1
        );
        // The tower and the isle mouth stand on the painted isle.
        for (_, e) in &entrances[1..] {
            let on_isle = ISLE_TERRAIN
                .iter()
                .any(|&(r, c, _)| r == e.row && c == e.col);
            let by_shallows = ISLE_TERRAIN
                .iter()
                .any(|&(r, c, w)| w && r + 1 == e.row && c == e.col);
            assert!(on_isle || by_shallows, "{e:?}");
        }
    }

    #[test]
    fn compiled_bytes() {
        let c = compile(&tower()).unwrap();
        assert_eq!(c.world, 4);
        assert_eq!(c.rooms.first().unwrap().map, 53);
        assert_eq!(c.rooms.last().unwrap().map, 62);
        // bridge_hall (map 57): down to lift_a's elevator page, the pit on
        // page 2 to the atrium's page 2 (a fall keeps its page), right to
        // upper_lift.
        let bridge = &c.rooms[4];
        assert_eq!(
            bridge.connections,
            [OUTSIDE, 56 << 2, (54 << 2) | 2, 58 << 2]
        );
        // Town doors: market_street's pages 1 and 3 lead into mend_house.
        let t = compile(&town()).unwrap();
        assert_eq!(t.rooms[1].doors, [0xFF, 61 << 2, 0xFF, 61 << 2]);
        assert_eq!(t.rooms[2].doors, [0xFF, 62 << 2, 0xFF, 0xFF]);
        assert_eq!(t.npcs.len(), 7);
        let caves = compile(&caves()).unwrap();
        assert_eq!(caves.entrances[1].target, (28 << 2) | 1);
        assert!(caves.rooms.iter().all(|r| r.map < 0x1D));
        // isle_stair's right edge (byte 1) leaves by the isle mouth.
        assert_eq!(caves.rooms[3].exits[1], Dest::Byte(OUTSIDE | 1));
    }
}
