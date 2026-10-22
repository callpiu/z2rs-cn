//! New areas (group `N`): rooms that are not in the original game.
//!
//! ZALiA adds a palace, a town, challenge rooms, minigames and caves. z2rs
//! gets new areas through **virtual rooms**: every authored room is data
//! (`z2-content`, registered in [`super::content_areas`]), never bytes
//! copied from the ROM or from ZALiA. Options ([`ContentOpts`]):
//!
//! * `virtual_rooms`: the master switch. Installs the slot redirection and
//!   the hooks below. Every other option only acts with it on.
//! * `test_room`: a developer room between Great Palace maps 1 and 2.
//! * `parapa_annex`, `challenge_rooms`, `minigames`, `hidden_caves`,
//!   `dark_isle`: the areas of `z2_content::areas::<option>`.
//!
//! # Mechanism
//!
//! Each of the eight side-view table sets ([`content_mem::SETS`]) gets one
//! **slot**: a map the original game never loads (West 28, Death Mountain
//! 27, East 28, Maze Island 46, towns 62, Great Palace 60), or map 63 for
//! the two palace groups, whose 63 maps are all in use. Every virtual room
//! of a table runs in that table's slot: the slot's side-view and enemy
//! pointers lead to two fixed WRAM buffers ([`WRAM_ROOM_BUF`],
//! [`WRAM_ENEMY_BUF`]), and right before the game loads the slot the engine
//! copies the room being entered into them, its exits into the WRAM
//! connection mirror (`$6AFC + map * 4`) and its persistent page bits into
//! the slot's live item nibble.
//!
//! * **Slot pointers**: maps 0-62 are table entries (a PRG patch of two
//!   words). Map 63 has no entry (its fetch would read map 0's enemy word),
//!   so the pointer read in `bank7_code13` (`$C524`, `LDA $0561 : ASL :
//!   TAY`) becomes `JSR $C527 : NOP : NOP`, and the trap at
//!   [`HOOK_MAP63_PTR`] does the same three instructions, pointing `($00)`
//!   / `($02)` at a two-word table in scratch WRAM ([`WRAM_MAP63_PTRS`])
//!   when the map is 63.
//! * **Area load** (`bank7_code13`, `$C4CB`, hooked): when the map being
//!   loaded is the slot, enter the pending virtual room (set by an exit
//!   hook or [`enter_from_overworld`]), or refresh the current one (death,
//!   reload); any other map clears the current room.
//! * **Exits** (`$CF4C` side exits, `$C68A` falls, `$C644` elevator, `$CFFC`
//!   town doors; hooked, chained over the default ports): work out which
//!   connection or door byte the routine reads (world 0's `>= $1D` rule
//!   included), and when it belongs to a virtual room or to an original
//!   room's linked exit ([`z2_content::StaticLink`]) put the right byte
//!   there for that one call: an original byte as is, a virtual room as a
//!   proxy map (`62`, or the slot) whose result is then moved to the slot
//!   and the room queued for the area load. The ROM's connection and door
//!   tables are never patched; the WRAM mirror and the door byte are
//!   restored right after the call.
//! * **Items and doors**: every page of a virtual room with a collectable
//!   or a palace locked door has one persistent bit
//!   ([`super::content_areas::page_bit`]) in the pool of free-map item
//!   nibbles ([`content_mem::PROGRESS_POOL`]): battery-saved by the game's
//!   own save code, all set (not collected) on a new game and in saves
//!   made without this group. On entry the slot's live nibble gets the
//!   pages not yet taken; after every frame in the room, pages the game
//!   cleared (pickup `$E771`, a door opened with a key `$D9EA`) are
//!   recorded. (A boss kill clears page 3's bit, bank 4 `$9B83`; it is kept
//!   only when page 3 also holds an item or door.)
//! * **Container pieces**: a collectable byte [`HEART_PIECE`] /
//!   [`MAGIC_PIECE`] is shown and picked up as a heart / magic container
//!   (the buffer copy gets `$0F` / `$0E`). After the pickup the engine
//!   takes the container back unless that piece completed a set of
//!   [`PIECES_PER_CONTAINER`] (counted from the persistent bits, so the
//!   count is battery-saved too).
//!
//! # Runtime state
//!
//! [`EnhState`](super::EnhState) fields: `vroom` (current virtual room id
//! plus 1, 0 = none), `vroom_pending` (room the next slot load enters, id
//! plus 1 in bits 0-11; a fall also sets bit 15 and its page in bits 12-13),
//! `vroom_items` (fallback bits for pool maps a randomized ROM uses, see
//! below) and `vroom_cont` (container counts last frame, for pieces). The
//! buffers, the mirror and the pool nibbles are RAM/WRAM, so save states
//! and netplay rollback carry them. [`Runtime`] (slot maps, usable pool
//! maps) is static configuration in `Game::enh_hooks`.
//!
//! # Randomized ROMs
//!
//! [`register`] walks the ROM's own tables (overworld locations, random
//! battle maps, map 0 / ending restarts, connection and door closure, the
//! same walk as `cargo xtask content-survey`): a table is used only when
//! its slot map is still unreachable, a pool map only when it is
//! unreachable in every table sharing its block and its new-game template
//! nibble is all set. A pool bit that is not usable falls back to
//! `vroom_items` (save states only, not battery). Tables whose slot a seed
//! uses simply get no virtual rooms.
//!
//! # Limits
//!
//! * Enemies killed in a virtual room come back on every entry (the buffer
//!   is refilled), like rooms of a re-entered palace.
//! * Map 63 rooms (palace groups) are entered only through exits and
//!   overworld entries the engine drives: no connection byte can name 63.
//! * Palace code that keys behaviour on the map number sees the slot
//!   number (60, 63, ...).

use serde::{Deserialize, Serialize};

pub use super::content_areas::{
    all_areas, areas, find_room, flag_bit, item_pages, page_bit, room, room_count, room_id, rooms,
    tracked_pages, Group, GroupInfo, RoomRef, VRoomId, DEV_INN_FROM_MAP, DEV_PALACE_FROM_MAP,
    GROUPS, TEST_ROOM, TEST_ROOM_FROM_MAP,
};
use super::content_mem::{self as mem, SETS};
use crate::game::Game;
use z2_content::{Dest, StaticArea};

// ------------------------------------------------------------ options

/// New-area options. `Default` is the original game.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ContentOpts {
    /// Master switch: virtual rooms and the areas of the options below.
    pub virtual_rooms: bool,
    /// Developer test room between Great Palace maps 1 and 2.
    pub test_room: bool,
    /// Extra rooms off an original palace.
    pub parapa_annex: bool,
    /// Challenge rooms.
    pub challenge_rooms: bool,
    /// Minigame rooms (rules in `content_minigames`).
    pub minigames: bool,
    /// New overworld caves.
    pub hidden_caves: bool,
    /// The southern isle: eighth palace, monster town.
    pub dark_isle: bool,
}

