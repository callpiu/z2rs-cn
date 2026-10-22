//! New overworld entrances and terrain for the `content` group.
//!
//! Every enabled area's [`z2_content::CompiledEntrance`] becomes a live
//! location entry of its region, its tile is painted into the overworld
//! map, and its [`z2_content::CompiledTerrain`] cells repaint the map
//! (a new island, shallows). Stepping on the entry runs the game's own
//! entry code, which loads the area's slot map; the engine queues the
//! virtual room first ([`super::content::enter_from_overworld`]). Called
//! from `content::register` / `content::end_of_frame` only while
//! `ContentOpts::virtual_rooms` is on.
//!
//! # Location slots
//!
//! [`plan`] runs once per [`register`] (a pure function of the ROM image
//! and the options). Candidate slots are
//! `content_mem::{WEST,EAST,DEATH_MOUNTAIN,MAZE_ISLAND}_FREE_SLOTS`,
//! checked against the image: West and East slots must have all four
//! bytes zero; a Death Mountain / Maze Island slot must hold the same four
//! bytes in both banks' copies (the shared leftover layout; a randomized
//! table that writes one continent's entries breaks the match).
//!
//! * **Claimed**: the slots `z2_content::areas::SLOTS` hands out are used
//!   as given when every mouth of the area in that region has a claim,
//!   the claims are free on this image and lie 0-3 above mouth 0's (the
//!   entrance index, below). Claimed slots are kept out of the runtime
//!   pools (both Death Mountain and Maze Island for a slot of either).
//! * **Runtime** (areas without claims, the developer rooms): every
//!   registered area, enabled or not, in registry order, gets one run of
//!   consecutive slots per region (mouth `n` on `base + n`, at most four),
//!   so an area keeps its slots whatever else is on. Town entrances take
//!   the highest free run (after the original towns, so the flute warp's
//!   town ordinals keep their meaning), everything else the lowest.
//!
//! A run that does not fit is skipped, and so is an entrance whose room
//! has no live slot map ([`super::content::location_target`]).
//!
//! The four location bytes (`$6A00/$6A3F/$6A7E/$6ABD + slot`):
//!
//! * byte 0: row + 30, bit 7 ("external": the world load path) when the
//!   area is not in world 0;
//! * byte 1: column, bits 6-7 the entrance index `n` (the entry code sets
//!   `$0748 = slot - n`, so the area's `$FC + n` exit bytes,
//!   `Target::OutsideAt(n)`, come out at entrance `n`);
//! * byte 2: the table's slot map, bits 6-7 the entry page (`$075C`:
//!   page 0 enters at the left end facing right, page 3 at the right end
//!   of a four-page room, pages 1 and 2 in the middle of that page);
//! * byte 3: world << 2 | region (`$0706` value) for an external entry;
//!   bit 5 ("enter from the right") when the page is the room's last and
//!   is page 1 or 2: Link then starts at that page's right edge facing
//!   left (measured: without it he starts mid-page, and the original
//!   tables only use page 3 for right-hand mouths of four-page rooms).
//!
//! # When they are written
//!
//! The world loader (`bank7_code18`, `$CD40`, untrapped: it spans NMIs)
//! copies the region's table over `$6A00` on every world load. The
//! engine writes its slots in the `bank7_go_outside` hook (`$CCB3`, mode
//! 1, which reads `$6A00[$0748]` right after every load and after every
//! cave), and again at the end of every overworld frame (mode 5, world 0)
//! in case the options changed or a state from without them was loaded.
//!
//! # Entering
//!
//! Hook on `bank7_code17` (`$CB35`, mode 6, the key-area entry, chained
//! over its port and the `qol` hook): when `$0748` is one of the region's
//! placed slots and the WRAM bytes are the engine's, queue the room
//! ([`super::content::enter_from_overworld`]) and run the original. For an
//! external entry it then supplies the codes the original derives from
//! the slot number (`$056B` town code from `$2C` up, `$056C` palace code
//! from `$34` up), which are garbage for a free slot: the code of the
//! first original location of the same world in that region's table
//! (West towns 0, East towns 4, palace codes by group), else 0 / 4 / 0.
//! Entering from Death Mountain or Maze Island it also puts `$070A` back
//! to its value before the entry (the original copies `$0706` into it,
//! which would bring Link back out on Maze Island).
//!
//! # Map tiles and terrain
//!
//! The overworld map in WRAM is run-length coded (one byte per run, high
//! nibble length - 1, low nibble terrain; window start from the bank-0
//! row-pointer build's operands at `$87F3`, `$7C00`, or `$7A00` after the
//! randomizer's size patch; window end `$7F80`), and `overworld4` (mode 3)
//! builds 75 row pointers at `$6000`. Painting a tile splits only the run
//! that holds it, so the rows after it shift by up to two bytes per tile.
//! The palace-to-stone routine (`$E01B`, mode 2) writes absolute window
//! addresses, so the engine hooks it: before it runs, the rows the engine
//! painted get their original bytes back (a run of one tile that changed
//! since, a broken boulder, keeps its new tile); after it, the cells are
//! painted again, before mode 3 builds the pointers and draws the screen.
//! The original bytes of each painted row and its edits are kept in
//! [`SCRATCH`] (`$6C58-$6CFF`, free WRAM); [`MARKER`] in `$6BF8-$6BFB`
//! (map 63's connection bytes, written by every world load and never read
//! in world 0) says the window holds painted rows, so a fresh world load
//! is never "restored". A frame on the overworld that finds no marker
//! paints and rebuilds the row pointers itself (options changed on the
//! overworld; the screen shows it once it scrolls or reloads). A row that
//! would overflow the window or the scratch keeps its original bytes.
//!
//! Hidden entrances ([`z2_content::TileKind::Hidden`]) change no tile.
//!
//! # Turning it off
//!
//! [`teardown`] (from `enh::apply` before the options change) restores the
//! painted rows and, on the overworld, the original bytes of the slots.
//!
//! # Limits
//!
//! * A world-0 (cave) area is entered only from its own region's table
//!   (validation rejects other entrances): an exit from a cave entered
//!   through a world load would come out in the wrong region.
//! * Exiting puts Link on the entrance tile through the game's own
//!   `bank7_go_outside`, facing down (mode 2 sets `$0562 = 4`), like every
//!   original cave; the entry scan only runs after a step, so standing on
//!   the tile does not re-enter.
//! * A virtual town's town code is a stand-in (the town engine keys the
//!   people on its own data); a wise man there would teach that town's
//!   spell.

