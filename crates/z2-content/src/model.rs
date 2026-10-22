//! The typed authoring model: areas, rooms, objects, enemies, exits,
//! entrances, town people and their messages.
//!
//! Everything here is plain data built in Rust code (see
//! [`crate::samples`]). Coordinates are absolute: columns 0-63 across up to
//! four 16-column pages, rows 0-12 from the top (row 12 is the bottom row).
//! [`crate::compile`] turns an [`Area`] into the game's byte formats and
//! [`crate::validate`] checks it.

use crate::sideview::{Command, ObjectSet, COLLECTABLE, ROW_EXTENDED, ROW_FLOOR};
use crate::ContentError;

/// A room's id inside its area (unique per area).
pub type RoomId = &'static str;

/// Player abilities: items, spells and techniques. A set of bits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub struct Abilities(pub u16);

impl Abilities {
    /// Nothing.
    pub const NONE: Abilities = Abilities(0);
    /// Jump spell (higher, longer jumps).
    pub const JUMP: Abilities = Abilities(1 << 0);
    /// Fairy spell (fly through free cells and locked doors).
    pub const FAIRY: Abilities = Abilities(1 << 1);
    /// Hammer (overworld boulders).
    pub const HAMMER: Abilities = Abilities(1 << 2);
    /// Power glove (break blocks).
    pub const GLOVE: Abilities = Abilities(1 << 3);
    /// Candle (dark rooms).
    pub const CANDLE: Abilities = Abilities(1 << 4);
    /// Boots (walk on water).
    pub const BOOTS: Abilities = Abilities(1 << 5);
    /// Raft.
    pub const RAFT: Abilities = Abilities(1 << 6);
    /// Flute.
    pub const FLUTE: Abilities = Abilities(1 << 7);
    /// Cross (see invisible enemies).
    pub const CROSS: Abilities = Abilities(1 << 8);
    /// Small key(s) for locked doors.
    pub const KEY: Abilities = Abilities(1 << 9);
    /// Downward thrust.
    pub const DOWNSTAB: Abilities = Abilities(1 << 10);
    /// Upward thrust.
    pub const UPSTAB: Abilities = Abilities(1 << 11);
    /// Reflect spell.
    pub const REFLECT: Abilities = Abilities(1 << 12);
    /// Every ability.
    pub const ALL: Abilities = Abilities((1 << 13) - 1);

    /// Union.
    #[must_use]
    pub const fn with(self, o: Abilities) -> Abilities {
        Abilities(self.0 | o.0)
    }

    /// Every bit of `self` is in `o`.
    #[must_use]
    pub const fn within(self, o: Abilities) -> bool {
        self.0 & !o.0 == 0
    }

    /// Has all of `o`.
    #[must_use]
    pub const fn has(self, o: Abilities) -> bool {
        self.0 & o.0 == o.0
    }
}

/// The world a room belongs to: the value the game keeps in `$0707`. It
/// selects the object meanings, the enemy ID space and the tile set the
/// engine loads for a virtual room.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum World {
    /// Overworld scenes and caves (`0`).
    Overworld,
    /// West towns (`1`).
    WestTowns,
    /// East towns (`2`).
    EastTowns,
    /// Palaces 1/2/5 (`3`).
    PalaceA,
    /// Palaces 3/4/6 (`4`).
    PalaceB,
    /// Great Palace (`5`).
    GreatPalace,
}

impl World {
    /// The `$0707` value.
    #[must_use]
    pub const fn id(self) -> u8 {
        match self {
            World::Overworld => 0,
            World::WestTowns => 1,
            World::EastTowns => 2,
            World::PalaceA => 3,
            World::PalaceB => 4,
            World::GreatPalace => 5,
        }
    }

    /// Object meanings of this world's rooms.
    #[must_use]
    pub const fn object_set(self) -> ObjectSet {
        match self {
            World::PalaceA | World::PalaceB => ObjectSet::Palace,
            World::GreatPalace => ObjectSet::GreatPalace,
            _ => ObjectSet::Other,
        }
    }
}