impl ContentOpts {
    /// Whether anything in this group is on (the master switch; a
    /// sub-option alone is not).
    #[must_use]
    pub fn is_active(&self) -> bool {
        self.virtual_rooms
    }

    /// Append this group's stable identity encoding: the seven option
    /// bits, then [`content_hash`] (8 bytes, little endian), so peers built
    /// with different authored data do not match.
    pub fn write_identity(&self, out: &mut Vec<u8>) {
        out.extend_from_slice(&[
            u8::from(self.virtual_rooms),
            u8::from(self.test_room),
            u8::from(self.parapa_annex),
            u8::from(self.challenge_rooms),
            u8::from(self.minigames),
            u8::from(self.hidden_caves),
            u8::from(self.dark_isle),
        ]);
        out.extend_from_slice(&content_hash().to_le_bytes());
    }
}

// ------------------------------------------------------------ addresses

/// WRAM buffer every slot's side-view pointer leads to: the current
/// virtual room's side-view bytes (length byte first). First room buffer
/// of the verified free range ([`content_mem::VROOM_DATA_BASE`]).
pub const WRAM_ROOM_BUF: u16 = mem::VROOM_DATA_BASE;
/// Bytes reserved at [`WRAM_ROOM_BUF`].
pub const ROOM_BUF_LEN: usize = mem::VROOM_DATA_STRIDE as usize;
/// WRAM buffer every slot's enemy pointer leads to: the current virtual
/// room's enemy list (length byte first; [`content_mem::VROOM_ENEMY_BASE`]).
pub const WRAM_ENEMY_BUF: u16 = mem::VROOM_ENEMY_BASE;
/// Bytes reserved at [`WRAM_ENEMY_BUF`].
pub const ENEMY_BUF_LEN: usize = mem::VROOM_ENEMY_STRIDE as usize;
/// WRAM mirror of the world's connection table (`$6AFC + map * 4 + i`).
pub const CONN_MIRROR: u16 = mem::CONN_WRAM;
/// Two words (room buffer, enemy buffer) the map-63 pointer read goes
/// through: the first 4 bytes of [`content_mem::VROOM_SCRATCH`].
pub const WRAM_MAP63_PTRS: u16 = mem::VROOM_SCRATCH.start;
/// Town door table (bank 3, 4 bytes per map, read by `$CFFC`).
pub const DOOR_TABLE: u16 = 0x8817;
/// Bank of [`DOOR_TABLE`].
pub const DOOR_TABLE_BANK: u8 = 3;

/// `bank7_code13` (area loader).
pub const HOOK_AREA_LOAD: u16 = 0xC4CB;
/// `bank7_take_side_exit` (side exits, and the side exit that ends some falls).
pub const HOOK_SIDE_EXIT: u16 = 0xCF4C;
/// `bank7_take_elevator_exit`.
pub const HOOK_ELEVATOR_EXIT: u16 = 0xC644;
/// `bank7_take_door_exit` (town doorways, door table [`DOOR_TABLE`]).
pub const HOOK_DOOR_EXIT: u16 = 0xCFFC;
/// `EnhState::vroom_pending` bits: the room id + 1, and for a room queued
/// by a fall ([`HOOK_FALL`]) the fall flag and the landing page.
const PENDING_ID: u16 = 0x0FFF;
const PENDING_FALL: u16 = 0x8000;
const PENDING_PAGE_SHIFT: u16 = 12;

/// `$C68A`, game mode `$0C`: Link fell out of the bottom of the room. The
/// destination is read from the connection mirror later (bank 0, mode
/// `$0D`), by no routine the exit hooks see.
pub const HOOK_FALL: u16 = 0xC68A;
/// The pointer read patched for map 63 (`LDA $0561 : ASL : TAY`).
pub const MAP63_PATCH_AT: u16 = 0xC524;
/// Original bytes at [`MAP63_PATCH_AT`].
pub const MAP63_ORIGINAL: [u8; 5] = [0xAD, 0x61, 0x05, 0x0A, 0xA8];
/// Trap the patch calls (`JSR $C527`; the bytes there become `NOP NOP`,
/// which run after the trap returns).
pub const HOOK_MAP63_PTR: u16 = 0xC527;
/// Map number written for a virtual-room exit byte when the slot itself
/// cannot be (map 63, which reads as `$FC` "outside"; side exits always,
/// so the `< $24` town-house branch of `$CF4C` never runs).
pub const PROXY_MAP: u8 = 62;
/// The map number with no table entry (palace group slot).
pub const VIRTUAL_ONLY_MAP: u8 = mem::VIRTUAL_ONLY_MAP;

/// Collectable byte of a heart-container piece (shown as `$0F`).
pub const HEART_PIECE: u8 = 0x13;
/// Collectable byte of a magic-container piece (shown as `$0E`).
pub const MAGIC_PIECE: u8 = 0x14;
/// Pieces that make one container.
pub const PIECES_PER_CONTAINER: u32 = 4;
/// Heart / magic container item bytes.
const HEART_CONTAINER: u8 = 0x0F;
const MAGIC_CONTAINER: u8 = 0x0E;
/// Magic / heart containers (`$0783` / `$0784`).
const ADDR_CONTAINERS: usize = 0x0783;

const ADDR_SCENE: usize = 0x0561;
const ADDR_REGION: usize = 0x0706;
const ADDR_WORLD: usize = 0x0707;
const ADDR_PREV_REGION: usize = 0x070A;
const ADDR_PAGE: usize = 0x003B;
const ADDR_ELEV_DIR: usize = 0x0743;
/// `$0743` value while riding the elevator up.
const ELEV_UP: u8 = 0x08;
/// Towns table index.
const TOWNS: u8 = 4;

// ------------------------------------------------------------ runtime config

/// Static per-game configuration computed by [`register`] (kept in
/// `Game::enh_hooks`, not in save states).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Runtime {
    /// Live slot map per table ([`content_mem::SETS`] order); `None` when
    /// the table has no enabled room or its slot is in use on this ROM.
    pub slots: [Option<u8>; 8],
    /// Usable [`content_mem::PROGRESS_POOL`] entries (bit `i` = entry `i`).
    pub pool_ok: u64,
}

/// This game's [`Runtime`].
#[must_use]
pub fn runtime(game: &Game) -> &Runtime {
    &game.enh_hooks.content
}

/// The live slot map of table `table`, if virtual rooms run there.
#[must_use]
pub fn slot_map(game: &Game, table: u8) -> Option<u8> {
    runtime(game)
        .slots
        .get(usize::from(table))
        .copied()
        .flatten()
}

/// Table set index of the side-view Link is in (or about to load), from
/// `$0707` / `$0706` / `$070A` the way the world loader picks the bank.
#[must_use]
pub fn current_table(game: &Game) -> Option<u8> {
    match game.ram[ADDR_WORLD] {
        0 => match game.ram[ADDR_REGION] {
            0 => Some(0),
            2 => Some(2),
            1 => Some(if game.ram[ADDR_PREV_REGION] == 0 {
                1
            } else {
                3
            }),
            _ => None,
        },
        1 | 2 => Some(TOWNS),
        3 => Some(5),
        4 => Some(6),
        5 => Some(7),
        _ => None,
    }
}

