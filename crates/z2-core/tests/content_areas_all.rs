//! Every authored area of every `content` option at once
//! (`z2_content::areas::GROUPS`, as `z2_core::enh::content_areas`
//! registers them): one test for the invariants no single option can check
//! alone.
//!
//! * Every area validates, and with `Z2_ROM` set every authored area (all
//!   but the developer rooms) passes the movement check on the ROM's real
//!   floor/ceiling patterns.
//! * The registered statics are what the model areas compile to, and
//!   `content_areas::areas(opts)` turns on each option's areas.
//! * Overworld slots (`z2_content::areas::SLOTS`): one per entrance, unique
//!   per continent (Death Mountain and Maze Island as one layout), each on
//!   its continent's free list (`content_mem::*_FREE_SLOTS`); entrance
//!   tiles unique.
//! * Vanilla links: no original exit is redirected by two areas.
//! * Persistent bits: every option's tracked pages plus flags fit its range
//!   of `content_areas::GROUPS`, and the ranges tile the pool.
//! * Container pieces: four heart pieces and four magic pieces, one extra
//!   container of each.
//!
//! The ROM is read in place for the floor table only; nothing is stored.

mod common;

use z2_content::areas::{self, shared_layout, SLOTS};
use z2_content::sideview::FloorTable;
use z2_content::{compile, validate, validate_geometry, Area, Object, Region};
use z2_core::enh::content::{ContentOpts, HEART_PIECE, MAGIC_PIECE};
use z2_core::enh::content_areas::{self as reg, Group};
use z2_core::enh::content_mem::{
    DEATH_MOUNTAIN_FREE_SLOTS, EAST_FREE_SLOTS, MAZE_ISLAND_FREE_SLOTS, PROGRESS_POOL_BITS,
    WEST_FREE_SLOTS,
};
use z2_core::enh::Enhancements;

fn report(r: Result<(), Vec<z2_content::Issue>>) -> String {
    match r {
        Ok(()) => String::new(),
        Err(v) => v
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join("\n"),
    }
}

fn free_slots(r: Region) -> &'static [u8] {
    match r {
        Region::West => WEST_FREE_SLOTS,
        Region::East => EAST_FREE_SLOTS,
        Region::DeathMountain => DEATH_MOUNTAIN_FREE_SLOTS,
        Region::MazeIsland => MAZE_ISLAND_FREE_SLOTS,
    }
}

fn group_of(name: &str) -> Group {
    reg::GROUPS
        .iter()
        .find(|g| g.name == name)
        .unwrap_or_else(|| panic!("no registry group {name}"))
        .group
}

/// The floor table from `$Z2_ROM` (headered or not), if set.
fn floors() -> Option<FloorTable> {
    let rom = common::rom_bytes("every_area_of_every_option")?;
    let body = if rom.starts_with(b"NES\x1A") {
        &rom[16..]
    } else {
        &rom[..]
    };
    Some(FloorTable::from_body(body).expect("ROM too short for the floor table"))
}

#[test]
fn every_area_of_every_option() {
    // ---- registry and statics
    reg::validate().unwrap();
    z2_core::enh::content::validate().unwrap();
    assert_eq!(areas::GROUPS.len(), reg::GROUPS.len());
    let mut all: Vec<(&str, Area)> = Vec::new();
    for g in areas::GROUPS {
        let model = (g.areas)();
        let statics = reg::group_areas(group_of(g.name));
        assert_eq!(model.len(), statics.len(), "{}", g.name);
        assert!(!model.is_empty(), "{} has no areas", g.name);
        for (a, s) in model.iter().zip(statics) {
            assert_eq!(report(validate(a)), "", "{}::{}", g.name, a.name);
            assert!(
                compile(a).unwrap().matches(s),
                "{}::{} is stale",
                g.name,
                a.name
            );
            assert_eq!(reg::find_room(a.name, 0), reg::room_id(s, 0), "{}", a.name);
        }
        all.extend(model.into_iter().map(|a| (g.name, a)));
    }

    // ---- options turn on their areas
    let every = ContentOpts {
        virtual_rooms: true,
        test_room: true,
        parapa_annex: true,
        challenge_rooms: true,
        minigames: true,
        hidden_caves: true,
        dark_isle: true,
    };
    assert_eq!(reg::areas(&every).count(), all.len());
    assert_eq!(reg::areas(&ContentOpts::default()).count(), 0);
    let zalia = Enhancements::zalia_preset().content;
    let on: Vec<&str> = reg::areas(&zalia).map(|a| a.name).collect();
    for (g, a) in &all {
        let want = !matches!(*g, "test_room" | "dark_isle");
        assert_eq!(on.contains(&a.name), want, "ZALiA preset: {g}::{}", a.name);
    }

    // ---- overworld slots and tiles
    let authored: Vec<&(&str, Area)> = all.iter().filter(|(g, _)| *g != "test_room").collect();
    let mut tiles = Vec::new();
    for (g, a) in &authored {
        for (i, e) in a.entrances.iter().enumerate() {
            let c = areas::slot_of(a.name, i)
                .unwrap_or_else(|| panic!("{g}::{} entrance {i} has no slot", a.name));
            assert_eq!(c.region, e.region, "{}", a.name);
            assert!(
                free_slots(c.region).contains(&c.slot),
                "{}: slot {} is not free on {:?}",
                a.name,
                c.slot,
                c.region
            );
            let t = (e.region.id(), e.row, e.col);
            assert!(!tiles.contains(&t), "{}: tile {t:?} used twice", a.name);
            tiles.push(t);
        }
    }
    let entrances: usize = authored.iter().map(|(_, a)| a.entrances.len()).sum();
    assert_eq!(SLOTS.len(), entrances, "a claim names no entrance");
    for (i, a) in SLOTS.iter().enumerate() {
        for b in &SLOTS[..i] {
            assert!(
                !(shared_layout(a.region, b.region) && a.slot == b.slot),
                "{} and {} both take {:?} slot {}",
                a.area,
                b.area,
                a.region,
                a.slot
            );
        }
    }

    // ---- no original exit is redirected by two areas
    let mut links = Vec::new();
    for (g, a) in &all {
        for l in &a.links {
            let k = (a.table(), l.map, l.via);
            assert!(!links.contains(&k), "{g}::{}: {k:?} linked twice", a.name);
            links.push(k);
        }
    }

    // ---- persistent bits
    let mut end = 0;
    for g in &reg::GROUPS {
        assert_eq!(g.bit_base, end, "{}", g.name);
        let need = reg::group_page_bits(g.group);
        assert!(
            need + g.flags <= g.bits,
            "{}: {need} pages + {} flags > {} bits",
            g.name,
            g.flags,
            g.bits
        );
        end += g.bits;
    }
    assert_eq!(end, PROGRESS_POOL_BITS);

    // ---- container pieces
    let (mut heart, mut magic) = (0, 0);
    for (_, a) in &authored {
        for r in &a.rooms {
            for o in &r.objects {
                if let Object::Item { item, .. } = o {
                    heart += u8::from(*item == HEART_PIECE);
                    magic += u8::from(*item == MAGIC_PIECE);
                }
            }
        }
    }
    assert_eq!((heart, magic), (4, 4), "pieces: one container of each");

    // ---- geometry on the real floor table (the developer rooms are
    // walked for real in `enh_content_rom.rs` instead)
    let Some(floors) = floors() else { return };
    for (g, a) in &authored {
        assert_eq!(
            report(validate_geometry(a, &floors)),
            "",
            "{g}::{} on the ROM's floor patterns",
            a.name
        );
    }
}