/// Collectable item bytes (the third byte of an item command; same values
/// as the game's item ids).
pub mod item {
    /// Candle.
    pub const CANDLE: u8 = 0x00;
    /// Power glove.
    pub const GLOVE: u8 = 0x01;
    /// Raft.
    pub const RAFT: u8 = 0x02;
    /// Boots.
    pub const BOOTS: u8 = 0x03;
    /// Flute.
    pub const FLUTE: u8 = 0x04;
    /// Cross.
    pub const CROSS: u8 = 0x05;
    /// Hammer.
    pub const HAMMER: u8 = 0x06;
    /// Magic key.
    pub const MAGIC_KEY: u8 = 0x07;
    /// Palace small key.
    pub const SMALL_KEY: u8 = 0x08;
    /// 50-experience P-bag.
    pub const BAG_50: u8 = 0x0A;
    /// 100-experience P-bag.
    pub const BAG_100: u8 = 0x0B;
    /// 200-experience P-bag.
    pub const BAG_200: u8 = 0x0C;
    /// 500-experience P-bag.
    pub const BAG_500: u8 = 0x0D;
    /// Magic container.
    pub const MAGIC_CONTAINER: u8 = 0x0E;
    /// Heart container.
    pub const HEART_CONTAINER: u8 = 0x0F;
    /// Blue magic jar.
    pub const BLUE_JAR: u8 = 0x10;
    /// Red magic jar.
    pub const RED_JAR: u8 = 0x11;
    /// Extra life.
    pub const ONE_UP: u8 = 0x12;
}

/// One drawing object of a room. `x` is the left column (0-63), `y` the
/// top row (0-12). `len`/`width`/`height` are 1-16.
///
/// Which kinds a world supports (see [`Object::to_command`]):
///
/// | object | palace | Great Palace | towns, caves, scenes |
/// |---|---|---|---|
/// | `Floor`, `Lava`, `Elevator`, `Item`, `Raw` | yes | yes | yes |
/// | `Platform` | yes | yes | towns: drawn, but background (Link walks and falls through it, measured); world 0 has no kind-1 object (the loader crashes, validation rejects it) |
/// | `Block` | yes | yes | world 0 rooms with `alt_objects` (the cave tiles: two rows of rock, the top one walkable; object kind 2, measured); towns not measured |
/// | `Breakable`, `BreakableBlock`, `Crumble`, `Bridge`, `FalseWall`, `Column`, `LockedDoor` | yes | yes | no |
/// | `Pit` | yes | no | yes |
/// | `Statue` | yes | no | no |
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Object {
    /// From column `x` on, use floor/ceiling `pattern` (0-15); `no_ceiling`
    /// leaves the top two rows open.
    Floor {
        /// Column.
        x: u8,
        /// Pattern 0-15.
        pattern: u8,
        /// No ceiling.
        no_ceiling: bool,
    },
    /// A 1-row solid ledge `len` columns wide.
    Platform {
        /// Column.
        x: u8,
        /// Row.
        y: u8,
        /// Width.
        len: u8,
    },
    /// A 2-row solid block `len` columns wide.
    Block {
        /// Column.
        x: u8,
        /// Row.
        y: u8,
        /// Width.
        len: u8,
    },
    /// A 1-row breakable ledge (glove).
    Breakable {
        /// Column.
        x: u8,
        /// Row.
        y: u8,
        /// Width.
        len: u8,
    },
    /// A 2-row breakable block (glove).
    BreakableBlock {
        /// Column.
        x: u8,
        /// Row.
        y: u8,
        /// Width.
        len: u8,
    },
    /// A bridge that crumbles after being stood on.
    Crumble {
        /// Column.
        x: u8,
        /// Row.
        y: u8,
        /// Width.
        len: u8,
    },
    /// A solid bridge.
    Bridge {
        /// Column.
        x: u8,
        /// Row.
        y: u8,
        /// Width.
        len: u8,
    },
    /// Bricks that can be walked through (2 rows).
    FalseWall {
        /// Column.
        x: u8,
        /// Row.
        y: u8,
        /// Width.
        len: u8,
    },
    /// A background column `height` rows tall (not solid).
    Column {
        /// Column.
        x: u8,
        /// Top row.
        y: u8,
        /// Height.
        height: u8,
    },
    /// A bottomless pit `width` columns wide (clears whole columns). The
    /// game has no wide hole object: the compiler writes one command per
    /// column (world 0: the extended pit with 13 rows; palaces: a
    /// 13-row background strip), see `compile::room_sideview`.
    Pit {
        /// Column.
        x: u8,
        /// Width.
        width: u8,
    },
    /// Lava along the bottom rows, `width` columns wide.
    Lava {
        /// Column.
        x: u8,
        /// Width.
        width: u8,
    },
    /// The elevator (two columns from `x`); needed for `Up`/`Down` exits.
    Elevator {
        /// Left column.
        x: u8,
    },
    /// A locked door (needs a key, or fairy).
    LockedDoor {
        /// Column.
        x: u8,
        /// Row.
        y: u8,
    },
    /// An Iron Knuckle statue (palace).
    Statue {
        /// Column.
        x: u8,
        /// Row.
        y: u8,
    },
    /// A collectable item (see [`item`]).
    Item {
        /// Column.
        x: u8,
        /// Row.
        y: u8,
        /// Item byte.
        item: u8,
    },
    /// A raw command for anything not named above.
    Raw(Command),
}

