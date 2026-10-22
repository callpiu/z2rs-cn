//! `xtask content-survey`: verified memory facts for the `content`
//! enhancement group (virtual rooms + new overworld entrances).
//!
//! ```text
//! cargo xtask content-survey [--rom PATH] [--movie M]... [--frames N] [--no-sweep]
//! ```
//!
//! Three surveys, all printed as addresses / indices / counts only (never
//! ROM bytes):
//!
//! 1. **WRAM access survey.** Every movie is replayed twice in lockstep on
//!    one thread: once with the full trap table (the shipping hybrid) and
//!    once with the table off (pure interpretation of the original code).
//!    Both runs carry the `z2-core` `wram-probe` bus hook, which counts
//!    every `$6000-$7FFF` read and write made by interpreted code and by
//!    trap bodies that use the bus. Trap bodies that index `Game::wram`
//!    directly are covered twice over: by the untrapped run (the original
//!    code they replace goes through the bus) and by a frame-by-frame WRAM
//!    diff of the trapped run (any value change counts as a write). The
//!    two runs' CPU RAM + WRAM are compared every frame and the bytes that
//!    differ are tallied, so a desync is reported, not hidden.
//!
//!    The corpus movies never enter a palace, East Hyrule or Maze Island,
//!    so a **scene sweep** follows: from a mid-movie side-view state, every
//!    reachable map of the town and palace sets is loaded through the
//!    game's own loader (scene registers, then `$0736 = 0`; see
//!    README.md) and played for a few hundred frames
//!    on a scripted pad, and every key area of each continent is entered
//!    the same way. The sweep runs on the same trapped/untrapped pair.
//! 2. **Map-slot survey.** For each sideview table set, the map indices
//!    reachable from the vanilla data: overworld location tables (every
//!    slot with any nonzero byte), the random-battle table, then the
//!    closure over the connection bytes (and the town door table), plus
//!    the Great Palace ending maps. Cross-checked against the maps the
//!    runs actually load (`$0707`/`$0706`/`$0561` in side-scroll mode,
//!    with `$D4/$D5` compared against the assumed table entry).
//! 3. **Progress bits.** OR of every value the save-mirrored CPU RAM
//!    (`$0600-$06DF`, `$0777-$07A8`) holds at any movie frame end, so bits
//!    that are never set can be told apart from bits the game uses.
//!
//! Results feed README.md and
//! `crates/z2-core/src/enh/content_mem.rs`.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use z2_core::game::Game;
use z2_core::state::GameState;
use z2_core::wram_probe::{self, WramAccess};

const WRAM_LEN: usize = 0x2000;
const RAM_LEN: usize = 0x800;
const MAPS: usize = 63;

fn usage() -> &'static str {
    "cargo xtask content-survey [--rom PATH] [--movie M.fm2|M.bk2]... [--frames N] [--no-sweep]\n\
     Default movies: every .fm2/.bk2 in $Z2_CORPUS/movies (else ../z2-corpus/movies).\n\
     --frames N caps each movie (0 = whole movie, the default).\n\
     --no-sweep skips the scene sweep (movies only)."
}

// ------------------------------------------------------------------ ROM view

/// Read-only CPU view of one PRG bank (`$8000-$BFFF` switchable, `$C000+`
/// fixed bank 7) over the hash-gated ROM body.
struct Prg<'a>(&'a [u8]);

impl Prg<'_> {
    fn rd(&self, bank: u8, addr: u16) -> u8 {
        let off = if addr >= 0xC000 {
            7 * 0x4000 + usize::from(addr - 0xC000)
        } else {
            usize::from(bank) * 0x4000 + usize::from(addr - 0x8000)
        };
        self.0[off]
    }
    fn word(&self, bank: u8, addr: u16) -> u16 {
        u16::from(self.rd(bank, addr)) | (u16::from(self.rd(bank, addr.wrapping_add(1))) << 8)
    }
}

/// One 63-map sideview table set.
#[derive(Clone, Copy)]
struct TableSet {
    name: &'static str,
    bank: u8,
    sideview: u16,
    conn: u16,
    /// Town door table (`bank7_take_door_exit` reads `$8817`).
    door: Option<u16>,
    /// Part-2 item-presence block (`$0600 + 32 * n`).
    part2: u16,
}

const fn set(
    name: &'static str,
    bank: u8,
    second: bool,
    door: Option<u16>,
    part2: u16,
) -> TableSet {
    TableSet {
        name,
        bank,
        sideview: if second { 0xA000 } else { 0x8523 },
        conn: if second { 0xA1F8 } else { 0x871B },
        door,
        part2,
    }
}

