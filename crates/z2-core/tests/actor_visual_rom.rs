//! ROM-gated checks that enemy, NPC and boss actor anchors follow the
//! cartridge's displayed OAM (`actor_visual`): the ground anchor sits on the
//! object's own origin in both facings, the masked OAM is the slot's drawn
//! cells, and observing changes nothing.
//!
//! Scenes are loaded through the game's own loader (scene registers, then
//! game mode 0), as README.md describes. Nothing derived
//! from the ROM is stored. Skips (never fails) without `$Z2_ROM`.

#![cfg(feature = "interp")]

mod common;

use std::collections::BTreeMap;
use z2_core::actor_visual::{
    body_entries, character_key, group, ActorClass, ActorVisual, AnchorRule,
};
use z2_core::game::Game;

const A: u8 = 0x01;
const SELECT: u8 = 0x04;
const START: u8 = 0x08;
const RIGHT: u8 = 0x80;

fn register_all(g: &mut Game) {
    z2_core::bank7_traps::register_bank7_traps(g);
    z2_core::sideview_traps::register_sideview_traps(g);
    z2_core::sideview_traps::register_overworld_traps(g);
    z2_core::player_traps::register_player_traps(g);
    z2_core::enemy_traps::register_enemy_traps(g);
    z2_core::town_traps::register_town_traps(g);
    z2_core::palace_traps::register_palace_traps(g);
    z2_core::title_traps::register_title_traps(g);
    z2_core::boot_traps::register_boot_traps(g);
}

fn step_n(g: &mut Game, n: usize, input: u8) {
    for _ in 0..n {
        g.step(input);
    }
}

/// Fresh cartridge, slot 0 named, North Castle side view, then the scene
/// registers (`AAAA:VV/...`) and game mode 0 so the game loads that scene.
fn scene(test: &str, pokes: &str) -> Option<Game> {
    scene_with(test, pokes, None, |_| {})
}

/// [`scene`], with `setup` applied just before the scene loads (display or
/// widescreen options that must be on when its enemies spawn), and WRAM byte
/// `encounter.0` held at `encounter.1` while it loads: the overworld
/// encounter tables pick the side-view enemies this way (the art gates'
/// `@n:6A88:VV` pokes).
fn scene_with(
    test: &str,
    pokes: &str,
    encounter: Option<(usize, u8)>,
    setup: impl FnOnce(&mut Game),
) -> Option<Game> {
    let raw = common::rom_bytes(test)?;
    let mut g = Game::from_ines(&raw).ok()?;
    register_all(&mut g);
    g.reset();
    step_n(&mut g, 30, 0);
    step_n(&mut g, 5, START);
    step_n(&mut g, 20, 0);
    step_n(&mut g, 5, START);
    step_n(&mut g, 20, 0);
    for _ in 0..8 {
        step_n(&mut g, 2, A);
        step_n(&mut g, 8, 0);
    }
    for _ in 0..3 {
        step_n(&mut g, 2, SELECT);
        step_n(&mut g, 8, 0);
    }
    step_n(&mut g, 5, START);
    step_n(&mut g, 30, 0);
    step_n(&mut g, 5, START);
    step_n(&mut g, 1160, 0);
    assert_eq!(g.ram[0x0736], 0x0B, "side-view gameplay");
    let stats = "0777:08/0778:08/0779:08/0773:FF/0774:FF/0783:08/0784:08/0785:01";
    for poke in pokes.split('/').chain(stats.split('/')) {
        let (a, v) = poke.split_once(':').unwrap();
        g.ram[usize::from_str_radix(a, 16).unwrap()] = u8::from_str_radix(v, 16).unwrap();
    }
    setup(&mut g);
    g.ram[0x0736] = 0;
    for _ in 0..150 {
        if let Some((a, v)) = encounter {
            g.wram[a - 0x6000] = v;
        }
        g.step(0);
    }
    assert_eq!(g.ram[0x0736], 0x0B, "scene loaded");
    Some(g)
}

/// Screen X the display routine used for slot `s` (`$CD = $4E,x - $072C`,
/// `bank7_code29` `$DD20`) and its `$2A,x`, before a step.
fn origins(g: &Game) -> [(i32, i32); 6] {
    std::array::from_fn(|s| {
        (
            i32::from(g.ram[0x4E + s].wrapping_sub(g.ram[0x072C])),
            i32::from(g.ram[0x2A + s]),
        )
    })
}

/// The 8x16 sprite cell `tile` has an all-zero pattern (the `$F5` tile).
fn blank(g: &Game, tile: u8) -> bool {
    let at = g.mmc1.map_chr4(tile & 1) * 0x1000 + usize::from(tile & 0xFE) * 16;
    g.chr[at..at + 32].iter().all(|&b| b == 0)
}