fn sized(kind: u8, n: u8) -> Result<u8, ContentError> {
    if !(1..=16).contains(&n) {
        return Err(ContentError::Encode(format!("object size {n} not in 1-16")));
    }
    Ok((kind << 4) | (n - 1))
}

fn unsupported(o: &Object, set: ObjectSet) -> ContentError {
    ContentError::Encode(format!("{o:?} has no {set:?} encoding"))
}

impl Object {
    /// Left column.
    #[must_use]
    pub const fn x(&self) -> u8 {
        match *self {
            Object::Floor { x, .. }
            | Object::Platform { x, .. }
            | Object::Block { x, .. }
            | Object::Breakable { x, .. }
            | Object::BreakableBlock { x, .. }
            | Object::Crumble { x, .. }
            | Object::Bridge { x, .. }
            | Object::FalseWall { x, .. }
            | Object::Column { x, .. }
            | Object::Pit { x, .. }
            | Object::Lava { x, .. }
            | Object::Elevator { x }
            | Object::LockedDoor { x, .. }
            | Object::Statue { x, .. }
            | Object::Item { x, .. } => x,
            Object::Raw(c) => c.x,
        }
    }

    /// The sideview command for this object in `set`.
    pub fn to_command(&self, set: ObjectSet) -> Result<Command, ContentError> {
        use ObjectSet::{GreatPalace as Gp, Other, Palace};
        let row = |y: u8| -> Result<u8, ContentError> {
            if y < ROW_FLOOR {
                Ok(y)
            } else {
                Err(ContentError::Encode(format!("row {y} not in 0-12")))
            }
        };
        let c = match (*self, set) {
            (
                Object::Floor {
                    x,
                    pattern,
                    no_ceiling,
                },
                _,
            ) => {
                if pattern > 15 {
                    return Err(ContentError::Encode(format!("floor pattern {pattern}")));
                }
                Command::floor(x, (u8::from(no_ceiling) << 7) | pattern)
            }
            // The object kinds below are measured in the game (see
            // `sideview::large_cells` for the whole table): Palace and
            // Great Palace number their solid, breakable and crumbling
            // objects differently.
            (Object::Platform { x, y, len }, Palace) => Command::new(x, row(y)?, sized(2, len)?),
            (Object::Platform { x, y, len }, Gp | Other) => {
                Command::new(x, row(y)?, sized(1, len)?)
            }
            (Object::Block { x, y, len }, Palace) => Command::new(x, row(y)?, sized(7, len)?),
            (Object::Block { x, y, len }, Gp) => Command::new(x, row(y)?, sized(9, len)?),
            // World 0: kind 2 is the rock block of the original caves (a
            // walkable top row over solid rock, measured: tiles `$52` /
            // `$53`). Kind 8 there draws `$81`, which Link falls through.
            (Object::Block { x, y, len }, Other) => Command::new(x, row(y)?, sized(2, len)?),
            (Object::Breakable { x, y, len }, Palace | Gp) => {
                Command::new(x, row(y)?, sized(3, len)?)
            }
            (Object::BreakableBlock { x, y, len }, Palace) => {
                Command::new(x, row(y)?, sized(9, len)?)
            }
            (Object::BreakableBlock { x, y, len }, Gp) => {
                Command::new(x, row(y)?, sized(0x0C, len)?)
            }
            (Object::Crumble { x, y, len }, Palace) => Command::new(x, row(y)?, sized(5, len)?),
            (Object::Crumble { x, y, len }, Gp) => Command::new(x, row(y)?, sized(7, len)?),
            (Object::Bridge { x, y, len }, Palace | Gp) => Command::new(x, row(y)?, sized(4, len)?),
            (Object::FalseWall { x, y, len }, Palace | Gp) => {
                Command::new(x, row(y)?, sized(0x0A, len)?)
            }
            (Object::Column { x, y, height }, Palace) => {
                Command::new(x, row(y)?, sized(0x0C, height)?)
            }
            (Object::Column { x, y, height }, Gp) => Command::new(x, row(y)?, sized(0x0F, height)?),
            // Palace: a one-column background strip (kind D) over all 13
            // rows is a hole. The extended kind 1 the model used before
            // draws lava (`$83`, deadly) over the floor rows (measured).
            (Object::Pit { x, width: 1 }, Palace) => Command::new(x, 0, 0xDC),
            // World 0: the size is the number of rows cleared from the top
            // (measured; 0 is 16 and runs into the next page's level RAM),
            // so one full column is size 13.
            (Object::Pit { x, width: 1 }, Other) => Command::new(x, ROW_EXTENDED, 0x1D),
            (Object::Lava { x, width }, _) => Command::new(x, ROW_EXTENDED, sized(2, width)?),
            (Object::Elevator { x }, _) => Command::new(x, ROW_EXTENDED, 0x50),
            (Object::LockedDoor { x, y }, Palace | Gp) => Command::new(x, row(y)?, 5),
            (Object::Statue { x, y }, Palace) => Command::new(x, row(y)?, 9),
            (Object::Item { x, y, item }, _) => Command::collectable(x, row(y)?, item),
            (Object::Raw(c), _) => {
                if c.y == 14 || (c.item.is_some() != (c.b == COLLECTABLE && c.y != ROW_FLOOR)) {
                    return Err(ContentError::Encode(format!(
                        "raw command {c:?} is malformed"
                    )));
                }
                c
            }
            (o, s) => return Err(unsupported(&o, s)),
        };
        Ok(c)
    }