const SETS: [TableSet; 8] = [
    set("W0 West Hyrule", 1, false, None, 0x0600),
    set("W0 Death Mountain", 1, true, None, 0x0620),
    set("W0 East Hyrule", 2, false, None, 0x0640),
    set("W0 Maze Island", 2, true, None, 0x0620),
    set("W1/W2 towns", 3, false, Some(0x8817), 0x0660),
    set("W3 palace group A (1,2,5)", 4, false, None, 0x0680),
    set("W4 palace group B (3,4,6)", 4, true, None, 0x06A0),
    set("W5 Great Palace", 5, false, None, 0x06C0),
];

/// Table set for world `$0707` / region `$0706` / PRG bank register.
///
/// Region 1 is both Death Mountain (bank 1) and Maze Island (bank 2); the
/// bank in use tells them apart.
fn set_of(world: u8, region: u8, bank: u8) -> Option<usize> {
    match (world, region) {
        (0, 0) => Some(0),
        (0, 1) => Some(if bank == 2 { 3 } else { 1 }),
        (0, 2) => Some(2),
        (1 | 2, _) => Some(4),
        (3, _) => Some(5),
        (4, _) => Some(6),
        (5, _) => Some(7),
        _ => None,
    }
}

/// Overworld location table per continent: (name, bank, base, own set).
const LOCS: [(&str, u8, u16, usize); 4] = [
    ("West Hyrule", 1, 0x861F, 0),
    ("Death Mountain", 1, 0xA0FC, 1),
    ("East Hyrule", 2, 0x861F, 2),
    ("Maze Island", 2, 0xA0FC, 3),
];

/// Random-battle map table (`$8409` in the overworld bank, 9 terrains x 2).
const RANDOM_BATTLE: u16 = 0x8409;
const RANDOM_BATTLE_LEN: u16 = 18;

struct MapSurvey {
    used: Vec<BTreeSet<u8>>,
    roots: Vec<BTreeSet<u8>>,
}

fn loc_bytes(prg: &Prg, bank: u8, base: u16, i: u16) -> [u8; 4] {
    std::array::from_fn(|k| prg.rd(bank, base + i + 63 * k as u16))
}

/// Set a location-table entry leads into (`sv_entry_body`: only external
/// entries change region, from byte 3 bits 0-1; others keep the
/// continent's own region and bank).
fn loc_target(b: [u8; 4], bank: u8, own: usize) -> Option<usize> {
    let world = (b[3] >> 2) & 7;
    if world != 0 {
        return set_of(world, b[3] & 3, bank);
    }
    if b[0] & 0x80 != 0 {
        set_of(0, b[3] & 3, bank)
    } else {
        Some(own)
    }
}

fn static_maps(prg: &Prg) -> MapSurvey {
    let mut roots: Vec<BTreeSet<u8>> = vec![BTreeSet::new(); SETS.len()];
    for &(_, bank, base, own) in &LOCS {
        for i in 0..MAPS as u16 {
            let b = loc_bytes(prg, bank, base, i);
            if b == [0; 4] {
                continue;
            }
            if let Some(s) = loc_target(b, bank, own) {
                roots[s].insert(b[2] & 0x3F);
            }
        }
    }
    for (bank, sets) in [(1u8, [0usize, 1]), (2, [2, 3])] {
        for i in 0..RANDOM_BATTLE_LEN {
            let m = prg.rd(bank, RANDOM_BATTLE + i) & 0x3F;
            for s in sets {
                roots[s].insert(m);
            }
        }
    }
    // Great Palace ending scenes.
    roots[7].extend([61u8, 62]);
    // Every palace / town restart lands on map 0 of its set.
    for r in roots.iter_mut().skip(4) {
        r.insert(0);
    }
    let mut used = roots.clone();
    for (s, set) in SETS.iter().enumerate() {
        let mut todo: Vec<u8> = used[s].iter().copied().collect();
        let push = |m: u8, used: &mut BTreeSet<u8>, todo: &mut Vec<u8>| {
            if usize::from(m) < MAPS && used.insert(m) {
                todo.push(m);
            }
        };
        let mut area0_done = false;
        while let Some(m) = todo.pop() {
            let mut sources = vec![m];
            // World 0: areas >= $1D take side exits through area 0's bytes.
            if s < 4 && m >= 0x1D && !area0_done {
                area0_done = true;
                sources.push(0);
            }
            for src in sources {
                for page in 0..4u16 {
                    let c = prg.rd(set.bank, set.conn + u16::from(src) * 4 + page);
                    if c & 0xFC != 0xFC {
                        push(c >> 2, &mut used[s], &mut todo);
                    }
                    if let Some(d) = set.door {
                        let c = prg.rd(set.bank, d + u16::from(src) * 4 + page);
                        if c & 0xFC != 0xFC {
                            push(c >> 2, &mut used[s], &mut todo);
                        }
                    }
                }
            }
        }
    }
    MapSurvey { used, roots }
}

