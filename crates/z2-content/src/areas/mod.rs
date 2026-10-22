//! The authored areas behind each `content` option (one module per
//! option of `z2_core::enh::ContentOpts`).
//!
//! Each module has two functions:
//!
//! * `areas() -> Vec<Area>`: the group's areas in model form. Fill this in.
//! * `statics() -> &'static [StaticArea]`: the same areas in static form,
//!   from the module's generated file (`<module>_generated.rs`, whose
//!   `AREAS` slice names each area's static after it, [`static_ident`]).
//!   This is what the engine registers (`z2_core::enh::content_areas`).
//!
//! The test below validates every authored area and fails when a
//! generated file is out of date. After changing `areas()`:
//!
//! ```sh
//! Z2_CONTENT_BLESS=1 cargo test -p z2-content areas
//! ```
//!
//! # Rules for authors
//!
//! * Original data only (no bytes from the ROM or another hack).
//! * Every virtual room of a table runs in that table's one reserved map
//!   slot. `Area::first_map` only numbers the rooms in the tool-facing
//!   `CompiledRoom::connections` bytes; the engine never loads those maps.
//!   Room-to-room exits are `Target::Room` (typed `Dest::Room` in the
//!   static form); `Target::Map` must name an original map of the same
//!   table; `Target::Outside` returns to the overworld at the entrance Link
//!   came in by, `Target::OutsideAt(n)` at the location slot `n` after it
//!   (the game adds `n` to `$0748`; a multi-mouth cave puts its mouths on
//!   consecutive slots).
//! * Ways in: overworld [`crate::Entrance`]s (the overworld engine places
//!   them on the location slot [`SLOTS`] gives them) and
//!   [`crate::VanillaLink`]s (an original room's exit redirected into the
//!   area while the option is on).
//! * Multi-mouth areas: entrance `i` of an area sits on slot `s + i`
//!   ([`SLOTS`] checks this), and whichever mouth Link walks in by, the
//!   overworld runtime keeps `$0748` on entrance 0's slot `s`, so
//!   `Target::Outside` always leaves by entrance 0 and `Target::OutsideAt(n)`
//!   by entrance `n`.
//! * Persistence: every page with a collectable or a locked door gets one
//!   battery-saved bit from the group's budget
//!   (`z2_core::enh::content_areas::GROUPS`), in the order areas, rooms and
//!   pages are listed. Append new rooms at the end of a group so existing
//!   saves keep their bits.

use crate::model::Region;
use crate::{Area, NpcScript, StaticArea};

pub mod challenge_rooms;
pub mod dark_isle;
pub mod dev_rooms;
pub mod hidden_caves;
pub mod minigames;
pub mod parapa_annex;

/// One option group's areas.
#[derive(Debug, Clone, Copy)]
pub struct Group {
    /// Module name (also the option name in `ContentOpts`).
    pub name: &'static str,
    /// Model form.
    pub areas: fn() -> Vec<Area>,
    /// Static form.
    pub statics: fn() -> &'static [StaticArea],
}

/// Every group, in the engine's registration order.
pub const GROUPS: [Group; 6] = [
    Group {
        name: "test_room",
        areas: dev_rooms::areas,
        statics: dev_rooms::statics,
    },
    Group {
        name: "parapa_annex",
        areas: parapa_annex::areas,
        statics: parapa_annex::statics,
    },
    Group {
        name: "challenge_rooms",
        areas: challenge_rooms::areas,
        statics: challenge_rooms::statics,
    },
    Group {
        name: "minigames",
        areas: minigames::areas,
        statics: minigames::statics,
    },
    Group {
        name: "hidden_caves",
        areas: hidden_caves::areas,
        statics: hidden_caves::statics,
    },
    Group {
        name: "dark_isle",
        areas: dark_isle::areas,
        statics: dark_isle::statics,
    },
];

impl Group {
    /// The module under `src/areas/` (`dev_rooms` for `test_room`).
    #[must_use]
    pub fn module(&self) -> &'static str {
        if self.name == "test_room" {
            "dev_rooms"
        } else {
            self.name
        }
    }
}

// ------------------------------------------------------------ overworld slots