/// Article-shot style input script (`_150,L6,B4,DB4`): buttons then frames.
fn script(s: &str) -> Vec<u8> {
    let mut out = Vec::new();
    for tok in s.split(',') {
        let split = tok.find(|c: char| c.is_ascii_digit()).unwrap_or(tok.len());
        let (keys, n) = tok.split_at(split);
        let b = keys.chars().fold(0u8, |b, c| {
            b | match c {
                'A' => A,
                'B' => 0x02,
                'U' => 0x10,
                'D' => 0x20,
                'L' => 0x40,
                'R' => RIGHT,
                _ => 0,
            }
        });
        out.extend(std::iter::repeat_n(b, n.parse().unwrap_or(1)));
    }
    out
}

/// Step `input`, handing each frame's visuals to `check`.
fn run(g: &mut Game, input: &[u8], mut check: impl FnMut(&Game, &[ActorVisual])) {
    for &b in input {
        g.step(b);
        let visuals = g.actor_visuals();
        check(g, &visuals);
    }
}

/// Place every live enemy 40 pixels and more to Link's left, so the
/// cartridge turns them round (`$DC91`).
fn move_behind_link(g: &mut Game) {
    let link = i32::from(g.ram[0x3B]) * 256 + i32::from(g.ram[0x4D]);
    for s in 0..6 {
        if g.ram[0xB6 + s] != 0 {
            let x = (link - 40 - 24 * s as i32).max(0);
            g.ram[0x4E + s] = x as u8;
            g.ram[0x3C + s] = (x >> 8) as u8;
        }
    }
}

#[derive(Default)]
struct Seen {
    facings: [bool; 2],
    rows: Vec<u8>,
    clips: Vec<&'static str>,
    frames: usize,
}

/// Step `frames` frames holding `input`, checking every visual against the
/// displayed OAM and the cartridge's object origin; returns what was seen
/// per character key.
fn observe(
    g: &mut Game,
    frames: usize,
    input: u8,
    turn_at: Option<usize>,
    seen: &mut BTreeMap<String, Seen>,
) {
    let mut last: BTreeMap<u8, (bool, f32)> = BTreeMap::new();
    for f in 0..frames {
        if Some(f) == turn_at {
            move_behind_link(g);
            last.clear();
        }
        let before = origins(g);
        g.step(input);
        let state = (g.ram, g.wram, g.cpu, g.mmc1, *g.oam());
        let visuals = g.actor_visuals();
        assert_eq!(
            (g.ram, g.wram, g.cpu, g.mmc1, *g.oam()),
            state,
            "observing never writes"
        );
        let oam = g.oam();
        let mut masks = 0u64;
        for v in &visuals {
            assert_eq!(masks & v.oam_mask, 0, "slots never share OAM");
            masks |= v.oam_mask;
            assert_eq!(v.oam_mask & !((1u64 << 56) - (1u64 << 20)), 0);
            let cells: Vec<_> = (0..64)
                .filter(|i| v.oam_mask & (1u64 << i) != 0)
                .map(|i| &oam[i * 4..i * 4 + 4])
                .collect();
            assert!(cells.iter().all(|c| c[0] < 0xEF), "only visible cells");
            assert!(
                cells.iter().all(|c| !blank(g, c[1])),
                "{} frame {f}: blank cells are never replaced",
                v.character
            );
            if v.rule == AnchorRule::Pairs {
                // The anchor is the bottom of a body column on the grid of
                // cells the cartridge actually drew: a cell at (anchor x - 8
                // or anchor x) in one of the body's rows lies in the mask,
                // even when the bottom row itself was not drawn.
                let [ax, ay] = v.position.map(|p| p as i32);
                assert!(
                    cells.iter().any(|c| {
                        let below = ay - (i32::from(c[0]) + 16);
                        (i32::from(c[3]) == ax - 8 || i32::from(c[3]) == ax)
                            && below >= 0
                            && below % 16 == 0
                            && below < 16 * i32::from(v.rows)
                    }),
                    "{} frame {f}: anchor {:?} not on a drawn body column",
                    v.character,
                    v.position
                );
                // ... and on the display routine's origin (`$00 = $2A,x`,
                // `$01 = $CD`), within a frame of motion or a type's own
                // display bob (the Tinsuit draws up to 2 pixels off `$2A`).
                let (cd, y) = before[usize::from(v.slot)];
                let dx = (ax - 8 - cd).rem_euclid(256);
                let dy = ay - 16 * i32::from(v.rows) - y;
                assert!(
                    (dx <= 3 || dx >= 253) && dy.abs() <= 3,
                    "{} frame {f}: anchor {:?} vs $CD {cd}, $2A {y}",
                    v.character,
                    v.position
                );
            }
            // Turning round keeps the anchor in place (the cartridge mirrors
            // tiles, never positions).
            if let Some(&(flip, x)) = last.get(&v.slot) {
                if flip != v.flip_h {
                    assert!(
                        (v.position[0] - x).abs() <= 3.0,
                        "{}: anchor jumped {x} -> {} on turning",
                        v.character,
                        v.position[0]
                    );
                }
            }
            last.insert(v.slot, (v.flip_h, v.position[0]));
            let e = seen.entry(v.character.clone()).or_default();
            e.facings[usize::from(v.flip_h)] = true;
            if !e.rows.contains(&v.rows) {
                e.rows.push(v.rows);
            }
            if !e.clips.contains(&v.clip) {
                e.clips.push(v.clip);
            }
            e.frames += 1;
        }
    }
}

