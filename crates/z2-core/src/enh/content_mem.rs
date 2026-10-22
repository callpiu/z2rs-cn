//! Verified memory facts for the `content` group (new areas through
//! "virtual rooms" and new overworld entrances).
//!
//! Every value here was measured, not assumed: `cargo xtask content-survey`
//! replays the whole movie corpus plus a scene sweep (every town / palace
//! map and every world-0 key area loaded through the game's own loader)
//! on a trapped and an untrapped game in lockstep, with a bus probe on
//! `$6000-$7FFF`, and walks the vanilla map tables. Method, coverage and
//! the full tables are in README.md. Addresses, counts
//! and indices only: nothing here is ROM data.
//!
//! Self-contained (no intra-crate imports). The `enh` module tree must
//! declare it (`pub mod content_mem;`) before anything can use it.
//!
//! # How the pieces fit
//!
//! * A room's sideview pointer is a word in the bank's table
//!   ([`SideviewSet::sideview_table`] `+ 2 * map`); the loader reads the
//!   room through `($D4),Y`, and the bus maps `$6000-$7FFF` to WRAM, so a
//!   pointer to [`VROOM_DATA_BASE`] makes the game read authored bytes
//!   from WRAM. The enemy pointer ([`SideviewSet::enemy_table`]) already
//!   points into WRAM in vanilla (the `$7000` copy), so a WRAM enemy list
//!   is the normal case.
//! * Connections are read from the WRAM copy at [`CONN_WRAM`]
//!   `+ 4 * map + page`, which the world load rewrites from the ROM table
//!   every time a world loads.
//! * Item / collectable state is the per-map nibble in the save-mirrored
//!   CPU RAM `$0600-$06DF` ([`item_nibble`]), saved with the slot.

/// A WRAM range: first byte and length.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WramRange {
    /// First byte (`$6000-$7FFF`).
    pub start: u16,
    /// Length in bytes.
    pub len: u16,
}

impl WramRange {
    /// Last byte (inclusive).
    pub const fn last(self) -> u16 {
        self.start + self.len - 1
    }

    /// Whether `addr` falls inside.
    pub const fn contains(self, addr: u16) -> bool {
        addr >= self.start && addr <= self.last()
    }
}

const fn r(start: u16, last: u16) -> WramRange {
    WramRange {
        start,
        len: last - start + 1,
    }
}

// ---------------------------------------------------------------------------
// WRAM: never read or written by the game (vanilla, all modes measured).
// ---------------------------------------------------------------------------

/// Scratch WRAM the game never reads or writes, largest first. Union over
/// 535,932 movie frames and 156,000 sweep frames, trapped and untrapped
/// (the two runs touch exactly the same bytes).
///
/// `$7401`, `$7499` (the pad bytes after the slot 0/1 headers) and `$7918`
/// are also untouched but too small to matter; `$773B-$77FF` is battery
/// RAM and listed separately ([`BATTERY_SPARE`]).
pub const WRAM_FREE: [WramRange; 6] = [
    // After the side-view level buffer ($6000-$633F, 4 screens x $D0).
    r(0x6340, 0x6957),
    // After the overworld map window when the randomizer's map-size patch
    // is off. The patch moves the window start to $7A00 (z2-rando
    // `BIG_WINDOW`), leaving only $79F9-$79FF: do not use with rando.
    r(0x79F9, 0x7BFF),
    // After the world-data copy ($6A00-$6C57).
    r(0x6C58, 0x6CFF),
    // Above the enemy tables the game uses up to $6FA0.
    r(0x6FA1, 0x6FFF),
    // After the overworld map window's end ($7F7F) and the bytes the game
    // uses up to $7FA5.
    r(0x7FA6, 0x7FFF),
    // Just below the world-data copy ($6A00).
    r(0x69E0, 0x69FF),
];

/// Battery-RAM bytes outside the save-slot protocol that the game never
/// touches (after slot 2's header `$773A` and its pad byte `$773B`, up to
/// the in-use `$7800`). Persist in `sram.sav` like the rest of `$6000-$7FFF`
/// but are not copied, staged, restored or cleared by the slot code.
pub const BATTERY_SPARE: WramRange = r(0x773C, 0x77FF);