// ------------------------------------------------------------ data checks

/// FNV-1a over the buffer addresses, the table sets, the group budgets and
/// every registered area (netplay identity input).
#[must_use]
pub fn content_hash() -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    let mut eat = |bytes: &[u8]| {
        for &b in bytes {
            h ^= u64::from(b);
            h = h.wrapping_mul(0x0000_0100_0000_01b3);
        }
    };
    eat(&WRAM_ROOM_BUF.to_le_bytes());
    eat(&WRAM_ENEMY_BUF.to_le_bytes());
    eat(&WRAM_MAP63_PTRS.to_le_bytes());
    for s in &SETS {
        eat(&[s.bank, s.reserved_map]);
        eat(&s.sideview_table.to_le_bytes());
        eat(&s.enemy_table.to_le_bytes());
        eat(&s.conn_table.to_le_bytes());
        eat(&s.item_block.to_le_bytes());
    }
    for p in &mem::PROGRESS_POOL {
        eat(&p.block.to_le_bytes());
        eat(&[p.map]);
    }
    for g in &GROUPS {
        eat(&[g.group as u8]);
        eat(&g.bit_base.to_le_bytes());
        eat(&g.bits.to_le_bytes());
        eat(&g.flags.to_le_bytes());
    }
    let dest = |d: &Dest| match *d {
        Dest::Byte(b) => [0, b, 0],
        Dest::Room { room, page } => [1, room, page],
    };
    for (g, a) in all_areas() {
        eat(&[0xA5, g as u8, a.world, a.table]);
        eat(a.name.as_bytes());
        eat(&(a.rooms.len() as u32).to_le_bytes());
        for r in a.rooms {
            eat(&[r.world, r.map]);
            eat(&(r.sideview.len() as u32).to_le_bytes());
            eat(r.sideview);
            eat(&(r.enemies.len() as u32).to_le_bytes());
            eat(r.enemies);
            for d in &r.exits {
                eat(&dest(d));
            }
            for d in &r.door_exits {
                match d {
                    Some(d) => eat(&dest(d)),
                    None => eat(&[2]),
                }
            }
        }
        for l in a.links {
            eat(&[l.map, u8::from(l.door), l.index, l.room, l.page]);
        }
        for e in a.entrances {
            eat(&[e.region, e.row, e.col, e.tile, e.room, e.page]);
        }
        for n in a.npcs {
            eat(&[n.room, n.enemy_index, n.message]);
        }
        for t in a.terrain {
            eat(&[t.region, t.row, t.col, t.tile]);
        }
        for m in a.messages {
            eat(&(m.len() as u32).to_le_bytes());
            eat(m);
        }
    }
    eat(&super::content_town::identity_bytes());
    h
}

/// Check the registered data ([`super::content_areas::validate`]) plus the
/// slot rules: no original exit byte or link names its table's slot map.
///
/// # Errors
/// The first problem found, in words.
pub fn validate() -> Result<(), String> {
    super::content_areas::validate()?;
    for (_, a) in all_areas() {
        let slot = SETS[usize::from(a.table)].reserved_map;
        for r in a.rooms {
            for d in r.exits.iter().chain(r.door_exits.iter().flatten()) {
                if let Dest::Byte(b) = *d {
                    if b < 0xFC && b >> 2 == slot {
                        return Err(format!(
                            "area {}: exit byte {b:#04x} names the slot",
                            a.name
                        ));
                    }
                }
            }
        }
        if a.links.iter().any(|l| l.map == slot) {
            return Err(format!("area {}: link from the slot map", a.name));
        }
    }
    Ok(())
}

// ------------------------------------------------------------ PRG helpers

fn prg_byte(game: &Game, bank: u8, addr: u16) -> Option<u8> {
    game.prg
        .get(super::bank_offset(game.prg.len(), bank, addr))
        .copied()
}

#[cfg(test)]
fn prg_word(game: &Game, bank: u8, addr: u16) -> Option<u16> {
    Some(u16::from_le_bytes([
        prg_byte(game, bank, addr)?,
        prg_byte(game, bank, addr.wrapping_add(1))?,
    ]))
}

/// Overworld location tables: (bank, base, own set).
const LOCS: [(u8, u16, u8); 4] = [
    (1, 0x861F, 0),
    (1, 0xA0FC, 1),
    (2, 0x861F, 2),
    (2, 0xA0FC, 3),
];
/// Random-battle map table (`$8409` in the overworld bank, 18 entries).
const RANDOM_BATTLE: u16 = 0x8409;

fn set_of(world: u8, region: u8, bank: u8) -> Option<u8> {
    match (world, region) {
        (0, 0) => Some(0),
        (0, 1) => Some(if bank == 2 { 3 } else { 1 }),
        (0, 2) => Some(2),
        (1 | 2, _) => Some(TOWNS),
        (3, _) => Some(5),
        (4, _) => Some(6),
        (5, _) => Some(7),
        _ => None,
    }
}

/// Maps (bit `m`) of each table the ROM can reach: roots from the
/// overworld location tables, the random-battle tables, map 0 of every
/// town / palace set and the Great Palace ending maps 61-62, closed over
/// every connection byte (and the town door table); world 0 areas `>= $1D`
/// also pull in area 0's bytes. Same walk as `cargo xtask content-survey`.
#[must_use]
pub fn reachable_maps(game: &Game) -> [u64; 8] {
    let mut used = [0u64; 8];
    for &(bank, base, own) in &LOCS {
        for i in 0..u16::from(mem::MAPS) {
            let b: [u8; 4] = std::array::from_fn(|k| {
                prg_byte(game, bank, base + i + 63 * k as u16).unwrap_or(0)
            });
            if b == [0; 4] {
                continue;
            }
            let world = (b[3] >> 2) & 7;
            let set = if world != 0 {
                set_of(world, b[3] & 3, bank)
            } else if b[0] & 0x80 != 0 {
                set_of(0, b[3] & 3, bank)
            } else {
                Some(own)
            };
            if let Some(s) = set {
                used[usize::from(s)] |= 1 << (b[2] & 0x3F);
            }
        }
    }
    for (bank, sets) in [(1u8, [0usize, 1]), (2, [2, 3])] {
        for i in 0..18 {
            let m = prg_byte(game, bank, RANDOM_BATTLE + i).unwrap_or(0) & 0x3F;
            for s in sets {
                used[s] |= 1 << m;
            }
        }
    }
    used[7] |= (1 << 61) | (1 << 62);
    for u in used.iter_mut().skip(usize::from(TOWNS)) {
        *u |= 1;
    }
    for (s, set) in SETS.iter().enumerate() {
        let mut todo: Vec<u8> = (0..64).filter(|m| used[s] & (1u64 << m) != 0).collect();
        let mut area0_done = false;
        while let Some(m) = todo.pop() {
            let mut sources = vec![m];
            if s < 4 && m >= mem::WORLD0_OWN_CONN_LIMIT && !area0_done {
                area0_done = true;
                sources.push(0);
            }
            for src in sources.into_iter().filter(|&x| x < mem::MAPS) {
                for page in 0..4u16 {
                    let at = u16::from(src) * 4 + page;
                    let mut bytes = vec![prg_byte(game, set.bank, set.conn_table + at)];
                    if s == usize::from(TOWNS) {
                        bytes.push(prg_byte(game, DOOR_TABLE_BANK, DOOR_TABLE + at));
                    }
                    for c in bytes.into_iter().flatten() {
                        let t = c >> 2;
                        if c & 0xFC != 0xFC && t < mem::MAPS && used[s] & (1 << t) == 0 {
                            used[s] |= 1 << t;
                            todo.push(t);
                        }
                    }
                }
            }
        }
    }
    used
}

