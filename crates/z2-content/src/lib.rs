//! `z2-content`: authoring new areas for z2rs (the `content` enhancement
//! group: an extra palace, a monster town, challenge rooms, caves).
//!
//! New areas are "virtual rooms": the engine copies a room's sideview
//! bytes, enemy list and connection bytes into WRAM buffers that a reserved
//! map slot points at. This crate is where those bytes come from:
//!
//! 1. [`model`]: a typed description of an [`Area`] (rooms, objects,
//!    enemies, exits, overworld entrances, town people and their messages);
//! 2. [`compile`]: encoders to the game's byte formats ([`sideview`],
//!    [`enemies`], connection bytes, [`text`]) producing
//!    [`CompiledRoom`]s, plus [`compile::generate_source`], which prints
//!    the same bytes as `static` tables ([`generated`]) so the engine's hot
//!    path reads `&'static [u8]` and never allocates;
//! 3. [`validate`]: size limits, bounds, exits, and reachability over the
//!    area graph and (with the ROM's floor patterns) inside each room.
//!
//! See `README.md` for how to author an area. Everything authored here is
//! original; the crate holds no ROM bytes and no data derived from them.
//! The sideview, enemy list and text codecs are ported from `z2-rando`
//! (copied, not shared, so `z2-core` can depend on this crate) and checked
//! against it on vanilla rooms by the ROM-gated `tests/rom_crosscheck.rs`.
//!
//! No dependencies; builds for `wasm32-unknown-unknown`.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use std::fmt;

pub mod areas;
pub mod compile;
pub mod enemies;
#[rustfmt::skip]
pub mod generated;
pub mod model;
pub mod npc;
pub mod samples;
pub mod sideview;
pub mod text;
pub mod validate;

pub use compile::{
    compile, CompiledArea, CompiledEntrance, CompiledNpc, CompiledRoom, CompiledTerrain, Dest,
    StaticArea, StaticLink, StaticRoom,
};
pub use model::{
    Abilities, Area, Dir, EnemySpawn, Entrance, Exit, LinkVia, MapTile, Message, Object, Region,
    Room, RoomId, Table, Target, TerrainEdit, TileKind, TownNpc, VanillaLink, World,
};
pub use npc::{Gift, NpcRole, NpcScript};
pub use validate::{validate, validate_geometry, Issue};

/// Encode / decode failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContentError {
    /// Something has no byte encoding (out of range, unsupported in the
    /// world's object set, too long).
    Encode(String),
    /// Bytes that do not decode.
    Decode(String),
    /// Dialog text that does not fit the font or the box.
    Text(String),
}

impl fmt::Display for ContentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ContentError::Encode(m) => write!(f, "encode: {m}"),
            ContentError::Decode(m) => write!(f, "decode: {m}"),
            ContentError::Text(m) => write!(f, "text: {m}"),
        }
    }
}

