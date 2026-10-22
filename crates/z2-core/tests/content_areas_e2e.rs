//! Every authored `content` area end to end in a real run
//! (`z2_content::areas::{parapa_annex, challenge_rooms, hidden_caves,
//! dark_isle, minigames}`).
//!
//! For each area the test enters through its real ways in:
//!
//! * overworld entrances: Link is put on the overworld through the game's
//!   own world loader (`$0709 = 1`, `$0748` = the entrance's slot, game
//!   mode 0), steps off the tile and walks back onto it, so the game's own
//!   entry scan and the engine's entry hook run;
//! * vanilla links: the original room is loaded and Link walks out of the
//!   linked edge (Parapa Annex, P1 map 0 right edge);
//! * doorways: Up on the street doorway (the minigames).
//!
//! Then it explores the area breadth first: from each room's arrival
//! state, every exit of the model room is taken with the game's own
//! transition routines (side exits by walking off the edge, elevators by
//! riding them, falls by dropping into the pit, doorways with Up). Link is
//! moved to the exit (allowed: the geometry check in `z2-content` models
//! the walk itself) with the side-view entry anchor the game uses
//! (`LCC50`) and the screen redraw of game mode `$12`. Each arrival is
//! checked: the virtual room and its buffers, the page, the original map,
//! or the overworld tile. Every room is checked for spawned enemies, every
//! ledge for being solid where Link lands on it, every collectable for
//! being collected with its effect (XP, containers, key, 1-up, pieces), and
//! every locked door for opening with a key and staying open. One area per
//! option is carried through a battery save and a reload from the file
//! select.
//!
//! No interpreter fault and no stuck mode anywhere. ROM-gated (self-skips
//! unless `Z2_ROM` names a file). With `Z2_CONTENT_SHOTS=DIR` (outside the
//! work tree: frames come from the ROM) every room's arrival and every page
//! is saved as a PNG.

mod common;

use std::collections::{BTreeSet, VecDeque};

use z2_content::areas;
use z2_content::{Area, Dest, Dir, Object, Target};
use z2_core::enh::content::{
    self, ContentOpts, VRoomId, HEART_PIECE, MAGIC_PIECE, WRAM_ENEMY_BUF, WRAM_ROOM_BUF,
};
use z2_core::enh::content_overworld as ow;
use z2_core::enh::Enhancements;
use z2_core::game::{
    Game, INPUT_A, INPUT_B, INPUT_DOWN, INPUT_LEFT, INPUT_RIGHT, INPUT_SELECT, INPUT_START,
    INPUT_UP,
};
use z2_core::state::GameState;

const A: u8 = 1 << INPUT_A;
const B: u8 = 1 << INPUT_B;
const DOWN: u8 = 1 << INPUT_DOWN;
const UP: u8 = 1 << INPUT_UP;
const SELECT: u8 = 1 << INPUT_SELECT;
const START: u8 = 1 << INPUT_START;
const LEFT: u8 = 1 << INPUT_LEFT;
const RIGHT: u8 = 1 << INPUT_RIGHT;

const MODE: usize = 0x0736;
const MODE_SIDEVIEW: u8 = 0x0B;
const MODE_OVERWORLD: u8 = 5;
/// Side-view screen redraw after the entry anchor (`LCC50` sets it).
const MODE_REDRAW: u8 = 0x12;
const SCENE: usize = 0x0561;
const WORLD: usize = 0x0707;
const REGION: usize = 0x0706;
const PREV_REGION: usize = 0x070A;
const AREA_KEY: usize = 0x0748;
const PAGE: usize = 0x003B;
const LINK_X: usize = 0x004D;
const LINK_Y: usize = 0x0029;
const OW_Y: usize = 0x0073;
const OW_X: usize = 0x0074;
const ROW_BASE: u8 = 30;
const XP_HI: usize = 0x0775;
const XP_LO: usize = 0x0776;
const NEXT_HI: usize = 0x0770;
const NEXT_LO: usize = 0x0771;
const MAGIC_CONTAINERS: usize = 0x0783;
const HEART_CONTAINERS: usize = 0x0784;
const KEYS: usize = 0x0793;
const LIVES: usize = 0x0700;
const STAGE: usize = 0x076C;
const MENU_SEL: usize = 0x0488;
const HOUSE_DEPTH: usize = 0x075B;
/// Overworld encounter wave timer (`$0516`).
const WAVE_TIMER: usize = 0x0516;
/// Elevator object id (`$A1,x`).
const ELEVATOR: u8 = 0x13;
/// Item object id (`$A1,x`).
const ITEM_OBJECT: u8 = 0x01;
/// Cabin Y (`$2A,x`) at rest, where Link walking in finds it.
const CABIN_REST: u8 = 0x98;

// ------------------------------------------------------------ game setup

fn register_default_groups(game: &mut Game) {
    z2_core::bank7_traps::register_bank7_traps(game);
    z2_core::sideview_traps::register_sideview_traps(game);
    z2_core::sideview_traps::register_overworld_traps(game);
    z2_core::player_traps::register_player_traps(game);
    z2_core::enemy_traps::register_enemy_traps(game);
    z2_core::town_traps::register_town_traps(game);
    z2_core::palace_traps::register_palace_traps(game);
    z2_core::title_traps::register_title_traps(game);
    z2_core::boot_traps::register_boot_traps(game);
}

fn opts(f: impl FnOnce(&mut ContentOpts)) -> Enhancements {
    let mut c = ContentOpts {
        virtual_rooms: true,
        ..ContentOpts::default()
    };
    f(&mut c);
    Enhancements {
        content: c,
        ..Enhancements::default()
    }
}

fn game(rom: &[u8], e: Enhancements) -> Game {
    let mut g = Game::from_ines(rom).expect("ROM");
    register_default_groups(&mut g);
    g.set_enhancements(e);
    g.reset();
    g
}

/// Frames with Link topped up.
fn step(g: &mut Game, frames: usize, pad: u8) {
    for _ in 0..frames {
        buff(g);
        g.step(pad);
    }
}

/// Plain frames (menus).
fn g_steps(g: &mut Game, n: usize, pad: u8) {
    for _ in 0..n {
        g.step(pad);
    }
}

/// Title -> file select -> name -> load -> side-view play.
fn reach_gameplay(g: &mut Game) {
    for (n, pad) in [(30, 0), (5, START), (20, 0), (5, START), (20, 0)] {
        g_steps(g, n, pad);
    }
    for _ in 0..8 {
        g_steps(g, 2, A);
        g_steps(g, 8, 0);
    }
    for _ in 0..3 {
        g_steps(g, 2, SELECT);
        g_steps(g, 8, 0);
    }
    for (n, pad) in [(5, START), (30, 0), (5, START), (170, 0)] {
        g_steps(g, n, pad);
    }
    assert_eq!(g.ram[MODE], MODE_SIDEVIEW, "reached side-view play");
}

/// Every item, spell and technique; four containers of each (so the
/// collected ones show), next level far away (no level-up menu from
/// P-bags).
fn equip(g: &mut Game) {
    for i in 0..7 {
        g.ram[0x0785 + i] = 1; // candle .. hammer (not the magic key)
    }
    for i in 0..8 {
        g.ram[0x077B + i] = 1; // spells
    }
    g.ram[0x0796] |= 0x14; // up and down thrust
    g.ram[MAGIC_CONTAINERS] = 4;
    g.ram[HEART_CONTAINERS] = 4;
    g.ram[NEXT_HI] = 0x0F;
    g.ram[NEXT_LO] = 0xFF;
    buff(g);
}

/// Keep Link alive and topped up; no overworld encounters.
fn buff(g: &mut Game) {
    g.ram[0x0774] = 0xFF;
    g.ram[0x0773] = 0xFF;
    if g.ram[WORLD] == 0 && g.ram[MODE] == MODE_OVERWORLD {
        g.ram[WAVE_TIMER] = 0xFF;
    }
}

fn xp(g: &Game) -> u16 {
    u16::from_be_bytes([g.ram[XP_HI], g.ram[XP_LO]])
}

