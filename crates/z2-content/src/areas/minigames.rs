//! `minigames`: two challenge rooms behind new town doorways, each with a
//! heart piece ([`HEART_PIECE`]) for a prize. This module holds the rooms
//! and the rules; the runtime that plays them lives in `z2-core`
//! (`enh/content_minigames.rs`) and implements the rules below.
//!
//! Both layouts are original (no ZALiA maps, no rooms of the game).
//!
//! # Rooms and attach points
//!
//! | area | world | `first_map` | host town | street map | door slot (page) | rows |
//! |---|---|---|---|---|---|---|
//! | Target Gallery | West towns (1) | 61 | Mido | 10 | 2 | 2 pages, six ledges, the prize on the floor |
//! | Pogo Tower | East towns (2) | 62 | Darunia | 16 | 1 | 4 pages, two flights climbed on bouncers |
//!
//! [`ATTACH`] has the same as data. `first_map` only numbers the room in
//! the tool-facing connection bytes; the engine runs every town-world
//! virtual room in the towns table's reserved slot (62). The host street's
//! door table (`$8817`, bank 3, 4 bytes per street map, one per page) has
//! a zero byte at the chosen slot; the ROM-gated `tests/caves_minigames_rom.rs`
//! checks that, and that the street belongs to the host town (its location
//! table entry leads into that town's street set).
//!
//! Neither area has an overworld [`crate::Entrance`]: Link comes from the
//! host town (Mido, Darunia), so the area's way in is a door
//! [`VanillaLink`] on the street (the engine redirects that doorway into
//! the room, page 0, while the option is on), and both side exits lead
//! back to the street on the door's page ([`Target::Map`]).
//!
//! # Target Gallery rules (as implemented)
//!
//! Layout (floor pattern 1, Link stands on row 9): ledges P1 (columns
//! 6-8, top row 8), P2 (10-12, row 6), P3 (14-17, row 8), P4 (14-17,
//! row 4), P5 (19-21, row 6), P6 (23-25, row 8). Town ledges are drawn
//! but are background: Link walks and falls through them (measured,
//! `tests/content_areas_e2e.rs`), so the round is played from the floor
//! (crouching stabs on row 9, jumping stabs on row 7) and the prize
//! ([`GALLERY_PRIZE`]) sits on the floor at column 13.
//!
//! * **Arming.** On entry, unless the gallery is won (persistent
//!   minigames flag 0) or its prize page is taken, the round is armed and
//!   the prize's item object is removed every frame until the win.
//! * **Start.** [`GALLERY_START_DELAY`] frames of play after entry (a
//!   dialog blip). There is no dialog box for virtual rooms yet, so
//!   [`GALLERY_INTRO`], [`GALLERY_WIN`] and [`GALLERY_LOSE`] are not shown;
//!   sounds mark the start, the win and the loss instead.
//! * **Targets** ([`TARGET`], a Moa) are spawned by the runtime only; the
//!   room's enemy list is empty. At most [`GALLERY_LIVE_TARGETS`] are up,
//!   each on one of [`TARGET_SPOTS`] (column, row: row 9 is hit with a
//!   crouching stab from the floor, row 7 with a jumping stab), free and
//!   not the last one used, preferring spots near Link; the choice mixes
//!   the frame counter `$12` with the number spawned, so replays, movies
//!   and netplay agree. A target stays up [`GALLERY_TARGET_LIFE`] frames
//!   (a miss), the next comes [`GALLERY_SPAWN_GAP`] frames later; at most
//!   [`GALLERY_MAX_TARGETS`] per round.
//! * **Hits.** A target counts once when the sword takes HP off it (or
//!   its `$B6,x` reaches 2); it is removed at once. Targets do no contact
//!   damage, are never killed, and give no experience or drops.
//! * **Win.** [`GALLERY_HITS_NEEDED`] hits within [`GALLERY_TIME_LIMIT`]
//!   frames: targets cleared, the win flag set, the heart piece put out.
//!   Link takes it with the normal pickup (its page bit).
//! * **Lose.** Time (or the targets) run out first: targets cleared, no
//!   prize. The round re-arms the next time Link enters; leaving mid-round
//!   abandons it.
//!
//! # Pogo Tower rules (as implemented)
//!
//! Layout (floor pattern 1, Link stands on row 9): flight 1 is a chain of
//! three bouncers at (4, 9), (6, 7) and (8, 5) up to a wall top (columns
//! 10-13, floor pattern 7, Link on row 3); past it a trough (columns 14-37)
//! with the return chain (18, 9), (16, 7), (14, 5) back up to the wall;
//! flight 2 is the chain (32, 9), (34, 7), (36, 5) up to the plateau
//! (columns 38-63, row 3) with the prize at (60, 3) and the right exit.
//! No ledge or floor in either flight is within a plain or a Jump-spell
//! jump of the next: the climb needs the downward thrust (Fairy flies up
//! as well; that is allowed).
//!
//! * **Bouncers** are the room's enemy list: [`BOUNCER`] (a Moa) at the
//!   [`BOUNCER_SPOTS`]. The runtime takes them over from the ROM's scroll
//!   spawner and keeps one on every spot near Link (so a missed flight can
//!   be retried), pinned, unhurt, never killed, harmless, and drawn even
//!   without the cross (East towns hide Moas otherwise).
//! * **Bounce.** The game's own down-stab bounce (`$057D = $FE`) lifts
//!   Link only about 5 pixels, far short of the next bouncer, so in this
//!   room the runtime replaces it with the Jump spell's take-off speed.
//! * **No timer, no fail state.** On entry nothing is shown
//!   ([`TOWER_INTRO`] waits for virtual-room dialog).
//! * **Win.** Reaching the plateau and taking the heart piece with the
//!   normal pickup. The prize is ordinary room data.
//! * Leaving: either side exit returns to Darunia's street.
//!
//! # RAM observables
//!
//! | address | use |
//! |---|---|
//! | `$0012` | frame counter (spot choice) |
//! | `$0029` / `$004D` / `$003B` | Link's Y, X, page |
//! | `$057D` | Link's Y speed (`$FE` on a down-stab bounce) |
//! | `$0479` | Link in the air |
//! | `$0796` | thrust techniques (`$10` downward) |
//! | `$002A,x` / `$004E,x` / `$003C,x` | enemy slot Y, X, page (`x` = 0-5) |
//! | `$00A1,x` | enemy slot ID |
//! | `$00B6,x` | enemy slot exists (0 no, 1 yes, 2 killed) |
//! | `$00C2,x` | enemy slot HP |
//! | `$074C` | dialog open |
//! | `$0561` / `$0707` | current map / world |
//! | `EnhState::vroom`, `vroom_items` | current virtual room; prize pages taken |
//!
//! # Engine features these rooms need
//!
//! * Door exits into virtual rooms: a street door byte (`$8817` table,
//!   read at `$CFFC`) that leads to the town-world virtual room, and the
//!   room's side exits back to the street.
//! * [`HEART_PIECE`] and its pickup (see `hidden_caves`).
//! * The gallery and tower runtimes above (`z2_core::enh::content_minigames`).