#[test]
fn enemy_anchors_follow_displayed_oam_in_both_facings() {
    let Some(mut g) = scene(
        "enemy_anchors_both_facings",
        // Palace 3, a room with a red Iron Knuckle and Myus.
        "0706:00/0707:04/0748:36/056B:00/056C:02/075C:00/0701:00/0561:05",
    ) else {
        return;
    };
    let mut seen = BTreeMap::new();
    observe(&mut g, 100, RIGHT, None, &mut seen);
    observe(&mut g, 200, 0, Some(20), &mut seen);
    let ik = &seen["enemy_b4b_19"];
    assert_eq!(
        ik.facings,
        [true, true],
        "Iron Knuckle seen facing both ways"
    );
    assert_eq!(ik.rows, [2], "two 16-pixel body rows");
    assert!(ik.clips.contains(&"walk"));
    assert!(
        ik.clips.contains(&"attack_high") || ik.clips.contains(&"attack_low"),
        "{:?}",
        ik.clips
    );
    let myu = &seen["enemy_b4b_03"];
    assert_eq!(myu.rows, [1], "Myu is one row tall");
    assert!(myu.frames > 50);

    // Palace 1: the bouncing Tinsuit.
    let Some(mut g) = scene(
        "enemy_anchors_tinsuit",
        "0706:00/0707:03/0748:34/056B:00/056C:00/075C:00/0701:00/0561:04",
    ) else {
        return;
    };
    let mut seen = BTreeMap::new();
    observe(&mut g, 200, RIGHT, Some(170), &mut seen);
    observe(&mut g, 60, 0, None, &mut seen);
    let tinsuit = &seen["enemy_b4_0c"];
    assert_eq!(tinsuit.rows, [2]);
    assert!(tinsuit.clips.contains(&"jump"));

    // Palace 1 boss: Horsehead's body is three rows; its lance is a second
    // fixed block replaced with it.
    let Some(mut g) = scene(
        "enemy_anchors_horsehead",
        "0706:00/0707:03/0748:34/056B:00/056C:00/075C:00/0701:00/0561:0D",
    ) else {
        return;
    };
    let mut seen = BTreeMap::new();
    observe(&mut g, 300, RIGHT, None, &mut seen);
    observe(&mut g, 60, 0, None, &mut seen);
    let horsehead = &seen["enemy_b4_20"];
    assert_eq!(horsehead.rows, [3]);
    assert!(horsehead.clips.contains(&"attack"));
    let class = z2_core::actor_visual::alias("b4", 0x20).1;
    assert_eq!(class, ActorClass::Boss);
}

#[test]
fn town_people_are_npc_actors() {
    let Some(mut g) = scene(
        "town_people_are_npc_actors",
        "0706:00/0707:01/0748:2C/056B:00/056C:00/075C:00/0701:00/0561:01",
    ) else {
        return;
    };
    let mut seen = BTreeMap::new();
    observe(&mut g, 300, RIGHT, None, &mut seen);
    assert!(
        seen.keys().any(|k| k.starts_with("npc_b3_")),
        "{:?}",
        seen.keys().collect::<Vec<_>>()
    );
    assert!(!seen.contains_key("npc_b3_23"), "generators draw nothing");
    for (key, s) in &seen {
        assert_eq!(s.rows, [2], "{key}: townspeople are two rows tall");
        assert!(s.clips.iter().all(|c| matches!(*c, "idle" | "walk")));
    }
}

#[test]
fn iron_knuckles_report_high_and_low_thrusts() {
    // Palace 2, an orange Iron Knuckle room.
    let Some(mut g) = scene(
        "iron_knuckles_high_low",
        "0706:00/0707:03/0748:35/056B:00/056C:01/075C:02/0701:00/0561:10",
    ) else {
        return;
    };
    let mut seen = BTreeMap::new();
    observe(&mut g, 500, 0, None, &mut seen);
    let ik = &seen["enemy_b4_18"];
    assert!(
        ik.clips.contains(&"attack_high") && ik.clips.contains(&"attack_low"),
        "{:?}",
        ik.clips
    );
}

