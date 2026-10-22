//! "Parapa Annex": an optional wing of Palace 1 (Parapa, world 3, palace
//! group A), inspired by ZALiA's palace expansions. Original layout.
//!
//! Seven rooms. The wing opens off the right end of P1's entrance room,
//! crosses an open-air terrace, drops into a cistern, and hides its key in a
//! false-brick pocket that only shows once the player has climbed over the
//! tower that covers it. The key opens the stair to a heart container. An
//! optional branch with a 4-row ledge (jump spell) holds a P-bag.
//!
//! | room | pages | what is in it |
//! |---|---|---|
//! | `gatehouse` | 4 | In from P1 (left edge); a low wall to hop; an elevator down to the cistern; right to the terrace. |
//! | `terrace` | 4 | Open sky, ruined pillars. Two pits spanned by bridges and crumbling spans with one-column gaps; falling lands in the cistern. Right edge: the outdoor back door. |
//! | `cistern` | 4 | Landing for terrace falls; two one-column water channels; the elevator back up; left to the lantern hall, right to the sluice. |
//! | `lantern_hall` | 2 | A brick tower with two ledges up each side; the small key in a pocket inside its foot, behind false bricks on the side away from the cistern; a locked door before the left exit. |
//! | `vault_stair` | 2 | Behind the locked door; an elevator up. |
//! | `reliquary` | 1 | Top of the elevator; the heart container on a pedestal. |
//! | `sluice` | 2 | Optional: a 500 P-bag on a ledge four rows up (jump spell). |
//!
//! # Attach points
//!
//! | how | vanilla side | authored side |
//! |---|---|---|
//! | vanilla link ([`Area::links`]) | world 3 (palaces 1/2/5), map 0 (P1 entrance room), connection byte 3 (right edge, page 3; vanilla leads outside) | `gatehouse` page 0 |
//! | return | `gatehouse` left edge | world 3 map 0, page 3 (`Target::Map`) |
//! | overworld back door ([`crate::areas::SLOTS`]) | West Hyrule slot 55, row 4 col 60 (desert two tiles from P1's tile) | `terrace` page 3 (right edge), and out again the same way |
//!
//! `first_map` 52 only numbers the rooms (52-58) in the tool-facing
//! connection bytes: palace group A has no spare map, and the engine runs
//! every virtual room of the group in its map-63 slot.
//!
//! # Abilities and rewards
//!
//! `needs = KEY | JUMP`: the key is found inside the annex, and the jump
//! spell is only for the optional sluice ledge. Nothing else is needed, so
//! a player who has just reached Palace 1 can clear everything but the
//! sluice. Items (one "taken" bit each in the content engine's per-room
//! item state): small key (`lantern_hall` page 1; its locked door has page
//! 0's bit), heart container (`reliquary` page 0), 500 P-bag (`sluice` page
//! 1).
//!
//! Enemies are palace 1/2/5 IDs that palace 1 itself uses (Bot, slow and
//! fast bubbles, Stalfos, orange Iron Knuckle).

use crate::model::{
    item, Abilities, Area, Dir, EnemySpawn, Entrance, Exit, Object, Region, Room, Target, TileKind,
    VanillaLink, World,
};
use crate::StaticArea;

#[rustfmt::skip]
#[path = "parapa_annex_generated.rs"]
mod generated;

/// Area name.
pub const NAME: &str = "Parapa Annex";

/// Palace 1/2/5 enemy IDs used here.
mod ids {
    /// Bot.
    pub const BOT: u8 = 0x04;
    /// Slow bubble.
    pub const BUBBLE: u8 = 0x06;
    /// Fast bubble.
    pub const FAST_BUBBLE: u8 = 0x0E;
    /// Orange Iron Knuckle.
    pub const IRON_KNUCKLE: u8 = 0x18;
    /// Stalfos.
    pub const STALFOS: u8 = 0x1F;
}

/// Palace 1's entrance room (palace group A map 0), whose right edge
/// (connection byte 3, which leads outside in the original) is re-pointed
/// into the gatehouse.
pub const P1_ENTRANCE_MAP: u8 = 0;

/// Floor pattern used throughout: ceiling on row 0, floor from row 11
/// (Link stands on row 10, as in P1's own rooms, so side exits line up).
const FLOOR: u8 = 0;
/// Standing row on [`FLOOR`].
const STAND: u8 = 10;
/// Row of an enemy standing on [`FLOOR`]: the list row above Link's
/// standing row (measured: a list entry on row 10 never spawns; one on row
/// 9 spawns and drops onto the floor).
const ENEMY_ROW: u8 = 9;

fn e(x: u8, y: u8, id: u8) -> EnemySpawn {
    EnemySpawn { x, y, id }
}

fn column(x: u8) -> Object {
    Object::Column {
        x,
        y: 1,
        height: 10,
    }
}