// ------------------------------------------------------------- replay side

/// (world, region, area) of a side-scroll load.
type LoadKey = (u8, u8, u8);

/// Movie frame whose state seeds the scene sweep (the `make hd-sheets`
/// scene specs graft from the same any% frame: Link on the overworld).
const GRAFT_FRAME: u32 = 600;

/// Trapped + untrapped games stepped in lockstep, each with its own probe
/// record, plus everything the survey accumulates from them.
struct Pair {
    a: Game,
    b: Game,
    ra: Option<WramAccess>,
    rb: Option<WramAccess>,
    prev: Box<[u8; WRAM_LEN]>,
    frame: u32,
    diff_writes: Vec<u32>,
    desync_frames: usize,
    first_desync: Option<usize>,
    ram_diff: Vec<u32>,
    wram_diff: Vec<u32>,
    /// Load key -> `$D4/$D5` at the first side-scroll frame.
    loads: BTreeMap<LoadKey, u16>,
    last_key: Option<LoadKey>,
    or_part1: Vec<u8>,
    or_part2: Vec<u8>,
    /// State at movie frame [`GRAFT_FRAME`] (graft source for the sweep).
    ref_state: Option<GameState>,
}

fn build(raw: &[u8], traps: bool) -> Result<Game, String> {
    let mut g = Game::from_ines(raw).map_err(|e| format!("build Game: {e:?}"))?;
    g.reset();
    if traps {
        crate::verify::register_all_traps(&mut g);
    }
    Ok(g)
}

fn fresh_record() -> Option<WramAccess> {
    wram_probe::arm();
    wram_probe::take()
}

impl Pair {
    fn new(raw: &[u8]) -> Result<Pair, String> {
        let a = build(raw, true)?;
        let b = build(raw, false)?;
        let prev = Box::new(a.wram);
        Ok(Pair {
            a,
            b,
            ra: fresh_record(),
            rb: fresh_record(),
            prev,
            frame: 0,
            diff_writes: vec![0; WRAM_LEN],
            desync_frames: 0,
            first_desync: None,
            ram_diff: vec![0; RAM_LEN],
            wram_diff: vec![0; WRAM_LEN],
            loads: BTreeMap::new(),
            last_key: None,
            or_part1: vec![0; 0x32],
            or_part2: vec![0; 0xE0],
            ref_state: None,
        })
    }

    /// Load the same state into both games, then apply `pokes`.
    fn graft(&mut self, st: &GameState, pokes: &[(u16, u8)]) {
        self.a.load_state(st);
        self.b.load_state(st);
        for g in [&mut self.a, &mut self.b] {
            for &(addr, v) in pokes {
                if addr < 0x800 {
                    g.ram[usize::from(addr)] = v;
                } else {
                    g.wram[usize::from(addr - 0x6000)] = v;
                }
            }
        }
        *self.prev = self.a.wram;
        self.last_key = None;
    }

    fn step(&mut self, input: u8, movie: bool) {
        let f = self.frame;
        for (g, rec) in [(&mut self.a, &mut self.ra), (&mut self.b, &mut self.rb)] {
            if let Some(r) = rec.take() {
                wram_probe::restore(r);
            }
            wram_probe::set_frame(f);
            g.step(input);
            *rec = wram_probe::take();
        }
        for (i, (p, &c)) in self.prev.iter_mut().zip(self.a.wram.iter()).enumerate() {
            if *p != c {
                self.diff_writes[i] = self.diff_writes[i].saturating_add(1);
                *p = c;
            }
        }
        if self.a.ram != self.b.ram || self.a.wram != self.b.wram {
            self.desync_frames += 1;
            self.first_desync.get_or_insert(f as usize);
            for i in 0..RAM_LEN {
                if self.a.ram[i] != self.b.ram[i] {
                    self.ram_diff[i] += 1;
                }
            }
            for i in 0..WRAM_LEN {
                if self.a.wram[i] != self.b.wram[i] {
                    self.wram_diff[i] += 1;
                }
            }
        }
        let r = &self.a.ram;
        if movie {
            for (o, &v) in self.or_part2.iter_mut().zip(&r[0x600..0x6E0]) {
                *o |= v;
            }
            for (o, &v) in self.or_part1.iter_mut().zip(&r[0x777..0x7A9]) {
                *o |= v;
            }
        }
        if r[0x736] == 0x0B {
            let key = (r[0x707], r[0x706], r[0x561]);
            if self.last_key != Some(key) {
                let d4 = u16::from(r[0xD4]) | (u16::from(r[0xD5]) << 8);
                self.loads.entry(key).or_insert(d4);
                self.last_key = Some(key);
            }
        }
        if movie && f == GRAFT_FRAME {
            self.ref_state = Some(self.a.save_state());
        }
        self.frame += 1;
    }
}