#[test]
fn deaths_keep_full_height_and_score_popups_stay_original() {
    // Palace 1 Tinsuits, attacked to death facing left.
    let Some(mut g) = scene(
        "deaths_and_scores",
        "0706:00/0707:03/0748:34/056B:00/056C:00/075C:02/0701:00/0561:01",
    ) else {
        return;
    };
    let input = script(&"L6,B4,_6,".repeat(40));
    let (mut deaths, mut scores) = (0, 0);
    run(&mut g, &input, |g, visuals| {
        let ram = g.shown_ram();
        for s in 0..6 {
            if ram[0xB6 + s] == 3 {
                scores += 1;
                assert!(
                    visuals.iter().all(|v| usize::from(v.slot) != s),
                    "score popups are never replaced"
                );
            }
        }
        for v in visuals
            .iter()
            .filter(|v| ram[0xB6 + usize::from(v.slot)] == 2)
        {
            deaths += 1;
            assert_eq!(v.clip, "death");
            assert_eq!(v.rows, 2, "the dying Tinsuit keeps its two-row anchor");
            // On the puff's row (`$2A,x` when it was drawn), two rows down.
            let dy = v.position[1] as i32 - i32::from(ram[0x2A + usize::from(v.slot)]) - 32;
            assert!(dy.abs() <= 3, "death anchor {:?}", v.position);
        }
    });
    assert!(
        deaths > 20 && scores > 20,
        "deaths {deaths}, scores {scores}"
    );
}

#[test]
fn carock_anchor_follows_its_body_not_the_multiplexed_block() {
    let Some(mut g) = scene(
        "carock_anchor",
        "0706:01/0707:04/0748:34/056B:00/056C:00/075C:00/0701:00/0561:1C",
    ) else {
        return;
    };
    let input = script(
        "R250,_10,B4,_14,B4,_20,DB4,_20,A16,_10,B4,_30,L16,_20,B4,_20,R30,_10,B4,_30,DB4,_40,B4,_60",
    );
    let mut last: Option<(usize, f32)> = None;
    let (mut steady, mut frame) = (0, 0);
    run(&mut g, &input, |_, visuals| {
        frame += 1;
        for v in visuals.iter().filter(|v| v.character == "enemy_b4b_22") {
            assert_eq!(v.rule, AnchorRule::Box);
            if let Some((f, x)) = last {
                let dx = (v.position[0] - x).abs();
                if f + 1 == frame && dx < 24.0 {
                    // Walking moves a pixel; the old block measurement
                    // jumped 16 pixels every other frame.
                    assert!(dx <= 3.0, "frame {frame}: Carock anchor jumped {dx}");
                    steady += 1;
                }
            }
            last = Some((frame, v.position[0]));
        }
    });
    assert!(
        steady > 100,
        "Carock observed on {steady} consecutive frames"
    );
}

#[test]
fn shared_ids_get_qualified_keys_with_the_plain_key_as_fallback() {
    // Palace 2's boss room: Helmethead (palace 5's Gooma has the same id).
    let Some(mut g) = scene(
        "helmethead_key",
        "0706:00/0707:03/0748:35/056B:00/056C:01/075C:00/0701:00/0561:22",
    ) else {
        return;
    };
    let mut helmethead = 0;
    run(&mut g, &script("R250,_10,B4,_14,B4,_60"), |_, visuals| {
        for v in visuals.iter().filter(|v| v.base_character == "enemy_b4_21") {
            assert_eq!(
                (v.character.as_str(), v.alias),
                ("enemy_b4_21_p2", "helmethead")
            );
            helmethead += 1;
        }
    });
    assert!(helmethead > 50, "Helmethead seen {helmethead} frames");

    // Kasuto's townspeople: keys carry the town's appearance index.
    let Some(mut g) = scene(
        "town_npc_keys",
        "0706:00/0707:01/0748:32/056B:03/056C:00/075C:00/0701:00/0561:09",
    ) else {
        return;
    };
    let mut npcs = 0;
    run(&mut g, &[0; 300], |g, visuals| {
        for v in visuals.iter().filter(|v| v.class == ActorClass::Npc) {
            let appearance = g.wram[0x0DD5 + usize::from(v.type_id)];
            assert_eq!(
                v.character,
                format!("{}_{appearance:02x}", v.base_character)
            );
            assert!(v.base_character.starts_with("npc_b3_"));
            npcs += 1;
        }
    });
    assert!(npcs > 100, "townspeople seen {npcs} frames");
}