fn on_overworld(g: &Game) -> bool {
    g.ram[WORLD] == 0 && g.ram[MODE] == MODE_OVERWORLD
}

fn ow_pos(g: &Game) -> (u8, u8) {
    (g.ram[OW_Y].wrapping_sub(ROW_BASE), g.ram[OW_X])
}

fn link_px(g: &Game) -> i32 {
    i32::from(g.ram[PAGE] & 3) * 256 + i32::from(g.ram[LINK_X])
}

/// Level-RAM tile at room column `x`, row `y`.
fn tile(g: &Game, x: usize, y: usize) -> u8 {
    g.wram()[(x / 16) * 0xD0 + y * 16 + x % 16]
}

fn shots_dir() -> Option<std::path::PathBuf> {
    common::var_present("Z2_CONTENT_SHOTS").map(std::path::PathBuf::from)
}

fn shot(g: &Game, name: &str) {
    if let Some(dir) = shots_dir() {
        std::fs::create_dir_all(&dir).expect("shots dir");
        let png = z2_ppu::encode_indexed_png(g.frame_indexed());
        std::fs::write(dir.join(format!("{name}.png")), png).expect("write shot");
    }
}

fn slug(s: &str) -> String {
    s.to_ascii_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect()
}

fn ok_s(ok: bool) -> &'static str {
    if ok {
        "ok"
    } else {
        "FAIL"
    }
}

// ------------------------------------------------------------ where Link is

/// Where a transition left Link.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum At {
    /// Side-view play: world, map, virtual room, page.
    Side {
        world: u8,
        map: u8,
        room: Option<VRoomId>,
        page: u8,
    },
    /// The overworld, on (row, col), `$0706` = `region`.
    Over { region: u8, row: u8, col: u8 },
}

fn at(g: &Game) -> Option<At> {
    if on_overworld(g) {
        let (row, col) = ow_pos(g);
        return Some(At::Over {
            region: g.ram[REGION],
            row,
            col,
        });
    }
    (g.ram[MODE] == MODE_SIDEVIEW).then(|| At::Side {
        world: g.ram[WORLD],
        map: g.ram[SCENE],
        room: content::current_room(g),
        page: g.ram[PAGE] & 3,
    })
}

/// Step until Link has been in side-view play or on the overworld for 20
/// frames in a row, unchanged (at most `max` frames).
fn settle(g: &mut Game, max: usize) -> Option<At> {
    let mut calm = 0;
    let mut last = None;
    for _ in 0..max {
        buff(g);
        g.step(0);
        let now = at(g);
        if now.is_some() && now == last {
            calm += 1;
            if calm >= 20 {
                return now;
            }
        } else {
            calm = 0;
        }
        last = now;
    }
    None
}

/// Hold `pad` until a transition starts (the mode leaves side-view play,
/// or the virtual room / map / world changes), then settle. `None` when
/// nothing happened in `max` frames or the game never settled.
fn transit(g: &mut Game, pad: u8, max: usize) -> Option<At> {
    let from = (g.ram[SCENE], content::current_room(g), g.ram[WORLD]);
    for _ in 0..max {
        buff(g);
        g.step(pad);
        if g.ram[MODE] != MODE_SIDEVIEW
            || (g.ram[SCENE], content::current_room(g), g.ram[WORLD]) != from
        {
            return settle(g, 1500);
        }
    }
    None
}

/// Move Link to `page`, pixel `x`, `y` of the room he is in: the side-view
/// entry anchor (`LCC50`'s registers) and the redraw mode it sets, then
/// the exact spot.
fn place(g: &mut Game, page: u8, x: u8, y: u8) {
    if g.ram[PAGE] & 3 != page {
        g.ram[0x075C] = page;
        g.ram[PAGE] = page;
        g.ram[0x072A] = page;
        g.ram[0x072C] = 0;
        g.ram[0xFD] = 0;
        g.ram[0x0732] = page.wrapping_sub(1);
        g.ram[0x0733] = page.wrapping_add(1);
        g.ram[LINK_X] = x;
        g.ram[MODE] = MODE_REDRAW;
        settle(g, 300).expect("redraw settled");
    }
    g.ram[LINK_X] = x;
    g.ram[LINK_Y] = y;
    // At rest: no horizontal speed, off any elevator cabin.
    g.ram[0x70] = 0;
    g.ram[0x0754] = 0;
    step(g, 2, 0);
}

/// Put Link so that his feet are over room column `col` (they sit 16 px
/// right of `$4D`, measured), at `y`.
fn place_on(g: &mut Game, col: u8, y: u8) {
    let px = (u16::from(col) * 16).saturating_sub(16);
    place(g, (px / 256) as u8, (px % 256) as u8, y);
}

// ------------------------------------------------------------ model access

/// The model form of area `name`.
fn model(name: &str) -> Area {
    areas::GROUPS
        .iter()
        .find_map(|g| (g.areas)().into_iter().find(|a| a.name == name))
        .unwrap_or_else(|| panic!("no area {name}"))
}

fn vid(name: &str, index: usize) -> VRoomId {
    content::find_room(name, index as u8).unwrap_or_else(|| panic!("{name} room {index}"))
}

/// Connection byte index an exit of a `pages`-wide room reads.
fn exit_index(dir: Dir, pages: u8) -> usize {
    match dir {
        Dir::Left | Dir::Door { .. } => 0,
        Dir::Right => usize::from(pages - 1),
        Dir::Down => 1,
        Dir::Up => 2,
        Dir::Drop { page } => usize::from(page),
    }
}

fn buffers_hold(g: &Game, id: VRoomId) -> Result<(), String> {
    let r = content::room(id).unwrap().room;
    let rb = usize::from(WRAM_ROOM_BUF - 0x6000);
    // A container piece is shown as the container (`$13` -> `$0F`,
    // `$14` -> `$0E`, by design).
    let same = |got: u8, want: u8| {
        got == want || (want, got) == (HEART_PIECE, 0x0F) || (want, got) == (MAGIC_PIECE, 0x0E)
    };
    if (0..r.sideview.len()).any(|k| !same(g.wram()[rb + k], r.sideview[k])) {
        let diff: Vec<String> = (0..r.sideview.len())
            .filter(|&k| !same(g.wram()[rb + k], r.sideview[k]))
            .map(|k| format!("{k}: {:02X}/{:02X}", g.wram()[rb + k], r.sideview[k]))
            .collect();
        return Err(format!(
            "side-view buffer differs (len {}): {}",
            r.sideview.len(),
            diff.join(" ")
        ));
    }
    let eb = usize::from(WRAM_ENEMY_BUF - 0x6000);
    // The game flags a spawned entry in place (bit 7 of its second byte).
    let strip = |l: &[u8]| -> Vec<u8> {
        l.iter()
            .enumerate()
            .map(|(i, &b)| if i % 2 == 1 { b & 0x7F } else { b })
            .collect()
    };
    if strip(&g.wram()[eb..eb + r.enemies.len()]) != strip(r.enemies) {
        return Err("enemy buffer differs".into());
    }
    Ok(())
}

// ------------------------------------------------------------ the run

/// What one area's run found, for the report.
#[derive(Default)]
struct Report {
    entries: Vec<String>,
    rooms: BTreeSet<usize>,
    exits: Vec<String>,
    items: Vec<String>,
    doors: Vec<String>,
    ledges: Vec<String>,
    enemies: Vec<String>,
    persistence: Option<String>,
    failures: Vec<String>,
}

struct Run<'a> {
    rom: &'a [u8],
    e: Enhancements,
    name: &'static str,
    area: Area,
    /// Arrival state per room index (the first time it was reached).
    arrivals: Vec<Option<GameState>>,
    /// Enemy ids seen alive per room.
    seen: Vec<BTreeSet<u8>>,
    report: Report,
}