struct MovieResult {
    name: String,
    frames: usize,
    exec_errors: (u64, u64),
    pair: Pair,
}

fn replay(raw: &[u8], path: &Path, frames_cap: usize) -> Result<MovieResult, String> {
    let track = crate::verify::load_movie_track(path)?;
    let n = if frames_cap == 0 {
        track.len()
    } else {
        frames_cap.min(track.len())
    };
    let mut p = Pair::new(raw)?;
    for &input in track.iter().take(n) {
        p.step(input, true);
    }
    Ok(MovieResult {
        name: path
            .file_name()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default(),
        frames: n,
        exec_errors: (p.a.exec_errors, p.b.exec_errors),
        pair: p,
    })
}

// ------------------------------------------------------------- scene sweep

const PAD_A: u8 = 0x01;
const PAD_LEFT: u8 = 0x40;
const PAD_RIGHT: u8 = 0x80;

/// Scripted pad for a swept scene: walk right jumping, then walk back.
fn sweep_input(i: usize) -> u8 {
    if i < 260 {
        PAD_RIGHT | if (i / 24).is_multiple_of(2) { PAD_A } else { 0 }
    } else {
        PAD_LEFT
    }
}

const SWEEP_FRAMES: usize = 400;

/// Scene-register pokes that make the game run its own load of `map` in
/// set `s` (README.md, "Loading any scene").
fn scene_pokes(s: usize, map: u8) -> Vec<(u16, u8)> {
    let (world, region, key) = match s {
        4 if map < 12 || (36..48).contains(&map) => (1u8, 0u8, 0x2C + 2 * ((map % 12) / 3)),
        4 => (2, 2, 0x2C + 2 * ((map % 12) / 3)),
        5 => (3, 0, 0x34),
        6 => (4, 0, 0x36),
        _ => (5, 2, 0x36),
    };
    let town = if s == 4 {
        (key - 0x2C) / 2 + if world == 2 { 4 } else { 0 }
    } else {
        0
    };
    vec![
        (0x0785, 1), // candle, so caves draw
        (0x0706, region),
        (0x0707, world),
        (0x0748, key),
        (0x056B, town),
        (0x056C, key.wrapping_sub(0x34)),
        (0x0561, map),
        (0x075C, 0),
        (0x0701, 0),
        (0x0709, 0),
        (0x0736, 0),
    ]
}

/// World-0 key-area entry pokes (world 0 takes the scene from `$0748`).
fn key_area_pokes(region: u8, key: u8) -> Vec<(u16, u8)> {
    vec![
        (0x0785, 1),
        (0x0706, region),
        (0x0707, 0),
        (0x0748, key),
        (0x0709, 0),
        (0x075C, 0),
        (0x0701, 0),
        (0x0736, 0),
    ]
}

/// Overworld load pokes (`$0709 = 1`, stand on key area `key`).
fn overworld_pokes(region: u8, key: u8) -> Vec<(u16, u8)> {
    vec![
        (0x0706, region),
        (0x0707, 0),
        (0x0748, key),
        (0x0709, 1),
        (0x0736, 0),
    ]
}

fn run_graft(p: &mut Pair, st: &GameState, pokes: &[(u16, u8)], frames: usize) {
    p.graft(st, pokes);
    for i in 0..frames {
        p.step(sweep_input(i), false);
    }
}