use super::content::{self, ContentOpts, VRoomId};
use super::content_mem as mem;
use crate::game::Game;

/// `bank7_code17` (mode 6, key-area entry).
pub const HOOK_ENTRY: u16 = 0xCB35;
/// `bank7_go_outside` (mode 1).
pub const HOOK_GO_OUTSIDE: u16 = 0xCCB3;
/// `bank7_Turn_Palaces_into_Stone_Bank_1` (mode 2).
pub const HOOK_STONE: u16 = 0xE01B;
/// Bank-0 row-pointer build (`overworld4`); its first 8 bytes are `LDA
/// #lo : STA $0E : LDA #hi : STA $0F`, the window start.
pub const ROW_BUILD: u16 = 0x87F3;
/// End (exclusive) of the overworld map window (vanilla and randomizer).
pub const WINDOW_END: u16 = 0x7F80;
/// Row-pointer table (75 little-endian words) the overworld walks.
pub const ROW_POINTERS: u16 = 0x6000;
/// Rows the row-pointer table holds.
pub const ROWS: usize = 75;
/// Free WRAM the engine keeps the painted rows' original bytes in.
pub const SCRATCH: mem::WramRange = mem::WramRange {
    start: 0x6C58,
    len: 0x6D00 - 0x6C58,
};
/// Where [`MARKER`] goes (map 63's connection bytes; world 0 never reads
/// them and every world load rewrites them).
pub const MARKER_ADDR: u16 = 0x6BF8;
/// First three marker bytes; the fourth is the region (0-3).
pub const MARKER: [u8; 3] = [0xC3, 0x7E, 0x5A];

const ADDR_MODE: usize = 0x0736;
const ADDR_WORLD: usize = 0x0707;
const ADDR_PREV_REGION: usize = 0x070A;
const ADDR_AREA: usize = 0x0748;
const ADDR_TOWN_CODE: usize = 0x056B;
const ADDR_PALACE_CODE: usize = 0x056C;
const MODE_OVERWORLD: u8 = 5;
/// Raw row of map row 0 in the location tables.
const ROW_BASE: u8 = 30;

/// Location tables per region (0 West, 1 Death Mountain, 2 East, 3 Maze
/// Island): bank and base.
const LOC_TABLES: [(u8, u16); 4] = [(1, 0x861F), (1, 0xA0FC), (2, 0x861F), (2, 0xA0FC)];

/// One entrance on the overworld.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Placement {
    /// Region (0 West, 1 Death Mountain, 2 East, 3 Maze Island).
    pub region: u8,
    /// Location slot.
    pub slot: u8,
    /// The four location bytes.
    pub bytes: [u8; 4],
    /// Room entered.
    pub room: VRoomId,
    /// World of the room (`$0707` after the entry).
    pub world: u8,
    /// Map row and column.
    pub row: u8,
    /// See [`Self::row`].
    pub col: u8,
    /// Terrain code painted on the tile (`None` for a hidden entrance).
    pub tile: Option<u8>,
    /// `$056B` / `$056C` after an external entry.
    pub codes: Option<(u8, u8)>,
}