use crate::model::{Abilities, Area, Dir, EnemySpawn, Exit, Message, Object, Region};
use crate::model::{Room, Target, VanillaLink, World};
use crate::StaticArea;

#[rustfmt::skip]
#[path = "minigames_generated.rs"]
mod generated;

pub use super::hidden_caves::HEART_PIECE;

/// Where a minigame room attaches to a town.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DoorAttach {
    /// The area's name.
    pub area: &'static str,
    /// Town world.
    pub world: World,
    /// Nominal map of the room ([`Area::first_map`], a label only).
    pub map: u8,
    /// Host town.
    pub town: &'static str,
    /// Overworld region of the host town.
    pub region: Region,
    /// The host town's location-table slot in that region.
    pub town_slot: u8,
    /// Street map that gets the new doorway.
    pub street: u8,
    /// Door slot (page) on the street map.
    pub door_slot: u8,
}

/// Both rooms' attach points (see the module docs).
pub const ATTACH: &[DoorAttach] = &[
    DoorAttach {
        area: "Target Gallery",
        world: World::WestTowns,
        map: 61,
        town: "Mido",
        region: Region::West,
        town_slot: 51,
        street: 10,
        door_slot: 2,
    },
    DoorAttach {
        area: "Pogo Tower",
        world: World::EastTowns,
        map: 62,
        town: "Darunia",
        region: Region::East,
        town_slot: 47,
        street: 16,
        door_slot: 1,
    },
];

/// Palette selection for the rooms (the town houses' palette).
const HOUSE_PALETTE: u8 = 6;

// ---------------------------------------------------------------- gallery