/// One sweep job: every reachable map of set `s` (or, for a world-0 set,
/// every key area of its continent), grafted from `ref_state`.
fn sweep(
    raw: &[u8],
    prg: &Prg,
    used: &BTreeSet<u8>,
    s: usize,
    ref_state: &GameState,
) -> Result<(Pair, usize), String> {
    let mut p = Pair::new(raw)?;
    let mut grafts = 0;
    if s >= 4 {
        for &map in used {
            run_graft(&mut p, ref_state, &scene_pokes(s, map), SWEEP_FRAMES);
            grafts += 1;
        }
        return Ok((p, grafts));
    }
    // World 0: first load the continent's overworld (the reference state
    // is already in West Hyrule), then enter each key area from there.
    let (_, bank, base, _) = LOCS[s];
    let region = [0u8, 1, 2, 1][s];
    let base_state = match s {
        0 => ref_state.clone(),
        1 => {
            run_graft(&mut p, ref_state, &overworld_pokes(1, 42), 300);
            p.a.save_state()
        }
        _ => {
            run_graft(&mut p, ref_state, &overworld_pokes(2, 45), 300);
            let east = p.a.save_state();
            if s == 2 {
                east
            } else {
                run_graft(&mut p, &east, &overworld_pokes(1, 40), 300);
                p.a.save_state()
            }
        }
    };
    for k in 0..0x3E_u16 {
        let b = loc_bytes(prg, bank, base, k);
        // Entries into towns / palaces are swept with their own world.
        if b == [0; 4] || (b[3] >> 2) & 7 != 0 {
            continue;
        }
        run_graft(&mut p, &base_state, &key_area_pokes(region, k as u8), 300);
        grafts += 1;
    }
    Ok((p, grafts))
}

// ------------------------------------------------------------------ report

fn default_movies() -> Vec<PathBuf> {
    let dir = std::env::var("Z2_CORPUS")
        .map(|d| PathBuf::from(d).join("movies"))
        .unwrap_or_else(|_| PathBuf::from("../z2-corpus/movies"));
    let mut v: Vec<PathBuf> = std::fs::read_dir(&dir)
        .map(|rd| {
            rd.filter_map(|e| e.ok().map(|e| e.path()))
                .filter(|p| matches!(p.extension().and_then(|e| e.to_str()), Some("fm2" | "bk2")))
                .collect()
        })
        .unwrap_or_default();
    v.sort();
    v
}

/// Merge consecutive offsets satisfying `pred` into `$6000`-based ranges.
fn ranges(pred: impl Fn(usize) -> bool) -> Vec<(u16, u16)> {
    let mut v = Vec::new();
    let mut start = None;
    for i in 0..=WRAM_LEN {
        let hit = i < WRAM_LEN && pred(i);
        match (hit, start) {
            (true, None) => start = Some(i),
            (false, Some(s)) => {
                v.push((0x6000 + s as u16, 0x6000 + i as u16 - 1));
                start = None;
            }
            _ => {}
        }
    }
    v
}

fn print_ranges(title: &str, r: &[(u16, u16)], min: usize) {
    let total: usize = r.iter().map(|(a, b)| usize::from(b - a) + 1).sum();
    println!(
        "{title}: {total} bytes in {} runs (runs >= {min} B listed)",
        r.len()
    );
    for (a, b) in r {
        let len = usize::from(b - a) + 1;
        if len >= min {
            println!("  ${a:04X}-${b:04X}  {len:5} B");
        }
    }
}

/// Union of per-byte access counts over a set of pairs.
struct Union {
    reads: Vec<u64>,
    writes: Vec<u64>,
    first_write: Vec<u32>,
    trapped: Vec<bool>,
    untrapped: Vec<bool>,
}

fn union<'a>(pairs: impl Iterator<Item = &'a Pair>) -> Union {
    let mut u = Union {
        reads: vec![0; WRAM_LEN],
        writes: vec![0; WRAM_LEN],
        first_write: vec![u32::MAX; WRAM_LEN],
        trapped: vec![false; WRAM_LEN],
        untrapped: vec![false; WRAM_LEN],
    };
    for p in pairs {
        for (rec, trapped) in [(&p.ra, true), (&p.rb, false)] {
            let Some(rec) = rec else { continue };
            for i in 0..WRAM_LEN {
                u.reads[i] += u64::from(rec.reads[i]);
                u.writes[i] += u64::from(rec.writes[i]);
                u.first_write[i] = u.first_write[i].min(rec.first_write[i]);
                let hit = rec.reads[i] > 0 || rec.writes[i] > 0;
                if trapped {
                    u.trapped[i] |= hit;
                } else {
                    u.untrapped[i] |= hit;
                }
            }
        }
        for i in 0..WRAM_LEN {
            u.writes[i] += u64::from(p.diff_writes[i]);
            u.trapped[i] |= p.diff_writes[i] > 0;
        }
    }
    u
}