fn gatehouse() -> Room {
    let mut r = Room::new("gatehouse", 4, FLOOR);
    r.objects = vec![
        column(5),
        Object::Block {
            x: 14,
            y: 9,
            len: 2,
        },
        column(22),
        Object::Platform {
            x: 26,
            y: 7,
            len: 4,
        },
        Object::Elevator { x: 39 },
        column(46),
        Object::Block {
            x: 52,
            y: 9,
            len: 1,
        },
        Object::Block {
            x: 53,
            y: 7,
            len: 1,
        },
        Object::Block {
            x: 53,
            y: 9,
            len: 1,
        },
        Object::Block {
            x: 54,
            y: 9,
            len: 1,
        },
        column(60),
    ];
    r.enemies = vec![
        e(20, ENEMY_ROW, ids::BOT),
        e(30, ENEMY_ROW, ids::BOT),
        e(35, 4, ids::BUBBLE),
        e(57, ENEMY_ROW, ids::STALFOS),
    ];
    r.exits = vec![
        Exit::new(
            Dir::Left,
            Target::Map {
                map: P1_ENTRANCE_MAP,
                page: 3,
            },
        ),
        Exit::new(Dir::Right, Target::Room("terrace")),
        Exit::new(Dir::Down, Target::Room("cistern")),
    ];
    r
}

fn terrace() -> Room {
    let mut r = Room::new("terrace", 4, FLOOR);
    r.no_ceiling = true;
    r.objects = vec![
        Object::Column {
            x: 4,
            y: 4,
            height: 7,
        },
        Object::Column {
            x: 11,
            y: 6,
            height: 5,
        },
        // Page 1: a bridge, a crumbling span, a bridge; one-column gaps.
        Object::Pit { x: 18, width: 12 },
        Object::Bridge {
            x: 18,
            y: 11,
            len: 3,
        },
        Object::Crumble {
            x: 22,
            y: 11,
            len: 3,
        },
        Object::Bridge {
            x: 26,
            y: 11,
            len: 4,
        },
        // Page 2: two crumbling spans.
        Object::Pit { x: 36, width: 7 },
        Object::Crumble {
            x: 37,
            y: 11,
            len: 2,
        },
        Object::Crumble {
            x: 40,
            y: 11,
            len: 2,
        },
        // Page 3: a broken parapet to stand on.
        Object::Column {
            x: 48,
            y: 3,
            height: 8,
        },
        Object::Platform {
            x: 51,
            y: 8,
            len: 4,
        },
        Object::Column {
            x: 58,
            y: 5,
            height: 6,
        },
    ];
    r.enemies = vec![
        e(9, ENEMY_ROW, ids::BOT),
        e(24, 3, ids::FAST_BUBBLE),
        e(39, 3, ids::FAST_BUBBLE),
        e(46, ENEMY_ROW, ids::BOT),
        e(56, ENEMY_ROW, ids::BOT),
    ];
    r.exits = vec![
        Exit::new(Dir::Left, Target::Room("gatehouse")),
        Exit::new(Dir::Right, Target::Outside),
        Exit::new(Dir::Drop { page: 1 }, Target::Room("cistern")),
        Exit::new(Dir::Drop { page: 2 }, Target::Room("cistern")),
    ];
    r
}

fn cistern() -> Room {
    let mut r = Room::new("cistern", 4, FLOOR);
    r.objects = vec![
        Object::Block { x: 8, y: 9, len: 2 },
        Object::Lava { x: 18, width: 1 },
        Object::Platform {
            x: 28,
            y: 8,
            len: 3,
        },
        Object::Elevator { x: 39 },
        Object::Lava { x: 49, width: 1 },
        Object::Block {
            x: 56,
            y: 9,
            len: 2,
        },
    ];
    r.enemies = vec![
        e(12, ENEMY_ROW, ids::BOT),
        e(23, ENEMY_ROW, ids::STALFOS),
        e(33, 5, ids::BUBBLE),
        e(53, ENEMY_ROW, ids::BOT),
    ];
    r.exits = vec![
        Exit::new(Dir::Left, Target::Room("lantern_hall")),
        Exit::new(Dir::Up, Target::Room("gatehouse")),
        Exit::new(Dir::Right, Target::Room("sluice")),
    ];
    r
}