    /// The object a decoded command stands for in `set` (anything without a
    /// name comes back as [`Object::Raw`]). Inverse of
    /// [`Object::to_command`].
    #[must_use]
    pub fn from_command(c: &Command, set: ObjectSet) -> Object {
        use ObjectSet::{GreatPalace as Gp, Other, Palace};
        let (x, y, n) = (c.x, c.y, c.size() + 1);
        if c.is_floor() {
            return Object::Floor {
                x,
                pattern: c.b & 0x0F,
                no_ceiling: c.b & 0x80 != 0,
            };
        }
        if let Some(item) = c.item {
            return Object::Item { x, y, item };
        }
        if c.is_extended() {
            return match (c.kind(), set) {
                (1, Other) if c.b == 0x1D => Object::Pit { x, width: 1 },
                (2, _) => Object::Lava { x, width: n },
                (5, _) if c.b == 0x50 => Object::Elevator { x },
                _ => Object::Raw(*c),
            };
        }
        if c.is_small() {
            return match (c.b, set) {
                (5, Palace | Gp) => Object::LockedDoor { x, y },
                (9, Palace) => Object::Statue { x, y },
                _ => Object::Raw(*c),
            };
        }
        match (c.kind(), set) {
            (2, Palace) | (1, Gp | Other) => Object::Platform { x, y, len: n },
            (7, Palace) | (9, Gp) | (2, Other) => Object::Block { x, y, len: n },
            (3, Palace | Gp) => Object::Breakable { x, y, len: n },
            (9, Palace) | (0x0C, Gp) => Object::BreakableBlock { x, y, len: n },
            (5, Palace) | (7, Gp) => Object::Crumble { x, y, len: n },
            (4, Palace | Gp) => Object::Bridge { x, y, len: n },
            (0x0A, Palace | Gp) => Object::FalseWall { x, y, len: n },
            (0x0C, Palace) | (0x0F, Gp) => Object::Column { x, y, height: n },
            (0x0D, Palace) if y == 0 && n == 13 => Object::Pit { x, width: 1 },
            _ => Object::Raw(*c),
        }
    }
}