impl<'a> Run<'a> {
    fn new(rom: &'a [u8], e: Enhancements, name: &'static str) -> Run<'a> {
        let area = model(name);
        let n = area.rooms.len();
        Run {
            rom,
            e,
            name,
            area,
            arrivals: vec![None; n],
            seen: vec![BTreeSet::new(); n],
            report: Report::default(),
        }
    }

    /// The movement check's grid of room `i` (the ROM's floor patterns).
    fn grid(&self, i: usize) -> z2_content::sideview::Grid {
        let body = if self.rom.starts_with(b"NES\x1A") {
            &self.rom[16..]
        } else {
            self.rom
        };
        let floors = z2_content::sideview::FloorTable::from_body(body).expect("floor table");
        z2_content::compile::room_sideview(&self.area.rooms[i], &self.area)
            .expect("room compiles")
            .grid(self.area.world.object_set(), &floors)
    }

    /// Link's `$29` standing on the lowest standable spot of column `col`
    /// of room `i` (his feet row `r` puts `$29` at `(r + 1) * 16`).
    fn stand_y(&self, i: usize, col: usize, fallback: u8) -> u8 {
        self.grid(i)
            .standable_rows(col)
            .last()
            .map_or(fallback, |&r| ((r + 1) * 16) as u8)
    }

    fn fresh(&self, s: &GameState) -> Game {
        let mut g = game(self.rom, self.e);
        g.load_state(s);
        g
    }

    fn fail(&mut self, what: String) {
        eprintln!("FAIL {}: {what}", self.name);
        self.report.failures.push(what);
    }

    fn check(&mut self, ok: bool, what: impl FnOnce() -> String) {
        if !ok {
            let w = what();
            self.fail(w);
        }
    }

    fn room_name(&self, i: usize) -> &'static str {
        self.area.rooms[i].id
    }

    /// Note the live enemies of room `i`.
    fn watch_enemies(&mut self, g: &Game, i: usize) {
        if content::current_room(g) != Some(vid(self.name, i)) {
            return;
        }
        for s in 0..6 {
            if g.ram[0xB6 + s] != 0 {
                self.seen[i].insert(g.ram[0xA1 + s]);
            }
        }
    }

    /// Check an arrival in room `i` on `page` and record its state.
    fn arrived(&mut self, g: &mut Game, i: usize, page: Option<u8>, how: &str) {
        let id = vid(self.name, i);
        let rn = self.room_name(i);
        match at(g) {
            Some(At::Side { room, page: p, .. }) if room == Some(id) => {
                if let Some(want) = page {
                    self.check(p == want, || {
                        format!("{how}: arrived in {rn} on page {p}, want {want}")
                    });
                }
            }
            other => {
                self.fail(format!("{how}: want {rn}, got {other:?}"));
                return;
            }
        }
        if let Err(e) = buffers_hold(g, id) {
            self.fail(format!("{how}: {rn}: {e}"));
        }
        self.check(g.exec_errors == 0, || format!("{how}: interpreter fault"));
        for _ in 0..40 {
            buff(g);
            g.step(0);
            self.watch_enemies(g, i);
        }
        if self.arrivals[i].is_none() {
            shot(g, &format!("{}-{}-arrive", slug(self.name), rn));
            self.arrivals[i] = Some(g.save_state());
            self.report.rooms.insert(i);
        }
    }

    /// Check that `got` is where exit `dir` / target `t` of room `from`
    /// leads. Returns the room index reached, if any.
    fn expect(
        &mut self,
        g: &mut Game,
        from: usize,
        dir: Dir,
        t: Target,
        got: Option<At>,
    ) -> Option<usize> {
        let from_room = self.area.rooms[from].clone();
        let what = format!("{} {:?} -> {:?}", from_room.id, dir, t);
        let compiled = content::room(vid(self.name, from)).unwrap().room;
        let dest = match dir {
            Dir::Door { x } => compiled.door_exits[usize::from(x / 16)],
            _ => Some(compiled.exits[exit_index(dir, from_room.pages)]),
        };
        let Some(got) = got else {
            self.fail(format!("{what}: no transition, or it never settled"));
            return None;
        };
        match (t, dest) {
            (Target::Room(rid), Some(Dest::Room { room, page })) => {
                let i = self.area.room_index(rid).unwrap();
                assert_eq!(usize::from(room), i);
                let n = self.report.failures.len();
                // Out of a house: back at the doorway Link came in by
                // (`content_town`'s door return), any of the street's
                // doorways into this house.
                let doors: Vec<u8> = self.area.rooms[i]
                    .exits
                    .iter()
                    .filter_map(|e| match (e.dir, e.target) {
                        (Dir::Door { x }, Target::Room(t)) if t == from_room.id => Some(x / 16),
                        _ => None,
                    })
                    .collect();
                let want = if doors.is_empty() {
                    Some(page)
                } else {
                    match at(g) {
                        Some(At::Side { page: p, .. }) if doors.contains(&p) => Some(p),
                        _ => Some(doors[0]),
                    }
                };
                self.arrived(g, i, want, &what);
                let ok = self.report.failures.len() == n;
                self.report.exits.push(format!("{what}: {}", ok_s(ok)));
                Some(i)
            }
            (Target::Map { map, page }, _) => {
                let ok = matches!(got, At::Side { map: m, room: None, page: p, world }
                    if m == map && p == page && world == self.area.world.id());
                self.check(ok, || format!("{what}: got {got:?}"));
                self.report.exits.push(format!("{what}: {}", ok_s(ok)));
                None
            }
            (Target::Outside | Target::OutsideAt(_), _) => {
                let n = match t {
                    Target::OutsideAt(n) => usize::from(n),
                    _ => 0,
                };
                let e = self.area.entrances[n];
                let ok = match got {
                    At::Over { region, row, col } => {
                        region == region_ram(e.region.id()) && (row, col) == (e.row, e.col)
                    }
                    At::Side { .. } => false,
                };
                self.check(ok, || {
                    format!(
                        "{what}: out at {got:?}, want entrance {n} ({}, {})",
                        e.row, e.col
                    )
                });
                if ok {
                    shot(g, &format!("{}-out-{}-{n}", slug(self.name), from_room.id));
                }
                self.report.exits.push(format!("{what}: {}", ok_s(ok)));
                None
            }
            _ => {
                self.fail(format!("{what}: compiled exit {dest:?} does not match"));
                None
            }
        }
    }

    /// Take exit `k` of room `i` from its arrival state.
    fn take_exit(&mut self, i: usize, k: usize) -> Option<usize> {
        let state = self.arrivals[i].clone()?;
        let mut g = self.fresh(&state);
        let room = self.area.rooms[i].clone();
        let ex = room.exits[k];
        let last = room.pages - 1;
        // A key for an exit behind a locked door (the door test is apart).
        if ex.requires.has(z2_content::Abilities::KEY) {
            g.ram[KEYS] = 1;
        }
        let got = match ex.dir {
            Dir::Left => {
                let y = self.stand_y(i, 0, g.ram[LINK_Y]);
                place(&mut g, 0, 0x08, y);
                transit(&mut g, LEFT, 600)
            }
            Dir::Right => {
                let y = self.stand_y(i, room.width() - 1, g.ram[LINK_Y]);
                place(&mut g, last, 0xD8, y);
                transit(&mut g, RIGHT, 600)
            }
            Dir::Up | Dir::Down => self.ride(&mut g, i, ex.dir == Dir::Up),
            Dir::Drop { page } => {
                // The widest run of columns open to the bottom on that page
                // (the movement check's grid; the elevator shaft excluded).
                let grid = self.grid(i);
                let shaft = grid.elevator.map(usize::from);
                let open: Vec<usize> = grid
                    .open_bottom_columns()
                    .into_iter()
                    .filter(|&c| c / 16 == usize::from(page))
                    .filter(|&c| shaft.is_none_or(|e| c != e && c != e + 1))
                    // Through a real gap, not onto a crumbling span.
                    .filter(|&c| {
                        (8..13).all(|y| grid.cell(c, y) == z2_content::sideview::Cell::Empty)
                    })
                    .collect();
                let mut best: Option<(usize, usize)> = None;
                let mut k = 0;
                while k < open.len() {
                    let mut n = 1;
                    while k + n < open.len() && open[k + n] == open[k] + n {
                        n += 1;
                    }
                    if best.is_none_or(|(_, bn)| n > bn) {
                        best = Some((open[k], n));
                    }
                    k += n;
                }
                // A one-column gap is too narrow to drop through (Link
                // walks over it, measured): use a crumbling span then.
                let best = best.filter(|&(_, n)| n >= 2);
                match best {
                    Some((c, n)) => {
                        // Link falls once his feet (16-31 px right of $4D)
                        // are over the hole: centre them on it.
                        let px = (c * 16 + n * 8).saturating_sub(0x18);
                        place(&mut g, (px / 256) as u8, (px % 256) as u8, 0x60);
                        transit(&mut g, 0, 900)
                    }
                    None => {
                        // Only crumbling spans over the pit: stand on one and
                        // let it give way.
                        let crumble = (usize::from(page) * 16..usize::from(page) * 16 + 16)
                            .filter(|&c| c > 0)
                            .find(|&c| {
                                // (Both columns under Link's feet.)
                                [c - 1, c].iter().all(|&cc| {
                                    (0..13).any(|y| {
                                        grid.cell(cc, y) == z2_content::sideview::Cell::Crumble
                                    }) && grid.cell(cc, 12) == z2_content::sideview::Cell::Empty
                                })
                            });
                        match crumble {
                            Some(c) => {
                                let y = (0..13)
                                    .find(|&y| {
                                        grid.cell(c, y) == z2_content::sideview::Cell::Crumble
                                    })
                                    .unwrap();
                                let px = (c * 16).saturating_sub(16);
                                place(&mut g, (px / 256) as u8, (px % 256) as u8, (y * 16) as u8);
                                // Shuffle on the span until it gives way.
                                let from = (g.ram[SCENE], content::current_room(&g));
                                let mut got = None;
                                for f in 0..1500 {
                                    buff(&mut g);
                                    let pad = match f % 32 {
                                        0..=5 => RIGHT,
                                        16..=21 => LEFT,
                                        _ => 0,
                                    };
                                    g.step(pad);
                                    if g.ram[MODE] != MODE_SIDEVIEW
                                        || (g.ram[SCENE], content::current_room(&g)) != from
                                    {
                                        got = settle(&mut g, 1500);
                                        break;
                                    }
                                }
                                got
                            }
                            None => {
                                self.fail(format!("{}: no hole on page {page}", room.id));
                                None
                            }
                        }
                    }
                }
            }
            Dir::Door { x } => {
                let y = g.ram[LINK_Y];
                place(&mut g, x / 16, ((x % 16) * 16).wrapping_sub(8), y);
                let depth = g.ram[HOUSE_DEPTH];
                let mut entered = false;
                for _ in 0..120 {
                    buff(&mut g);
                    g.step(UP);
                    if g.ram[HOUSE_DEPTH] != depth || g.ram[MODE] != MODE_SIDEVIEW {
                        entered = true;
                        break;
                    }
                }
                if entered {
                    settle(&mut g, 1500)
                } else {
                    None
                }
            }
        };
        self.watch_enemies(&g, i);
        self.check(g.exec_errors == 0, || {
            format!("{} exit {:?}: interpreter fault", room.id, ex.dir)
        });
        self.expect(&mut g, i, ex.dir, ex.target, got)
    }

    /// Ride the elevator of room `i` up or down.
    fn ride(&mut self, g: &mut Game, i: usize, up: bool) -> Option<At> {
        let room = self.area.rooms[i].clone();
        let x = room.objects.iter().find_map(|o| match *o {
            Object::Elevator { x } => Some(x),
            _ => None,
        })?;
        let y = g.ram[LINK_Y];
        place(g, x / 16, (x % 16) * 16, y);
        step(g, 10, 0);
        let Some(s) = (0..6).find(|&s| g.ram[0xB6 + s] != 0 && g.ram[0xA1 + s] == ELEVATOR) else {
            self.fail(format!("{}: no elevator object", room.id));
            return None;
        };
        // The redraw spawns the cabin at the top of the shaft; walking in,
        // Link finds it at rest at $98 (measured, any floor pattern).
        // Board it there: Link rides at cabin + 8.
        g.ram[0x2A + s] = CABIN_REST;
        g.ram[LINK_X] = g.ram[0x4E + s];
        g.ram[LINK_Y] = CABIN_REST + 8;
        transit(g, if up { UP } else { DOWN }, 900)
    }

    /// Explore every room reachable from the rooms with an arrival state.
    fn explore(&mut self) {
        let mut queue: VecDeque<usize> = (0..self.area.rooms.len())
            .filter(|&i| self.arrivals[i].is_some())
            .collect();
        let mut done = BTreeSet::new();
        while let Some(i) = queue.pop_front() {
            if !done.insert(i) {
                continue;
            }
            for k in 0..self.area.rooms[i].exits.len() {
                if let Some(j) = self.take_exit(i, k) {
                    if !done.contains(&j) {
                        queue.push_back(j);
                    }
                }
            }
        }
        for i in 0..self.area.rooms.len() {
            let rn = self.room_name(i);
            self.check(self.arrivals[i].is_some(), || {
                format!("room {rn} never reached")
            });
        }
    }

    /// Every page of every reached room as a PNG.
    fn page_shots(&mut self) {
        if shots_dir().is_none() {
            return;
        }
        for i in 0..self.area.rooms.len() {
            let Some(s) = self.arrivals[i].clone() else {
                continue;
            };
            for p in 0..self.area.rooms[i].pages {
                let mut g = self.fresh(&s);
                let y = g.ram[LINK_Y];
                place(&mut g, p, 0x70, y);
                step(&mut g, 20, 0);
                self.watch_enemies(&g, i);
                shot(
                    &g,
                    &format!("{}-{}-p{p}", slug(self.name), self.area.rooms[i].id),
                );
            }
        }
    }

    /// Enemies: every room with an enemy list showed at least one of them
    /// (on arrival, while leaving, or on a page visit).
    fn check_enemies(&mut self) {
        for i in 0..self.area.rooms.len() {
            let r = self.area.rooms[i].clone();
            let want: BTreeSet<u8> = r.enemies.iter().map(|e| e.id).collect();
            if want.is_empty() {
                continue;
            }
            if let Some(s) = self.arrivals[i].clone() {
                for p in 0..r.pages {
                    let mut g = self.fresh(&s);
                    let y = g.ram[LINK_Y];
                    place(&mut g, p, 0x30, y);
                    for _ in 0..60 {
                        buff(&mut g);
                        g.step(0);
                        self.watch_enemies(&g, i);
                    }
                }
            }
            let got: BTreeSet<u8> = self.seen[i].intersection(&want).copied().collect();
            let line = format!("{}: list {want:02X?}, seen {got:02X?}", r.id);
            let ok = !got.is_empty();
            self.report.enemies.push(format!("{line} {}", ok_s(ok)));
            self.check(ok, || format!("no enemy of {line}"));
        }
    }

    /// Ledges: Link put on the top row of every ledge object stays there
    /// (a background tile would let him fall through).
    fn check_ledges(&mut self) {
        for i in 0..self.area.rooms.len() {
            let Some(s) = self.arrivals[i].clone() else {
                continue;
            };
            let r = self.area.rooms[i].clone();
            for o in &r.objects {
                let (x, y, len) = match *o {
                    Object::Platform { x, y, len }
                    | Object::Block { x, y, len }
                    | Object::Bridge { x, y, len } => (x, y, len),
                    _ => continue,
                };
                // Town "ledges" are background (measured, see the model).
                if y < 2
                    || matches!(
                        (self.area.world, *o),
                        (
                            z2_content::World::WestTowns | z2_content::World::EastTowns,
                            Object::Platform { .. }
                        )
                    )
                {
                    continue;
                }
                // A column of the ledge with Link well inside the screen
                // (a $4D near a page edge sends him out of the room).
                // Link's feet cover the column left of `place_on`'s too
                // (measured: half on lava, he sinks), so start one in.
                let first = if len > 1 { x + 1 } else { x };
                let mid = (first..x + len)
                    .map(|c| (c, (u16::from(c) * 16).saturating_sub(16) % 256))
                    .filter(|&(_, px)| (0x10..=0xD0).contains(&px))
                    .map(|(c, _)| c)
                    .min_by_key(|&c| (i16::from(c) - i16::from(x + len / 2)).abs())
                    .unwrap_or(x + len / 2);
                // Skip a ledge with another object right above its middle.
                let covered = r.objects.iter().any(|p| match *p {
                    Object::Platform {
                        x: px,
                        y: py,
                        len: pl,
                    }
                    | Object::Block {
                        x: px,
                        y: py,
                        len: pl,
                    }
                    | Object::Bridge {
                        x: px,
                        y: py,
                        len: pl,
                    } => p != o && px <= mid + 1 && mid <= px + pl && py < y && py + 3 >= y,
                    _ => false,
                });
                // Or with anything in the rows Link drops through.
                let grid = self.grid(i);
                let blocked = (y.saturating_sub(3)..y).any(|row| {
                    grid.cell(usize::from(mid), usize::from(row))
                        != z2_content::sideview::Cell::Empty
                });
                if covered || blocked {
                    continue;
                }
                // Stand Link on the top row, every 4 px along the ledge:
                // on a solid ledge he stays put somewhere (next to lava or
                // an edge some spots slide off, measured).
                let want = y * 16;
                let mut land = 0u8;
                let mut g = self.fresh(&s);
                let lo = (u16::from(x) * 16).saturating_sub(24);
                let hi = u16::from(x + len) * 16 - 8;
                'spots: for px in (lo..=hi).step_by(4) {
                    if px % 256 < 0x08 || px % 256 > 0xE0 {
                        continue;
                    }
                    // Two ways in: straight onto the spot, or anchored
                    // mid-page first (right after a redraw a spot near the
                    // page's left edge reads wrong, measured).
                    for anchored in [false, true] {
                        g = self.fresh(&s);
                        let page = (px / 256) as u8;
                        if anchored {
                            place(&mut g, page, 0x80, want);
                            step(&mut g, 20, 0);
                            g.ram[LINK_X] = (px % 256) as u8;
                            g.ram[LINK_Y] = want;
                            g.ram[0x70] = 0;
                        } else {
                            place(&mut g, page, (px % 256) as u8, want);
                        }
                        for _ in 0..60 {
                            buff(&mut g);
                            // No enemy to knock Link off, no cabin to
                            // carry him.
                            for s in 0..6 {
                                if g.ram[0xA1 + s] != ITEM_OBJECT {
                                    g.ram[0xB6 + s] = 0;
                                }
                            }
                            g.step(0);
                        }
                        land = g.ram[LINK_Y];
                        if land == want {
                            break 'spots;
                        }
                    }
                }
                // Held where the arrival left him (an elevator cabin he rode
                // in on keeps carrying him): no verdict.
                if land != want && (land == s_y(&s, self) || s_y(&s, self) >= 0xC0) {
                    self.report
                        .ledges
                        .push(format!("{} {:?}: not checked (held by the cabin)", r.id, o));
                    continue;
                }
                let ok = land == want;
                let line = format!(
                    "{} {:?}: landed y {land:02X}, top {want:02X} (tile {:02X})",
                    r.id,
                    o,
                    tile(&g, usize::from(mid), usize::from(y))
                );
                self.report.ledges.push(format!("{line} {}", ok_s(ok)));
                self.check(ok, || format!("ledge not solid: {line}"));
            }
        }
    }

    /// Collect every item of every room (and open every locked door);
    /// returns the state after the last pickup (for the persistence check).
    fn check_items(&mut self) -> Option<GameState> {
        let mut last = None;
        for i in 0..self.area.rooms.len() {
            let Some(s) = self.arrivals[i].clone() else {
                continue;
            };
            let r = self.area.rooms[i].clone();
            let id = vid(self.name, i);
            let mut g = self.fresh(&s);
            for o in &r.objects {
                let Object::Item { x, y, item } = *o else {
                    continue;
                };
                if let Some(st) = self.pick(&mut g, i, x, y, item) {
                    last = Some(st);
                }
            }
            let _ = id;
            for o in &r.objects {
                if let Object::LockedDoor { x, .. } = *o {
                    self.check_door(i, x);
                }
            }
        }
        last
    }

    /// Pick up item `item` at (`x`, `y`) in room `i` (Link is there).
    fn pick(&mut self, g: &mut Game, i: usize, x: u8, y: u8, item: u8) -> Option<GameState> {
        let id = vid(self.name, i);
        let rn = self.room_name(i);
        let page = x / 16;
        let line = format!("{rn} item {item:02X} at ({x}, {y})");
        if content::taken_pages(g, id) & (1 << page) != 0 {
            self.fail(format!("{line}: already taken on arrival"));
            return None;
        }
        let before = Effects::of(g);
        place_on(g, x, (y + 1).min(12) * 16);
        // The item object (its `$2A,x` is 2 rows below its row, measured).
        let obj = (0..6)
            .filter(|&s| g.ram[0xB6 + s] != 0 && g.ram[0xA1 + s] == ITEM_OBJECT)
            .find(|&s| {
                let ox = i32::from(g.ram[0x3C + s] & 3) * 256 + i32::from(g.ram[0x4E + s]);
                (ox - i32::from(x) * 16).abs() <= 8
            });
        let rest = obj.map(|s| g.ram[0x2A + s]);
        // Next to the object, facing it: on the same floor Link's $29 is 16
        // px above an item's $2A; a crouching stab from 32 px to its left
        // takes a P-bag or a key (measured), walking into it the rest.
        let snap = g.save_state();
        for dx in [0x20u8, 0x18, 0x10, 0x08] {
            if content::taken_pages(g, id) & (1 << page) != 0 {
                break;
            }
            g.load_state(&snap);
            if let Some(s) = obj {
                g.ram[LINK_X] = g.ram[0x4E + s].wrapping_sub(dx);
                g.ram[LINK_Y] = g.ram[0x2A + s].wrapping_sub(0x10);
                step(g, 1, RIGHT);
                g.ram[LINK_X] = g.ram[0x4E + s].wrapping_sub(dx);
            }
            for _ in 0..3 {
                step(g, 2, B | DOWN);
                step(g, 12, 0);
            }
            step(g, 12, RIGHT);
            step(g, 12, LEFT);
        }
        // P-bag experience counts up over a few frames (no enemy around
        // to take any of it).
        for _ in 0..700 {
            for s in 0..6 {
                if !matches!(g.ram[0xA1 + s], ELEVATOR | ITEM_OBJECT) {
                    g.ram[0xB6 + s] = 0;
                }
            }
            step(g, 1, 0);
        }
        let taken = content::taken_pages(g, id) & (1 << page) != 0;
        let after = Effects::of(g);
        let (eff_ok, eff) = before.check(&after, item);
        let ok = eff_ok && taken;
        let line = format!(
            "{line}: taken {taken}, {eff}, object y {}",
            rest.map_or("?".into(), |y| format!("{y:02X}"))
        );
        self.report.items.push(format!("{line} {}", ok_s(ok)));
        self.check(ok, || line.clone());
        Some(g.save_state())
    }

    /// A key opens the locked door at column `x` of room `i`, and it
    /// stays open when the room is loaded again.
    fn check_door(&mut self, i: usize, x: u8) {
        let s = self.arrivals[i].clone().unwrap();
        let r = self.area.rooms[i].clone();
        let id = vid(self.name, i);
        let page = x / 16;
        let mut g = self.fresh(&s);
        g.ram[KEYS] = 1;
        // From the side away from the room's nearer edge.
        let from_right = u16::from(x) + 3 < u16::from(r.pages) * 16;
        let sx = if from_right {
            x + 2
        } else {
            x.saturating_sub(2)
        };
        let y = self.stand_y(i, usize::from(sx), g.ram[LINK_Y]);
        place_on(&mut g, sx, y);
        let pad = if from_right { LEFT } else { RIGHT };
        let door_px = i32::from(x) * 16;
        let past = |g: &Game| {
            if from_right {
                link_px(g) < door_px
            } else {
                link_px(g) > door_px
            }
        };
        let mut opened = false;
        for _ in 0..240 {
            buff(&mut g);
            g.step(pad);
            opened |= g.ram[KEYS] == 0;
            if past(&g) && (link_px(&g) - door_px).abs() > 12 {
                break;
            }
        }
        let passed = past(&g);
        let bit = content::taken_pages(&g, id) & (1 << page) != 0;
        // Revisit: the game's own load of the slot with the room queued
        // (as after a death), then walk through again with no key.
        let (key, region, world, slot) =
            (g.ram[AREA_KEY], g.ram[REGION], g.ram[WORLD], g.ram[SCENE]);
        assert!(content::enter_from_overworld(&mut g, id));
        g.ram[REGION] = region;
        g.ram[WORLD] = world;
        g.ram[AREA_KEY] = key;
        g.ram[SCENE] = slot;
        g.ram[0x075C] = sx / 16;
        g.ram[0x0709] = 0;
        g.ram[0x0704] = 0;
        g.ram[MODE] = 0;
        let back = settle(&mut g, 1500);
        let mut again = false;
        if matches!(back, Some(At::Side { room: Some(r2), .. }) if r2 == id) {
            g.ram[KEYS] = 0;
            let y = self.stand_y(i, usize::from(sx), g.ram[LINK_Y]);
            place_on(&mut g, sx, y);
            for _ in 0..240 {
                buff(&mut g);
                g.step(pad);
                if past(&g) && (link_px(&g) - door_px).abs() > 12 {
                    break;
                }
            }
            again = past(&g);
        }
        let ok = opened && passed && bit && again;
        let line = format!(
            "{} locked door at {x}: key used {opened}, passed {passed}, bit {bit}, open on reload {again} ({back:?})",
            r.id
        );
        self.report.doors.push(format!("{line} {}", ok_s(ok)));
        self.check(ok, || line.clone());
    }

    fn summary(&self) -> String {
        let r = &self.report;
        let mut s = format!("== {} ==\n", self.name);
        for e in &r.entries {
            s += &format!("  entry: {e}\n");
        }
        s += &format!(
            "  rooms reached: {}/{} ({})\n",
            r.rooms.len(),
            self.area.rooms.len(),
            r.rooms
                .iter()
                .map(|&i| self.area.rooms[i].id)
                .collect::<Vec<_>>()
                .join(", ")
        );
        for l in r
            .exits
            .iter()
            .chain(&r.items)
            .chain(&r.doors)
            .chain(&r.ledges)
            .chain(&r.enemies)
        {
            s += &format!("  {l}\n");
        }
        if let Some(p) = &r.persistence {
            s += &format!("  persistence: {p}\n");
        }
        s += &format!("  failures: {}\n", r.failures.len());
        for f in &r.failures {
            s += &format!("    - {f}\n");
        }
        s
    }
}