/// The overworld location slot one [`crate::Entrance`] takes. The model's
/// entrance has a tile but no slot; this table is the one place slots are
/// handed out, so groups written apart cannot collide.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SlotClaim {
    /// Area (by [`Area::name`]).
    pub area: &'static str,
    /// Index into the area's `entrances` (and its compiled `entrances`).
    pub entrance: u8,
    /// Continent (the entrance's [`crate::Entrance::region`]).
    pub region: Region,
    /// Location slot: one of `z2_core::enh::content_mem::{WEST, EAST,
    /// DEATH_MOUNTAIN, MAZE_ISLAND}_FREE_SLOTS`.
    pub slot: u8,
    /// A vanilla slot of the same continent whose tile the entrance tile
    /// is joined to over open ground (the ROM test walks it), when the
    /// author picked the tile by a landmark.
    pub near: Option<u8>,
}

const fn claim(
    area: &'static str,
    entrance: u8,
    region: Region,
    slot: u8,
    near: Option<u8>,
) -> SlotClaim {
    SlotClaim {
        area,
        entrance,
        region,
        slot,
        near,
    }
}

/// Every overworld entrance of every option except `test_room` (the
/// developer cave's entrance is placed by its own test). The free slots
/// are West and East 33-39 and 55-61 (all four bytes zero) and the slots
/// Death Mountain and Maze Island leave unused in their shared layout
/// (README.md); a slot is used once per continent, and
/// once across Death Mountain and Maze Island together.
///
/// Slots from `$34` (52) get a palace code from the original entry code,
/// so palace-world entrances take 55-61 and caves and towns 33-39 (no town
/// or palace code; the engine supplies the identity of a virtual entry).
///
/// | continent | slot | area | entrance |
/// |---|---|---|---|
/// | West | 33 | Ember Run | cave |
/// | West | 34 | Lakeside Hollow | hidden |
/// | West | 35 | Old Barrow | cave (boulder) |
/// | West | 55 | Parapa Annex | back door |
/// | West | 56 | Crumbling Causeway | palace |
/// | East | 33 | Lizalfos Pit | cave |
/// | East | 34 | Sandcrawl Burrow | hidden |
/// | East | 35 | Reed Grotto | hidden |
/// | East | 36, 37 | Dark Isle Caves | mainland mouth, isle mouth |
/// | East | 38 | Grimhollow | town |
/// | East | 55 | Thornspire | palace |
/// | East | 56 | Rubble Maze | palace |
/// | East | 57 | Skyward Stair | palace |
/// | Death Mountain | 53 | Ember Niche | hidden |
/// | Maze Island | 54 | Tidepool Cave | hidden |
pub const SLOTS: &[SlotClaim] = &[
    // challenge_rooms
    claim("Ember Run", 0, Region::West, 33, Some(0)), // North Palace
    claim("Crumbling Causeway", 0, Region::West, 56, Some(45)), // Rauru
    claim("Lizalfos Pit", 0, Region::East, 33, Some(45)), // Nabooru
    claim("Rubble Maze", 0, Region::East, 56, Some(45)),
    claim("Skyward Stair", 0, Region::East, 57, Some(45)),
    // parapa_annex
    claim("Parapa Annex", 0, Region::West, 55, Some(52)), // Palace 1
    // hidden_caves
    claim("Lakeside Hollow", 0, Region::West, 34, None),
    claim("Old Barrow", 0, Region::West, 35, None),
    claim("Sandcrawl Burrow", 0, Region::East, 34, None),
    claim("Reed Grotto", 0, Region::East, 35, None),
    claim("Ember Niche", 0, Region::DeathMountain, 53, None),
    claim("Tidepool Cave", 0, Region::MazeIsland, 54, None),
    // dark_isle
    claim("Dark Isle Caves", 0, Region::East, 36, None),
    claim("Dark Isle Caves", 1, Region::East, 37, None),
    claim("Grimhollow", 0, Region::East, 38, None),
    claim("Thornspire", 0, Region::East, 55, None),
];

/// The slot of entrance `entrance` of the area named `area`.
#[must_use]
pub fn slot_of(area: &str, entrance: usize) -> Option<&'static SlotClaim> {
    SLOTS
        .iter()
        .find(|c| c.area == area && usize::from(c.entrance) == entrance)
}

/// Whether two continents' location tables share one slot layout (Death
/// Mountain and Maze Island), so a slot taken in one is taken in both.
#[must_use]
pub const fn shared_layout(a: Region, b: Region) -> bool {
    a.id() == b.id()
        || (matches!(a, Region::DeathMountain | Region::MazeIsland)
            && matches!(b, Region::DeathMountain | Region::MazeIsland))
}