/// One enemy (or town person, or other list object) placed in a room.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EnemySpawn {
    /// Column 0-63.
    pub x: u8,
    /// Row: 1 (ceiling) or 3-17 (see [`crate::enemies::Enemy`]).
    pub y: u8,
    /// Enemy ID 0-63 in the world's enemy group.
    pub id: u8,
}

/// Which way an exit leaves the room.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Dir {
    /// Walk off the left edge.
    Left,
    /// Walk off the right edge.
    Right,
    /// Ride the elevator up.
    Up,
    /// Ride the elevator down.
    Down,
    /// Fall out of the bottom of page `page` (0-3), through a pit.
    Drop {
        /// Page.
        page: u8,
    },
    /// Walk into a doorway at column `x` (town houses; engine-handled, not a
    /// vanilla connection byte).
    Door {
        /// Column of the doorway.
        x: u8,
    },
}

/// Where an exit leads.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Target {
    /// Another room of the same area.
    Room(RoomId),
    /// A vanilla map of the same world: `map` (0-62), arriving on `page`.
    Map {
        /// Map number.
        map: u8,
        /// Arrival page.
        page: u8,
    },
    /// Back out to the overworld, at the location Link came in by.
    Outside,
    /// Back out to the overworld at the location slot `n` (0-3) after the
    /// one Link came in by (the game adds `n` to `$0748`): the other mouth
    /// of a multi-mouth cave, whose entrances sit on consecutive slots.
    OutsideAt(u8),
}

/// One way out of a room.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Exit {
    /// Direction.
    pub dir: Dir,
    /// Destination.
    pub target: Target,
    /// Abilities needed to get between this exit and the rest of the room,
    /// on top of [`Room::requires`]. The movement check applies it both
    /// ways: reaching this exit, and leaving the side it is on when Link
    /// came in through it (so a locked door in front of the right exit is
    /// `KEY` on that exit only).
    pub requires: Abilities,
}

impl Exit {
    /// An exit that needs nothing beyond the room's own requirement.
    #[must_use]
    pub const fn new(dir: Dir, target: Target) -> Exit {
        Exit {
            dir,
            target,
            requires: Abilities::NONE,
        }
    }

    /// The same exit, also needing `a`.
    #[must_use]
    pub const fn needs(self, a: Abilities) -> Exit {
        Exit {
            requires: self.requires.with(a),
            ..self
        }
    }
}

/// One side-view room.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Room {
    /// Id, unique in the area.
    pub id: RoomId,
    /// Width in pages (1-4).
    pub pages: u8,
    /// Alternate object tiles (header byte 1 bit 7).
    pub alt_objects: bool,
    /// Starting floor/ceiling pattern (0-15).
    pub floor: u8,
    /// No ceiling at the start.
    pub no_ceiling: bool,
    /// Ground type (0-7).
    pub ground: u8,
    /// Palette selection (header byte 3 bits 7-3, 0-31).
    pub palette: u8,
    /// Shared background layout (0 = none, 1-7).
    pub background: u8,
    /// Objects in drawing order (later ones draw over earlier ones).
    pub objects: Vec<Object>,
    /// Enemies, in list order.
    pub enemies: Vec<EnemySpawn>,
    /// Ways out.
    pub exits: Vec<Exit>,
    /// Abilities needed to cross the room at all.
    pub requires: Abilities,
    /// Abilities needed to reach the room's items from its entries, on top
    /// of [`Room::requires`].
    pub item_requires: Abilities,
    /// An extra page to check a fall into (the movement check). A fall
    /// keeps the page Link fell on (measured), so the room below is entered
    /// on each `Dir::Drop` page of the rooms above whatever this says.
    pub landing: Option<u8>,
}