/// Static configuration [`register`] computes (kept in `Game::enh_hooks`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Plan {
    /// Entrances of the enabled areas that got a slot.
    pub entries: Vec<Placement>,
    /// Terrain cells `(region, row, col, tile)` of the enabled areas.
    pub terrain: Vec<(u8, u8, u8, u8)>,
    /// Overworld map window start (`None`: unknown loader, no painting).
    pub window: Option<u16>,
}

/// This game's [`Plan`].
#[must_use]
pub fn plan_of(game: &Game) -> &Plan {
    &game.enh_hooks.overworld
}

/// The placed entrances.
#[must_use]
pub fn placements(game: &Game) -> &[Placement] {
    &plan_of(game).entries
}

fn prg_byte(game: &Game, bank: u8, addr: u16) -> u8 {
    game.prg
        .get(super::bank_offset(game.prg.len(), bank, addr))
        .copied()
        .unwrap_or(0)
}

/// The image's four bytes of `slot` in `region`'s table.
fn rom_slot(game: &Game, region: u8, slot: u8) -> [u8; 4] {
    let (bank, base) = LOC_TABLES[usize::from(region & 3)];
    std::array::from_fn(|k| prg_byte(game, bank, base + u16::from(slot) + 63 * k as u16))
}

/// Slots of `region` the engine may take on this image.
#[must_use]
pub fn free_slots(game: &Game, region: u8) -> Vec<u8> {
    let list = match region {
        0 => mem::WEST_FREE_SLOTS,
        1 => mem::DEATH_MOUNTAIN_FREE_SLOTS,
        2 => mem::EAST_FREE_SLOTS,
        _ => mem::MAZE_ISLAND_FREE_SLOTS,
    };
    list.iter()
        .copied()
        .filter(|&s| match region {
            0 | 2 => rom_slot(game, region, s) == [0; 4],
            _ => rom_slot(game, 1, s) == rom_slot(game, 3, s),
        })
        .collect()
}

/// `$0706` value of a region (Maze Island is region 1 too).
const fn region_field(region: u8) -> u8 {
    if region == 3 {
        1
    } else {
        region
    }
}

/// `$056B` / `$056C` the original derives for the first original location
/// of `world` in `region`'s table.
fn codes_for(game: &Game, region: u8, world: u8) -> (u8, u8) {
    let r = region_field(region);
    let find = |from: u8| {
        (from..=mem::ENTRY_SCAN_LAST).find(|&s| {
            let b = rom_slot(game, region, s);
            b[0] & 0x80 != 0 && (b[3] >> 2) & 7 == world
        })
    };
    let east = if r != 0 { 4 } else { 0 };
    let town = find(mem::TOWN_CODE_SLOT_BASE).map_or(if world == 2 { 4 } else { 0 }, |s| {
        ((s - mem::TOWN_CODE_SLOT_BASE) >> 1) + east
    });
    let palace = find(PALACE_CODE_SLOT_BASE).map_or(0, |s| s - PALACE_CODE_SLOT_BASE);
    (town, palace)
}

/// From this slot up the entry code derives a palace code.
const PALACE_CODE_SLOT_BASE: u8 = 0x34;

/// Pages of a static room (side-view header byte 1, bits 5-6).
fn room_pages(sideview: &[u8]) -> u8 {
    sideview.get(1).map_or(1, |b| ((b >> 5) & 3) + 1)
}