/// Recommended home for authored room bytes: the start of the largest
/// free range (four 256-byte rooms; a room header's length byte caps one
/// room at 255 bytes).
pub const VROOM_DATA_BASE: u16 = 0x6340;
/// Bytes per room buffer.
pub const VROOM_DATA_STRIDE: u16 = 0x100;
/// Room buffers at [`VROOM_DATA_BASE`].
pub const VROOM_DATA_COUNT: u16 = 4;
/// Enemy lists for the virtual rooms (64 bytes each, one per room buffer).
pub const VROOM_ENEMY_BASE: u16 = 0x6740;
/// Bytes per enemy list.
pub const VROOM_ENEMY_STRIDE: u16 = 0x40;
/// Engine scratch: the rest of the largest free range.
pub const VROOM_SCRATCH: WramRange = r(0x6840, 0x6957);

// ---------------------------------------------------------------------------
// WRAM: what is in use (for orientation; measured).
// ---------------------------------------------------------------------------

/// Side-view level buffer (4 screens x `$D0`). Also the staging area of the
/// save protocol's backup copies (`$6002-$6337`, see `save_format`), so it
/// is scratch, not persistent.
pub const LEVEL_BUFFER: WramRange = r(0x6000, 0x633F);
/// World-data copy made on every world load: location table (`$6A00`,
/// four 63-byte strides), connections (`$6AFC`, 4 bytes per map) and 96
/// more bytes up to `$6C57`.
pub const WORLD_COPY: WramRange = r(0x6A00, 0x6C57);
/// WRAM copy of the connection table (`$6AFC + 4 * map + page`).
pub const CONN_WRAM: u16 = 0x6AFC;
/// WRAM copies of the location table strides (Y, X, map, world).
pub const LOC_WRAM: [u16; 4] = [0x6A00, 0x6A3F, 0x6A7E, 0x6ABD];
/// Enemy data copy (four 256-byte chunks per world load).
pub const ENEMY_COPY: WramRange = r(0x7000, 0x73FF);
/// Main save copies (slot headers `$7400`/`$7498`/`$773A` and the part-1 /
/// part-2 images); see `save_format`.
pub const SAVE_MAIN: WramRange = r(0x7400, 0x773B);
/// Overworld map window (vanilla loader; `$7A00` start after the rando
/// map-size patch).
pub const OVERWORLD_MAP: WramRange = r(0x7C00, 0x7F7F);

// ---------------------------------------------------------------------------
// Side-view map tables and reserved slots.
// ---------------------------------------------------------------------------

/// Maps per table set (indices `0..=62`; index 63 has no table entry).
pub const MAPS: u8 = 63;
/// The 6-bit map field's one index no vanilla table has (`$3F`). Its
/// pointer fetch would read past the sideview table into the enemy table
/// (map 0's enemy word), so it needs a code redirect, not a table patch.
/// A connection byte cannot name it (`$FC-$FF` is the wall sentinel), so
/// it is entered from the overworld or by the engine. Its item nibble
/// exists in every block (low nibble of byte 31) and starts all set.
pub const VIRTUAL_ONLY_MAP: u8 = 63;
/// World 0: side exits from areas `>= $1D` use area 0's connection bytes
/// (`bank7_take_side_exit`), so a room that links to other rooms through
/// its own bytes wants an index below this.
pub const WORLD0_OWN_CONN_LIMIT: u8 = 0x1D;

/// One 63-map side-view table set.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SideviewSet {
    /// Name for logs.
    pub name: &'static str,
    /// `$0707` values that use it.
    pub worlds: &'static [u8],
    /// PRG bank holding the tables.
    pub bank: u8,
    /// Sideview pointer table (63 words).
    pub sideview_table: u16,
    /// Enemy pointer table (63 words).
    pub enemy_table: u16,
    /// Connection table (63 x 4 bytes; copied to [`CONN_WRAM`]).
    pub conn_table: u16,
    /// Item-presence block in CPU RAM (32 bytes, two maps per byte).
    pub item_block: u16,
    /// Maps no vanilla data reaches (location tables, random battles,
    /// connection / door closure, restarts, endings) and that no measured
    /// run loaded. Safe to take over.
    pub free_maps: &'static [u8],
    /// The slot the content engine reserves ([`VIRTUAL_ONLY_MAP`] when the
    /// table has no free entry).
    pub reserved_map: u8,
}

