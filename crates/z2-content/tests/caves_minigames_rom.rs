//! ROM-gated checks for the hidden caves and the minigames
//! (`z2_content::areas::{hidden_caves, minigames}`): the layout details
//! their rewards depend on, and the minigames' town doors in the player's
//! ROM. Geometry of every area and the overworld slots are checked for all
//! options at once in `areas_rom.rs`. Nothing read here is stored.
//!
//! `Z2_ROM=/path/to/zelda2.nes cargo test -p z2-content --test caves_minigames_rom`
//! (skips when `Z2_ROM` is not set).

mod common;

use z2_content::areas::{hidden_caves, minigames};
use z2_content::sideview::FloorTable;
use z2_content::{validate_geometry, Abilities, LinkVia, Region};
use z2_rando::overworld::{loc, map::Cont};
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
fn the_tower_cannot_be_climbed_without_the_downward_thrust() {
    let Some(body) = body() else { return };
    let floors = FloorTable::from_body(&body).unwrap();
    // Declared with the jump spell instead: the plateau is out of reach.
    let mut a = minigames::pogo_tower();
    let jump = Abilities::JUMP;
    a.rooms[0].exits[1].requires = jump;
    a.rooms[0].item_requires = jump;
    a.needs = jump;
    let s = issues(validate_geometry(&a, &floors));
    assert!(s.contains("Right exit not reachable"), "{s}");
    assert!(s.contains("items not reachable"), "{s}");
    // With the thrust but nothing to bounce on, neither.
    let mut a = minigames::pogo_tower();
    a.rooms[0].enemies.clear();
    assert!(validate_geometry(&a, &floors).is_err());
    // And each flight needs its whole chain: drop the middle bouncer of
    // flight 2.
    let mut a = minigames::pogo_tower();
    a.rooms[0].enemies.retain(|e| (e.x, e.y) != (34, 7));
    let s = issues(validate_geometry(&a, &floors));
    assert!(s.contains("Right exit not reachable"), "{s}");
}

#[test]
fn cave_rewards_sit_where_the_layout_says() {
    let Some(body) = body() else { return };
    let floors = FloorTable::from_body(&body).unwrap();
    // Without its block stair the Lakeside heart piece is out of reach.
    let mut a = hidden_caves::lakeside_hollow();
    a.rooms[0]
        .objects
        .retain(|o| !matches!(o, z2_content::Object::Block { x: 13, .. }));
    let s = issues(validate_geometry(&a, &floors));
    assert!(s.contains("items not reachable"), "{s}");
    // Without its ledges the Ember Niche's lava cannot be crossed.
    let mut a = hidden_caves::ember_niche();
    a.rooms[0]
        .objects
        .retain(|o| !matches!(o, z2_content::Object::Block { x: 14, .. }));
    let s = issues(validate_geometry(&a, &floors));
    assert!(s.contains("Right exit not reachable"), "{s}");
}

fn cont(r: Region) -> Cont {
    match r {
        Region::West => Cont::West,
        Region::DeathMountain => Cont::DeathMountain,
        Region::East => Cont::East,
        Region::MazeIsland => Cont::Maze,
    }
}

/// Town tables (bank 3).
const TOWN_BANK: u8 = 3;
const TOWN_CONN: u16 = 0x871B;
const TOWN_DOORS: u16 = 0x8817;

#[test]
fn minigame_doors_are_free() {
    let Some(body) = body() else { return };
    let rom = Rom::from_body(&body).unwrap();
    let games = minigames::areas();
    let read = |addr: u16| rom.read_cpu(TOWN_BANK, addr).unwrap();
    for at in minigames::ATTACH {
        let a = games.iter().find(|a| a.name == at.area).unwrap();
        assert!((1..=24).contains(&at.street), "{}", at.area);
        // The doorway slot is unused.
        let door = read(TOWN_DOORS + 4 * u16::from(at.street) + u16::from(at.door_slot));
        assert_eq!(door, 0, "{}: door slot already used", at.area);
        // The area's way in is that doorway, and the street is in the
        // street set the host town's location leads into.
        assert_eq!(
            (a.links[0].map, a.links[0].via),
            (at.street, LinkVia::Door(at.door_slot))
        );
        let c = cont(at.region);
        let info = loc::info(c, usize::from(at.town_slot)).unwrap();
        assert!(info.name.starts_with(at.town), "{}: {}", at.area, info.name);
        let l = loc::read_table(&rom, c).unwrap()[usize::from(at.town_slot)];
        let mut set = vec![l.map];
        let mut grow = true;
        while grow {
            grow = false;
            for m in set.clone() {
                for i in [0u16, 3] {
                    let v = read(TOWN_CONN + 4 * u16::from(m) + i);
                    if v < 0xFC && (1..=24).contains(&(v >> 2)) && !set.contains(&(v >> 2)) {
                        set.push(v >> 2);
                        grow = true;
                    }
                }
            }
        }
        assert!(set.contains(&at.street), "{}: streets {set:?}", at.area);
    }
}