fn report_wram(title: &str, u: &Union) {
    println!("\n== WRAM $6000-$7FFF, {title}");
    print_ranges(
        "never read or written",
        &ranges(|i| u.reads[i] == 0 && u.writes[i] == 0),
        1,
    );
    print_ranges(
        "written, never read",
        &ranges(|i| u.reads[i] == 0 && u.writes[i] > 0),
        16,
    );
    print_ranges(
        "read, never written",
        &ranges(|i| u.reads[i] > 0 && u.writes[i] == 0),
        1,
    );
    print_ranges(
        "touched by the untrapped run only",
        &ranges(|i| u.untrapped[i] && !u.trapped[i]),
        1,
    );
    print_ranges(
        "touched by the trapped run only",
        &ranges(|i| u.trapped[i] && !u.untrapped[i]),
        1,
    );
    println!("candidate gaps:");
    for (a, b) in [
        (0x6C58u16, 0x6CFFu16),
        (0x6E66, 0x6FFF),
        (0x773C, 0x7918),
        (0x7F80, 0x7FFF),
        (0x6338, 0x6956),
        // Connection bytes map 63 would use ($6AFC + 63 * 4).
        (0x6BF8, 0x6BFB),
        // Overworld map window after the randomizer's size patch.
        (0x7A00, 0x7BFF),
    ] {
        let (lo, hi) = (usize::from(a - 0x6000), usize::from(b - 0x6000));
        let touched = ranges(|i| (lo..=hi).contains(&i) && (u.reads[i] > 0 || u.writes[i] > 0));
        let n: usize = touched.iter().map(|(x, y)| usize::from(y - x) + 1).sum();
        print!("  ${a:04X}-${b:04X} ({:4} B): ", hi - lo + 1);
        if n == 0 {
            println!("never touched");
            continue;
        }
        print!("{n} B touched:");
        for (x, y) in touched.iter().take(12) {
            let r = usize::from(*x - 0x6000)..=usize::from(*y - 0x6000);
            let fw = r
                .clone()
                .map(|i| u.first_write[i])
                .min()
                .unwrap_or(u32::MAX);
            let rd: u64 = r.map(|i| u.reads[i]).sum();
            let fw = if fw == u32::MAX {
                "-".to_string()
            } else {
                format!("f{fw}")
            };
            print!(" ${x:04X}-${y:04X}(reads {rd}, first write {fw})");
        }
        println!();
    }
}

fn report_desync(pairs: &[&Pair]) {
    let mut ram = vec![0u64; RAM_LEN];
    let mut wram = vec![0u64; WRAM_LEN];
    for p in pairs {
        for (o, &n) in ram.iter_mut().zip(&p.ram_diff) {
            *o += u64::from(n);
        }
        for (o, &n) in wram.iter_mut().zip(&p.wram_diff) {
            *o += u64::from(n);
        }
    }
    let mut top: Vec<(u64, String)> = ram
        .iter()
        .enumerate()
        .filter(|(_, &n)| n > 0)
        .map(|(i, &n)| (n, format!("${i:04X}")))
        .chain(
            wram.iter()
                .enumerate()
                .filter(|(_, &n)| n > 0)
                .map(|(i, &n)| (n, format!("${:04X}", 0x6000 + i))),
        )
        .collect();
    top.sort_by_key(|a| std::cmp::Reverse(a.0));
    let wram_bytes = wram.iter().filter(|&&n| n > 0).count();
    let ram_bytes = ram.iter().filter(|&&n| n > 0).count();
    println!(
        "  trapped-vs-untrapped differing bytes: {ram_bytes} CPU RAM, {wram_bytes} WRAM; most frequent (frames): {}",
        top.iter().take(16).map(|(n, a)| format!("{a}:{n}")).collect::<Vec<_>>().join(" ")
    );
    let wl: Vec<String> = wram
        .iter()
        .enumerate()
        .filter(|(_, &n)| n > 0)
        .map(|(i, &n)| format!("${:04X}:{n}", 0x6000 + i))
        .collect();
    println!("  differing WRAM bytes (frames): {}", wl.join(" "));
}