/// Work out the [`Plan`] for `opts` (needs `content`'s runtime first).
#[must_use]
pub fn plan(game: &Game, opts: &ContentOpts) -> Plan {
    let mut free: [Vec<u8>; 4] = std::array::from_fn(|r| free_slots(game, r as u8));
    // Slots the authors claimed (`z2_content::areas::SLOTS`) are taken
    // first, from every pool sharing the layout (Death Mountain and Maze
    // Island), so runtime runs for unclaimed areas never land on them.
    let shared = |a: u8, b: u8| a == b || (a % 2 == 1 && b % 2 == 1);
    let claimed = |region: u8, slot: u8| {
        z2_content::areas::SLOTS
            .iter()
            .any(|c| c.slot == slot && shared(c.region.id(), region))
    };
    let claim_ok: [Vec<u8>; 4] = std::array::from_fn(|r| {
        free[r]
            .iter()
            .copied()
            .filter(|&s| claimed(r as u8, s))
            .collect()
    });
    for (r, pool) in free.iter_mut().enumerate() {
        pool.retain(|&s| !claimed(r as u8, s));
    }
    let mut out = Plan {
        window: window_base(game),
        ..Plan::default()
    };
    for (group, area) in content::all_areas() {
        let on = opts.enables(group);
        let mut placed_any = false;
        let mut regions: Vec<u8> = Vec::new();
        for e in area.entrances {
            if !regions.contains(&e.region) {
                regions.push(e.region);
            }
        }
        for region in regions.into_iter().filter(|&r| r < 4) {
            let mouths: Vec<(usize, _)> = area
                .entrances
                .iter()
                .enumerate()
                .filter(|(_, e)| e.region == region)
                .collect();
            let k = mouths.len();
            if k == 0 || k > 4 {
                continue;
            }
            // A cave is entered only from its own region's table.
            if area.world == 0 && area.table != region {
                continue;
            }
            // The claimed slots of this area's mouths here, when every mouth
            // has one, they are free on this image and give entrance
            // indices 0-3 from mouth 0 (`slot - mouth 0's slot`).
            let claims: Option<Vec<u8>> = mouths
                .iter()
                .map(|(i, _)| {
                    z2_content::areas::slot_of(area.name, *i)
                        .filter(|c| c.region.id() == region)
                        .map(|c| c.slot)
                })
                .collect();
            let claims = claims.filter(|c| {
                c.iter().all(|s| claim_ok[usize::from(region)].contains(s))
                    && c.iter().all(|&s| s >= c[0] && s - c[0] <= 3)
            });
            let slots: Vec<u8> = if let Some(c) = claims {
                c
            } else {
                let fits = |s: u8, f: &[u8]| (0..k as u8).all(|i| f.contains(&s.wrapping_add(i)));
                let pool = &mut free[usize::from(region)];
                let town = matches!(area.world, 1 | 2);
                let base = if town {
                    pool.iter().rev().copied().find(|&s| fits(s, pool))
                } else {
                    pool.iter().copied().find(|&s| fits(s, pool))
                };
                let Some(base) = base else {
                    continue;
                };
                pool.retain(|&s| s < base || s >= base + k as u8);
                (0..k as u8).map(|n| base + n).collect()
            };
            if !on {
                continue;
            }
            for (&(_, e), &slot) in mouths.iter().zip(&slots) {
                let n = slot - slots[0];
                let Some(room) = content::room_id(area, e.room) else {
                    continue;
                };
                let Some(target) = content::location_target(game, room) else {
                    continue;
                };
                let world = area.world;
                let external = world != 0;
                let pages = area
                    .rooms
                    .get(usize::from(e.room))
                    .map_or(1, |r| room_pages(r.sideview));
                let page = e.page & 3;
                let code = page;
                let right = page != 0 && page != 3 && page + 1 == pages;
                let b3 = if external {
                    (world << 2) | region_field(region)
                } else {
                    0
                } | if right { 0x20 } else { 0 };
                let bytes = [
                    (e.row + ROW_BASE) & 0x7F | if external { 0x80 } else { 0 },
                    (e.col & 0x3F) | (n << 6),
                    (target.map & 0x3F) | (code << 6),
                    b3,
                ];
                let tile = match e.tile {
                    0 => Some(0x1),
                    1 => Some(0x0),
                    2 => Some(0x2),
                    _ => None,
                };
                out.entries.push(Placement {
                    region,
                    slot,
                    bytes,
                    room,
                    world,
                    row: e.row,
                    col: e.col,
                    tile,
                    codes: external.then(|| codes_for(game, region, world)),
                });
                placed_any = true;
            }
        }
        if on && (placed_any || area.entrances.is_empty()) {
            out.terrain.extend(
                area.terrain
                    .iter()
                    .filter(|t| t.region < 4)
                    .map(|t| (t.region, t.row, t.col, t.tile & 0x0F)),
            );
        }
    }
    out
}

/// Window start the row-pointer build uses, when its bytes are the known
/// `LDA #lo : STA $0E : LDA #hi : STA $0F`.
fn window_base(game: &Game) -> Option<u16> {
    let b: [u8; 8] = std::array::from_fn(|i| prg_byte(game, 0, ROW_BUILD + i as u16));
    let base = u16::from_le_bytes([b[1], b[5]]);
    (b[0] == 0xA9
        && b[2] == 0x85
        && b[3] == 0x0E
        && b[4] == 0xA9
        && b[6] == 0x85
        && b[7] == 0x0F
        && (0x6200..WINDOW_END - 0x100).contains(&base))
    .then_some(base)
}

// ------------------------------------------------------------ register