/// Link's `$29` in state `s`.
fn s_y(s: &GameState, run: &Run) -> u8 {
    run.fresh(s).ram[LINK_Y]
}

/// Item effects to compare before and after a pickup.
#[derive(Debug, Clone, Copy)]
struct Effects {
    xp: u16,
    hearts: u8,
    magics: u8,
    keys: u8,
    lives: u8,
    heart_pieces: u32,
    magic_pieces: u32,
}

impl Effects {
    fn of(g: &Game) -> Effects {
        Effects {
            xp: xp(g),
            hearts: g.ram[HEART_CONTAINERS],
            magics: g.ram[MAGIC_CONTAINERS],
            keys: g.ram[KEYS],
            lives: g.ram[LIVES],
            heart_pieces: content::pieces_collected(g, HEART_PIECE),
            magic_pieces: content::pieces_collected(g, MAGIC_PIECE),
        }
    }

    /// Whether `after` shows item `item` collected, and a description.
    fn check(&self, after: &Effects, item: u8) -> (bool, String) {
        use z2_content::model::item as it;
        let d = |a: u8, b: u8| i32::from(b) - i32::from(a);
        let containers = (d(self.hearts, after.hearts), d(self.magics, after.magics));
        match item {
            it::BAG_50 | it::BAG_100 | it::BAG_200 | it::BAG_500 => {
                let want = [50u16, 100, 200, 500][usize::from(item - it::BAG_50)];
                let got = after.xp.wrapping_sub(self.xp);
                (got == want, format!("xp +{got} (want {want})"))
            }
            it::HEART_CONTAINER => (containers == (1, 0), format!("containers {containers:?}")),
            it::MAGIC_CONTAINER => (containers == (0, 1), format!("containers {containers:?}")),
            it::SMALL_KEY => (
                d(self.keys, after.keys) == 1,
                format!("keys {} -> {}", self.keys, after.keys),
            ),
            it::ONE_UP => (
                d(self.lives, after.lives) == 1,
                format!("lives {} -> {}", self.lives, after.lives),
            ),
            HEART_PIECE => {
                let n = after.heart_pieces;
                let want = (i32::from(n.is_multiple_of(4)), 0);
                (
                    n == self.heart_pieces + 1 && containers == want,
                    format!(
                        "heart pieces {} -> {n}, containers {containers:?}",
                        self.heart_pieces
                    ),
                )
            }
            MAGIC_PIECE => {
                let n = after.magic_pieces;
                let want = (0, i32::from(n.is_multiple_of(4)));
                (
                    n == self.magic_pieces + 1 && containers == want,
                    format!(
                        "magic pieces {} -> {n}, containers {containers:?}",
                        self.magic_pieces
                    ),
                )
            }
            _ => (true, "no effect checked".into()),
        }
    }
}