impl std::error::Error for ContentError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compile::{NO_DOOR, OUTSIDE};
    use crate::sideview::{Command, FloorTable, ObjectSet, Sideview, ROW_EXTENDED};

    /// Synthetic floor patterns for geometry tests without a ROM: pattern 1
    /// = ceiling row 0 + floor rows 10-11 (Link stands on row 9), pattern
    /// 15 = solid. (Not the game's table; the real one comes from the ROM.)
    fn floors() -> FloorTable {
        let mut m = [[0u8; 2]; 16];
        m[1] = [0x40, 0x06];
        m[15] = [0x7E, 0x7E];
        FloorTable::from_masks(m)
    }

    #[test]
    fn every_sample_area_validates() {
        for (name, a) in samples::all() {
            if let Err(issues) = validate(&a) {
                let lines: Vec<String> = issues.iter().map(ToString::to_string).collect();
                panic!("{name}:\n{}", lines.join("\n"));
            }
        }
    }

    #[test]
    fn every_sample_area_passes_geometry_on_synthetic_floors() {
        for (name, a) in samples::all() {
            if let Err(issues) = validate_geometry(&a, &floors()) {
                let lines: Vec<String> = issues.iter().map(ToString::to_string).collect();
                panic!("{name}:\n{}", lines.join("\n"));
            }
        }
    }

    /// `src/generated.rs` is exactly what the samples compile to. Run with
    /// `Z2_CONTENT_BLESS=1` to rewrite it.
    #[test]
    fn generated_tables_are_current() {
        let all = samples::all();
        let refs: Vec<(&str, &Area)> = all.iter().map(|(n, a)| (*n, a)).collect();
        let src = compile::generate_source(&refs).unwrap();
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/src/generated.rs");
        if std::env::var_os("Z2_CONTENT_BLESS").is_some() {
            std::fs::write(path, &src).unwrap();
            return;
        }
        let on_disk = std::fs::read_to_string(path).unwrap();
        assert!(
            on_disk == src,
            "src/generated.rs is stale: run Z2_CONTENT_BLESS=1 cargo test -p z2-content generated"
        );
        let statics = [&generated::HOLLOW_ANNEX, &generated::WAYSIDE_HUT];
        for ((_, a), s) in all.iter().zip(statics) {
            assert!(compile(a).unwrap().matches(s));
        }
        assert_eq!(generated::HOLLOW_ANNEX.room_for_map(61).unwrap().map, 61);
    }

    #[test]
    fn sample_bytes_and_connections() {
        let a = samples::hollow_annex();
        let c = compile(&a).unwrap();
        assert_eq!(c.world, 3);
        let [hall, shaft, vault] = &c.rooms[..] else {
            panic!()
        };
        // hall: left out, right (byte 1 of a 2-page room) to shaft page 0.
        assert_eq!(hall.connections, [OUTSIDE, 61 << 2, OUTSIDE, OUTSIDE]);
        // The engine's typed view of the same exits.
        assert_eq!(c.table, Table::PalaceA.index());
        assert_eq!(
            hall.exits,
            [
                Dest::Byte(OUTSIDE),
                Dest::Room { room: 1, page: 0 },
                Dest::Byte(OUTSIDE),
                Dest::Byte(OUTSIDE)
            ]
        );
        assert_eq!(shaft.exits[1], Dest::Room { room: 2, page: 0 });
        assert_eq!(vault.exits[2], Dest::Room { room: 1, page: 1 });
        assert_eq!(hall.door_exits, [None; 4]);
        assert_eq!((c.entrances[0].room, c.entrances[0].page), (0, 0));
        // shaft: left to hall's last page, down to vault's elevator page.
        assert_eq!(
            shaft.connections,
            [(60 << 2) | 1, 62 << 2, OUTSIDE, OUTSIDE]
        );
        // vault: up to shaft's elevator page (1).
        assert_eq!(
            vault.connections,
            [OUTSIDE, OUTSIDE, (61 << 2) | 1, OUTSIDE]
        );
        assert_eq!(hall.item, Some(model::item::SMALL_KEY));
        assert_eq!(vault.item, Some(model::item::HEART_CONTAINER));
        assert_eq!(hall.doors, [NO_DOOR; 4]);
        for r in &c.rooms {
            assert_eq!(usize::from(r.sideview[0]), r.sideview.len());
            assert_eq!(usize::from(r.enemies[0]), r.enemies.len());
        }
        let hut = compile(&samples::wayside_hut()).unwrap();
        assert_eq!(hut.npcs.len(), 1);
        assert_eq!(hut.npcs[0].enemy_index, 0);
        assert_eq!(
            text::decode(&hut.messages[0]),
            samples::wayside_hut().npcs[0].message.text
        );
        assert_eq!(hut.entrances[0].target, 40 << 2);
    }

    fn all_objects() -> Vec<(ObjectSet, Object)> {
        use ObjectSet::{GreatPalace as Gp, Other, Palace};
        let mut v = vec![];
        for set in [Palace, Gp, Other] {
            let mut add = |o: Object| v.push((set, o));
            add(Object::Floor {
                x: 3,
                pattern: 7,
                no_ceiling: true,
            });
            add(Object::Platform {
                x: 4,
                y: 5,
                len: 16,
            });
            add(Object::Block { x: 5, y: 6, len: 1 });
            add(Object::Lava { x: 6, width: 3 });
            add(Object::Elevator { x: 7 });
            add(Object::Item {
                x: 8,
                y: 9,
                item: model::item::MAGIC_CONTAINER,
            });
            add(Object::Raw(Command::new(9, 4, 0xD2)));
            if set != Other {
                add(Object::Breakable {
                    x: 10,
                    y: 3,
                    len: 2,
                });
                add(Object::BreakableBlock {
                    x: 11,
                    y: 3,
                    len: 2,
                });
                add(Object::Crumble {
                    x: 12,
                    y: 7,
                    len: 4,
                });
                add(Object::Bridge {
                    x: 13,
                    y: 7,
                    len: 5,
                });
                add(Object::FalseWall {
                    x: 14,
                    y: 2,
                    len: 2,
                });
                add(Object::Column {
                    x: 15,
                    y: 1,
                    height: 9,
                });
                add(Object::LockedDoor { x: 16, y: 7 });
            }
            if set != Gp {
                add(Object::Pit { x: 17, width: 1 });
            }
            if set == Palace {
                add(Object::Statue { x: 18, y: 8 });
            }
        }
        v
    }

    #[test]
    fn every_object_kind_round_trips_through_bytes() {
        for (set, o) in all_objects() {
            let c = o.to_command(set).unwrap();
            let sv = Sideview {
                header: [0, 0x60, 0x01, 0x08],
                commands: vec![c],
            };
            let raw = sv.encode().unwrap();
            let back = Sideview::parse(&raw).unwrap();
            assert_eq!(back.commands, vec![c], "{set:?} {o:?}");
            assert_eq!(Object::from_command(&back.commands[0], set), o, "{set:?}");
            assert_eq!(back.encode().unwrap(), raw);
        }
    }

    #[test]
    fn unsupported_objects_are_errors() {
        assert!(Object::Pit { x: 0, width: 1 }
            .to_command(ObjectSet::GreatPalace)
            .is_err());
        assert!(Object::Statue { x: 0, y: 0 }
            .to_command(ObjectSet::Other)
            .is_err());
        assert!(Object::Platform {
            x: 0,
            y: 13,
            len: 1
        }
        .to_command(ObjectSet::Palace)
        .is_err());
        assert!(Object::Platform {
            x: 0,
            y: 1,
            len: 17
        }
        .to_command(ObjectSet::Palace)
        .is_err());
        assert!(Object::Raw(Command::new(0, 14, 0))
            .to_command(ObjectSet::Palace)
            .is_err());
    }

    #[test]
    fn whole_room_round_trips() {
        let a = samples::hollow_annex();
        let set = a.world.object_set();
        for r in &a.rooms {
            let bytes = compile::compile_room(r, &a).unwrap();
            let sv = Sideview::parse(&bytes.sideview).unwrap();
            let objs: Vec<Object> = sv
                .commands
                .iter()
                .map(|c| Object::from_command(c, set))
                .collect();
            // An elevator is followed by its shaft.
            let want: Vec<Object> = r
                .objects
                .iter()
                .flat_map(|o| {
                    let mut v = match *o {
                        Object::Pit { x, width } => (x..x + width)
                            .map(|x| Object::Pit { x, width: 1 })
                            .collect(),
                        o => vec![o],
                    };
                    if let Object::Elevator { x } = *o {
                        v.extend(
                            compile::elevator_shaft(r, set, x)
                                .iter()
                                .map(|c| Object::from_command(c, set)),
                        );
                    }
                    v
                })
                .collect();
            assert_eq!(objs, want);
            assert_eq!(sv.pages(), r.pages);
            assert_eq!(sv.start_floor(), r.floor);
            let es = enemies::decode_list(&bytes.enemies).unwrap();
            assert_eq!(es.len(), r.enemies.len());
            for (e, s) in es.iter().zip(&r.enemies) {
                assert_eq!((e.x, e.y, e.id), (s.x, s.y, s.id));
            }
        }
    }

    fn corridor(id: RoomId) -> Room {
        let mut r = Room::new(id, 2, 1);
        // (A room needs one command; this one restates the floor.)
        r.objects = vec![Object::Floor {
            x: 0,
            pattern: 1,
            no_ceiling: false,
        }];
        r.exits = vec![
            Exit::new(Dir::Left, Target::Outside),
            Exit::new(Dir::Right, Target::Outside),
        ];
        r
    }

    fn area(rooms: Vec<Room>) -> Area {
        Area {
            name: "test",
            world: World::PalaceA,
            region: Region::West,
            first_map: 10,
            links: vec![],
            entrances: vec![Entrance {
                region: Region::West,
                row: 1,
                col: 1,
                tile: TileKind::Cave,
                room: rooms[0].id,
                page: 0,
                requires: Abilities::NONE,
            }],
            rooms,
            terrain: vec![],
            npcs: vec![],
            needs: Abilities::NONE,
        }
    }

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
    fn validation_catches_problems() {
        assert_eq!(issues(&area(vec![corridor("a")])), "");
        // Missing exit target.
        let mut r = corridor("a");
        r.exits[1].target = Target::Room("nowhere");
        assert!(issues(&area(vec![r])).contains("missing room"));
        // Object past the edge, too many pages, bad enemy row.
        let mut r = corridor("a");
        r.objects.push(Object::Platform {
            x: 30,
            y: 5,
            len: 4,
        });
        r.enemies.push(EnemySpawn { x: 2, y: 2, id: 3 });
        let s = issues(&area(vec![r]));
        assert!(
            s.contains("runs past") && s.contains("not encodable"),
            "{s}"
        );
        let mut r = corridor("a");
        r.pages = 5;
        assert!(issues(&area(vec![r])).contains("pages"));
        // Vertical object below the bottom row.
        let mut r = corridor("a");
        r.objects.push(Object::Column {
            x: 3,
            y: 10,
            height: 5,
        });
        assert!(issues(&area(vec![r])).contains("below row"));
        // Too many enemies on a page.
        let mut r = corridor("a");
        r.enemies = vec![EnemySpawn { x: 2, y: 9, id: 3 }; 7];
        assert!(issues(&area(vec![r])).contains("at most 6"));
        // Too long a sideview.
        let mut r = corridor("a");
        r.objects = vec![Object::Platform { x: 1, y: 3, len: 1 }; 130];
        assert!(issues(&area(vec![r])).contains("exceeds 255"));
        // Up without an elevator; pit without a drop; locked door without
        // a key; connection byte clash (1-page room, left and right differ).
        let mut r = corridor("a");
        r.exits.push(Exit::new(Dir::Up, Target::Outside));
        r.objects.push(Object::Pit { x: 20, width: 2 });
        r.objects.push(Object::LockedDoor { x: 5, y: 7 });
        let s = issues(&area(vec![r]));
        assert!(s.contains("without an elevator"), "{s}");
        assert!(s.contains("no drop exit"), "{s}");
        assert!(s.contains("KEY"), "{s}");
        let mut r = corridor("a");
        r.pages = 1;
        r.exits[1].target = Target::Map { map: 3, page: 0 };
        assert!(issues(&area(vec![r])).contains("both need connection byte 0"));
        // Unreachable room, room with no way out.
        let mut b = corridor("b");
        b.exits.clear();
        b.exits.push(Exit::new(Dir::Left, Target::Room("c")));
        let mut c = corridor("c");
        c.exits = vec![Exit::new(Dir::Right, Target::Room("b"))];
        let mut a0 = corridor("a");
        a0.exits[1].target = Target::Room("b");
        let s = issues(&area(vec![a0, b, c]));
        assert!(s.contains("no way back out"), "{s}");
        // Requirement gating.
        let mut a0 = corridor("a");
        a0.exits[1] = Exit::new(Dir::Right, Target::Room("b")).needs(Abilities::JUMP);
        let mut ar = area(vec![a0, corridor("b")]);
        assert!(issues(&ar).contains("not reachable"));
        ar.needs = Abilities::JUMP;
        assert_eq!(issues(&ar), "");
        let reach = validate::reachable(&ar, Abilities::NONE);
        assert_eq!(reach.rooms, vec![true, false]);
        // Bad message.
        let mut ar = area(vec![corridor("a")]);
        ar.npcs.push(TownNpc {
            room: "a",
            spawn: EnemySpawn { x: 1, y: 9, id: 1 },
            message: Message {
                text: "THIS LINE IS TOO WIDE",
            },
        });
        assert!(issues(&ar).contains("characters"));
        ar.npcs[0].message.text = "NO # SIGN";
        assert!(issues(&ar).contains("glyph"));
        // Map slots run out.
        let mut ar = area(vec![corridor("a"), corridor("b")]);
        ar.first_map = 62;
        assert!(issues(&ar).contains("last map slot"));
    }

    #[test]
    fn geometry_finds_walls_and_needs() {
        let f = floors();
        // A full-height wall needs more than walking.
        let mut r = corridor("a");
        r.objects.push(Object::Floor {
            x: 15,
            pattern: 15,
            no_ceiling: false,
        });
        r.objects.push(Object::Floor {
            x: 17,
            pattern: 1,
            no_ceiling: false,
        });
        let a = area(vec![r.clone()]);
        assert!(validate(&a).is_ok());
        assert!(validate_geometry(&a, &f).is_err());
        // A breakable wall is fine with the glove declared.
        let mut r = corridor("a");
        for y in [1, 3, 5, 7] {
            r.objects.push(Object::BreakableBlock { x: 15, y, len: 1 });
        }
        r.objects.push(Object::Platform {
            x: 15,
            y: 9,
            len: 1,
        });
        let a = area(vec![r.clone()]);
        let e = validate_geometry(&a, &f).unwrap_err();
        assert!(
            e.iter().any(|i| i.message.contains("not reachable")),
            "{e:?}"
        );
        r.requires = Abilities::GLOVE;
        let mut a = area(vec![r]);
        a.needs = Abilities::GLOVE;
        // The platform under the wall at row 9 is a plain step.
        assert!(
            validate_geometry(&a, &f).is_ok(),
            "{:?}",
            validate_geometry(&a, &f)
        );
        // A locked door blocks without a key; fairy flies through.
        let mut r = corridor("a");
        r.objects.push(Object::LockedDoor { x: 20, y: 7 });
        r.exits[1].requires = Abilities::FAIRY;
        r.exits[0].requires = Abilities::NONE;
        let mut a = area(vec![r.clone()]);
        a.needs = Abilities::FAIRY.with(Abilities::KEY);
        assert!(validate(&a).is_err(), "door needs KEY declared");
        r.exits[1].requires = Abilities::KEY;
        let mut a = area(vec![r]);
        a.needs = Abilities::KEY;
        assert!(
            validate_geometry(&a, &f).is_ok(),
            "{:?}",
            validate_geometry(&a, &f)
        );
        // An open-bottom page needs a drop exit; with one, falling reaches it.
        let mut r = corridor("a");
        r.objects.push(Object::Pit { x: 20, width: 2 });
        r.exits
            .push(Exit::new(Dir::Drop { page: 1 }, Target::Room("b")));
        let mut b = corridor("b");
        b.landing = Some(0);
        let a = area(vec![r, b]);
        // Byte 1 is both "right" and "drop on page 1" in a 2-page room.
        assert!(validate(&a).is_err());
        let mut r = corridor("a");
        r.pages = 3;
        // (The movement model is conservative: a plain jump clears a
        // one-column gap.)
        r.objects.push(Object::Pit { x: 20, width: 1 });
        r.exits
            .push(Exit::new(Dir::Drop { page: 1 }, Target::Room("b")));
        let mut b = corridor("b");
        b.landing = Some(1);
        let a = area(vec![r, b]);
        assert!(
            validate_geometry(&a, &f).is_ok(),
            "{:?}",
            validate_geometry(&a, &f)
        );
        let c = compile(&a).unwrap();
        assert_eq!(c.rooms[0].connections[1], (11 << 2) | 1);
        // A wide pit cannot be jumped: the right exit is cut off.
        let mut r = corridor("a");
        r.pages = 3;
        r.objects.push(Object::Pit { x: 20, width: 8 });
        r.exits
            .push(Exit::new(Dir::Drop { page: 1 }, Target::Outside));
        let a = area(vec![r]);
        assert!(validate_geometry(&a, &f).is_err());
    }

    #[test]
    fn grid_of_the_vault_shows_the_breakable_wall() {
        let a = samples::hollow_annex();
        let vault = a.room("vault").unwrap();
        let g = validate::room_grid(&a, vault, &floors()).unwrap();
        let pic = g.render();
        let rows: Vec<&str> = pic.lines().collect();
        assert_eq!(rows.len(), 13);
        assert_eq!(&rows[5][10..11], "%");
        assert_eq!(&rows[9][13..14], "I");
        assert_eq!(g.elevator, Some(3));
    }

    #[test]
    fn extended_and_page_jump_layout() {
        // An object on page 3 after one on page 0 gets a page jump.
        let sv = Sideview {
            header: [0, 0x60, 0, 0],
            commands: vec![
                Command::new(1, 3, 0x10),
                Command::new(50, ROW_EXTENDED, 0x50),
            ],
        };
        let raw = sv.encode().unwrap();
        assert_eq!(&raw[4..], &[0x31, 0x10, 0xE3, 0x00, 0xF2, 0x50]);
    }
}