/// West Hyrule side-views (world 0, region 0).
pub const WEST: SideviewSet = SideviewSet {
    name: "West Hyrule",
    worlds: &[0],
    bank: 1,
    sideview_table: 0x8523,
    enemy_table: 0x85A1,
    conn_table: 0x871B,
    item_block: 0x0600,
    free_maps: &[25, 26, 27, 28, 32, 37, 49, 55, 61, 62],
    reserved_map: 28,
};

/// Death Mountain side-views (world 0, region 1, bank 1).
pub const DEATH_MOUNTAIN: SideviewSet = SideviewSet {
    name: "Death Mountain",
    worlds: &[0],
    bank: 1,
    sideview_table: 0xA000,
    enemy_table: 0xA07E,
    conn_table: 0xA1F8,
    item_block: 0x0620,
    free_maps: &[27, 31, 38, 45, 46, 61, 62],
    reserved_map: 27,
};

/// East Hyrule side-views (world 0, region 2).
pub const EAST: SideviewSet = SideviewSet {
    name: "East Hyrule",
    worlds: &[0],
    bank: 2,
    sideview_table: 0x8523,
    enemy_table: 0x85A1,
    conn_table: 0x871B,
    item_block: 0x0640,
    free_maps: &[15, 17, 18, 21, 22, 27, 28, 32, 43, 49, 54, 55],
    reserved_map: 28,
};

/// Maze Island side-views (world 0, region 1, bank 2). Shares its item
/// block with Death Mountain, so its reserved map must be free in both
/// tables and differ from Death Mountain's (46: free in both; `>= $1D`,
/// so its side exits go through area 0's bytes).
pub const MAZE_ISLAND: SideviewSet = SideviewSet {
    name: "Maze Island",
    worlds: &[0],
    bank: 2,
    sideview_table: 0xA000,
    enemy_table: 0xA07E,
    conn_table: 0xA1F8,
    item_block: 0x0620,
    free_maps: &[27, 31, 38, 42, 43, 45, 46, 61, 62],
    reserved_map: 46,
};

/// Towns (worlds 1 and 2 share one table set in bank 3).
pub const TOWNS: SideviewSet = SideviewSet {
    name: "Towns",
    worlds: &[1, 2],
    bank: 3,
    sideview_table: 0x8523,
    enemy_table: 0x85A1,
    conn_table: 0x871B,
    item_block: 0x0660,
    free_maps: &[25, 27, 54, 59, 61, 62],
    reserved_map: 62,
};

/// Palaces 1, 2 and 5 (world 3): all 63 maps are vanilla rooms.
pub const PALACE_A: SideviewSet = SideviewSet {
    name: "Palace group A",
    worlds: &[3],
    bank: 4,
    sideview_table: 0x8523,
    enemy_table: 0x85A1,
    conn_table: 0x871B,
    item_block: 0x0680,
    free_maps: &[],
    reserved_map: VIRTUAL_ONLY_MAP,
};

/// Palaces 3, 4 and 6 (world 4): all 63 maps are vanilla rooms.
pub const PALACE_B: SideviewSet = SideviewSet {
    name: "Palace group B",
    worlds: &[4],
    bank: 4,
    sideview_table: 0xA000,
    enemy_table: 0xA07E,
    conn_table: 0xA1F8,
    item_block: 0x06A0,
    free_maps: &[],
    reserved_map: VIRTUAL_ONLY_MAP,
};

/// The Great Palace (world 5). Maps 61 and 62 are the ending scenes.
pub const GREAT_PALACE: SideviewSet = SideviewSet {
    name: "Great Palace",
    worlds: &[5],
    bank: 5,
    sideview_table: 0x8523,
    enemy_table: 0x85A1,
    conn_table: 0x871B,
    item_block: 0x06C0,
    free_maps: &[55, 56, 57, 58, 59, 60],
    reserved_map: 60,
};

/// All eight sets.
pub const SETS: [SideviewSet; 8] = [
    WEST,
    DEATH_MOUNTAIN,
    EAST,
    MAZE_ISLAND,
    TOWNS,
    PALACE_A,
    PALACE_B,
    GREAT_PALACE,
];