#[test]
fn immobile_townspeople_stand_idle() {
    let Some(mut g) = scene(
        "immobile_townspeople",
        "0706:00/0707:01/0748:2C/056B:00/056C:00/075C:01/0701:00/0561:30",
    ) else {
        return;
    };
    let mut standing = 0;
    run(&mut g, &[0; 300], |g, visuals| {
        for v in visuals.iter().filter(|v| v.type_id == 0x11) {
            // `$71,x` is set, but the type never walks (`$6DF9,id` bit 7).
            assert_ne!(g.shown_ram()[0x71 + usize::from(v.slot)], 0);
            assert_eq!((v.clip, v.clip_fallback), ("idle", Some("walk")));
            standing += 1;
        }
    });
    assert!(
        standing > 100,
        "standing townsperson seen {standing} frames"
    );
}

#[test]
fn guma_restarts_its_throw_on_every_ball() {
    let Some(mut g) = scene(
        "guma_restarts",
        "0706:02/0707:03/0748:34/056B:00/056C:00/075C:03/0701:00/0561:2E",
    ) else {
        return;
    };
    let (mut restarts, mut throws) = (0, 0);
    run(&mut g, &[0; 400], |_, visuals| {
        for v in visuals.iter().filter(|v| v.character == "enemy_b4_1e") {
            throws += usize::from(v.clip == "throw");
            if v.restart {
                assert_eq!(v.clip, "throw");
                restarts += 1;
            }
        }
    });
    // One ball every 32 frames, and the clip name never changes between them.
    assert!(
        restarts >= 6 && throws > 200,
        "restarts {restarts}, throws {throws}"
    );
}

/// Every masked cell of `v` lies in the OAM and draws pixels; returns the
/// masked cells as `(y, x)` (OAM Y).
fn masked(g: &Game, v: &ActorVisual) -> Vec<(i32, i32)> {
    let oam = g.oam();
    (0..64)
        .filter(|i| v.oam_mask & (1u64 << i) != 0)
        .map(|i| {
            assert!(oam[i * 4] < 0xEF && !blank(g, oam[i * 4 + 1]));
            (i32::from(oam[i * 4]), i32::from(oam[i * 4 + 3]))
        })
        .collect()
}

/// Non-blank cells in enemy OAM (from `first`) that no visual claimed,
/// leaving out the boss life meter at X 16.
fn unclaimed(g: &Game, visuals: &[ActorVisual], first: usize) -> usize {
    let oam = g.oam();
    let all = visuals.iter().fold(0u64, |m, v| m | v.oam_mask);
    (first..56)
        .filter(|&i| {
            oam[i * 4] < 0xEF
                && !blank(g, oam[i * 4 + 1])
                && oam[i * 4 + 3] != 16
                && all & (1u64 << i) == 0
        })
        .count()
}

#[test]
fn dark_link_stands_on_his_feet_in_both_facings() {
    // Great Palace, Dark Link's room.
    let Some(mut g) = scene(
        "dark_link_anchor",
        "0706:02/0707:05/0748:36/056B:00/056C:02/075C:01/0701:00/0561:36",
    ) else {
        return;
    };
    let mut input = script("R250");
    input.extend(script(&"B4,_10,".repeat(40)));
    let (mut frames, mut facings) = (0, [false; 2]);
    run(&mut g, &input, |g, visuals| {
        for v in visuals.iter().filter(|v| v.character == "enemy_b5_23") {
            let cells = masked(g, v);
            // Link-style 3x2 drawer: the anchor is the bottom of his body,
            // the centre of the two columns behind the sword column.
            let feet = cells.iter().map(|c| c.0).max().unwrap() + 16;
            assert_eq!(v.position[1] as i32, feet, "{:?}", v.position);
            let top = cells.iter().map(|c| c.0).min().unwrap();
            if feet - top < 32 {
                // Before the fight a one-row figure stands in his slot.
                return;
            }
            assert_eq!(v.rows, 2);
            let xs: Vec<i32> = cells.iter().map(|c| c.1).collect();
            let x = v.position[0] as i32;
            assert!(xs.contains(&(x - 8)) && xs.contains(&x), "{x} vs {xs:?}");
            facings[usize::from(v.flip_h)] = true;
            frames += 1;
        }
    });
    assert!(frames > 100, "Dark Link seen {frames} frames");
    assert_eq!(facings, [true, true], "both facings");
}

#[test]
fn gooma_claims_his_whole_body_and_stands_on_his_feet() {
    // Palace 5's boss room.
    let Some(mut g) = scene(
        "gooma_body",
        "0706:02/0707:03/0748:34/056B:00/056C:00/075C:01/0701:00/0561:29",
    ) else {
        return;
    };
    let input = script(&"R6,B4,_6,".repeat(40));
    let mut frames = 0;
    run(&mut g, &input, |g, visuals| {
        for v in visuals.iter().filter(|v| v.base_character == "enemy_b4_21") {
            assert_eq!(v.character, "enemy_b4_21_p5");
            assert_eq!(v.rule, AnchorRule::Box);
            let cells = masked(g, v);
            let feet = cells.iter().map(|c| c.0).max().unwrap() + 16;
            assert_eq!(v.position[1] as i32, feet, "anchor on his feet");
            assert!(cells.len() >= 7, "body rows claimed: {}", cells.len());
            assert_eq!(unclaimed(g, visuals, 20), 0, "none of his cells left");
            frames += 1;
        }
    });
    assert!(frames > 100, "Gooma seen {frames} frames");
}