/// Gallery: frames from the intro closing to the first target.
pub const GALLERY_START_DELAY: u16 = 60;
/// Gallery: round length in frames (30 seconds).
pub const GALLERY_TIME_LIMIT: u16 = 1800;
/// Gallery: hits needed to win.
pub const GALLERY_HITS_NEEDED: u8 = 12;
/// Gallery: most targets spawned in one round.
pub const GALLERY_MAX_TARGETS: u8 = 24;
/// Gallery: targets up at the same time.
pub const GALLERY_LIVE_TARGETS: u8 = 2;
/// Gallery: frames a target stays up before it counts as a miss.
pub const GALLERY_TARGET_LIFE: u16 = 150;
/// Gallery: frames between a target leaving and the next appearing.
pub const GALLERY_SPAWN_GAP: u16 = 16;
/// Gallery: where targets appear, as (column, row Link would stand on).
pub const TARGET_SPOTS: [(u8, u8); 7] =
    [(3, 9), (7, 7), (11, 9), (15, 7), (20, 9), (24, 7), (28, 9)];
/// Gallery: enemy ID of the targets the runtime spawns (the same Moa as
/// [`BOUNCER`], held at its spot, harmless and never killed).
pub const TARGET: u8 = 0x06;
/// Gallery: the prize's spot: on the floor between P2 and P3 (an item
/// object falls through town ledges, so it is not put on one).
pub const GALLERY_PRIZE: (u8, u8) = (13, 9);

/// Gallery: shown on entry.
pub const GALLERY_INTRO: Message = Message {
    text: "HIT 12\nTARGETS IN\n30 SECONDS!",
};
/// Gallery: shown on a win.
pub const GALLERY_WIN: Message = Message {
    text: "WELL DONE!\nTAKE YOUR\nPRIZE.",
};
/// Gallery: shown when time runs out.
pub const GALLERY_LOSE: Message = Message {
    text: "TIME IS UP.\nCOME BACK\nAND TRY\nAGAIN!",
};

/// Target Gallery (behind a new door in Mido): two pages of ledges for the
/// targets, the prize on the floor between them.
#[must_use]
pub fn target_gallery() -> Area {
    let back = Target::Map {
        map: ATTACH[0].street,
        page: ATTACH[0].door_slot,
    };
    let mut r = Room::new("gallery", 2, 1);
    r.palette = HOUSE_PALETTE;
    r.objects = vec![
        Object::Platform { x: 6, y: 8, len: 3 },
        Object::Platform {
            x: 10,
            y: 6,
            len: 3,
        },
        Object::Platform {
            x: 14,
            y: 8,
            len: 4,
        },
        Object::Platform {
            x: 14,
            y: 4,
            len: 4,
        },
        Object::Platform {
            x: 19,
            y: 6,
            len: 3,
        },
        Object::Platform {
            x: 23,
            y: 8,
            len: 3,
        },
        Object::Item {
            x: GALLERY_PRIZE.0,
            y: GALLERY_PRIZE.1,
            item: HEART_PIECE,
        },
    ];
    r.exits = vec![Exit::new(Dir::Left, back), Exit::new(Dir::Right, back)];
    Area {
        name: "Target Gallery",
        world: World::WestTowns,
        region: Region::West,
        first_map: ATTACH[0].map,
        rooms: vec![r],
        entrances: vec![],
        links: vec![VanillaLink::door(
            ATTACH[0].street,
            ATTACH[0].door_slot,
            "gallery",
            0,
        )],
        terrain: vec![],
        npcs: vec![],
        needs: Abilities::NONE,
    }
}

// ------------------------------------------------------------------ tower

/// Tower: enemy ID of the bouncers in the town enemy group: the Moa, the
/// floating eye of Old Kasuto's streets (town maps 21-23). Measured in the
/// town world: 48 HP (`$30`), a down stab bounces Link off it, and it
/// survives the hit; the runtime pins it at its spot, keeps it unhurt and
/// takes away its contact damage.
pub const BOUNCER: u8 = 0x06;
/// Tower: bouncer spots as (column, row): flight 1, the return chain,
/// flight 2.
pub const BOUNCER_SPOTS: [(u8, u8); 9] = [
    (4, 9),
    (6, 7),
    (8, 5),
    (18, 9),
    (16, 7),
    (14, 5),
    (32, 9),
    (34, 7),
    (36, 5),
];
/// Tower: the prize's spot on the plateau.
pub const TOWER_PRIZE: (u8, u8) = (60, 3);
/// Tower: shown once per visit.
pub const TOWER_INTRO: Message = Message {
    text: "STAB DOWN\nON THE ORBS\nTO CLIMB!",
};