/// `$0706` while Link stands on continent `region` (`Region::id`).
fn region_ram(region: u8) -> u8 {
    match region {
        0 => 0,
        2 => 2,
        _ => 1,
    }
}

// ------------------------------------------------------------ overworld

/// Put Link on the overworld of continent `region` (0 West, 1 Death
/// Mountain, 2 East, 3 Maze Island) at location slot `slot`, through the
/// game's world loader.
fn ow_load(g: &mut Game, region: u8, slot: u8) -> bool {
    let (r, prev) = match region {
        0 => (0, 0),
        1 => (1, 0),
        2 => (2, 2),
        _ => (1, 2),
    };
    g.ram[REGION] = r;
    g.ram[PREV_REGION] = prev;
    g.ram[WORLD] = 0;
    g.ram[0x0709] = 1;
    g.ram[AREA_KEY] = slot;
    g.ram[MODE] = 0;
    matches!(settle(g, 1500), Some(At::Over { .. }))
}

/// Hold `pad` on the overworld for at most one tile's walk: `Some(true)`
/// if Link left the overworld (an entry), `Some(false)` if he moved a
/// tile, `None` if he could not move.
fn ow_step(g: &mut Game, pad: u8) -> Option<bool> {
    let from = ow_pos(g);
    // (Swamp is half speed.)
    for _ in 0..128 {
        buff(g);
        g.step(pad);
        if !on_overworld(g) {
            return Some(true);
        }
        if ow_pos(g) != from {
            // Finish the step (half speed in a swamp).
            for _ in 0..80 {
                buff(g);
                g.step(0);
                if !on_overworld(g) {
                    return Some(true);
                }
            }
            return Some(false);
        }
    }
    None
}