pub fn run(args: &[String]) -> i32 {
    let mut rom: Option<String> = None;
    let mut movies: Vec<PathBuf> = Vec::new();
    let mut frames = 0usize;
    let mut do_sweep = true;
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--rom" => rom = it.next().cloned(),
            "--movie" => movies.extend(it.next().map(PathBuf::from)),
            "--frames" => frames = it.next().and_then(|s| s.parse().ok()).unwrap_or(0),
            "--no-sweep" => do_sweep = false,
            "-h" | "--help" => {
                println!("{}", usage());
                return 0;
            }
            other => {
                eprintln!("content-survey: unknown argument '{other}'\n{}", usage());
                return 2;
            }
        }
    }
    let rom = match crate::verify::rom_path(rom.as_deref()) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("content-survey: {e}");
            return 2;
        }
    };
    let body = match z2_assets::rom::open_at(&rom) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("content-survey: ROM gate: {e:?}");
            return 2;
        }
    };
    let raw = match std::fs::read(&rom) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("content-survey: read {}: {e}", rom.display());
            return 2;
        }
    };
    if movies.is_empty() {
        movies = default_movies();
    }
    let prg = Prg(&body);
    let ms = static_maps(&prg);

    // ---- movie replays (one thread per movie)
    let results: Vec<Result<MovieResult, String>> = std::thread::scope(|s| {
        let hs: Vec<_> = movies
            .iter()
            .map(|m| {
                let raw = &raw;
                std::thread::Builder::new()
                    .stack_size(64 << 20)
                    .spawn_scoped(s, move || replay(raw, m, frames))
                    .expect("spawn")
            })
            .collect();
        hs.into_iter()
            .map(|h| {
                h.join()
                    .unwrap_or_else(|_| Err("replay thread panicked".into()))
            })
            .collect()
    });
    let mut ok: Vec<MovieResult> = Vec::new();
    println!("== movie coverage");
    for r in results {
        match r {
            Ok(m) => {
                let p = &m.pair;
                println!(
                    "  {:22} {:7} frames  exec_errors trapped/untrapped {}/{}  RAM+WRAM differ on {} frames (first {:?})  side-scroll loads {}",
                    m.name,
                    m.frames,
                    m.exec_errors.0,
                    m.exec_errors.1,
                    p.desync_frames,
                    p.first_desync,
                    p.loads.len(),
                );
                ok.push(m);
            }
            Err(e) => println!("  FAILED: {e}"),
        }
    }
    let total: usize = ok.iter().map(|m| m.frames).sum();
    println!("  total {total} frames over {} movies", ok.len());
    report_desync(&ok.iter().map(|m| &m.pair).collect::<Vec<_>>());
    report_wram("movies only", &union(ok.iter().map(|m| &m.pair)));

    // ---- scene sweep (one thread per table set)
    let ref_state = ok
        .iter()
        .filter(|m| m.name.starts_with("anypct"))
        .chain(ok.iter())
        .find_map(|m| m.pair.ref_state.clone());
    let mut swept: Vec<Pair> = Vec::new();
    if do_sweep {
        match &ref_state {
            None => println!("\n== scene sweep skipped: no movie reached the graft frame"),
            Some(st) => {
                let res: Vec<Result<(Pair, usize), String>> = std::thread::scope(|s| {
                    let hs: Vec<_> = (0..SETS.len())
                        .map(|k| {
                            let (raw, prg, used) = (&raw, &prg, &ms.used[k]);
                            std::thread::Builder::new()
                                .stack_size(64 << 20)
                                .spawn_scoped(s, move || sweep(raw, prg, used, k, st))
                                .expect("spawn")
                        })
                        .collect();
                    hs.into_iter()
                        .map(|h| {
                            h.join()
                                .unwrap_or_else(|_| Err("sweep thread panicked".into()))
                        })
                        .collect()
                });
                println!("\n== scene sweep ({SWEEP_FRAMES} frames per town/palace map, 300 per world-0 key area)");
                for (k, r) in res.into_iter().enumerate() {
                    match r {
                        Ok((p, n)) => {
                            println!(
                                "  {:28} {n:3} grafts, {:6} frames, exec_errors {}/{}, side-scroll loads {}",
                                SETS[k].name,
                                p.frame,
                                p.a.exec_errors,
                                p.b.exec_errors,
                                p.loads.len()
                            );
                            swept.push(p);
                        }
                        Err(e) => println!("  {}: FAILED: {e}", SETS[k].name),
                    }
                }
                report_desync(&swept.iter().collect::<Vec<_>>());
                report_wram(
                    "movies + scene sweep",
                    &union(ok.iter().map(|m| &m.pair).chain(swept.iter())),
                );
            }
        }
    }

    // ---- map slots
    let mut observed: Vec<BTreeSet<u8>> = vec![BTreeSet::new(); SETS.len()];
    let (mut checked, mut matched) = (0usize, 0usize);
    let mut mism: Vec<String> = Vec::new();
    for p in ok.iter().map(|m| &m.pair).chain(swept.iter()) {
        for (&(w, r, area), &d4) in &p.loads {
            let area = area & 0x3F;
            // Region 1 is Death Mountain or Maze Island: the pointer decides.
            let cands: Vec<usize> = if (w, r) == (0, 1) {
                vec![1, 3]
            } else {
                set_of(w, r, 0).into_iter().collect()
            };
            if cands.is_empty() {
                continue;
            }
            checked += 1;
            let hits: Vec<usize> = cands
                .iter()
                .copied()
                .filter(|&s| prg.word(SETS[s].bank, SETS[s].sideview + 2 * u16::from(area)) == d4)
                .collect();
            if hits.is_empty() {
                if mism.len() < 24 {
                    mism.push(format!(
                        "w{w} r{r} area{area} $D4=${d4:04X} (assumed {})",
                        SETS[cands[0]].name
                    ));
                }
            } else {
                matched += 1;
                for s in hits {
                    observed[s].insert(area);
                }
            }
        }
    }
    println!("\n== sideview map slots (63 per set)");
    for (s, set) in SETS.iter().enumerate() {
        let free: Vec<u8> = (0..MAPS as u8)
            .filter(|m| !ms.used[s].contains(m) && !observed[s].contains(m))
            .collect();
        let stray: Vec<u8> = observed[s].difference(&ms.used[s]).copied().collect();
        println!(
            "  {:28} bank {} sv ${:04X}: roots {:2}, reachable {:2}, observed {:2}, observed-not-reachable {:?}, FREE {} {:?}",
            set.name,
            set.bank,
            set.sideview,
            ms.roots[s].len(),
            ms.used[s].len(),
            observed[s].len(),
            stray,
            free.len(),
            free
        );
        let tmpl: Vec<String> = free
            .iter()
            .chain(std::iter::once(&63u8))
            .map(|&m| {
                let a = 0xBB15 + (set.part2 - 0x0600) + u16::from(m) / 2;
                let b = prg.rd(5, a);
                let nib = if m % 2 == 0 { b >> 4 } else { b & 0x0F };
                format!("{m}:{}", if nib == 0 { "clear" } else { "set" })
            })
            .collect();
        println!(
            "      free maps (+ map 63) new-game item nibble (part2 ${:04X}+map/2): {}",
            set.part2,
            tmpl.join(" ")
        );
    }
    println!("  table-association check: {checked} loads, {matched} with $D4/$D5 == the assumed table entry");
    for m in &mism {
        println!("    mismatch: {m}");
    }

    // ---- overworld location slots
    println!("\n== overworld location slots (entry scan checks $3D..0; slot $3E is the random-battle scratch)");
    let conts = [
        z2_rando::overworld::map::Cont::West,
        z2_rando::overworld::map::Cont::DeathMountain,
        z2_rando::overworld::map::Cont::East,
        z2_rando::overworld::map::Cont::Maze,
    ];
    for (ci, &(name, bank, base, _)) in LOCS.iter().enumerate() {
        let listed: BTreeSet<u8> = z2_rando::overworld::loc::slots(conts[ci])
            .iter()
            .map(|s| s.slot)
            .collect();
        let (mut zero, mut unlisted_data) = (Vec::new(), Vec::new());
        for i in 0..MAPS as u16 {
            if listed.contains(&(i as u8)) {
                continue;
            }
            if loc_bytes(&prg, bank, base, i) == [0; 4] {
                zero.push(i);
            } else {
                unlisted_data.push(i);
            }
        }
        println!(
            "  {name:15} listed {:2}; unlisted all-zero {:?}; unlisted with data {:?}",
            listed.len(),
            zero,
            unlisted_data
        );
    }

    // ---- progress bits
    println!("\n== save-mirrored CPU RAM bits never set at any movie frame end");
    let mut p1 = [0u8; 0x32];
    let mut p2 = vec![0u8; 0xE0];
    for m in &ok {
        for (o, v) in p1.iter_mut().zip(&m.pair.or_part1) {
            *o |= v;
        }
        for (o, v) in p2.iter_mut().zip(&m.pair.or_part2) {
            *o |= v;
        }
    }
    let line: String = p1
        .iter()
        .enumerate()
        .filter(|(_, v)| **v != 0xFF)
        .map(|(i, v)| format!(" ${:04X}:{:02X}", 0x777 + i, !v))
        .collect();
    println!("  part1 $0777-$07A8 (addr:never-set-mask):{line}");
    let zero2: Vec<String> = p2
        .iter()
        .enumerate()
        .filter(|(_, v)| **v == 0)
        .map(|(i, _)| format!("${:04X}", 0x600 + i))
        .collect();
    println!(
        "  part2 $0600-$06DF bytes never nonzero: {} {}",
        zero2.len(),
        zero2.join(" ")
    );
    0
}