#[test]
fn king_bot_is_reported_on_every_drawn_frame() {
    // Great Palace room 0x30, page 3: King Bot wakes as Link walks in.
    let Some(mut g) = scene(
        "king_bot",
        "0706:02/0707:05/0748:36/056B:00/056C:02/075C:03/0701:00/0561:30",
    ) else {
        return;
    };
    let mut input = script("R60");
    input.extend([0; 240]);
    let (mut drawn, mut seen) = (0, 0);
    run(&mut g, &input, |g, visuals| {
        let oam = g.oam();
        let body = (20..32)
            .filter(|&i| oam[i * 4] < 0xEF && !blank(g, oam[i * 4 + 1]))
            .count();
        let ram = g.shown_ram();
        let king = (0..6).find(|&s| ram[0xB6 + s] == 1 && ram[0xA1 + s] == 0x1E);
        if body >= 8 && king.is_some() {
            drawn += 1;
            let v = visuals
                .iter()
                .find(|v| v.character == "enemy_b5_1e")
                .expect("King Bot drawn but not reported");
            assert_eq!(masked(g, v).len(), body, "his whole 6x2 figure");
            assert_eq!(v.rule, AnchorRule::Bbox);
            seen += 1;
        }
    });
    assert!(
        drawn > 100 && seen == drawn,
        "drawn {drawn}, reported {seen}"
    );
}

#[test]
fn barba_is_reported_with_his_head_as_anchor() {
    // Palace 6's boss room: hop the lava pillars from the left until the
    // screen locks and Barba rises.
    let Some(mut g) = scene(
        "barba",
        "0706:02/0707:04/0748:35/056B:00/056C:01/075C:00/0701:00/0561:3A/0793:03",
    ) else {
        return;
    };
    let input = script("R145,RA30,_2,RA30,_2,RA30,_2,RA30,_600");
    let mut frames = 0;
    run(&mut g, &input, |g, visuals| {
        for v in visuals.iter().filter(|v| v.character == "enemy_b4b_21") {
            assert_eq!(v.rule, AnchorRule::Head);
            let cells = masked(g, v);
            let head = cells.iter().map(|c| c.0).min().unwrap();
            assert_eq!(v.position[1] as i32, head + 16, "bottom of his head row");
            assert_eq!(unclaimed(g, visuals, 18), 0, "head and neck all claimed");
            frames += 1;
        }
    });
    assert!(frames > 100, "Barba seen {frames} frames");
}

#[test]
fn doomknocker_throws_on_its_timer_and_restarts_each_cycle() {
    // Palace 3, a Doomknocker room.
    let Some(mut g) = scene(
        "doomknocker",
        "0706:00/0707:04/0748:36/056B:00/056C:02/075C:01/0701:00/0561:01",
    ) else {
        return;
    };
    let (mut throws, mut restarts) = (0, 0);
    run(&mut g, &[0; 800], |g, visuals| {
        for v in visuals.iter().filter(|v| v.character == "enemy_b4b_1e") {
            let timer = g.shown_ram()[0x0504 + usize::from(v.slot)];
            if v.clip != "hurt" && v.clip != "death" {
                assert_eq!(v.clip == "throw", timer != 0, "throw while $0504 runs");
            }
            throws += usize::from(v.clip == "throw");
            restarts += usize::from(v.restart);
        }
    });
    assert!(
        throws > 30 && restarts >= 2,
        "throws {throws}, restarts {restarts}"
    );
}