/// Pogo Tower (behind a new door in Darunia): two flights of bouncers up
/// to a wall top and a plateau, which no jump reaches.
#[must_use]
pub fn pogo_tower() -> Area {
    let back = Target::Map {
        map: ATTACH[1].street,
        page: ATTACH[1].door_slot,
    };
    let mut r = Room::new("tower", 4, 1);
    r.palette = HOUSE_PALETTE;
    let floor = |x: u8, pattern: u8| Object::Floor {
        x,
        pattern,
        no_ceiling: false,
    };
    r.objects = vec![
        // Pattern 7: solid from row 4 down, Link stands on row 3.
        floor(10, 7),
        floor(14, 1),
        floor(38, 7),
        Object::Item {
            x: TOWER_PRIZE.0,
            y: TOWER_PRIZE.1,
            item: HEART_PIECE,
        },
    ];
    r.enemies = BOUNCER_SPOTS
        .iter()
        .map(|&(x, y)| EnemySpawn { x, y, id: BOUNCER })
        .collect();
    r.exits = vec![
        Exit::new(Dir::Left, back),
        Exit::new(Dir::Right, back).needs(Abilities::DOWNSTAB),
    ];
    r.item_requires = Abilities::DOWNSTAB;
    Area {
        name: "Pogo Tower",
        world: World::EastTowns,
        region: Region::East,
        first_map: ATTACH[1].map,
        rooms: vec![r],
        entrances: vec![],
        links: vec![VanillaLink::door(
            ATTACH[1].street,
            ATTACH[1].door_slot,
            "tower",
            0,
        )],
        terrain: vec![],
        npcs: vec![],
        needs: Abilities::DOWNSTAB,
    }
}

/// Both minigame areas.
#[must_use]
pub fn areas() -> Vec<Area> {
    vec![target_gallery(), pogo_tower()]
}

/// The group's areas in static form (what the engine registers).
#[must_use]
pub fn statics() -> &'static [StaticArea] {
    generated::AREAS
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{compile, text, validate};

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
    fn both_rooms_validate() {
        for a in areas() {
            assert_eq!(issues(&a), "", "{}", a.name);
        }
        for (a, at) in areas().iter().zip(ATTACH) {
            assert_eq!(a.name, at.area);
            assert_eq!(a.world, at.world);
            assert_eq!(a.first_map, at.map);
            assert!(a.entrances.is_empty());
            assert_eq!(a.links.len(), 1);
            assert_eq!(
                (a.links[0].map, a.links[0].via),
                (at.street, crate::LinkVia::Door(at.door_slot))
            );
        }
    }

    #[test]
    fn messages_fit_the_box() {
        for m in [GALLERY_INTRO, GALLERY_WIN, GALLERY_LOSE, TOWER_INTRO] {
            text::validate_dialog(m.text, text::MAX_LINES).unwrap();
            text::encode_message(m.text).unwrap();
        }
    }

    #[test]
    fn exits_lead_back_to_the_host_street_door() {
        for (a, at) in areas().iter().zip(ATTACH) {
            let c = compile(a).unwrap();
            let back = (at.street << 2) | at.door_slot;
            let r = &c.rooms[0];
            assert_eq!(r.connections[0], back, "{}", a.name);
            assert_eq!(r.connections[usize::from(a.rooms[0].pages) - 1], back);
        }
    }

    #[test]
    fn the_tower_needs_the_downward_thrust() {
        let mut a = pogo_tower();
        a.needs = Abilities::JUMP.with(Abilities::UPSTAB);
        let s = issues(&a);
        assert!(s.contains("item not reachable"), "{s}");
        // The left exit stays usable without it: no softlock.
        assert!(!s.contains("no way back out"), "{s}");
        let a = target_gallery();
        assert_eq!(
            a.rooms[0].enemies,
            vec![],
            "targets are spawned at run time"
        );
        assert!(a.rooms[0]
            .objects
            .iter()
            .any(|o| matches!(o, Object::Platform { x: 14, y: 4, .. })));
    }

    #[test]
    fn spots_are_inside_the_rooms() {
        let g = target_gallery();
        for (x, y) in TARGET_SPOTS {
            assert!(usize::from(x) < g.rooms[0].width() && (3..=9).contains(&y));
        }
        let t = pogo_tower();
        assert_eq!(t.rooms[0].enemies.len(), BOUNCER_SPOTS.len());
    }

    #[test]
    fn reachability_without_downstab_still_reaches_the_room() {
        let r = validate::reachable(&pogo_tower(), Abilities::NONE);
        assert_eq!(
            (r.rooms, r.items, r.escapes),
            (vec![true], vec![false], vec![true])
        );
    }
}