impl Room {
    /// A blank room: `pages` wide, floor pattern `floor`, nothing in it.
    #[must_use]
    pub const fn new(id: RoomId, pages: u8, floor: u8) -> Room {
        Room {
            id,
            pages,
            alt_objects: false,
            floor,
            no_ceiling: false,
            ground: 0,
            palette: 1,
            background: 0,
            objects: Vec::new(),
            enemies: Vec::new(),
            exits: Vec::new(),
            requires: Abilities::NONE,
            item_requires: Abilities::NONE,
            landing: None,
        }
    }

    /// Width in columns.
    #[must_use]
    pub const fn width(&self) -> usize {
        self.pages as usize * crate::sideview::PAGE_COLS
    }

    /// Page of the elevator, if the room has one.
    #[must_use]
    pub fn elevator_page(&self) -> Option<u8> {
        self.objects.iter().find_map(|o| match o {
            Object::Elevator { x } => Some(x / 16),
            _ => None,
        })
    }

    /// The first collectable's item byte.
    #[must_use]
    pub fn item(&self) -> Option<u8> {
        self.objects.iter().find_map(|o| match o {
            Object::Item { item, .. } => Some(*item),
            Object::Raw(c) => c.item,
            _ => None,
        })
    }
}

/// Overworld region an entrance sits in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Region {
    /// West Hyrule.
    West,
    /// Death Mountain.
    DeathMountain,
    /// East Hyrule.
    East,
    /// Maze Island.
    MazeIsland,
}

impl Region {
    /// The `$0706` value.
    #[must_use]
    pub const fn id(self) -> u8 {
        match self {
            Region::West => 0,
            Region::DeathMountain => 1,
            Region::East => 2,
            Region::MazeIsland => 3,
        }
    }
}

/// What the entrance tile looks like on the overworld (the engine picks
/// the tile byte).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TileKind {
    /// Cave mouth.
    Cave = 0,
    /// Town.
    Town = 1,
    /// Palace.
    Palace = 2,
    /// Something hidden under a plain tile (found by walking onto it).
    Hidden = 3,
}

/// Overworld rows and columns.
pub const OVERWORLD_ROWS: u8 = 75;
/// See [`OVERWORLD_ROWS`].
pub const OVERWORLD_COLS: u8 = 64;

/// An overworld way into the area.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Entrance {
    /// Region.
    pub region: Region,
    /// Tile row (0-74).
    pub row: u8,
    /// Tile column (0-63).
    pub col: u8,
    /// Tile look.
    pub tile: TileKind,
    /// Room Link enters.
    pub room: RoomId,
    /// Page he appears on.
    pub page: u8,
    /// Abilities needed to reach the tile on the overworld.
    pub requires: Abilities,
}

/// Overworld terrain: the game's 16 map tile codes (the low nibble of an
/// overworld map run byte).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MapTile {
    /// Town (`$0`).
    Town = 0x0,
    /// Cave mouth (`$1`).
    Cave = 0x1,
    /// Palace (`$2`).
    Palace = 0x2,
    /// Bridge (`$3`).
    Bridge = 0x3,
    /// Desert (`$4`).
    Desert = 0x4,
    /// Grass (`$5`).
    Grass = 0x5,
    /// Forest (`$6`).
    Forest = 0x6,
    /// Swamp (`$7`, half speed).
    Swamp = 0x7,
    /// Graveyard (`$8`).
    Graveyard = 0x8,
    /// Road (`$9`).
    Road = 0x9,
    /// Lava (`$A`).
    Lava = 0xA,
    /// Mountain (`$B`, blocks).
    Mountain = 0xB,
    /// Deep water (`$C`, blocks).
    Water = 0xC,
    /// Shallow water (`$D`): walkable with the boots.
    ShallowWater = 0xD,
    /// Rock (`$E`, blocks; the hammer breaks it).
    Rock = 0xE,
    /// Spider (`$F`, blocks; the flute clears it).
    Spider = 0xF,
}