/// Item-presence nibble of `map` in `block`: (CPU RAM byte, high nibble?).
/// Even maps use the high nibble. The new-game template (bank 5 `$BB15`)
/// starts every free map's nibble, and map 63's, with all four bits set;
/// the game only clears bits when an item in that room is taken, so a map
/// that never loads keeps them set.
pub const fn item_nibble(block: u16, map: u8) -> (u16, bool) {
    (block + (map as u16) / 2, map.is_multiple_of(2))
}

// ---------------------------------------------------------------------------
// Overworld location slots.
// ---------------------------------------------------------------------------

/// The entry scan (`Check_if_Link_stepped_on_a_Key_Area`, `$857D`) checks
/// slots `$3D..=0` only.
pub const ENTRY_SCAN_LAST: u8 = 0x3D;
/// Slot `$3E` is rewritten for every random battle (`$6A7E/$6A00/$6A3F +
/// $3E`), so it cannot hold an entrance.
pub const RANDOM_BATTLE_SLOT: u8 = 0x3E;
/// From `$2C` up the entry code derives a town code (`($0748 - $2C) / 2`)
/// and from `$34` a palace code (`$0748 - $34`); harmless for cave entries
/// (world 0) but they matter for any entry into worlds 1-5.
pub const TOWN_CODE_SLOT_BASE: u8 = 0x2C;

/// West Hyrule slots with all four bytes zero and no vanilla use.
pub const WEST_FREE_SLOTS: &[u8] = &[33, 34, 35, 36, 37, 38, 39, 55, 56, 57, 58, 59, 60, 61];
/// East Hyrule slots with all four bytes zero and no vanilla use.
pub const EAST_FREE_SLOTS: &[u8] = &[33, 34, 35, 36, 37, 38, 39, 55, 56, 57, 58, 59, 60, 61];
/// Death Mountain slots Death Mountain never uses. Its table copy (bank 1)
/// shares the layout with Maze Island's, so these hold Maze Island
/// coordinates or leftovers and are live in the entry scan: overwrite all
/// four bytes when taking one. (No slot `<= $3D` is all zero.)
pub const DEATH_MOUNTAIN_FREE_SLOTS: &[u8] = &[
    37, 38, 39, 44, 45, 46, 47, 48, 49, 50, 51, 52, 53, 54, 55, 57, 58, 59, 60, 61,
];
/// Maze Island slots Maze Island never uses (its bank-2 copy holds Death
/// Mountain coordinates there; same caveat). (No slot `<= $3D` is all zero.)
pub const MAZE_ISLAND_FREE_SLOTS: &[u8] = &[
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25,
    26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 44, 45, 46, 47, 48, 49, 50, 51, 53, 54, 56,
];

// ---------------------------------------------------------------------------
// Progress storage.
// ---------------------------------------------------------------------------

/// The item nibble of each set's [`SideviewSet::reserved_map`] (4 bits per
/// set). These bytes are part of the save image (`$0600-$06DF`, saved and
/// restored by the slot code, three slots), start all set on a new game,
/// and are cleared by the game's own item pickup in that room.
///
/// The content engine uses these as the **live** presence nibble of the
/// virtual room in the slot (rewritten on every entry from the persistent
/// bits), not as storage: many virtual rooms share one slot. Storage is
/// [`PROGRESS_POOL`].
///
/// Spare bits elsewhere in the save image (`$0777-$07A8`) could not be
/// proven unused: the corpus never finishes a palace, and the quest-flag
/// bytes `$0795-$079E` are bit fields the game reads.
pub const PROGRESS_SLOTS: [(u16, bool); 8] = [
    item_nibble(WEST.item_block, WEST.reserved_map),
    item_nibble(DEATH_MOUNTAIN.item_block, DEATH_MOUNTAIN.reserved_map),
    item_nibble(EAST.item_block, EAST.reserved_map),
    item_nibble(MAZE_ISLAND.item_block, MAZE_ISLAND.reserved_map),
    item_nibble(TOWNS.item_block, TOWNS.reserved_map),
    item_nibble(PALACE_A.item_block, PALACE_A.reserved_map),
    item_nibble(PALACE_B.item_block, PALACE_B.reserved_map),
    item_nibble(GREAT_PALACE.item_block, GREAT_PALACE.reserved_map),
];