fn lantern_hall() -> Room {
    let mut r = Room::new("lantern_hall", 2, FLOOR);
    r.objects = vec![
        Object::LockedDoor { x: 3, y: 8 },
        column(6),
        // Two ledges up the tower's left side.
        Object::Platform { x: 7, y: 9, len: 2 },
        Object::Platform {
            x: 10,
            y: 7,
            len: 2,
        },
        // The tower (columns 12-19): solid on top, a false-brick face on
        // its left at the bottom with the key's pocket behind it.
        Object::Block {
            x: 12,
            y: 5,
            len: 8,
        },
        Object::Block {
            x: 12,
            y: 7,
            len: 8,
        },
        Object::FalseWall {
            x: 12,
            y: 9,
            len: 2,
        },
        Object::Item {
            // Page 1: a page has one persistent bit, and the locked door
            // is on page 0 (sharing it, taking the key opened the door for
            // the next visit and opening the door took the key).
            x: 17,
            y: STAND,
            item: item::SMALL_KEY,
        },
        Object::Block {
            x: 18,
            y: 9,
            len: 2,
        },
        // Two ledges up the tower's right side.
        Object::Platform {
            x: 21,
            y: 7,
            len: 2,
        },
        Object::Platform {
            x: 24,
            y: 9,
            len: 2,
        },
        column(28),
    ];
    r.enemies = vec![
        e(5, ENEMY_ROW, ids::BOT),
        e(16, 4, ids::STALFOS),
        e(27, ENEMY_ROW, ids::BOT),
    ];
    r.exits = vec![
        Exit::new(Dir::Left, Target::Room("vault_stair")).needs(Abilities::KEY),
        Exit::new(Dir::Right, Target::Room("cistern")),
    ];
    r
}

fn vault_stair() -> Room {
    let mut r = Room::new("vault_stair", 2, FLOOR);
    r.objects = vec![
        Object::Elevator { x: 6 },
        Object::Block {
            x: 12,
            y: 9,
            len: 1,
        },
        column(18),
        Object::Block {
            x: 24,
            y: 9,
            len: 2,
        },
    ];
    r.enemies = vec![
        e(15, ENEMY_ROW, ids::IRON_KNUCKLE),
        e(28, ENEMY_ROW, ids::BOT),
    ];
    r.exits = vec![
        Exit::new(Dir::Right, Target::Room("lantern_hall")),
        Exit::new(Dir::Up, Target::Room("reliquary")),
    ];
    r
}

fn reliquary() -> Room {
    let mut r = Room::new("reliquary", 1, FLOOR);
    r.objects = vec![
        Object::Elevator { x: 6 },
        Object::Block {
            x: 11,
            y: 9,
            len: 3,
        },
        Object::Item {
            x: 12,
            y: 8,
            item: item::HEART_CONTAINER,
        },
    ];
    r.exits = vec![Exit::new(Dir::Down, Target::Room("vault_stair"))];
    r
}

fn sluice() -> Room {
    let mut r = Room::new("sluice", 2, FLOOR);
    r.objects = vec![
        Object::Block { x: 9, y: 9, len: 2 },
        // Four rows above the floor: the jump spell's reach, not a plain
        // jump's.
        Object::Platform {
            x: 20,
            y: 7,
            len: 6,
        },
        Object::Item {
            x: 23,
            y: 6,
            item: item::BAG_500,
        },
        column(29),
    ];
    r.item_requires = Abilities::JUMP;
    r.enemies = vec![e(14, ENEMY_ROW, ids::BOT), e(24, 3, ids::BUBBLE)];
    r.exits = vec![Exit::new(Dir::Left, Target::Room("cistern"))];
    r
}

/// The annex.
#[must_use]
pub fn parapa_annex() -> Area {
    Area {
        name: NAME,
        world: World::PalaceA,
        region: Region::West,
        first_map: 52,
        rooms: vec![
            gatehouse(),
            terrace(),
            cistern(),
            lantern_hall(),
            vault_stair(),
            reliquary(),
            sluice(),
        ],
        entrances: vec![Entrance {
            region: Region::West,
            row: 4,
            col: 60,
            tile: TileKind::Cave,
            room: "terrace",
            page: 3,
            requires: Abilities::NONE,
        }],
        links: vec![VanillaLink::byte(P1_ENTRANCE_MAP, 3, "gatehouse", 0)],
        terrain: vec![],
        npcs: vec![],
        needs: Abilities::KEY.with(Abilities::JUMP),
    }
}

/// Every area of this module.
#[must_use]
pub fn areas() -> Vec<Area> {
    vec![parapa_annex()]
}

/// The group's areas in static form (what the engine registers).
#[must_use]
pub fn statics() -> &'static [StaticArea] {
    generated::AREAS
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::validate::reachable;

    /// Without the jump spell everything but the sluice's P-bag is still
    /// there to take, and both ways out work.
    #[test]
    fn only_the_sluice_needs_the_jump_spell() {
        let a = parapa_annex();
        let r = reachable(&a, Abilities::KEY);
        assert!(r.rooms.iter().all(|&x| x));
        assert!(r.escapes.iter().all(|&x| x));
        for (i, room) in a.rooms.iter().enumerate() {
            if room.item().is_some() {
                assert_eq!(r.items[i], room.id != "sluice", "{}", room.id);
            }
        }
        // Without the key the stair and the heart stay shut.
        let r = reachable(&a, Abilities::JUMP);
        let shut = ["vault_stair", "reliquary"];
        for (i, room) in a.rooms.iter().enumerate() {
            assert_eq!(r.rooms[i], !shut.contains(&room.id), "{}", room.id);
        }
    }
}