/// Install this module's patches and hooks.
pub(crate) fn register(game: &mut Game, opts: &ContentOpts) {
    let p = plan(game, opts);
    let any = !p.entries.is_empty() || !p.terrain.is_empty();
    game.enh_hooks.overworld = p;
    if !any {
        return;
    }
    let hooks: [(&'static str, u16, crate::traps::TrapFn); 3] = [
        ("enh_content_ow_entry", HOOK_ENTRY, hook_entry),
        (
            "enh_content_ow_go_outside",
            HOOK_GO_OUTSIDE,
            hook_go_outside,
        ),
        ("enh_content_ow_stone", HOOK_STONE, hook_stone),
    ];
    for (name, addr, func) in hooks {
        let cycles = if game.traps.get(addr).is_some() {
            None
        } else {
            Some(0)
        };
        super::hook(game, name, Some(7), addr, func, cycles);
    }
}

/// Per-frame work after [`Game::step`]: on the overworld, keep the slots
/// written and the map painted.
pub(crate) fn end_of_frame(game: &mut Game, opts: &ContentOpts) {
    let _ = opts;
    if game.ram[ADDR_WORLD] != 0 || game.ram[ADDR_MODE] != MODE_OVERWORLD {
        return;
    }
    let p = plan_of(game);
    if p.entries.is_empty() && p.terrain.is_empty() {
        return;
    }
    inject(game);
    if let Some(region) = current_region(game) {
        if !marked(game, region) {
            paint(game, region, true);
        }
    }
}

/// Undo the painted rows (and, on the overworld, the written slots) of the
/// options in force; `enh::apply` calls it before changing them.
pub(crate) fn teardown(game: &mut Game) {
    if game.ram[ADDR_WORLD] != 0 {
        return;
    }
    let Some(region) = current_region(game) else {
        return;
    };
    let overworld = game.ram[ADDR_MODE] == MODE_OVERWORLD;
    if marked(game, region) {
        unpaint(game, region);
        if overworld {
            if let Some(base) = plan_of(game).window {
                rebuild_row_pointers(game, base);
            }
        }
    }
    if overworld {
        let slots: Vec<u8> = placements(game)
            .iter()
            .filter(|p| p.region == region)
            .map(|p| p.slot)
            .collect();
        for s in slots {
            let b = rom_slot(game, region, s);
            write_slot(game, s, b);
        }
    }
}

// ------------------------------------------------------------ slots

/// Region (0-3) of the overworld table in use, in world 0.
fn current_region(game: &Game) -> Option<u8> {
    if game.ram[ADDR_WORLD] != 0 {
        return None;
    }
    content::current_table(game).filter(|&t| t < 4)
}

fn slot_bytes(game: &Game, slot: u8) -> [u8; 4] {
    std::array::from_fn(|k| game.wram[usize::from(mem::LOC_WRAM[k] - 0x6000) + usize::from(slot)])
}

fn write_slot(game: &mut Game, slot: u8, b: [u8; 4]) {
    for (k, v) in b.into_iter().enumerate() {
        game.wram[usize::from(mem::LOC_WRAM[k] - 0x6000) + usize::from(slot)] = v;
    }
}

/// Write the current region's placed slots into the WRAM table copy.
fn inject(game: &mut Game) {
    let Some(region) = current_region(game) else {
        return;
    };
    let todo: Vec<(u8, [u8; 4])> = placements(game)
        .iter()
        .filter(|p| p.region == region)
        .map(|p| (p.slot, p.bytes))
        .collect();
    for (s, b) in todo {
        if slot_bytes(game, s) != b {
            write_slot(game, s, b);
        }
    }
}

fn hook_go_outside(game: &mut Game) {
    inject(game);
    super::call_original(game, HOOK_GO_OUTSIDE);
}

fn hook_entry(game: &mut Game) {
    let raw = game.ram[ADDR_AREA];
    let hit = current_region(game).and_then(|region| {
        placements(game)
            .iter()
            .find(|p| p.region == region && p.slot == raw)
            .copied()
    });
    let hit = hit.filter(|p| raw != 0xFF && slot_bytes(game, p.slot) == p.bytes);
    if let Some(p) = hit {
        content::enter_from_overworld(game, p.room);
    }
    let prev = game.ram[ADDR_PREV_REGION];
    super::call_original(game, HOOK_ENTRY);
    let Some(p) = hit else {
        return;
    };
    if p.world != 0 && game.ram[ADDR_WORLD] == p.world {
        if let Some((town, palace)) = p.codes {
            game.ram[ADDR_TOWN_CODE] = town;
            game.ram[ADDR_PALACE_CODE] = palace;
        }
        if region_field(p.region) == 1 {
            game.ram[ADDR_PREV_REGION] = prev;
        }
    }
}

// ------------------------------------------------------------ map painting

/// Rows of a region's map.
const fn region_height(region: u8) -> usize {
    match region {
        0 | 2 => 75,
        _ => 60,
    }
}

fn marked(game: &Game, region: u8) -> bool {
    let m = usize::from(MARKER_ADDR - 0x6000);
    game.wram[m..m + 3] == MARKER && game.wram[m + 3] == region
}

fn set_marker(game: &mut Game, region: Option<u8>) {
    let m = usize::from(MARKER_ADDR - 0x6000);
    match region {
        Some(r) => {
            game.wram[m..m + 3].copy_from_slice(&MARKER);
            game.wram[m + 3] = r;
        }
        None => game.wram[m..m + 4].fill(0),
    }
}

/// Cells `(row, col, tile)` the plan paints in `region`, sorted, terrain
/// first and entrance tiles over it.
#[must_use]
pub fn edits(plan: &Plan, region: u8) -> Vec<(u8, u8, u8)> {
    let mut v: Vec<(u8, u8, u8)> = Vec::new();
    let mut put = |row: u8, col: u8, tile: u8| {
        if let Some(e) = v.iter_mut().find(|e| (e.0, e.1) == (row, col)) {
            e.2 = tile;
        } else {
            v.push((row, col, tile));
        }
    };
    for &(r, row, col, tile) in &plan.terrain {
        if r == region {
            put(row, col, tile);
        }
    }
    for p in plan.entries.iter().filter(|p| p.region == region) {
        if let Some(t) = p.tile {
            put(p.row, p.col, t);
        }
    }
    v.retain(|e| usize::from(e.0) < region_height(region) && e.1 < 64);
    v.sort_unstable();
    v
}

/// Paint the plan's cells into the window (the window must hold the
/// original layout) and set the marker; `pointers` also rebuilds the row
/// pointers.
fn paint(game: &mut Game, region: u8, pointers: bool) {
    let Some(base) = plan_of(game).window else {
        return;
    };
    let cells = edits(plan_of(game), region);
    let (start, end) = (usize::from(base - 0x6000), usize::from(WINDOW_END - 0x6000));
    let (win, backup) = paint_window(
        &game.wram[start..end],
        region_height(region),
        &cells,
        usize::from(SCRATCH.len),
    );
    game.wram[start..end].copy_from_slice(&win);
    let s = usize::from(SCRATCH.start - 0x6000);
    game.wram[s..s + backup.len()].copy_from_slice(&backup);
    set_marker(game, Some(region));
    if pointers {
        rebuild_row_pointers(game, base);
    }
}

/// Give the painted rows their original bytes back and clear the marker.
fn unpaint(game: &mut Game, region: u8) {
    let Some(base) = plan_of(game).window else {
        set_marker(game, None);
        return;
    };
    let (start, end) = (usize::from(base - 0x6000), usize::from(WINDOW_END - 0x6000));
    let s = usize::from(SCRATCH.start - 0x6000);
    let backup = game.wram[s..s + usize::from(SCRATCH.len)].to_vec();
    if let Some(win) = unpaint_window(&game.wram[start..end], region_height(region), &backup) {
        game.wram[start..end].copy_from_slice(&win);
    }
    game.wram[s] = 0;
    set_marker(game, None);
}

fn hook_stone(game: &mut Game) {
    let region = current_region(game);
    if let Some(r) = region {
        if marked(game, r) {
            unpaint(game, r);
        }
    }
    super::call_original(game, HOOK_STONE);
    if let Some(r) = region {
        paint(game, r, false);
    }
}

/// The `$6000` row pointers for the window at `base`, as `overworld4`
/// builds them.
fn rebuild_row_pointers(game: &mut Game, base: u16) {
    let start = usize::from(base - 0x6000);
    let spans = row_spans(&game.wram[start..], ROWS);
    let p0 = usize::from(ROW_POINTERS - 0x6000);
    for (r, (s, _)) in spans.into_iter().enumerate() {
        let [lo, hi] = (base + s as u16).to_le_bytes();
        game.wram[p0 + 2 * r] = lo;
        game.wram[p0 + 2 * r + 1] = hi;
    }
}

const fn run_len(b: u8) -> usize {
    (b >> 4) as usize + 1
}

/// Byte spans of the first `rows` rows of the map `win`, by the row walk
/// of `overworld4` / `L8C30`: a row takes bytes while the tile count stays
/// under 65.
#[must_use]
pub fn row_spans(win: &[u8], rows: usize) -> Vec<(usize, usize)> {
    let mut out = Vec::with_capacity(rows);
    let mut off = 0;
    for _ in 0..rows {
        let start = off;
        let mut acc = 0;
        while off < win.len() {
            let l = run_len(win[off]);
            if acc + l >= 65 {
                break;
            }
            acc += l;
            off += 1;
        }
        out.push((start, off));
    }
    out
}

/// The 64 tiles of one row's bytes (`None` unless exactly 64).
#[must_use]
pub fn decode_row(bytes: &[u8]) -> Option<[u8; 64]> {
    let mut t = [0u8; 64];
    let mut c = 0;
    for &b in bytes {
        for _ in 0..run_len(b) {
            *t.get_mut(c)? = b & 0x0F;
            c += 1;
        }
    }
    (c == 64).then_some(t)
}

fn encode_runs(tiles: &[u8], out: &mut Vec<u8>) {
    let mut i = 0;
    while i < tiles.len() {
        let t = tiles[i];
        let mut n = 1;
        while i + n < tiles.len() && tiles[i + n] == t && n < 16 {
            n += 1;
        }
        out.push(((n as u8 - 1) << 4) | t);
        i += n;
    }
}

/// One row's bytes with `cells` (`(col, tile)`) painted: every byte whose
/// run holds no changed tile is kept as is (single-tile runs stay single,
/// the way the hammer and the flute expect them), the others are split.
#[must_use]
pub fn paint_row(bytes: &[u8], cells: &[(u8, u8)]) -> Option<Vec<u8>> {
    let orig = decode_row(bytes)?;
    let mut tiles = orig;
    for &(c, t) in cells {
        *tiles.get_mut(usize::from(c))? = t & 0x0F;
    }
    let mut out = Vec::with_capacity(bytes.len() + 2 * cells.len());
    let mut c = 0;
    for &b in bytes {
        let l = run_len(b);
        if tiles[c..c + l] == orig[c..c + l] {
            out.push(b);
        } else {
            encode_runs(&tiles[c..c + l], &mut out);
        }
        c += l;
    }
    Some(out)
}

/// Paint `cells` (`(row, col, tile)`, sorted) into a map window holding
/// the original layout (`height` rows). Returns the new window (same
/// length; later rows shift, the bytes past the last row are padding and
/// drop off the end) and the backup record: count, then per painted row
/// `row, original length, original bytes, cell count, (col, tile)...`,
/// at most `cap` bytes. A row that would not fit stays unpainted.
#[must_use]
pub fn paint_window(
    win: &[u8],
    height: usize,
    cells: &[(u8, u8, u8)],
    cap: usize,
) -> (Vec<u8>, Vec<u8>) {
    let spans = row_spans(win, height);
    let used = spans.last().map_or(0, |s| s.1);
    let mut content = Vec::with_capacity(win.len());
    let mut backup = vec![0u8];
    let mut growth = 0usize;
    for (r, &(s, e)) in spans.iter().enumerate() {
        let orig = &win[s..e];
        let row_cells: Vec<(u8, u8)> = cells
            .iter()
            .filter(|c| usize::from(c.0) == r)
            .map(|c| (c.1, c.2))
            .collect();
        let new = (!row_cells.is_empty())
            .then(|| paint_row(orig, &row_cells))
            .flatten()
            .filter(|n| n.as_slice() != orig);
        match new {
            Some(n)
                if used + growth + (n.len() - orig.len()) <= win.len()
                    && backup.len() + 3 + orig.len() + 2 * row_cells.len() <= cap
                    && backup[0] < u8::MAX
                    && orig.len() <= usize::from(u8::MAX)
                    && row_cells.len() <= usize::from(u8::MAX) =>
            {
                growth += n.len() - orig.len();
                backup[0] += 1;
                backup.push(r as u8);
                backup.push(orig.len() as u8);
                backup.extend_from_slice(orig);
                backup.push(row_cells.len() as u8);
                for (c, t) in &row_cells {
                    backup.push(*c);
                    backup.push(*t);
                }
                content.extend_from_slice(&n);
            }
            _ => content.extend_from_slice(orig),
        }
    }
    content.extend_from_slice(&win[used..]);
    content.truncate(win.len());
    (content, backup)
}

/// One painted row of a backup record: row, original bytes, cells.
type BackupRow = (usize, Vec<u8>, Vec<(u8, u8)>);

/// Undo [`paint_window`] with its backup record: each painted row gets its
/// original bytes back, except that a single-tile run whose tile changed
/// since the painting (not by the painting) keeps the new tile. `None`
/// when the record does not parse.
#[must_use]
pub fn unpaint_window(win: &[u8], height: usize, backup: &[u8]) -> Option<Vec<u8>> {
    let n = usize::from(*backup.first()?);
    let mut rows: Vec<BackupRow> = Vec::with_capacity(n);
    let mut i = 1;
    for _ in 0..n {
        let r = usize::from(*backup.get(i)?);
        let len = usize::from(*backup.get(i + 1)?);
        let orig = backup.get(i + 2..i + 2 + len)?.to_vec();
        i += 2 + len;
        let k = usize::from(*backup.get(i)?);
        let cells = backup
            .get(i + 1..i + 1 + 2 * k)?
            .chunks(2)
            .map(|c| (c[0], c[1]))
            .collect();
        i += 1 + 2 * k;
        rows.push((r, orig, cells));
    }
    let spans = row_spans(win, height);
    let used = spans.last().map_or(0, |s| s.1);
    let mut content = Vec::with_capacity(win.len());
    for (r, &(s, e)) in spans.iter().enumerate() {
        let cur = &win[s..e];
        let Some((_, orig, cells)) = rows.iter().find(|x| x.0 == r) else {
            content.extend_from_slice(cur);
            continue;
        };
        let mut restored = orig.clone();
        let now = decode_row(cur);
        let want = paint_row(orig, cells).and_then(|b| decode_row(&b));
        if let (Some(now), Some(want)) = (now, want) {
            let mut c = 0;
            for b in &mut restored {
                let l = run_len(*b);
                if l == 1 && now[c] != want[c] {
                    *b = (*b & 0xF0) | now[c];
                }
                c += l;
            }
        }
        content.extend_from_slice(&restored);
    }
    content.extend_from_slice(&win[used..]);
    content.resize(win.len().max(content.len()), 0x0B);
    content.truncate(win.len());
    Some(content)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A tiny map: rows of 64 tiles as given runs.
    fn map(rows: &[&[u8]]) -> Vec<u8> {
        let mut v: Vec<u8> = rows.iter().flat_map(|r| r.iter().copied()).collect();
        v.resize(v.len() + 16, 0x0B);
        v
    }

    #[test]
    fn spans_follow_the_row_walk() {
        let w = map(&[&[0xF5, 0xF5, 0xF5, 0xF5], &[0xFB, 0xFB, 0xFB, 0xEB, 0x0E]]);
        assert_eq!(row_spans(&w, 2), vec![(0, 4), (4, 9)]);
        assert_eq!(decode_row(&w[4..9]).unwrap()[63], 0x0E);
        assert!(decode_row(&w[0..3]).is_none());
    }

    #[test]
    fn painting_splits_only_the_run_it_touches() {
        let row = [0xF5, 0x0E, 0xE5, 0xF5, 0xF5];
        let out = paint_row(&row, &[(20, 0x1)]).unwrap();
        assert_eq!(out, vec![0xF5, 0x0E, 0x25, 0x01, 0xA5, 0xF5, 0xF5]);
        let t = decode_row(&out).unwrap();
        assert_eq!((t[16], t[19], t[20], t[21]), (0x0E, 0x05, 0x01, 0x05));
        // Painting a tile that is already there changes nothing.
        assert_eq!(paint_row(&row, &[(16, 0x0E)]).unwrap(), row.to_vec());
    }

    #[test]
    fn paint_and_unpaint_round_trip_and_keep_a_broken_boulder() {
        let w = map(&[
            &[0xF5, 0xF5, 0xF5, 0xF5],
            &[0xF5, 0x0E, 0xE5, 0xF5, 0xF5],
            &[0xFC, 0xFC, 0xFC, 0xFC],
        ]);
        let cells = [(1, 30, 0x1), (2, 3, 0x4), (2, 4, 0xD)];
        let (painted, backup) = paint_window(&w, 3, &cells, 64);
        assert_eq!(painted.len(), w.len());
        assert_eq!(backup[0], 2);
        let spans = row_spans(&painted, 3);
        let r1 = decode_row(&painted[spans[1].0..spans[1].1]).unwrap();
        let r2 = decode_row(&painted[spans[2].0..spans[2].1]).unwrap();
        assert_eq!(
            (r1[30], r1[16], r2[3], r2[4], r2[5]),
            (0x1, 0xE, 0x4, 0xD, 0xC)
        );
        // Round trip.
        let back = unpaint_window(&painted, 3, &backup).unwrap();
        assert_eq!(&back[..13], &w[..13]);
        // The hammer breaks the boulder (row 1, column 16) after painting.
        let mut hit = painted.clone();
        hit[spans[1].0 + 1] = 0x04;
        let back = unpaint_window(&hit, 3, &backup).unwrap();
        let mut want = w.clone();
        want[5] = 0x04;
        assert_eq!(&back[..13], &want[..13]);
    }

    #[test]
    fn a_row_that_does_not_fit_stays_unpainted() {
        let w: Vec<u8> = [0xF5u8, 0xF5, 0xF5, 0xF5].to_vec();
        let (out, backup) = paint_window(&w, 1, &[(0, 3, 0x1)], 64);
        assert_eq!(out, w);
        assert_eq!(backup, vec![0]);
        let w = map(&[&[0xF5, 0xF5, 0xF5, 0xF5]]);
        let (_, backup) = paint_window(&w, 1, &[(0, 3, 0x1)], 4);
        assert_eq!(backup, vec![0]);
    }

    #[test]
    fn plan_without_a_rom_is_empty_and_harmless() {
        let g = Game::new();
        let opts = ContentOpts {
            virtual_rooms: true,
            test_room: true,
            ..ContentOpts::default()
        };
        let p = plan(&g, &opts);
        // No PRG: no slot map is live, so nothing is placed.
        assert!(p.entries.is_empty());
        assert!(p.terrain.is_empty());
        assert_eq!(free_slots(&g, 0).len(), mem::WEST_FREE_SLOTS.len());
    }
}