/// Every area of every option but `test_room`, with its option name, in
/// registration order.
#[must_use]
pub fn authored() -> Vec<(&'static str, Area)> {
    GROUPS
        .iter()
        .filter(|g| g.name != "test_room")
        .flat_map(|g| (g.areas)().into_iter().map(move |a| (g.name, a)))
        .collect()
}

/// The identifier an area's static gets in a generated file: its name in
/// upper case, every run of other characters as one `_`.
#[must_use]
pub fn static_ident(name: &str) -> String {
    let mut s = String::new();
    for c in name.chars() {
        if c.is_ascii_alphanumeric() {
            s.push(c.to_ascii_uppercase());
        } else if !s.ends_with('_') {
            s.push('_');
        }
    }
    let s = s.trim_matches('_').to_string();
    if s.is_empty() || s.starts_with(|c: char| c.is_ascii_digit()) {
        format!("AREA_{s}")
    } else {
        s
    }
}

/// Every [`NpcScript`] of every group (the engine looks a person's script
/// up by area name and index). Each module that scripts its people lists
/// them in its own `NPC_SCRIPTS` slice, chained here.
pub fn npc_scripts() -> impl Iterator<Item = &'static NpcScript> {
    dev_rooms::NPC_SCRIPTS.iter()
}

/// Check every script against the model: its area exists, the person
/// does, every message encodes and fits a box, a gift has a flag to
/// record it, at most one script per person.
///
/// # Errors
/// The first problem found, in words.
pub fn validate_npc_scripts() -> Result<(), String> {
    let all: Vec<Area> = GROUPS.iter().flat_map(|g| (g.areas)()).collect();
    let mut seen = std::collections::HashSet::new();
    for s in npc_scripts() {
        let what = |m: &str| format!("script {}#{}: {m}", s.area, s.npc);
        let area = all
            .iter()
            .find(|a| a.name == s.area)
            .ok_or_else(|| what("no such area"))?;
        if usize::from(s.npc) >= area.npcs.len() {
            return Err(what("no such person"));
        }
        if !seen.insert((s.area, s.npc)) {
            return Err(what("two scripts"));
        }
        for m in s.messages() {
            crate::text::encode_message(m.text).map_err(|e| what(&e.to_string()))?;
            crate::text::validate_dialog(m.text, crate::text::MAX_LINES)
                .map_err(|e| what(&e.to_string()))?;
        }
        if let crate::NpcRole::Sage { flag: None, gift } = s.role {
            if gift != crate::Gift::None {
                return Err(what("a gift needs a flag"));
            }
        }
    }
    Ok(())
}