/// One map whose item nibble stores persistent content bits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PoolMap {
    /// Item-presence block (`$0600 + 32 * n`).
    pub block: u16,
    /// Map number in that block.
    pub map: u8,
    /// Indices into [`SETS`] of every table using the block; the map must
    /// be free (never loaded) in all of them.
    pub sets: &'static [u8],
}

const fn pm(block: u16, map: u8, sets: &'static [u8]) -> PoolMap {
    PoolMap { block, map, sets }
}

/// Persistent storage for content progress: the item nibbles of the free
/// maps that are not reserved slots, 4 bits each, 140 bits in all. Same
/// properties as the reserved nibbles (save image part 2, all set by the
/// new-game template and by the second-quest reset, never loaded so never
/// cleared by vanilla code), so a set bit means "not collected" and saves
/// made without the content group read as nothing collected.
///
/// Bit `i` is bit `$08 >> (i % 4)` of the nibble of entry `i / 4`. The
/// Death Mountain / Maze Island block (`$0620`) only offers maps free in
/// both tables and reserved by neither. The palace groups have no free map.
pub const PROGRESS_POOL: [PoolMap; 35] = [
    pm(0x0600, 25, &[0]),
    pm(0x0600, 26, &[0]),
    pm(0x0600, 27, &[0]),
    pm(0x0600, 32, &[0]),
    pm(0x0600, 37, &[0]),
    pm(0x0600, 49, &[0]),
    pm(0x0600, 55, &[0]),
    pm(0x0600, 61, &[0]),
    pm(0x0600, 62, &[0]),
    pm(0x0620, 31, &[1, 3]),
    pm(0x0620, 38, &[1, 3]),
    pm(0x0620, 45, &[1, 3]),
    pm(0x0620, 61, &[1, 3]),
    pm(0x0620, 62, &[1, 3]),
    pm(0x0640, 15, &[2]),
    pm(0x0640, 17, &[2]),
    pm(0x0640, 18, &[2]),
    pm(0x0640, 21, &[2]),
    pm(0x0640, 22, &[2]),
    pm(0x0640, 27, &[2]),
    pm(0x0640, 32, &[2]),
    pm(0x0640, 43, &[2]),
    pm(0x0640, 49, &[2]),
    pm(0x0640, 54, &[2]),
    pm(0x0640, 55, &[2]),
    pm(0x0660, 25, &[4]),
    pm(0x0660, 27, &[4]),
    pm(0x0660, 54, &[4]),
    pm(0x0660, 59, &[4]),
    pm(0x0660, 61, &[4]),
    pm(0x06C0, 55, &[7]),
    pm(0x06C0, 56, &[7]),
    pm(0x06C0, 57, &[7]),
    pm(0x06C0, 58, &[7]),
    pm(0x06C0, 59, &[7]),
];

/// Bits in [`PROGRESS_POOL`].
pub const PROGRESS_POOL_BITS: u16 = 4 * PROGRESS_POOL.len() as u16;

/// New-game item-presence template (bank 5, 224 bytes copied to
/// `$0600-$06DF`); the engine checks a pool nibble starts all set here.
pub const ITEM_TEMPLATE: u16 = 0xBB15;

/// Overflow for progress that does not fit nibbles: 64 bytes per save
/// slot inside [`BATTERY_SPARE`]. Persistent, but outside the slot
/// protocol: the engine must version it and clear a slot's block itself
/// when the game initialises that slot.
pub const PROGRESS_EXT_BASE: u16 = 0x7740;
/// Bytes per save slot at [`PROGRESS_EXT_BASE`].
pub const PROGRESS_EXT_STRIDE: u16 = 0x40;

#[cfg(test)]
mod tests {
    use super::*;

    const IN_USE: [WramRange; 6] = [
        LEVEL_BUFFER,
        WORLD_COPY,
        ENEMY_COPY,
        SAVE_MAIN,
        OVERWORLD_MAP,
        // Used bytes inside the candidate gaps (measured).
        r(0x6E66, 0x6FA0),
    ];

    fn overlaps(a: WramRange, b: WramRange) -> bool {
        a.start <= b.last() && b.start <= a.last()
    }