#[test]
fn zora_surfaces_upright_on_its_feet_with_its_own_clips() {
    // East Hyrule swamp encounter with Zoras (the art gate's zora_run).
    let Some(mut g) = scene_with(
        "zora_surface",
        "0706:02/0707:00/0748:0A/056B:00/056C:00/075C:01/0701:00/075A:02",
        Some((0x6A88, 0x5D)),
        |_| {},
    ) else {
        return;
    };
    let (mut low, mut surfaced, mut throws) = (0, 0, 0);
    run(&mut g, &[0; 600], |g, visuals| {
        let ram = g.shown_ram();
        for v in visuals.iter().filter(|v| v.character == "enemy_b2_18") {
            if matches!(v.clip, "hurt" | "death") {
                continue;
            }
            let s = usize::from(v.slot);
            // The stay timer, as the display saw it (it runs before the AI
            // reloads the timer).
            let up = ram[0x0504 + s] != 0 && g.shown_prev_ram()[0x0504 + s] != 0;
            let cells = masked(g, v);
            let top = cells.iter().map(|c| c.0).min().unwrap();
            let feet = cells.iter().map(|c| c.0).max().unwrap() + 16;
            if up {
                // Upright: two rows from `$2A,x`, anchored at its feet like
                // the other two-row walkers.
                let expected = if ram[0x81 + s] != 0 {
                    ("surface_throw", Some("throw"))
                } else {
                    ("surface", Some("idle"))
                };
                assert_eq!((v.clip, v.clip_fallback), expected);
                assert_eq!((v.rule, v.rows), (AnchorRule::Pairs, 2));
                assert_eq!(feet - top, 32, "upright body is two rows");
                assert_eq!(v.position[1] as i32, feet, "anchor at its feet");
                surfaced += 1;
                throws += usize::from(v.clip == "surface_throw");
            } else {
                assert!(!v.clip.starts_with("surface"), "{}", v.clip);
                assert_eq!(feet - top, 16, "swimming body is one row");
                assert_eq!(v.position[1] as i32, feet, "same ground line");
                low += 1;
            }
        }
    });
    assert!(
        surfaced > 50 && low > 50 && throws > 0,
        "surfaced {surfaced}, low {low}, throws {throws}"
    );
}

/// Widescreen margin sprites of one frame against the visuals: every visible
/// margin cell a live slot drew (Deeler threads aside) belongs to that
/// slot's actor, and no two actors share one. Returns, per slot that drew
/// margin cells, its key and whether it was drawn only in a margin (no OAM).
fn margin_cells_replaced(g: &Game, visuals: &[ActorVisual]) -> Vec<(String, bool)> {
    let ram = g.shown_ram();
    let Some(group) = group(ram[0x0707], ram[0x0769]) else {
        return Vec::new();
    };
    let mut owned = 0u64;
    for v in visuals {
        assert_eq!(owned & v.margin_mask, 0, "slots never share margin sprites");
        owned |= v.margin_mask;
    }
    let sources = z2_core::wide_sprites::shown_sources(g);
    let sprites = z2_core::wide_sprites::shown(g);
    let mut drew = 0u8;
    for (i, (&(slot, _), m)) in sources.iter().zip(sprites).enumerate().take(64) {
        let s = usize::from(slot);
        let id = ram[0xA1 + s];
        if m.y >= 0xEF
            || blank(g, m.tile)
            || !matches!(ram[0xB6 + s], 1 | 2)
            || z2_core::actor_visual::alias(group, id).1 == ActorClass::Hidden
            || body_entries(group, id) != 0x3F
        {
            continue;
        }
        let key = character_key(group, id);
        let v = visuals
            .iter()
            .find(|v| v.slot == slot)
            .unwrap_or_else(|| panic!("{key} drew margin sprite {i} at x {} unreported", m.x));
        assert!(
            v.margin_mask & (1u64 << i) != 0,
            "{key}: margin sprite {i} at x {} left to the original art",
            m.x
        );
        drew |= 1 << s;
    }
    visuals
        .iter()
        .filter(|v| drew & (1 << v.slot) != 0)
        .map(|v| (v.base_character.clone(), v.oam_mask == 0))
        .collect()
}

#[test]
fn rebonack_in_the_right_margin_is_reported_with_its_margin_sprites() {
    // Palace 3's boss room, page 1: Rebonack waits at the right edge, past
    // the 256-pixel window, so the cartridge hides his columns in OAM and
    // widescreen draws them as margin sprites.
    let Some(mut g) = scene_with(
        "rebonack_margin",
        "0706:00/0707:04/0748:36/056B:00/056C:02/075C:01/0701:00/0561:0E",
        None,
        |g| {
            g.set_margin_sprites(true);
            g.set_wide_gameplay(Some(11));
        },
    ) else {
        return;
    };
    let mut input = vec![0; 300];
    input.extend(script("R60,_200"));
    let (mut margin_only, mut partial) = (0, 0);
    run(&mut g, &input, |g, visuals| {
        for (key, only) in margin_cells_replaced(g, visuals) {
            if key != "enemy_b4b_20" {
                continue;
            }
            let v = visuals.iter().find(|v| v.base_character == key).unwrap();
            // On his feet whether he is drawn in the window or the margin.
            let sy = i32::from(g.shown_ram()[0x2A + usize::from(v.slot)]);
            assert!(
                (v.position[1] as i32 - sy - 32).abs() <= 3,
                "anchor {:?} vs $2A {sy}",
                v.position
            );
            if only {
                margin_only += 1;
            } else {
                partial += 1;
            }
        }
    });
    assert!(
        margin_only > 30 && partial > 0,
        "margin only {margin_only}, partly in the window {partial}"
    );
}

