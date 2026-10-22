//! ROM-gated checks of the Dark Isle areas (`z2_content::areas::dark_isle`):
//! the movement check on the game's real floor patterns, and the attach
//! points against the original overworld and connection tables, all read
//! from the player's ROM at test time. Nothing read here is stored.
//!
//! `Z2_ROM=/path/to/zelda2.nes cargo test -p z2-content --test dark_isle_rom`
//! (skips when `Z2_ROM` is not set).

mod common;

use z2_content::areas::dark_isle::{self, ISLE_TERRAIN};
use z2_content::areas::SLOTS;
use z2_content::sideview::FloorTable;
use z2_content::{validate_geometry, Abilities, Area};
use z2_rando::overworld::continent::Continent;
use z2_rando::overworld::loc;
use z2_rando::overworld::map::Cont;
use z2_rando::overworld::terrain::Terrain;
use z2_rando::rom::Rom;

use common::body;

fn report(name: &str, r: Result<(), Vec<z2_content::Issue>>) {
    if let Err(v) = r {
        let lines: Vec<String> = v.iter().map(ToString::to_string).collect();
        panic!("{name}:\n{}", lines.join("\n"));
    }
}

#[test]
fn dark_isle_passes_geometry_on_the_real_floor_patterns() {
    let Some(body) = body() else { return };
    let floors = FloorTable::from_body(&body).unwrap();
    for a in dark_isle::areas() {
        report(a.name, validate_geometry(&a, &floors));
    }
}

/// Taking a room's or an exit's requirement away makes the movement check
/// object: the requirements are what the geometry really needs.
#[test]
fn removing_a_required_ability_fails_geometry() {
    let Some(body) = body() else { return };
    let floors = FloorTable::from_body(&body).unwrap();
    let fails = |a: &Area| validate_geometry(a, &floors).is_err();
    let strip = |a: &mut Area, room: &'static str, b: Abilities| {
        let i = a.room_index(room).unwrap();
        let r = &mut a.rooms[i];
        r.requires = Abilities(r.requires.0 & !b.0);
        for e in &mut r.exits {
            e.requires = Abilities(e.requires.0 & !b.0);
        }
    };
    // Lava pools need the jump spell.
    let mut a = dark_isle::caves();
    strip(&mut a, "lava_run", Abilities::JUMP);
    assert!(fails(&a), "lava_run without JUMP");
    // The breakable wall needs the glove.
    let mut a = dark_isle::tower();
    strip(&mut a, "upper_lift", Abilities::GLOVE);
    assert!(fails(&a), "upper_lift without GLOVE");
    // The raised floor of the gallery needs the jump spell.
    let mut a = dark_isle::tower();
    strip(&mut a, "watch_gallery", Abilities::JUMP);
    assert!(fails(&a), "watch_gallery without JUMP");
    // The locked door needs the key (still declared on the item so the
    // area-level check is satisfied; only geometry objects).
    let mut a = dark_isle::tower();
    strip(&mut a, "bridge_hall", Abilities::KEY);
    let i = a.room_index("bridge_hall").unwrap();
    a.rooms[i].item_requires = Abilities::KEY;
    assert!(fails(&a), "bridge_hall without KEY");
    // And dropping anything from `needs` breaks the area graph.
    for a in dark_isle::areas() {
        for bit in 0..13 {
            let b = Abilities(1 << bit);
            if a.needs.has(b) {
                let mut less = a.clone();
                less.needs = Abilities(a.needs.0 & !b.0);
                assert!(fails(&less), "{} without {b:?}", a.name);
            }
        }
    }
}

#[test]
fn attach_points_are_free_in_the_original() {
    let Some(body) = body() else { return };
    let rom = Rom::from_body(&body).unwrap();
    let table = loc::read_table(&rom, Cont::East).unwrap();
    let east = Continent::vanilla(&rom, Cont::East).unwrap();
    // The slots are unused: all four bytes zero.
    let names: Vec<&str> = dark_isle::areas().iter().map(|a| a.name).collect();
    for c in SLOTS.iter().filter(|c| names.contains(&c.area)) {
        let (area, slot) = (c.area, c.slot);
        assert_eq!(
            table[usize::from(slot)].to_bytes(),
            [0; 4],
            "{area}: East slot {slot} is in use"
        );
    }
    // No original location sits on an entrance tile, and the isle the
    // engine paints covers deep water only.
    let entrances: Vec<_> = dark_isle::areas()
        .iter()
        .flat_map(|a| a.entrances.clone())
        .collect();
    for e in &entrances {
        let (r, c) = (usize::from(e.row), usize::from(e.col));
        for (i, l) in table.iter().enumerate() {
            assert_ne!(l.pos(), Some((r, c)), "slot {i} is at {r}, {c}");
        }
    }
    for &(r, c, _) in &ISLE_TERRAIN {
        assert_eq!(
            east.grid.get(usize::from(r), usize::from(c)),
            Terrain::Water,
            "isle cell {r}, {c}"
        );
    }
    // Isle-side entrances are on painted cells (or the palace below the
    // shallows); the mainland mouth is on open desert, next to the sea.
    let mainland = &entrances[0];
    assert_eq!(
        east.grid
            .get(usize::from(mainland.row), usize::from(mainland.col)),
        Terrain::Desert
    );
    for e in &entrances[1..] {
        let (r, c) = (usize::from(e.row), usize::from(e.col));
        assert!(
            ISLE_TERRAIN
                .iter()
                .any(|&(ir, ic, _)| (usize::from(ir), usize::from(ic)) == (r, c))
                || east.grid.get(r, c) == Terrain::Water,
            "{e:?}"
        );
    }
    // Deep water separates the isle from the mainland: every original cell
    // next to the isle is water.
    for &(r, c, _) in &ISLE_TERRAIN {
        for (dr, dc) in [(-1i32, 0i32), (1, 0), (0, -1), (0, 1)] {
            let (nr, nc) = (i32::from(r) + dr, i32::from(c) + dc);
            if !(0..75).contains(&nr) || !(0..64).contains(&nc) {
                continue;
            }
            let t = east.grid.get(nr as usize, nc as usize);
            assert!(
                matches!(t, Terrain::Water),
                "{nr}, {nc} next to the isle is {t:?}"
            );
        }
    }
}

/// The reserved maps the areas count on are not the target of any original
/// connection byte of their tables (East Hyrule map 28, town map 62).
#[test]
fn reserved_maps_have_no_original_connections() {
    let Some(body) = body() else { return };
    let rom = Rom::from_body(&body).unwrap();
    for (bank, table, reserved, what) in [
        (2u8, 0x871Bu16, 28u8, "East Hyrule"),
        (3, 0x871B, 62, "towns"),
    ] {
        for i in 0..63u16 * 4 {
            let off = rom.cpu_offset(bank, table + i).unwrap();
            let b = rom.read(off).unwrap();
            assert!(
                b >= 0xFC || b >> 2 != reserved,
                "{what}: map {} byte {} leads to map {reserved}",
                i / 4,
                i % 4
            );
        }
        // And no overworld location enters it.
        if what == "East Hyrule" {
            for l in loc::read_table(&rom, Cont::East).unwrap() {
                assert!(l.world != 0 || l.map != reserved || l.pos().is_none());
            }
        }
    }
}