impl MapTile {
    /// The terrain code.
    #[must_use]
    pub const fn code(self) -> u8 {
        self as u8
    }
}

/// One overworld cell an area repaints while its option is on (a new
/// island, a path, shallows): row 0-74 (0-59 on Death Mountain and Maze
/// Island), column 0-63.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TerrainEdit {
    /// Region.
    pub region: Region,
    /// Tile row.
    pub row: u8,
    /// Tile column.
    pub col: u8,
    /// New terrain.
    pub tile: MapTile,
}

impl TerrainEdit {
    /// Every cell of rows `rows` and columns `cols` set to `tile`.
    #[must_use]
    pub fn rect(
        region: Region,
        rows: std::ops::RangeInclusive<u8>,
        cols: std::ops::RangeInclusive<u8>,
        tile: MapTile,
    ) -> Vec<TerrainEdit> {
        rows.flat_map(|row| {
            cols.clone().map(move |col| TerrainEdit {
                region,
                row,
                col,
                tile,
            })
        })
        .collect()
    }
}

/// Map rows of `region` (West and East 75, Death Mountain and Maze Island
/// 60).
#[must_use]
pub const fn region_rows(region: Region) -> u8 {
    match region {
        Region::West | Region::East => OVERWORLD_ROWS,
        Region::DeathMountain | Region::MazeIsland => 60,
    }
}

/// Dialog text: at most [`crate::text::MAX_LINES`] lines of
/// [`crate::text::MAX_LINE_CHARS`] characters in the game's alphabet
/// (`A-Z 0-9 - . / , ! ? *` and space; `\n` breaks a line).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Message {
    /// The text.
    pub text: &'static str,
}

/// A town person who talks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TownNpc {
    /// Room they stand in.
    pub room: RoomId,
    /// Where and what they are (their enemy-list entry).
    pub spawn: EnemySpawn,
    /// What they say.
    pub message: Message,
}

/// One of the game's eight side-view table sets: the 63 maps a room's
/// connection bytes can name. The engine runs every virtual room of a set
/// in that set's one reserved map slot.
///
/// Order and indices match `z2_core::enh::content_mem::SETS`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Table {
    /// West Hyrule caves and scenes (world 0, region 0).
    West = 0,
    /// Death Mountain caves (world 0, region 1, bank 1).
    DeathMountain = 1,
    /// East Hyrule caves and scenes (world 0, region 2).
    East = 2,
    /// Maze Island (world 0, region 1, bank 2).
    MazeIsland = 3,
    /// Towns (worlds 1 and 2 share one set).
    Towns = 4,
    /// Palaces 1, 2 and 5 (world 3).
    PalaceA = 5,
    /// Palaces 3, 4 and 6 (world 4).
    PalaceB = 6,
    /// The Great Palace (world 5).
    GreatPalace = 7,
}

impl Table {
    /// Every set, in index order.
    pub const ALL: [Table; 8] = [
        Table::West,
        Table::DeathMountain,
        Table::East,
        Table::MazeIsland,
        Table::Towns,
        Table::PalaceA,
        Table::PalaceB,
        Table::GreatPalace,
    ];

    /// The set rooms of `world` use; `region` picks the continent for
    /// [`World::Overworld`] and is ignored otherwise.
    #[must_use]
    pub const fn of(world: World, region: Region) -> Table {
        match world {
            World::Overworld => match region {
                Region::West => Table::West,
                Region::DeathMountain => Table::DeathMountain,
                Region::East => Table::East,
                Region::MazeIsland => Table::MazeIsland,
            },
            World::WestTowns | World::EastTowns => Table::Towns,
            World::PalaceA => Table::PalaceA,
            World::PalaceB => Table::PalaceB,
            World::GreatPalace => Table::GreatPalace,
        }
    }

    /// Index (`0..8`).
    #[must_use]
    pub const fn index(self) -> u8 {
        self as u8
    }