/// (step, step back).
const DIRS: [(u8, u8); 4] = [(RIGHT, LEFT), (LEFT, RIGHT), (DOWN, UP), (UP, DOWN)];

/// Enter by overworld entrance `n` of the run's area: onto the overworld at
/// the entrance's slot, off the tile and back onto it.
fn enter_by_entrance(run: &mut Run, base: &GameState, n: usize) {
    let e = run.area.entrances[n];
    let what = format!("entrance {n} ({:?} {}, {})", e.region, e.row, e.col);
    let mut g = run.fresh(base);
    let p = *ow::placements(&g)
        .iter()
        .find(|p| {
            content::room(p.room).is_some_and(|r| r.area.name == run.name)
                && (p.region, p.row, p.col) == (e.region.id(), e.row, e.col)
        })
        .unwrap_or_else(|| panic!("{}: {what} placed", run.name));
    if !ow_load(&mut g, e.region.id(), p.slot) {
        run.fail(format!("{what}: overworld load failed"));
        return;
    }
    shot(&g, &format!("{}-ow-{n}", slug(run.name)));
    let start = ow_pos(&g);
    let tile = (e.row, e.col);
    let mut entered = false;
    if start == tile {
        // Step off onto any walkable neighbour, then back.
        let snap = g.save_state();
        for &(go, back) in &DIRS {
            let mut h = run.fresh(&snap);
            if ow_step(&mut h, go) == Some(false) && ow_step(&mut h, back) == Some(true) {
                g = h;
                entered = true;
                break;
            }
        }
    } else {
        // Next to the tile (an external entry's nudge): walk onto it.
        let dr = i16::from(tile.0) - i16::from(start.0);
        let dc = i16::from(tile.1) - i16::from(start.1);
        let pad = match (dr, dc) {
            (0, 1) => RIGHT,
            (0, -1) => LEFT,
            (1, 0) => DOWN,
            (-1, 0) => UP,
            _ => 0,
        };
        if pad != 0 {
            entered = ow_step(&mut g, pad) == Some(true);
        }
    }
    if !entered {
        run.fail(format!(
            "{what}: could not walk onto the tile from {start:?}"
        ));
        return;
    }
    let slot_hit = g.ram[AREA_KEY];
    let got = settle(&mut g, 2000);
    let room = run.area.room_index(e.room).unwrap();
    let n0 = run.report.failures.len();
    run.arrived(&mut g, room, Some(e.page), &what);
    let ok = run.report.failures.len() == n0;
    run.report.entries.push(format!(
        "{what}: slot {} (stepped on {slot_hit}), shown at {start:?}, arrived {got:?} {}",
        p.slot,
        ok_s(ok)
    ));
}