    #[test]
    fn free_ranges_avoid_everything_in_use() {
        for f in WRAM_FREE.iter().chain([&BATTERY_SPARE, &VROOM_SCRATCH]) {
            assert!(f.start >= 0x6000 && f.last() <= 0x7FFF);
            for u in IN_USE {
                assert!(!overlaps(*f, u), "{f:?} overlaps {u:?}");
            }
        }
        for (i, a) in WRAM_FREE.iter().enumerate() {
            for b in &WRAM_FREE[i + 1..] {
                assert!(!overlaps(*a, *b));
            }
        }
    }

    #[test]
    fn room_buffers_fit_the_largest_free_range() {
        let big = WRAM_FREE[0];
        let data_end = VROOM_DATA_BASE + VROOM_DATA_STRIDE * VROOM_DATA_COUNT;
        assert!(big.contains(VROOM_DATA_BASE));
        assert_eq!(data_end, VROOM_ENEMY_BASE);
        let enemy_end = VROOM_ENEMY_BASE + VROOM_ENEMY_STRIDE * VROOM_DATA_COUNT;
        assert_eq!(enemy_end, VROOM_SCRATCH.start);
        assert_eq!(VROOM_SCRATCH.last(), big.last());
    }

    #[test]
    fn reserved_maps_are_free_or_virtual() {
        for s in SETS {
            assert!(
                s.reserved_map == VIRTUAL_ONLY_MAP || s.free_maps.contains(&s.reserved_map),
                "{}",
                s.name
            );
            assert!(s.free_maps.iter().all(|&m| m < MAPS));
        }
        assert!(PALACE_A.free_maps.is_empty() && PALACE_B.free_maps.is_empty());
    }

    #[test]
    fn shared_item_block_does_not_alias() {
        // Death Mountain and Maze Island share $0620: the reserved maps
        // must differ and each must be free in both tables.
        assert_eq!(DEATH_MOUNTAIN.item_block, MAZE_ISLAND.item_block);
        assert_ne!(DEATH_MOUNTAIN.reserved_map, MAZE_ISLAND.reserved_map);
        for m in [DEATH_MOUNTAIN.reserved_map, MAZE_ISLAND.reserved_map] {
            assert!(DEATH_MOUNTAIN.free_maps.contains(&m));
            assert!(MAZE_ISLAND.free_maps.contains(&m));
        }
        for (i, a) in PROGRESS_SLOTS.iter().enumerate() {
            for b in &PROGRESS_SLOTS[i + 1..] {
                assert_ne!(a, b);
            }
            assert!((0x0600..0x06E0).contains(&a.0));
        }
    }

    #[test]
    fn overworld_slots_stay_inside_the_entry_scan() {
        for list in [
            WEST_FREE_SLOTS,
            EAST_FREE_SLOTS,
            DEATH_MOUNTAIN_FREE_SLOTS,
            MAZE_ISLAND_FREE_SLOTS,
        ] {
            assert!(list.iter().all(|&s| s <= ENTRY_SCAN_LAST));
            assert!(!list.contains(&RANDOM_BATTLE_SLOT));
        }
    }

    #[test]
    fn progress_pool_is_free_unreserved_and_distinct() {
        for (i, p) in PROGRESS_POOL.iter().enumerate() {
            for &s in p.sets {
                let set = &SETS[usize::from(s)];
                assert_eq!(set.item_block, p.block);
                assert!(set.free_maps.contains(&p.map), "{} map {}", set.name, p.map);
                assert_ne!(set.reserved_map, p.map);
            }
            // Every set sharing the block is listed.
            for (k, set) in SETS.iter().enumerate() {
                if set.item_block == p.block {
                    assert!(p.sets.contains(&(k as u8)));
                }
            }
            for q in &PROGRESS_POOL[i + 1..] {
                assert_ne!((p.block, p.map), (q.block, q.map));
            }
            assert!(!PROGRESS_SLOTS.contains(&item_nibble(p.block, p.map)));
        }
        assert_eq!(PROGRESS_POOL_BITS, 140);
    }

    #[test]
    fn progress_ext_fits_battery_spare() {
        assert!(BATTERY_SPARE.contains(PROGRESS_EXT_BASE));
        assert!(BATTERY_SPARE.contains(PROGRESS_EXT_BASE + 3 * PROGRESS_EXT_STRIDE - 1));
    }
}