/// The generated source of group `g` (what `<name>_generated.rs` holds).
///
/// # Errors
/// An area that does not compile.
pub fn group_source(g: &Group) -> Result<String, crate::ContentError> {
    let all = (g.areas)();
    let idents: Vec<String> = all.iter().map(|a| static_ident(a.name)).collect();
    let refs: Vec<(&str, &Area)> = idents.iter().map(String::as_str).zip(all.iter()).collect();
    crate::compile::generate_group_source(&refs, &format!("src/areas/{}.rs", g.module()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{compile, validate};

    #[test]
    fn areas_validate_and_generated_files_are_current() {
        for g in GROUPS {
            let all = (g.areas)();
            for a in &all {
                if let Err(issues) = validate(a) {
                    let lines: Vec<String> = issues.iter().map(ToString::to_string).collect();
                    panic!("{}::{}:\n{}", g.name, a.name, lines.join("\n"));
                }
            }
            let src = group_source(&g).unwrap();
            let path = format!(
                "{}/src/areas/{}_generated.rs",
                env!("CARGO_MANIFEST_DIR"),
                g.module()
            );
            if std::env::var_os("Z2_CONTENT_BLESS").is_some() {
                std::fs::write(&path, &src).unwrap();
                continue;
            }
            let on_disk = std::fs::read_to_string(&path).unwrap();
            assert!(
                on_disk == src,
                "src/areas/{}_generated.rs is stale: run Z2_CONTENT_BLESS=1 cargo test -p z2-content areas",
                g.module()
            );
            let statics = (g.statics)();
            assert_eq!(statics.len(), all.len(), "{}", g.name);
            for (a, s) in all.iter().zip(statics) {
                assert!(compile(a).unwrap().matches(s), "{}: {}", g.name, a.name);
            }
        }
    }

    /// Every overworld entrance of every option has exactly one slot
    /// claim, and every claim names a real entrance on its continent.
    #[test]
    fn every_entrance_has_one_slot() {
        let all = authored();
        for (g, a) in &all {
            for (i, e) in a.entrances.iter().enumerate() {
                let n = SLOTS
                    .iter()
                    .filter(|c| c.area == a.name && usize::from(c.entrance) == i)
                    .count();
                assert_eq!(n, 1, "{g}::{} entrance {i}", a.name);
                assert_eq!(slot_of(a.name, i).unwrap().region, e.region, "{}", a.name);
            }
        }
        for c in SLOTS {
            let (_, a) = all
                .iter()
                .find(|(_, a)| a.name == c.area)
                .unwrap_or_else(|| panic!("slot {} names no area {}", c.slot, c.area));
            assert!(usize::from(c.entrance) < a.entrances.len(), "{}", c.area);
        }
    }

    /// No two entrances share a slot (Death Mountain and Maze Island count
    /// as one layout) or a tile, and a multi-mouth area's mouths sit on
    /// consecutive slots.
    #[test]
    fn slots_and_tiles_are_unique() {
        for (i, a) in SLOTS.iter().enumerate() {
            for b in &SLOTS[..i] {
                assert!(
                    !(shared_layout(a.region, b.region) && a.slot == b.slot),
                    "{} and {} both take slot {}",
                    a.area,
                    b.area,
                    a.slot
                );
            }
            if a.entrance > 0 {
                let first = slot_of(a.area, 0).unwrap();
                assert_eq!(a.slot, first.slot + a.entrance, "{}", a.area);
            }
        }
        let mut tiles = Vec::new();
        for (_, a) in authored() {
            for e in &a.entrances {
                let t = (e.region.id(), e.row, e.col);
                assert!(!tiles.contains(&t), "{}: tile {t:?} used twice", a.name);
                tiles.push(t);
            }
        }
    }

    /// Dropping any ability from an area's `needs` makes it fail.
    #[test]
    fn every_declared_need_is_needed() {
        for (g, a) in authored() {
            for bit in 0..16 {
                let b = crate::Abilities(1 << bit);
                if !a.needs.has(b) {
                    continue;
                }
                let mut less = a.clone();
                less.needs = crate::Abilities(a.needs.0 & !b.0);
                assert!(
                    validate(&less).is_err(),
                    "{g}::{} passes without {b:?}",
                    a.name
                );
            }
        }
    }

    /// Every elevator is ridden and every ride has an elevator; no world-0
    /// room uses the ledge object.
    #[test]
    fn elevators_are_used_and_world_0_has_no_ledges() {
        use crate::{Dir, Object, World};
        for (g, a) in authored() {
            for r in &a.rooms {
                let has_el = r
                    .objects
                    .iter()
                    .any(|o| matches!(o, Object::Elevator { .. }));
                let uses_el = r.exits.iter().any(|e| matches!(e.dir, Dir::Up | Dir::Down));
                assert_eq!(has_el, uses_el, "{g}::{}/{}", a.name, r.id);
                if a.world == World::Overworld {
                    assert!(
                        !r.objects
                            .iter()
                            .any(|o| matches!(o, Object::Platform { .. })),
                        "{g}::{}/{}",
                        a.name,
                        r.id
                    );
                }
            }
        }
    }

    /// Each vanilla link's room leads back to the original map it came
    /// from.
    #[test]
    fn links_lead_back() {
        use crate::Target;
        for (g, a) in authored() {
            for l in &a.links {
                let r = a.room(l.room).unwrap();
                assert!(l.page < r.pages, "{g}::{}", a.name);
                assert!(
                    r.exits
                        .iter()
                        .any(|e| matches!(e.target, Target::Map { map, .. } if map == l.map)),
                    "{g}::{}: {} has no way back to map {}",
                    a.name,
                    l.room,
                    l.map
                );
            }
        }
    }

    #[test]
    fn npc_scripts_are_valid() {
        validate_npc_scripts().unwrap();
    }

    #[test]
    fn static_idents() {
        assert_eq!(static_ident("Hollow Annex"), "HOLLOW_ANNEX");
        assert_eq!(static_ident("8th palace - west"), "AREA_8TH_PALACE_WEST");
        assert_eq!(static_ident("--"), "AREA_");
    }
}