/// Whether `map`'s nibble in `block` starts all set in the new-game
/// template.
fn template_full(game: &Game, block: u16, map: u8) -> bool {
    let (addr, high) = mem::item_nibble(block, map);
    let off = addr - 0x0600;
    prg_byte(game, 5, mem::ITEM_TEMPLATE + off).is_some_and(|b| {
        let nib = if high { b >> 4 } else { b & 0x0F };
        nib == 0x0F
    })
}

/// Work out the [`Runtime`] for `opts` on this PRG image.
#[must_use]
pub fn scan(game: &Game, opts: &ContentOpts) -> Runtime {
    let used = reachable_maps(game);
    let free_in = |sets: &[u8], map: u8| {
        sets.iter()
            .all(|&s| used[usize::from(s)] & (1u64 << map) == 0)
    };
    let mut rt = Runtime::default();
    let wanted: Vec<u8> = areas(opts).map(|a| a.table).collect();
    for (t, set) in SETS.iter().enumerate() {
        if !wanted.contains(&(t as u8)) {
            continue;
        }
        let map = set.reserved_map;
        let sharing: Vec<u8> = SETS
            .iter()
            .enumerate()
            .filter(|(_, o)| o.item_block == set.item_block)
            .map(|(k, _)| k as u8)
            .collect();
        let ok = if map == VIRTUAL_ONLY_MAP {
            free_in(&sharing, map)
                && (0..5).all(|i| {
                    prg_byte(game, 7, MAP63_PATCH_AT + i) == Some(MAP63_ORIGINAL[usize::from(i)])
                })
        } else {
            free_in(&sharing, map)
        };
        if ok {
            rt.slots[t] = Some(map);
        }
    }
    for (i, p) in mem::PROGRESS_POOL.iter().enumerate() {
        if free_in(p.sets, p.map) && template_full(game, p.block, p.map) {
            rt.pool_ok |= 1 << i;
        }
    }
    rt
}

// ------------------------------------------------------------ register

/// Install this group's patches and hooks ([`super::apply`], only when
/// active).
pub(crate) fn register(game: &mut Game, opts: &ContentOpts) {
    if !opts.virtual_rooms {
        return;
    }
    if validate().is_ok() {
        install(game, opts);
    }
    super::content_overworld::register(game, opts);
    super::content_town::register(game, opts);
    super::content_minigames::register(game, opts);
}

fn install(game: &mut Game, opts: &ContentOpts) {
    let mut rt = scan(game, opts);
    let mut map63 = false;
    for (t, set) in SETS.iter().enumerate() {
        let Some(map) = rt.slots[t] else {
            continue;
        };
        if map == VIRTUAL_ONLY_MAP {
            map63 = true;
            continue;
        }
        let sv = set.sideview_table + 2 * u16::from(map);
        let en = set.enemy_table + 2 * u16::from(map);
        if !super::patch_prg_at(game, set.bank, sv, &WRAM_ROOM_BUF.to_le_bytes())
            || !super::patch_prg_at(game, set.bank, en, &WRAM_ENEMY_BUF.to_le_bytes())
        {
            rt.slots[t] = None;
        }
    }
    if map63 {
        let [lo, hi] = HOOK_MAP63_PTR.to_le_bytes();
        if super::patch_prg_at(game, 7, MAP63_PATCH_AT, &[0x20, lo, hi, 0xEA, 0xEA]) {
            super::hook(
                game,
                "enh_content_map63_ptr",
                Some(7),
                HOOK_MAP63_PTR,
                hook_map63_ptr,
                Some(6),
            );
        } else {
            rt.slots[5] = None;
            rt.slots[6] = None;
        }
    }
    game.enh_hooks.content = rt;
    if rt.slots.iter().all(Option::is_none) {
        return;
    }
    let mut hooks: Vec<(&'static str, u16, crate::traps::TrapFn)> = vec![
        ("enh_content_area_load", HOOK_AREA_LOAD, hook_area_load),
        ("enh_content_side_exit", HOOK_SIDE_EXIT, hook_side_exit),
        (
            "enh_content_elevator_exit",
            HOOK_ELEVATOR_EXIT,
            hook_elevator_exit,
        ),
        ("enh_content_fall", HOOK_FALL, hook_fall),
    ];
    if rt.slots[usize::from(TOWNS)].is_some() {
        hooks.push(("enh_content_door_exit", HOOK_DOOR_EXIT, hook_door_exit));
    }
    for (name, addr, func) in hooks {
        // Interpreted underneath when there is no default port: the ROM
        // bytes charge their own cycles.
        let cycles = if game.traps.get(addr).is_some() {
            None
        } else {
            Some(0)
        };
        super::hook(game, name, Some(7), addr, func, cycles);
    }
}

/// Per-frame work after [`Game::step`] (only while any enhancement is on):
/// record pages taken in the current virtual room, keep its connection
/// bytes in the mirror (a world load rewrites them), and forget a room the
/// game left without a hook seeing it.
pub(crate) fn end_of_frame(game: &mut Game, opts: &ContentOpts) {
    if let Some(id) = current_room(game) {
        match in_slot(game, id) {
            Some(slot) => {
                sync_pages(game, id, slot);
                write_mirror(game, id, slot);
            }
            None => game.enh_state.vroom = 0,
        }
    }
    game.enh_state.vroom_cont = [game.ram[ADDR_CONTAINERS], game.ram[ADDR_CONTAINERS + 1]];
    super::content_overworld::end_of_frame(game, opts);
    super::content_town::end_of_frame(game, opts);
    super::content_minigames::end_of_frame(game, opts);
}

// ------------------------------------------------------------ public runtime API

/// The virtual room Link is in, if any.
#[must_use]
pub fn current_room(game: &Game) -> Option<VRoomId> {
    game.enh_state.vroom.checked_sub(1).map(VRoomId)
}