// ------------------------------------------------------------ battery

/// SAVE on the game-over screen, then a fresh game on that battery RAM,
/// file 1 loaded from the menu.
fn battery_cycle(rom: &[u8], e: Enhancements, mut g: Game) -> Game {
    g.ram[LIVES] = 1;
    g.ram[STAGE] = 2;
    let mut ok = false;
    for _ in 0..3000 {
        g.step(0);
        if g.ram[STAGE] == 2 && g.ram[MODE] == 9 {
            ok = true;
            break;
        }
    }
    assert!(ok, "game-over screen");
    g.ram[MENU_SEL] = 1;
    g_steps(&mut g, 2, START);
    g_steps(&mut g, 300, 0);
    let sram = *g.wram();
    let mut h = Game::from_ines(rom).expect("ROM");
    register_default_groups(&mut h);
    h.set_enhancements(e);
    h.wram = sram;
    h.reset();
    g_steps(&mut h, 30, 0);
    g_steps(&mut h, 5, START);
    g_steps(&mut h, 20, 0);
    g_steps(&mut h, 5, START);
    let mut ok = false;
    for _ in 0..600 {
        h.step(0);
        if h.ram[MODE] == MODE_SIDEVIEW {
            ok = true;
            break;
        }
    }
    assert!(ok, "file loaded");
    g_steps(&mut h, 120, 0);
    h
}

/// The taken pages of the run's rooms survive a battery save and a reload
/// from the file select.
fn check_persistence(run: &mut Run, after: GameState) {
    let g = run.fresh(&after);
    let ids: Vec<VRoomId> = (0..run.area.rooms.len())
        .map(|i| vid(run.name, i))
        .collect();
    let before: Vec<u8> = ids.iter().map(|&id| content::taken_pages(&g, id)).collect();
    let h = battery_cycle(run.rom, run.e, g);
    let now: Vec<u8> = ids.iter().map(|&id| content::taken_pages(&h, id)).collect();
    let ok = before == now && before.iter().any(|&b| b != 0);
    run.report.persistence = Some(format!(
        "taken pages {before:?} -> after save and reload {now:?} {}",
        ok_s(ok)
    ));
    run.check(ok, || format!("persistence: {before:?} -> {now:?}"));
    run.check(h.exec_errors == 0, || "fault after the reload".into());
}

// ------------------------------------------------------------ the tests

/// A game in side-view play with everything equipped, as a state.
fn base_state(rom: &[u8], e: Enhancements) -> GameState {
    let mut g = game(rom, e);
    reach_gameplay(&mut g);
    equip(&mut g);
    g.save_state()
}

/// Enter by every overworld entrance, explore, check enemies and ledges,
/// shoot every page.
fn run_overworld_area<'a>(
    rom: &'a [u8],
    e: Enhancements,
    base: &GameState,
    name: &'static str,
) -> Run<'a> {
    let mut run = Run::new(rom, e, name);
    for n in 0..run.area.entrances.len() {
        enter_by_entrance(&mut run, base, n);
    }
    run.explore();
    run.check_enemies();
    run.check_ledges();
    run.page_shots();
    run
}

fn finish(runs: &[Run]) {
    let mut fails = 0;
    for r in runs {
        eprintln!("{}", r.summary());
        fails += r.report.failures.len();
    }
    assert_eq!(fails, 0, "see the report above");
}

#[test]
fn hidden_caves_end_to_end() {
    let Some(rom) = common::rom_bytes("hidden_caves_end_to_end") else {
        return;
    };
    let e = opts(|c| c.hidden_caves = true);
    let base = base_state(&rom, e);
    let mut runs = Vec::new();
    for name in [
        "Lakeside Hollow",
        "Sandcrawl Burrow",
        "Reed Grotto",
        "Ember Niche",
        "Tidepool Cave",
        "Old Barrow",
    ] {
        let mut run = run_overworld_area(&rom, e, &base, name);
        let last = run.check_items();
        if name == "Old Barrow" {
            if let Some(s) = last {
                check_persistence(&mut run, s);
            }
        }
        runs.push(run);
    }
    finish(&runs);
}

#[test]
fn challenge_rooms_end_to_end() {
    let Some(rom) = common::rom_bytes("challenge_rooms_end_to_end") else {
        return;
    };
    let e = opts(|c| c.challenge_rooms = true);
    let base = base_state(&rom, e);
    let mut runs = Vec::new();
    for name in [
        "Ember Run",
        "Crumbling Causeway",
        "Lizalfos Pit",
        "Rubble Maze",
        "Skyward Stair",
    ] {
        let mut run = run_overworld_area(&rom, e, &base, name);
        let last = run.check_items();
        if name == "Skyward Stair" {
            if let Some(s) = last {
                check_persistence(&mut run, s);
            }
        }
        runs.push(run);
    }
    finish(&runs);
}

#[test]
fn dark_isle_end_to_end() {
    let Some(rom) = common::rom_bytes("dark_isle_end_to_end") else {
        return;
    };
    let e = opts(|c| c.dark_isle = true);
    let base = base_state(&rom, e);
    let mut runs = Vec::new();
    for name in ["Dark Isle Caves", "Grimhollow", "Thornspire"] {
        let mut run = run_overworld_area(&rom, e, &base, name);
        let last = run.check_items();
        if name == "Thornspire" {
            if let Some(s) = last {
                check_persistence(&mut run, s);
            }
        }
        runs.push(run);
    }
    finish(&runs);
}

