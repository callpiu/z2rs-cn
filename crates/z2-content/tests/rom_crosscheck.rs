//! ROM-gated cross-checks: the codecs ported into `z2-content` agree with
//! `z2-rando`'s on vanilla data read from the player's ROM at test time, and
//! the sample areas pass the movement check on the game's real floor
//! patterns. Nothing read here is stored.
//!
//! `Z2_ROM=/path/to/zelda2.nes cargo test -p z2-content --test rom_crosscheck`
//! (skips when `Z2_ROM` is not set).

mod common;

use z2_content::sideview::{FloorTable, ObjectSet, Sideview};
use z2_content::{enemies, samples, text, validate_geometry};
use z2_rando::enemies::shuffle::{BLOB_ADDR, BLOB_WRAM, OVERWORLD, PALACES};
use z2_rando::rom::Rom;
use z2_rando::sideview as rs;

use common::body;

fn sets(name: &str) -> (ObjectSet, rs::ObjectSet) {
    if name.starts_with("Great") {
        (ObjectSet::GreatPalace, rs::ObjectSet::GreatPalace)
    } else if name.starts_with("Palaces") {
        (ObjectSet::Palace, rs::ObjectSet::Palace)
    } else {
        (ObjectSet::Other, rs::ObjectSet::Other)
    }
}

#[test]
fn sideviews_and_grids_match_z2_rando() {
    let Some(body) = body() else { return };
    let rom = Rom::from_body(&body).unwrap();
    let mine_floors = FloorTable::from_body(&body).unwrap();
    let their_floors = rs::FloorTable::from_rom(&rom).unwrap();
    let mut compared = 0;
    for area in OVERWORLD.iter().chain(PALACES.iter()) {
        let (set, rset) = sets(area.name);
        for m in 0..63u16 {
            let Ok(ptr) = rom.read_cpu_word(area.bank, area.sideviews + 2 * m) else {
                continue;
            };
            let Ok((theirs, raw)) = rs::read_at(&rom, area.bank, ptr) else {
                continue;
            };
            let mine = Sideview::parse(&raw).expect("z2-rando parsed it");
            assert_eq!(mine.header, theirs.header, "{} map {m}", area.name);
            let a: Vec<_> = mine
                .commands
                .iter()
                .map(|c| (c.x, c.y, c.b, c.item))
                .collect();
            let b: Vec<_> = theirs
                .commands
                .iter()
                .map(|c| (c.x, c.y, c.b, c.item))
                .collect();
            assert_eq!(a, b, "{} map {m}", area.name);
            assert_eq!(
                mine.encode().ok(),
                theirs.encode().ok(),
                "{} map {m}",
                area.name
            );
            // The floor sweep agrees. The object cells do not: z2-content's
            // palace, Great Palace and world-0 object kinds are measured in
            // the game (`sideview::large_cells`, checked end to end by
            // z2-core's `tests/content_areas_e2e.rs`), z2-rando's palace
            // reach table is not, so only the floors are compared.
            let mut floors_only = mine.clone();
            floors_only.commands.retain(|c| c.is_floor());
            let mut theirs_floors = theirs.clone();
            theirs_floors.commands.retain(|c| c.is_floor());
            assert_eq!(
                floors_only.grid(set, &mine_floors).render(),
                theirs_floors.grid(rset, &their_floors).render(),
                "{} map {m}",
                area.name
            );
            compared += 1;
        }
    }
    assert!(compared > 300, "only {compared} rooms compared");
}

#[test]
fn enemy_lists_match_z2_rando() {
    let Some(body) = body() else { return };
    let rom = Rom::from_body(&body).unwrap();
    let mut compared = 0;
    for area in OVERWORLD.iter().chain(PALACES.iter()) {
        for m in 0..63u16 {
            let Ok(wram) = rom.read_cpu_word(area.bank, area.enemies + 2 * m) else {
                continue;
            };
            if !(BLOB_WRAM..BLOB_WRAM + 0x400).contains(&wram) {
                continue;
            }
            let addr = wram - BLOB_WRAM + BLOB_ADDR;
            let theirs = z2_rando::enemies::shuffle::read_list(&rom, area.bank, addr).unwrap();
            let off = rom.cpu_offset(area.bank, addr).unwrap();
            let len = usize::from(rom.read(off).unwrap()).max(1);
            let raw = rom.read_slice(off, len).unwrap();
            // z2-rando treats even lengths as empty; so does the game's
            // 2-byte stride, so only compare well-formed lists.
            if raw[0].is_multiple_of(2) {
                continue;
            }
            let mine = enemies::decode_list(raw).unwrap();
            let a: Vec<_> = mine.iter().map(|e| (e.x, e.y, e.id)).collect();
            let b: Vec<_> = theirs.iter().map(|e| (e.x, e.y, e.id)).collect();
            assert_eq!(a, b, "{} map {m}", area.name);
            assert_eq!(enemies::encode_list(&mine).unwrap(), raw);
            compared += 1;
        }
    }
    assert!(compared > 200, "only {compared} lists compared");
}

#[test]
fn vanilla_messages_decode_the_same() {
    let Some(body) = body() else { return };
    let rom = Rom::from_body(&body).unwrap();
    let msgs = z2_rando::text::read_vanilla_messages(&rom).unwrap();
    for m in &msgs {
        let s = text::decode(m);
        assert_eq!(s, z2_rando::text::decode(m));
        assert_eq!(text::encode_message(&s).unwrap(), *m);
    }
}

#[test]
fn samples_pass_geometry_on_the_real_floor_patterns() {
    let Some(body) = body() else { return };
    let floors = FloorTable::from_body(&body).unwrap();
    for (name, a) in samples::all() {
        if let Err(v) = validate_geometry(&a, &floors) {
            let lines: Vec<String> = v.iter().map(ToString::to_string).collect();
            panic!("{name}:\n{}", lines.join("\n"));
        }
    }
    // The check bites: without the glove the vault's heart is cut off, and
    // without the key the hall's locked door blocks the way right.
    let mut a = samples::hollow_annex();
    a.rooms[2].item_requires = z2_content::Abilities::NONE;
    assert!(validate_geometry(&a, &floors).is_err());
    let mut a = samples::hollow_annex();
    a.rooms[0].exits[1].requires = z2_content::Abilities::NONE;
    // (Still declared somewhere, so only the geometry objects.)
    a.rooms[0].item_requires = z2_content::Abilities::KEY;
    assert!(validate_geometry(&a, &floors).is_err());
}