/// The virtual room the next load of its slot will show, if one is queued.
#[must_use]
pub fn pending_room(game: &Game) -> Option<VRoomId> {
    (game.enh_state.vroom_pending & PENDING_ID)
        .checked_sub(1)
        .map(VRoomId)
}

/// Where a location entry must send Link so the game loads `id`'s slot:
/// the `$0707` world and the map (the low 6 bits of the location's map
/// byte). `None` when the room's option is off or its table has no slot
/// on this ROM.
#[must_use]
pub fn location_target(game: &Game, id: VRoomId) -> Option<LocationTarget> {
    let r = room(id)?;
    if !game.enh.content.enables(r.group) {
        return None;
    }
    let map = slot_map(game, r.table())?;
    Some(LocationTarget {
        world: r.room.world,
        table: r.table(),
        map,
    })
}

/// See [`location_target`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LocationTarget {
    /// `$0707` value of the room.
    pub world: u8,
    /// Table set index ([`content_mem::SETS`]).
    pub table: u8,
    /// Slot map the location entry names.
    pub map: u8,
}

/// Enter virtual room `id` from the overworld: queue it so the next load
/// of its table's slot map shows it (call when Link steps on a location
/// entry that leads to [`location_target`]; the game's own entry code then
/// does the transition). Returns `false` (and queues nothing) when the
/// room's option is off or its table has no slot.
pub fn enter_from_overworld(game: &mut Game, id: VRoomId) -> bool {
    if location_target(game, id).is_none() {
        return false;
    }
    game.enh_state.vroom_pending = id.0 + 1;
    true
}

/// Whether persistent pool bit `bit` is set (taken / flag on).
#[must_use]
pub fn bit(game: &Game, bit: u16) -> bool {
    match pool_addr(game, bit) {
        Some((addr, mask)) => game.ram[addr] & mask == 0,
        None => {
            let i = usize::from(bit);
            i < 32 * game.enh_state.vroom_items.len()
                && (game.enh_state.vroom_items[i / 32] >> (i % 32)) & 1 != 0
        }
    }
}

/// Set or clear persistent pool bit `bit`.
pub fn set_bit(game: &mut Game, bit: u16, on: bool) {
    match pool_addr(game, bit) {
        Some((addr, mask)) => {
            if on {
                game.ram[addr] &= !mask;
            } else {
                game.ram[addr] |= mask;
            }
        }
        None => {
            let i = usize::from(bit);
            if i < 32 * game.enh_state.vroom_items.len() {
                let w = &mut game.enh_state.vroom_items[i / 32];
                if on {
                    *w |= 1 << (i % 32);
                } else {
                    *w &= !(1 << (i % 32));
                }
            }
        }
    }
}

/// Free-form persistent flag `i` of `group` ([`GroupInfo::flags`]).
#[must_use]
pub fn flag(game: &Game, group: Group, i: u16) -> bool {
    flag_bit(group, i).is_some_and(|b| bit(game, b))
}

/// Set free-form persistent flag `i` of `group` (ignored past the group's
/// flag count).
pub fn set_flag(game: &mut Game, group: Group, i: u16, on: bool) {
    if let Some(b) = flag_bit(group, i) {
        set_bit(game, b, on);
    }
}

/// Tracked pages of room `id` already taken (collected, opened).
#[must_use]
pub fn taken_pages(game: &Game, id: VRoomId) -> u8 {
    (0..4u8).fold(0, |m, p| {
        if page_bit(id, p).is_some_and(|b| bit(game, b)) {
            m | (1 << p)
        } else {
            m
        }
    })
}

/// Container pieces of kind `item` ([`HEART_PIECE`] / [`MAGIC_PIECE`])
/// collected (over every registered room, from the persistent bits).
#[must_use]
pub fn pieces_collected(game: &Game, item: u8) -> u32 {
    rooms()
        .map(|r| {
            let taken = taken_pages(game, r.id);
            (0..4u8)
                .filter(|&p| taken & (1 << p) != 0 && page_item(r.room.sideview, p) == Some(item))
                .count() as u32
        })
        .sum()
}

// ------------------------------------------------------------ runtime

/// The slot map when Link is in `id`'s slot of its table, else `None`.
fn in_slot(game: &Game, id: VRoomId) -> Option<u8> {
    let r = room(id)?;
    let t = current_table(game)?;
    let slot = slot_map(game, t)?;
    (t == r.table() && game.ram[ADDR_SCENE] == slot).then_some(slot)
}

/// RAM byte and mask of pool bit `bit`, when its map is usable.
fn pool_addr(game: &Game, bit: u16) -> Option<(usize, u8)> {
    let e = usize::from(bit / 4);
    let p = mem::PROGRESS_POOL.get(e)?;
    if runtime(game).pool_ok & (1 << e) == 0 {
        return None;
    }
    let (addr, high) = mem::item_nibble(p.block, p.map);
    let m = 0x08u8 >> (bit % 4);
    Some((usize::from(addr), if high { m << 4 } else { m }))
}

/// The collectable byte on page `page` of a side-view, if any.
fn page_item(sideview: &[u8], page: u8) -> Option<u8> {
    collectables(sideview)
        .into_iter()
        .find(|&(p, _, _)| p == page)
        .map(|(_, _, item)| item)
}

/// `(page, byte offset of the item byte, item)` of every collectable.
fn collectables(sideview: &[u8]) -> Vec<(u8, usize, u8)> {
    let len = sideview.first().map_or(0, |&b| usize::from(b));
    let data = &sideview[..len.min(sideview.len())];
    let mut out = Vec::new();
    let mut i = 4;
    let mut cursor: u16 = 0;
    while i + 1 < data.len() {
        let a = data[i];
        let b = data[i + 1];
        let row = a >> 4;
        if row == 14 {
            cursor = u16::from(a & 3) * 16;
            i += 2;
            continue;
        }
        cursor += u16::from(a & 0x0F);
        if row != 13 && b == 0x0F {
            if let Some(&item) = data.get(i + 2) {
                out.push((((cursor / 16) & 3) as u8, i + 2, item));
            }
            i += 3;
        } else {
            i += 2;
        }
    }
    out
}

/// Item-presence byte and half of the slot's live nibble.
fn live_nibble(game: &Game, slot: u8) -> Option<(usize, bool)> {
    let t = current_table(game)?;
    let (addr, high) = mem::item_nibble(SETS[usize::from(t)].item_block, slot);
    Some((usize::from(addr), high))
}

/// Pages present per the slot's live nibble (page `p` = bit `$08 >> p`).
fn present_pages(game: &Game, slot: u8) -> Option<u8> {
    let (addr, high) = live_nibble(game, slot)?;
    let b = game.ram[addr];
    let nib = if high { b >> 4 } else { b & 0x0F };
    Some((0..4).fold(0u8, |m, p| {
        if nib & (0x08 >> p) != 0 {
            m | (1 << p)
        } else {
            m
        }
    }))
}