#[test]
fn encounter_enemies_in_the_margins_are_reported_with_their_margin_sprites() {
    // Forest encounter with wide gameplay: enemies spawn out in the margins
    // and walk into the window (the blue Moblin take).
    let Some(mut g) = scene_with(
        "encounter_margin",
        "0706:01/0707:00/0748:FF/056B:00/056C:00/075C:00/0701:00/0563:06/075A:02/0759:00/0073:30/0709:00",
        None,
        |g| {
            g.set_margin_sprites(true);
            g.set_wide_gameplay(Some(11));
        },
    ) else {
        return;
    };
    let mut seen: BTreeMap<String, (usize, usize)> = BTreeMap::new();
    run(&mut g, &script("_200,R60,_200,L60,_200"), |g, visuals| {
        for (key, only) in margin_cells_replaced(g, visuals) {
            let e = seen.entry(key).or_default();
            if only {
                e.0 += 1;
            } else {
                e.1 += 1;
            }
        }
    });
    let moblin = seen.get("enemy_b1_16").copied().unwrap_or_default();
    assert!(
        moblin.0 > 30,
        "blue Moblins drawn only in a margin: {seen:?}"
    );
    assert!(
        seen.values().any(|&(only, partly)| only > 0 && partly > 0),
        "an enemy seen in a margin and crossing the window edge: {seen:?}"
    );
}

#[test]
fn helmethead_and_his_flying_helmets_each_keep_their_own_sprites() {
    // Palace 2's boss room: Helmethead throws his helmets (`enemy_b4_22`),
    // whose `$91,x` points at his block.
    let Some(mut g) = scene(
        "helmethead_helmets",
        "0706:00/0707:03/0748:35/056B:00/056C:01/075C:00/0701:00/0561:22",
    ) else {
        return;
    };
    let mut input = script("R250");
    input.extend(script(&"B4,_10,".repeat(80)));
    let (mut helmets, mut both) = (0, 0);
    run(&mut g, &input, |g, visuals| {
        let state = (g.ram, *g.oam(), g.collision_geometry());
        let again = g.actor_visuals();
        assert_eq!(
            (g.ram, *g.oam(), g.collision_geometry()),
            state,
            "read-only"
        );
        assert_eq!(again.len(), visuals.len());
        let ram = g.shown_ram();
        let oam = g.oam();
        let boss = visuals
            .iter()
            .find(|v| v.base_character == "enemy_b4_21" && ram[0xB6 + usize::from(v.slot)] == 1);
        for s in (0..6).filter(|&s| ram[0xB6 + s] == 1 && ram[0xA1 + s] == 0x22) {
            // The helmet's own sprite: tile $09 near ($CD + 3, $2A,x).
            let scroll = i32::from(ram[0x072A]) * 256 + i32::from(ram[0x072C]);
            let sx = i32::from(ram[0x3C + s]) * 256 + i32::from(ram[0x4E + s]) - scroll;
            let sy = i32::from(ram[0x2A + s]);
            // (The nearest one: at launch the head still on him is a $09 too.)
            let own = (20..56)
                .filter(|&i| oam[i * 4] < 0xEF && oam[i * 4 + 1] == 0x09)
                .map(|i| {
                    let d = (i32::from(oam[i * 4 + 3]) - sx - 3).abs()
                        + (i32::from(oam[i * 4]) - sy).abs();
                    (d, i)
                })
                .filter(|&(d, _)| d <= 16)
                .min()
                .map(|(_, i)| i);
            let Some(own) = own else { continue };
            let v = visuals
                .iter()
                .find(|v| usize::from(v.slot) == s)
                .expect("a drawn helmet is reported");
            assert_eq!(v.character, "enemy_b4_22");
            assert_eq!(v.oam_mask, 1u64 << own, "only its own sprite");
            assert_eq!(v.rule, AnchorRule::Bbox);
            let (x, y) = (i32::from(oam[own * 4 + 3]), i32::from(oam[own * 4]));
            assert_eq!(
                v.position,
                [(x + 4) as f32, (y + 16) as f32],
                "bottom centre"
            );
            if let Some(b) = boss {
                assert_eq!(
                    b.oam_mask & (1u64 << own),
                    0,
                    "Helmethead never takes a helmet"
                );
                both += 1;
            }
            helmets += 1;
        }
        if let Some(b) = boss {
            // His body: every drawn cell (the 2x2 block, the head on it;
            // fewer while he blinks after a hit) is his or a helmet's.
            assert!(!masked(g, b).is_empty());
            assert_eq!(unclaimed(g, visuals, 20), 0, "nothing of his left over");
            assert_eq!(b.character, "enemy_b4_21_p2");
        }
    });
    assert!(
        helmets > 200 && both > 200,
        "helmets {helmets}, with Helmethead {both}"
    );
}