#[test]
fn parapa_annex_end_to_end() {
    let Some(rom) = common::rom_bytes("parapa_annex_end_to_end") else {
        return;
    };
    let e = opts(|c| c.parapa_annex = true);
    let base = base_state(&rom, e);
    let mut run = Run::new(&rom, e, "Parapa Annex");
    // Through Palace 1's entrance room: walk off its right edge.
    let l = run.area.links[0];
    let mut g = run.fresh(&base);
    g.ram[REGION] = 0;
    g.ram[WORLD] = 3;
    g.ram[PREV_REGION] = 0;
    g.ram[AREA_KEY] = 0x34;
    g.ram[0x056C] = 0;
    g.ram[SCENE] = l.map;
    g.ram[0x075C] = 3;
    g.ram[0x0701] = 1;
    g.ram[0x0709] = 0;
    g.ram[0x0704] = 0;
    g.ram[MODE] = 0;
    let loaded = settle(&mut g, 1500);
    assert!(
        matches!(
            loaded,
            Some(At::Side {
                map: 0,
                room: None,
                ..
            })
        ),
        "P1 map 0: {loaded:?}"
    );
    shot(&g, "parapa-annex-p1-map0");
    let got = transit(&mut g, RIGHT, 900);
    let room = run.area.room_index(l.room).unwrap();
    let via = format!("P1 map {} right edge", l.map);
    let n0 = run.report.failures.len();
    run.arrived(&mut g, room, Some(l.page), &via);
    let ok = run.report.failures.len() == n0;
    run.report
        .entries
        .push(format!("{via}: arrived {got:?} {}", ok_s(ok)));
    // And by the back door.
    enter_by_entrance(&mut run, &base, 0);
    run.explore();
    run.check_enemies();
    run.check_ledges();
    run.page_shots();
    if let Some(s) = run.check_items() {
        check_persistence(&mut run, s);
    }
    finish(&[run]);
}

#[test]
fn minigames_end_to_end() {
    let Some(rom) = common::rom_bytes("minigames_end_to_end") else {
        return;
    };
    use z2_content::areas::minigames::ATTACH;
    let e = opts(|c| c.minigames = true);
    let base = base_state(&rom, e);
    let mut runs = Vec::new();
    for at in ATTACH {
        let mut run = Run::new(&rom, e, at.area);
        if !enter_by_doorway(&mut run, &base, at) {
            runs.push(run);
            continue;
        }
        run.explore();
        run.check_enemies();
        run.check_ledges();
        run.page_shots();
        // The gallery's prize is out only after a won round (played by
        // `enh_minigames_rom.rs`, and taken in
        // `container_pieces_add_up_to_containers`); the tower's is room
        // data.
        if at.area == "Pogo Tower" {
            if let Some(s) = run.check_items() {
                check_persistence(&mut run, s);
            }
        }
        runs.push(run);
    }
    finish(&runs);
}

/// Into a minigame room through its painted street doorway: load the host
/// street on the door's page, walk to the doorway, Up.
fn enter_by_doorway(
    run: &mut Run,
    base: &GameState,
    at: &z2_content::areas::minigames::DoorAttach,
) -> bool {
    let mut g = run.fresh(base);
    g.ram[REGION] = at.region.id();
    g.ram[WORLD] = at.world.id();
    g.ram[PREV_REGION] = 0;
    g.ram[AREA_KEY] = at.town_slot;
    g.ram[SCENE] = at.street;
    g.ram[0x075C] = at.door_slot;
    g.ram[0x0701] = 0;
    g.ram[0x0709] = 0;
    g.ram[0x0704] = 0;
    g.ram[MODE] = 0;
    let loaded = settle(&mut g, 1500);
    assert!(
        matches!(loaded, Some(At::Side { room: None, .. })),
        "{} street: {loaded:?}",
        at.town
    );
    // The painted doorway's threshold on the door page.
    let col = (0..13 * 16)
        .find(|&i| {
            g.wram()[usize::from(at.door_slot) * 0xD0 + i]
                == z2_core::enh::content_town::DOOR_TILES[3]
        })
        .map(|i| (i % 16) as u8);
    let via = format!(
        "{} street {} door page {}",
        at.town, at.street, at.door_slot
    );
    let Some(col) = col else {
        run.fail(format!("{via}: no doorway painted"));
        return false;
    };
    let y = g.ram[LINK_Y];
    place(&mut g, at.door_slot, (col * 16).wrapping_sub(8), y);
    let depth = g.ram[HOUSE_DEPTH];
    let mut entered = false;
    for _ in 0..120 {
        buff(&mut g);
        g.step(UP);
        if g.ram[HOUSE_DEPTH] != depth {
            entered = true;
            break;
        }
    }
    let got = settle(&mut g, 1500);
    let n0 = run.report.failures.len();
    run.arrived(&mut g, 0, Some(0), &via);
    let ok = run.report.failures.len() == n0;
    run.report.entries.push(format!(
        "{via}: Up entered {entered}, arrived {got:?} {}",
        ok_s(ok)
    ));
    true
}

/// Every container piece of the content group in one game: the four magic
/// pieces (hidden caves) and the four heart pieces (two caves, the two
/// minigame prizes), each taken with the game's pickup; the fourth of a
/// kind gives the container (the `Effects` check of every pickup), and a
/// battery save keeps the count.
#[test]
fn container_pieces_add_up_to_containers() {
    let Some(rom) = common::rom_bytes("container_pieces_add_up_to_containers") else {
        return;
    };
    use z2_content::areas::minigames::ATTACH;
    let e = opts(|c| {
        c.hidden_caves = true;
        c.minigames = true;
    });
    let mut state = base_state(&rom, e);
    let start = Effects::of(&game_from(&rom, e, &state));
    let mut runs = Vec::new();
    let caves = [
        ("Sandcrawl Burrow", 1),
        ("Tidepool Cave", 0),
        ("Old Barrow", 1),
        ("Old Barrow", 2),
        ("Lakeside Hollow", 0),
        ("Ember Niche", 0),
    ];
    for (name, room) in caves {
        let mut run = Run::new(&rom, e, name);
        enter_by_entrance(&mut run, &state, 0);
        run.explore();
        if let Some(s) = take_the_piece(&mut run, room) {
            state = s;
        }
        runs.push(run);
    }
    for at in ATTACH {
        let mut run = Run::new(&rom, e, at.area);
        // The gallery's prize is out once the round is won (minigames
        // flag 0); the tower's is room data.
        let mut g = game_from(&rom, e, &state);
        content::set_flag(&mut g, content::Group::Minigames, 0, true);
        let s = g.save_state();
        if enter_by_doorway(&mut run, &s, at) {
            run.explore();
            if let Some(s) = take_the_piece(&mut run, 0) {
                state = s;
            }
        }
        runs.push(run);
    }
    let g = game_from(&rom, e, &state);
    let end = Effects::of(&g);
    let line = format!(
        "pieces heart {} magic {}, containers heart {} -> {}, magic {} -> {}",
        end.heart_pieces, end.magic_pieces, start.hearts, end.hearts, start.magics, end.magics
    );
    eprintln!("{line}");
    let ok = (end.heart_pieces, end.magic_pieces) == (4, 4)
        && end.hearts == start.hearts + 1
        && end.magics == start.magics + 1;
    let h = battery_cycle(&rom, e, g);
    let kept = (
        content::pieces_collected(&h, HEART_PIECE),
        content::pieces_collected(&h, MAGIC_PIECE),
    );
    eprintln!("after a battery save and reload: pieces {kept:?}");
    finish(&runs);
    assert!(ok, "{line}");
    assert_eq!(kept, (4, 4), "pieces kept on the battery");
}

fn game_from(rom: &[u8], e: Enhancements, s: &GameState) -> Game {
    let mut g = game(rom, e);
    g.load_state(s);
    g
}

/// Take the (first) piece in room `room` of the run, from its arrival.
fn take_the_piece(run: &mut Run, room: usize) -> Option<GameState> {
    let r = run.area.rooms[room].clone();
    let (x, y, item) = r.objects.iter().find_map(|o| match *o {
        Object::Item { x, y, item } if item == HEART_PIECE || item == MAGIC_PIECE => {
            Some((x, y, item))
        }
        _ => None,
    })?;
    let Some(s) = run.arrivals[room].clone() else {
        run.fail(format!("{} not reached", r.id));
        return None;
    };
    let mut g = run.fresh(&s);
    run.pick(&mut g, room, x, y, item)
}