fn write_present_pages(game: &mut Game, slot: u8, pages: u8) {
    let Some((addr, high)) = live_nibble(game, slot) else {
        return;
    };
    let nib = (0..4).fold(0u8, |m, p| {
        if pages & (1 << p) != 0 {
            m | (0x08 >> p)
        } else {
            m
        }
    });
    let b = &mut game.ram[addr];
    *b = if high {
        (*b & 0x0F) | (nib << 4)
    } else {
        (*b & 0xF0) | nib
    };
}

/// Record the tracked pages the game cleared since entry, and settle
/// container pieces.
fn sync_pages(game: &mut Game, id: VRoomId, slot: u8) {
    let Some(r) = room(id) else {
        return;
    };
    let Some(present) = present_pages(game, slot) else {
        return;
    };
    let tracked = tracked_pages(r.room.world, r.room.sideview);
    let new = tracked & !present & !taken_pages(game, id);
    for p in (0..4u8).filter(|p| new & (1 << p) != 0) {
        if let Some(b) = page_bit(id, p) {
            set_bit(game, b, true);
        }
        let item = page_item(r.room.sideview, p);
        let slot_i = match item {
            Some(HEART_PIECE) => 1,
            Some(MAGIC_PIECE) => 0,
            _ => continue,
        };
        // The game granted a container for the stand-in item; keep it only
        // when this piece completes a set.
        let addr = ADDR_CONTAINERS + slot_i;
        let granted = game.ram[addr] > game.enh_state.vroom_cont[slot_i];
        let complete =
            pieces_collected(game, item.unwrap_or(0)).is_multiple_of(PIECES_PER_CONTAINER);
        if granted && !complete {
            game.ram[addr] -= 1;
        } else if !granted && complete && game.ram[addr] < 8 {
            game.ram[addr] += 1;
        }
    }
}

/// The four mirror bytes of `id` in slot `slot`.
fn mirror_bytes(id: VRoomId, slot: u8) -> [u8; 4] {
    let proxy = if slot == VIRTUAL_ONLY_MAP {
        PROXY_MAP
    } else {
        slot
    };
    room(id).map_or([0xFC; 4], |r| {
        r.room.exits.map(|e| match e {
            Dest::Byte(b) => b,
            Dest::Room { page, .. } => proxy * 4 + (page & 3),
        })
    })
}

fn write_mirror(game: &mut Game, id: VRoomId, slot: u8) {
    let cb = usize::from(CONN_MIRROR - 0x6000) + usize::from(slot) * 4;
    let want = mirror_bytes(id, slot);
    if game.wram[cb..cb + 4] != want {
        game.wram[cb..cb + 4].copy_from_slice(&want);
    }
}

/// Load virtual room `id` into slot `slot`: buffers (container pieces
/// shown as containers), connection mirror, live item nibble, `vroom`.
fn enter(game: &mut Game, id: VRoomId, slot: u8) {
    let Some(r) = room(id) else {
        game.enh_state.vroom = 0;
        return;
    };
    let rb = usize::from(WRAM_ROOM_BUF - 0x6000);
    let sv = r.room.sideview;
    game.wram[rb..rb + sv.len()].copy_from_slice(sv);
    for (_, off, item) in collectables(sv) {
        let shown = match item {
            HEART_PIECE => HEART_CONTAINER,
            MAGIC_PIECE => MAGIC_CONTAINER,
            _ => continue,
        };
        game.wram[rb + off] = shown;
    }
    let eb = usize::from(WRAM_ENEMY_BUF - 0x6000);
    game.wram[eb..eb + r.room.enemies.len()].copy_from_slice(r.room.enemies);
    write_mirror(game, id, slot);
    let present = tracked_pages(r.room.world, sv) & !taken_pages(game, id);
    write_present_pages(game, slot, present);
    game.enh_state.vroom = id.0 + 1;
}

/// `$C4CB` hook: enter the queued room (or refresh the current one) when
/// the map about to load is the slot.
fn hook_area_load(game: &mut Game) {
    let mut entered = false;
    if let Some(t) = current_table(game) {
        // A fall out of a map-63 room reads the mirror's proxy byte (no
        // connection byte can name 63): back to the slot.
        if game.ram[ADDR_SCENE] == PROXY_MAP
            && slot_map(game, t) == Some(VIRTUAL_ONLY_MAP)
            && pending_room(game).is_some_and(|id| room(id).is_some_and(|r| r.table() == t))
        {
            game.ram[ADDR_SCENE] = VIRTUAL_ONLY_MAP;
        }
        if let Some(slot) = slot_map(game, t).filter(|&s| s == game.ram[ADDR_SCENE]) {
            let in_table = |id: &VRoomId| room(*id).is_some_and(|r| r.table() == t);
            let pending = pending_room(game).filter(in_table);
            if pending.is_some() {
                game.enh_state.vroom_pending = 0;
            }
            match pending.or_else(|| current_room(game).filter(in_table)) {
                Some(id) => {
                    enter(game, id, slot);
                    entered = true;
                }
                None => write_empty_room(game, slot),
            }
        }
    }
    if !entered {
        game.enh_state.vroom = 0;
    }
    super::call_original(game, HOOK_AREA_LOAD);
}

/// What a slot shows when it is loaded with no virtual room queued (only
/// through a path the engine does not drive): one page of plain floor,
/// no enemies, every exit outside. Keeps the loader on sane bytes.
pub const EMPTY_ROOM_SIDEVIEW: [u8; 4] = [0x04, 0x00, 0x01, 0x08];

fn write_empty_room(game: &mut Game, slot: u8) {
    let rb = usize::from(WRAM_ROOM_BUF - 0x6000);
    game.wram[rb..rb + 4].copy_from_slice(&EMPTY_ROOM_SIDEVIEW);
    game.wram[usize::from(WRAM_ENEMY_BUF - 0x6000)] = 0x01;
    let cb = usize::from(CONN_MIRROR - 0x6000) + usize::from(slot) * 4;
    game.wram[cb..cb + 4].fill(0xFC);
}