    /// The set with index `i`.
    #[must_use]
    pub const fn from_index(i: u8) -> Option<Table> {
        if (i as usize) < Table::ALL.len() {
            Some(Table::ALL[i as usize])
        } else {
            None
        }
    }
}

/// Which exit of an original room a [`VanillaLink`] takes over.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LinkVia {
    /// Connection byte `0-3` of the original room (the README's
    /// connection-byte rules: 0 left or a fall on page 0, 1 elevator down
    /// or a fall on page 1, 2 elevator up or a fall on page 2, 3 a fall on
    /// page 3; right is byte `pages - 1` of that room).
    Byte(u8),
    /// The doorway on page `0-3` of an original town room (door table).
    Door(u8),
}

/// An exit of an original room that the engine sends into this area
/// instead of where it led in the original game. The original room is map
/// `map` of the area's own [`Table`]. The engine redirects the exit only
/// while the area's option is on, at the moment Link takes it; the ROM
/// image's connection tables are not changed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct VanillaLink {
    /// Original map (0-62) of the area's table.
    pub map: u8,
    /// Which of its exits.
    pub via: LinkVia,
    /// Room entered.
    pub room: RoomId,
    /// Page Link appears on (0-3).
    pub page: u8,
    /// Abilities needed to reach that exit in the original room.
    pub requires: Abilities,
}

impl VanillaLink {
    /// Connection byte `byte` of `map` leads to `room`, page `page`.
    #[must_use]
    pub const fn byte(map: u8, byte: u8, room: RoomId, page: u8) -> VanillaLink {
        VanillaLink {
            map,
            via: LinkVia::Byte(byte),
            room,
            page,
            requires: Abilities::NONE,
        }
    }

    /// The doorway on page `door_page` of town map `map` leads to `room`,
    /// page `page`.
    #[must_use]
    pub const fn door(map: u8, door_page: u8, room: RoomId, page: u8) -> VanillaLink {
        VanillaLink {
            map,
            via: LinkVia::Door(door_page),
            room,
            page,
            requires: Abilities::NONE,
        }
    }
}

/// A whole new area: rooms, the ways in and the people in it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Area {
    /// Name (for reports and the generated table).
    pub name: &'static str,
    /// World of every room.
    pub world: World,
    /// Continent of a [`World::Overworld`] area: which of the four world-0
    /// tables its rooms belong to ([`Table::of`]). Ignored for the other
    /// worlds.
    pub region: Region,
    /// Map number of the first room in `CompiledRoom::connections`: room
    /// `i` is written as map `first_map + i` there. Only tools read those
    /// bytes. The engine runs every room of a table in that table's one
    /// reserved slot and follows the typed `CompiledRoom::exits`, so any
    /// value with `first_map + rooms - 1 <= 62` works.
    pub first_map: u8,
    /// Rooms.
    pub rooms: Vec<Room>,
    /// Ways in from the overworld. The engine gives the entrances of one
    /// area in one region consecutive location slots in list order (at
    /// most four): the `n`-th of them is where [`Target::OutsideAt`]`(n)`
    /// comes out ([`Target::Outside`] is the first).
    pub entrances: Vec<Entrance>,
    /// Overworld cells repainted while the area's option is on.
    pub terrain: Vec<TerrainEdit>,
    /// Ways in from original rooms of the same table.
    pub links: Vec<VanillaLink>,
    /// Talking people.
    pub npcs: Vec<TownNpc>,
    /// The abilities the area is designed to be fully explored with.
    /// [`crate::validate::validate`] checks every room is reachable with
    /// them.
    pub needs: Abilities,
}

impl Area {
    /// The side-view table set of the area's rooms.
    #[must_use]
    pub const fn table(&self) -> Table {
        Table::of(self.world, self.region)
    }

    /// Index of room `id`.
    #[must_use]
    pub fn room_index(&self, id: RoomId) -> Option<usize> {
        self.rooms.iter().position(|r| r.id == id)
    }

    /// Room `id`.
    #[must_use]
    pub fn room(&self, id: RoomId) -> Option<&Room> {
        self.rooms.iter().find(|r| r.id == id)
    }
}