/// `$C527` (from the `$C524` patch): `LDA $0561 : ASL : TAY`, and for map
/// 63 in a palace group with a live slot point `($00)` / `($02)` so that
/// `Y = 126` reads the buffer addresses from [`WRAM_MAP63_PTRS`].
fn hook_map63_ptr(game: &mut Game) {
    use crate::cpu::{FLAG_C, FLAG_N, FLAG_Z};
    let m = game.ram[ADDR_SCENE];
    let a = m << 1;
    game.cpu.a = a;
    game.cpu.y = a;
    let mut p = game.cpu.p & !(FLAG_N | FLAG_Z | FLAG_C);
    if a == 0 {
        p |= FLAG_Z;
    }
    p |= a & FLAG_N;
    p |= m >> 7;
    game.cpu.p = p;
    if m != VIRTUAL_ONLY_MAP {
        return;
    }
    let live = current_table(game).is_some_and(|t| slot_map(game, t) == Some(VIRTUAL_ONLY_MAP));
    if !live {
        return;
    }
    let w = usize::from(WRAM_MAP63_PTRS - 0x6000);
    game.wram[w..w + 2].copy_from_slice(&WRAM_ROOM_BUF.to_le_bytes());
    game.wram[w + 2..w + 4].copy_from_slice(&WRAM_ENEMY_BUF.to_le_bytes());
    let base = WRAM_MAP63_PTRS.wrapping_sub(u16::from(a));
    let [lo, hi] = base.to_le_bytes();
    let [lo2, hi2] = base.wrapping_add(2).to_le_bytes();
    game.ram[0x00] = lo;
    game.ram[0x01] = hi;
    game.ram[0x02] = lo2;
    game.ram[0x03] = hi2;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Via {
    Side,
    Elevator,
    Door,
}

/// Where a redirected exit goes.
#[derive(Debug, Clone, Copy)]
enum Go {
    Byte(u8),
    Room(VRoomId, u8),
}

/// The linked exit of original map `map` (connection byte or door page
/// `k`) in table `t`, among the enabled areas.
fn link_for(game: &Game, t: u8, map: u8, door: bool, k: u8) -> Option<Go> {
    let opts = game.enh.content;
    areas(&opts)
        .filter(|a| a.table == t)
        .find_map(|a: &StaticArea| {
            let l = a
                .links
                .iter()
                .find(|l| l.map == map && l.door == door && l.index == k)?;
            Some(Go::Room(room_id(a, l.room)?, l.page))
        })
}

/// Shared exit-hook body.
fn exit_common(game: &mut Game, via: Via, addr: u16) {
    let world = game.ram[ADDR_WORLD];
    let map = game.ram[ADDR_SCENE];
    let page = game.ram[ADDR_PAGE] & 3;
    let t = current_table(game);
    let slot = t.and_then(|t| slot_map(game, t));
    let (idx, k) = match via {
        Via::Side => {
            let src = if world == 0 && map >= mem::WORLD0_OWN_CONN_LIMIT {
                0
            } else {
                map
            };
            (src.wrapping_mul(4).wrapping_add(page), page)
        }
        Via::Elevator => {
            let up = u8::from(game.ram[ADDR_ELEV_DIR] == ELEV_UP);
            (map.wrapping_mul(4).wrapping_add(1 + up), 1 + up)
        }
        Via::Door => (map.wrapping_mul(4).wrapping_add(page), page),
    };
    let from = current_room(game).filter(|&id| in_slot(game, id).is_some());
    if let (Some(id), Some(s)) = (from, slot) {
        sync_pages(game, id, s);
    }
    let fall = game.enh_state.vroom_pending;
    let go = match (from, t) {
        // The side exit that ends a fall (mode `$10` after `$0C`-`$0F`):
        // where the fall hook decided (the shaft rule included).
        (Some(_), _) if via == Via::Side && fall & PENDING_FALL != 0 => {
            pending_room(game).map(|id| Go::Room(id, ((fall >> PENDING_PAGE_SHIFT) & 3) as u8))
        }
        (Some(id), _) => room(id).and_then(|r| {
            let d = match via {
                Via::Door => r.room.door_exits[usize::from(k)],
                _ => Some(r.room.exits[usize::from(k)]),
            }?;
            Some(match d {
                Dest::Byte(b) => Go::Byte(b),
                Dest::Room { room, page } => Go::Room(r.sibling(room)?, page),
            })
        }),
        (None, Some(t)) if slot.is_some() && (via != Via::Door || t == TOWNS) => {
            link_for(game, t, map, via == Via::Door, k)
        }
        _ => None,
    };
    let Some(go) = go else {
        super::call_original(game, addr);
        if current_room(game).is_some_and(|id| in_slot(game, id).is_none()) {
            game.enh_state.vroom = 0;
        }
        return;
    };
    let (byte, target) = match go {
        Go::Byte(b) => (b, None),
        Go::Room(id, p) => {
            let s = slot.unwrap_or(PROXY_MAP);
            let m = if via == Via::Side || s == VIRTUAL_ONLY_MAP {
                PROXY_MAP
            } else {
                s
            };
            (m * 4 + (p & 3), Some(id))
        }
    };
    // Put `byte` where the routine reads, for this one call.
    let at = match via {
        Via::Door => super::bank_offset(
            game.prg.len(),
            DOOR_TABLE_BANK,
            DOOR_TABLE.wrapping_add(u16::from(idx)),
        ),
        _ => usize::from(CONN_MIRROR - 0x6000) + usize::from(idx),
    };
    let old = match via {
        Via::Door => game.prg.get(at).copied(),
        _ => game.wram.get(at).copied(),
    };
    let Some(old) = old else {
        super::call_original(game, addr);
        return;
    };
    match via {
        Via::Door => game.prg[at] = byte,
        _ => game.wram[at] = byte,
    }
    super::call_original(game, addr);
    match via {
        Via::Door => game.prg[at] = old,
        _ => game.wram[at] = old,
    }
    game.enh_state.vroom = 0;
    if let (Some(id), Some(s)) = (target, slot) {
        if game.ram[ADDR_WORLD] == world && game.ram[ADDR_SCENE] == byte >> 2 {
            game.ram[ADDR_SCENE] = s;
            game.enh_state.vroom_pending = id.0 + 1;
        }
    }
}

/// `$C68A` hook (mode `$0C`, a fall out of page `$3B`): queue the room
/// connection byte `page` stands for, with its landing page. The fall
/// then ends either in a side exit (mode `$10`, where [`exit_common`] takes
/// the queued room) or straight in the slot load (the mirror byte names
/// the slot or the proxy map; [`hook_area_load`] enters the room). Link
/// keeps the page he fell on. Falls out of original rooms are not
/// redirected (no `StaticLink` names a fall).
fn hook_fall(game: &mut Game) {
    // Mode `$0C` is also the side-view entry's redraw (`LCC50` picks it
    // for some location entries); only a fall leaves Link below the
    // screen (`$19 >= 2`, the fall check `$E19E`; a fairy at `$29 >= $E4`).
    let fell = game.ram[0x19] >= 2 || game.ram[0x29] >= 0xE4;
    let here = current_room(game).and_then(|id| Some((id, in_slot(game, id)?)));
    if let Some((id, slot)) = here.filter(|_| fell) {
        sync_pages(game, id, slot);
        let mut page = usize::from(game.ram[ADDR_PAGE] & 3);
        if let Some(r) = room(id) {
            // Into the elevator's shaft (open through the floor for a ride
            // down, `z2_content::compile::elevator_shaft`): the ride down,
            // byte 1. Link's feet are 16 px right of `$4D`.
            let feet =
                (usize::from(game.ram[ADDR_PAGE] & 3) * 256 + usize::from(game.ram[0x4D]) + 16)
                    / 16;
            let shaft = z2_content::sideview::Sideview::parse(r.room.sideview)
                .ok()
                .and_then(|sv| {
                    sv.commands
                        .iter()
                        .find(|c| c.is_extended() && c.kind() == 5)
                        .map(|c| usize::from(c.x))
                });
            if shaft.is_some_and(|e| feet == e || feet == e + 1) {
                page = 1;
            }
            if let Dest::Room {
                room: to,
                page: land,
            } = r.room.exits[page]
            {
                if let Some(to) = r.sibling(to) {
                    // With its page for the side exit that may end the fall
                    // (a fall keeps Link's page either way, `LC67C`; the
                    // compiled drop destination is that page).
                    game.enh_state.vroom_pending =
                        (to.0 + 1) | PENDING_FALL | (u16::from(land & 3) << PENDING_PAGE_SHIFT);
                }
            }
        }
    }
    super::call_original(game, HOOK_FALL);
}

fn hook_side_exit(game: &mut Game) {
    exit_common(game, Via::Side, HOOK_SIDE_EXIT);
}

fn hook_elevator_exit(game: &mut Game) {
    exit_common(game, Via::Elevator, HOOK_ELEVATOR_EXIT);
}

fn hook_door_exit(game: &mut Game) {
    if current_table(game) == Some(TOWNS) {
        exit_common(game, Via::Door, HOOK_DOOR_EXIT);
    } else {
        super::call_original(game, HOOK_DOOR_EXIT);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn all_on() -> ContentOpts {
        ContentOpts {
            virtual_rooms: true,
            test_room: true,
            parapa_annex: true,
            challenge_rooms: true,
            minigames: true,
            hidden_caves: true,
            dark_isle: true,
        }
    }

    #[test]
    fn authored_data_is_valid() {
        validate().unwrap();
        assert_eq!(room(TEST_ROOM).unwrap().area.name, "Test Room");
    }

    #[test]
    fn identity_tracks_options_and_data() {
        let on = ContentOpts {
            virtual_rooms: true,
            ..ContentOpts::default()
        };
        assert!(on.is_active());
        assert!(!ContentOpts {
            test_room: true,
            ..ContentOpts::default()
        }
        .is_active());
        let mut seen = std::collections::HashSet::new();
        for i in 0..7 {
            let mut o = on;
            match i {
                0 => {}
                1 => o.test_room = true,
                2 => o.parapa_annex = true,
                3 => o.challenge_rooms = true,
                4 => o.minigames = true,
                5 => o.hidden_caves = true,
                _ => o.dark_isle = true,
            }
            let mut v = Vec::new();
            o.write_identity(&mut v);
            assert_eq!(&v[7..], &content_hash().to_le_bytes());
            assert!(seen.insert(v));
        }
    }

    #[test]
    fn pool_bits_and_fallback() {
        let mut g = Game::new();
        // No pool map usable: bits live in `vroom_items`.
        set_bit(&mut g, 5, true);
        assert!(bit(&g, 5));
        assert_eq!(g.enh_state.vroom_items[0], 1 << 5);
        set_bit(&mut g, 5, false);
        assert!(!bit(&g, 5));
        // Usable: bit 5 is map 26 (even: high nibble of $060D), bit $04 there.
        g.enh_hooks.content.pool_ok = u64::MAX;
        g.ram[0x0600..0x06E0].fill(0xFF);
        g.ram[0x060D] = 0xFF;
        assert!(!bit(&g, 5));
        set_bit(&mut g, 5, true);
        assert_eq!(g.ram[0x060D], 0xBF);
        assert!(bit(&g, 5));
        // Bit 0 (map 25, odd: low nibble of $060C, bit $08).
        g.ram[0x060C] = 0xFF;
        set_bit(&mut g, 0, true);
        assert_eq!(g.ram[0x060C], 0xF7);
        assert_eq!(taken_pages(&g, TEST_ROOM), 0b0010);
        set_flag(&mut g, Group::Minigames, 2, true);
        assert!(flag(&g, Group::Minigames, 2));
        assert!(!flag(&g, Group::Minigames, 1));
    }

    #[test]
    fn collectables_and_pieces() {
        let sv = [
            0x0B, 0x60, 0x01, 0x08, 0xE3, 0x00, 0x92, 0x0F, 0x13, 0x50, 0x13,
        ];
        assert_eq!(collectables(&sv), vec![(3, 8, HEART_PIECE)]);
        assert_eq!(page_item(&sv, 3), Some(HEART_PIECE));
        assert_eq!(page_item(&sv, 0), None);
        assert_eq!(
            page_item(room(TEST_ROOM).unwrap().room.sideview, 1),
            Some(0x0A)
        );
    }

    #[test]
    fn map63_trap_matches_the_instructions_it_replaces() {
        let mut g = Game::new();
        g.ram[ADDR_SCENE] = 0x45;
        hook_map63_ptr(&mut g);
        assert_eq!((g.cpu.a, g.cpu.y), (0x8A, 0x8A));
        assert_eq!(g.cpu.p & 0x83, 0x80);
        // Map 63 in a live palace group: ($00),126 reads the buffer word.
        g.enh_hooks.content.slots[5] = Some(63);
        g.ram[ADDR_WORLD] = 3;
        g.ram[ADDR_SCENE] = 63;
        hook_map63_ptr(&mut g);
        let ptr = u16::from_le_bytes([g.ram[0], g.ram[1]]).wrapping_add(126);
        assert_eq!(ptr, WRAM_MAP63_PTRS);
        let w = usize::from(ptr - 0x6000);
        assert_eq!(&g.wram[w..w + 2], &WRAM_ROOM_BUF.to_le_bytes());
        let eptr = u16::from_le_bytes([g.ram[2], g.ram[3]]).wrapping_add(126);
        let e = usize::from(eptr - 0x6000);
        assert_eq!(&g.wram[e..e + 2], &WRAM_ENEMY_BUF.to_le_bytes());
    }

    #[test]
    fn register_redirects_slots_and_undoes_it() {
        // No ROM: a blank PRG reaches only map 0 of every table, so every
        // slot counts as unused (but the template is not "all set").
        let mut g = Game::new();
        let traps = g.traps.len();
        let prg = g.prg.clone();
        g.set_enhancements(super::super::Enhancements {
            content: all_on(),
            ..super::super::Enhancements::default()
        });
        if g.prg.len() >= 8 * 0x4000 {
            let gp = &SETS[7];
            assert_eq!(slot_map(&g, 7), Some(60));
            assert_eq!(
                prg_word(&g, gp.bank, gp.sideview_table + 120),
                Some(WRAM_ROOM_BUF)
            );
            assert!(g.traps.get(HOOK_SIDE_EXIT).is_some());
            assert!(g.traps.get(HOOK_AREA_LOAD).is_some());
            assert_eq!(runtime(&g).pool_ok, 0);
        }
        g.set_enhancements(super::super::Enhancements::default());
        assert_eq!(g.prg, prg);
        assert_eq!(g.traps.len(), traps);
        assert_eq!(*runtime(&g), Runtime::default());
    }
}
