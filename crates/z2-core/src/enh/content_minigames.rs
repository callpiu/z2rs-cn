//! Minigame rules for the `content` group: the Target Gallery and the
//! Pogo Tower of `z2_content::areas::minigames` (rooms, constants and the
//! rules spec live there; this module runs them).
//!
//! Active only with `ContentOpts::minigames` on (and the master switch),
//! and only while Link is in one of the two rooms
//! ([`super::content::current_room`]). Both runtimes act at the end of a
//! frame on the enemy slots (`$2A,x` Y, `$4E,x` X, `$3C,x` page, `$A1,x`
//! ID, `$B6,x` state, `$C2,x` HP), so they work the same trapped and
//! interpreted; the round clocks only advance on a frame of play (game mode
//! `$0B`, no dialog `$074C`, no pane `$0524`, so the spell menu pauses
//! them).
//!
//! # Shared mechanics
//!
//! * **Moa** ([`mg::TARGET`] / [`mg::BOUNCER`], town enemy `$06`): spawned
//!   by the ROM's own enemy spawn (`LD625`, entered at `$D633` past its
//!   screen-column read) with a one-entry list written over the start of
//!   the room's enemy buffer for that one call (restored at once), so the
//!   ROM runs the Moa's init. Every frame the runtime puts each of its Moas
//!   back on its spot (`$4E,x`, `$3C,x`, `$2A,x` = row * 16 + `$20`, X/Y
//!   speeds `$71,x` / `$057E,x` zeroed), so they hover in place (a pixel or
//!   two of drift inside a frame).
//! * **Harmless**: `bank7_Link_Collision_Detection` (`$D6C1`) is hooked;
//!   for a slot holding one of the runtime's Moas it returns "no contact"
//!   (`A = 0`, `Z` set, the ROM's own early exit), so touching one never
//!   hurts Link. Every other slot runs the routine it replaced.
//! * **Visible**: in the East towns (world 2, the Pogo Tower's) the Moa's
//!   routine draws it only with the cross (`$DAD6`: `$0707 == 2` and
//!   `$078A == 0` skip `JSR $EF11`, then `JSR $D6C1` follows). The same
//!   hook runs that draw (`$EF11`) for the runtime's Moas when it was
//!   skipped, so the bouncers show without the cross.
//! * **No XP, no drops**: a Moa has 48 HP; a sword hit only takes the
//!   level's sword damage off, and the runtime notices the drop at the end
//!   of the frame. The Moa never reaches the ROM's death code.
//!
//! # Target Gallery
//!
//! * **Arming**: on entry the round is armed unless the gallery is won
//!   (minigames flag [`FLAG_GALLERY_WON`], battery-saved) or its prize page
//!   is taken. Until the win the prize's item object (`$A1,x = 1`) is
//!   removed every frame, so the heart piece is not there.
//! * **Start**: [`mg::GALLERY_START_DELAY`] frames of play after entry the
//!   round starts (dialog blip, `$EE = $08`). Nothing shows the
//!   `GALLERY_INTRO` text yet: there is no dialog for virtual rooms.
//! * **Targets**: at most [`mg::GALLERY_LIVE_TARGETS`] up at once, each on
//!   one of [`mg::TARGET_SPOTS`] that is free and not the last one used,
//!   preferring the spots within [`GALLERY_NEAR`] pixels of Link. Spot
//!   choice: number `($12 + 3 * spawned) % count` of those. A target stays up [`mg::GALLERY_TARGET_LIFE`] frames (a miss), the
//!   next one comes [`mg::GALLERY_SPAWN_GAP`] frames after a target leaves
//!   or appears; [`mg::GALLERY_MAX_TARGETS`] per round.
//! * **Hits**: a target whose HP dropped below the spawn HP, or whose
//!   `$B6,x` reached 2, counts once and is removed (the ROM plays its own
//!   hit sound).
//! * **Win**: [`mg::GALLERY_HITS_NEEDED`] hits within
//!   [`mg::GALLERY_TIME_LIMIT`] frames: targets cleared, flag
//!   [`FLAG_GALLERY_WON`] set, `$EF = $08` (the crystal-placed chime), and
//!   the prize's item object is spawned the way `LC9A5` does on its page
//!   (it falls to the floor). Link takes it with the normal pickup; the
//!   engine records the page bit.
//! * **Lose**: time runs out first (or the targets run out): targets
//!   cleared, `$EC = $02` (the deflect "tink"). The round re-arms the next
//!   time Link enters. Leaving mid-round abandons it.
//!
//! # Pogo Tower
//!
//! * **Bouncers**: the room's enemy list ([`mg::BOUNCER`] at
//!   [`mg::BOUNCER_SPOTS`]) is what the validator models. At run time the
//!   runtime owns them instead of the ROM's scroll spawner: on entry it
//!   keeps the Moas the room load spawned (each one's list index `$BC,x`
//!   names its spot) and marks every list entry spawned (bit 7 of its
//!   first byte), and from then on it keeps a Moa on every spot within
//!   [`TOWER_RANGE`] pixels of Link that a free slot allows (nearest first)
//!   and removes the ones farther away, so a bouncer is always there to
//!   retry on. Each frame: pinned, HP back to its spawn value, hit flash
//!   `$040E,x` cleared (so it is hittable again at once), never killed.
//! * **Bounce**: a down stab on a bouncer gets the ROM's bounce
//!   (`$057D = $FE`), which only lifts Link a few pixels. On a frame where a
//!   bouncer was hit and `$057D` is `$FE`, the runtime writes the jump
//!   kick instead: `$057D` = [`TOWER_BOUNCE_VSPEED`], gravity counter
//!   `$03E6` = [`TOWER_BOUNCE_GRAV`], `$0479 = 2` (airborne). Only in this
//!   room.
//! * No timer and no fail state; the prize is ordinary room data.
//!
//! # State (`EnhState::minigame`, 16 bytes)
//!
//! | byte | use |
//! |---|---|
//! | 0 | room the state belongs to (`vroom`, id + 1; 0 none) |
//! | 1 | gallery phase ([`Phase`]) |
//! | 2-3 | gallery clock (little endian): start countdown, then round frames |
//! | 4 | gallery hits |
//! | 5 | gallery targets spawned |
//! | 6 | gallery spawn gap countdown |
//! | 7 | gallery last spot + 1 |
//! | 8-10, 11-13 | gallery targets: slot + 1, spot, frames left |
//! | 8-13 | tower: spot + 1 held by enemy slot 0-5 |
//!
//! Persistent: minigames flag [`FLAG_GALLERY_WON`] (pool bit, battery).
//! Everything else is `EnhState` (save states and rollback carry it).

use super::content::{self, ContentOpts, Group, VRoomId};
use crate::game::Game;
use z2_content::areas::minigames as mg;

/// Minigames flag: the gallery is won (its prize is shown from then on).
pub const FLAG_GALLERY_WON: u16 = 0;

/// `bank7_Link_Collision_Detection` (hooked: the Moas are harmless).
pub const HOOK_LINK_CONTACT: u16 = 0xD6C1;
/// The enemy draw the Moa's routine skips in East towns without the cross
/// (`$DAD6`: `LDA $0707 : CMP #$02 : BNE + : LDA $078A : BEQ ++ :
/// + JSR $EF11 : ++ JSR $D6C1`).
pub const DRAW_ENEMY: u16 = 0xEF11;
/// `LD625` past its `$0732,x` / `$0734,x` read (`$00`, `$01`, `X = $10`,
/// `Y = 1` set by the caller): the ROM's enemy spawn.
pub const SPAWN_ENTRY: u16 = 0xD633;

/// Gallery: a new target goes on a spot at most this many pixels left or
/// right of Link when there is one (so every target can be reached in its
/// lifetime).
pub const GALLERY_NEAR: u16 = 0xA0;
/// Tower: Y speed written on a bounce (the jump spell's take-off speed of
/// the bank 0 table `$9470`).
pub const TOWER_BOUNCE_VSPEED: u8 = 0xFB;
/// Tower: gravity counter written with it (bank 0 `$9474`).
pub const TOWER_BOUNCE_GRAV: u8 = 0x80;
/// Tower: bouncers are kept on spots at most this many pixels left or
/// right of Link.
pub const TOWER_RANGE: u16 = 0x90;

// The state bytes hold these as `u8`, and there are two target records.
const _: () = assert!(mg::TARGET_SPOTS.len() < 256 && mg::BOUNCER_SPOTS.len() < 255);
const _: () = assert!(mg::GALLERY_TARGET_LIFE < 256 && mg::GALLERY_SPAWN_GAP < 256);
const _: () = assert!(mg::GALLERY_LIVE_TARGETS == 2);

/// Gallery item object for the prize (`$A1,x`).
const ITEM_OBJECT: u8 = 0x01;
/// Heart container: how the engine shows a heart piece.
const HEART_CONTAINER: u8 = 0x0F;

const MODE: usize = 0x0736;
const WORLD: usize = 0x0707;
const CROSS: usize = 0x078A;
const MODE_SIDEVIEW: u8 = 0x0B;
const DIALOG: usize = 0x074C;
const PANE: usize = 0x0524;
const FRAME: usize = 0x0012;
const LINK_X: usize = 0x004D;
const LINK_PAGE: usize = 0x003B;
const VSPEED: usize = 0x057D;
const GRAV: usize = 0x03E6;
const AIRBORNE: usize = 0x0479;
const SAVED_BANK: usize = 0x0769;
const SFX_EC: usize = 0x00EC;
const SFX_EE: usize = 0x00EE;
const SFX_EF: usize = 0x00EF;
/// Enemy HP by ID (`$6D21`, WRAM world copy).
const HP_TABLE: usize = 0x0D21;

const SLOTS: usize = 6;
const E_Y: usize = 0x2A;
const E_X: usize = 0x4E;
const E_PAGE: usize = 0x3C;
const E_ID: usize = 0xA1;
const E_STATE: usize = 0xB6;
const E_HP: usize = 0xC2;
const E_LIST: usize = 0xBC;
const E_ITEM: usize = 0xAF;
const E_XSPEED: usize = 0x71;
const E_YSPEED: usize = 0x057E;
const E_FLASH: usize = 0x040E;
const E_1A: usize = 0x1A;

/// Gallery phase (`EnhState::minigame[1]`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Phase {
    /// Armed, counting down to the start.
    Armed = 0,
    /// Round running.
    Running = 1,
    /// Won (the prize is out, or taken).
    Won = 2,
    /// Lost (nothing runs until Link comes back).
    Lost = 3,
}

impl Phase {
    fn from(b: u8) -> Phase {
        match b {
            1 => Phase::Running,
            2 => Phase::Won,
            3 => Phase::Lost,
            _ => Phase::Armed,
        }
    }
}

/// Which minigame a room is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Minigame {
    /// Target Gallery.
    Gallery,
    /// Pogo Tower.
    Tower,
}

/// The minigame room `id` is, if any.
#[must_use]
pub fn minigame_of(id: VRoomId) -> Option<Minigame> {
    let r = content::room(id)?;
    if r.group != Group::Minigames {
        return None;
    }
    match r.area.name {
        "Target Gallery" => Some(Minigame::Gallery),
        "Pogo Tower" => Some(Minigame::Tower),
        _ => None,
    }
}

/// The minigame Link is in right now (option on, in the room's slot).
#[must_use]
pub fn active(game: &Game) -> Option<(VRoomId, Minigame)> {
    if !(game.enh.content.virtual_rooms && game.enh.content.minigames) {
        return None;
    }
    let id = content::current_room(game)?;
    Some((id, minigame_of(id)?))
}

/// Gallery phase, while Link is in the gallery.
#[must_use]
pub fn gallery_phase(game: &Game) -> Option<Phase> {
    matches!(active(game), Some((_, Minigame::Gallery))).then(|| Phase::from(st(game)[1]))
}

/// Gallery hits so far this round.
#[must_use]
pub fn gallery_hits(game: &Game) -> u8 {
    st(game)[4]
}

/// Gallery frames played this round (or the start countdown while armed).
#[must_use]
pub fn gallery_clock(game: &Game) -> u16 {
    u16::from_le_bytes([st(game)[2], st(game)[3]])
}

/// Gallery targets spawned this round.
#[must_use]
pub fn gallery_spawned(game: &Game) -> u8 {
    st(game)[5]
}

/// Live gallery targets: (enemy slot, spot index).
#[must_use]
pub fn gallery_targets(game: &Game) -> Vec<(usize, usize)> {
    let s = st(game);
    [8usize, 11]
        .iter()
        .filter(|&&o| s[o] != 0)
        .map(|&o| (usize::from(s[o] - 1), usize::from(s[o + 1])))
        .collect()
}

/// Whether the gallery has been won (persistent flag).
#[must_use]
pub fn gallery_won(game: &Game) -> bool {
    content::flag(game, Group::Minigames, FLAG_GALLERY_WON)
}

fn st(game: &Game) -> &[u8; 16] {
    &game.enh_state.minigame
}

fn st_mut(game: &mut Game) -> &mut [u8; 16] {
    &mut game.enh_state.minigame
}

// ------------------------------------------------------------ hooks

/// Install this module's patches and hooks.
pub(crate) fn register(game: &mut Game, opts: &ContentOpts) {
    if !(opts.virtual_rooms && opts.minigames) {
        return;
    }
    super::hook(
        game,
        "enh_content_minigame_contact",
        Some(7),
        HOOK_LINK_CONTACT,
        hook_contact,
        None,
    );
}

/// `$D6C1` wrapper: no contact with the runtime's Moas.
fn hook_contact(game: &mut Game) {
    let x = usize::from(game.cpu.x);
    if x < SLOTS && owns_slot(game, x) {
        use crate::cpu::{FLAG_N, FLAG_Z};
        if game.ram[WORLD] == 2 && game.ram[CROSS] == 0 {
            // East towns hide the Moa without the cross (`$DAD6`): draw
            // the runtime's own Moas anyway, as the skipped `JSR` would.
            let (y, p) = (game.cpu.y, game.cpu.p);
            let _ = game.try_call_asm(DRAW_ENEMY, 4000);
            game.cpu.x = x as u8;
            game.cpu.y = y;
            game.cpu.p = p;
        }
        game.cpu.a = 0;
        game.cpu.p = (game.cpu.p & !FLAG_N) | FLAG_Z;
        return;
    }
    super::call_original(game, HOOK_LINK_CONTACT);
}

/// Whether enemy slot `x` holds one of the runtime's Moas.
fn owns_slot(game: &Game, x: usize) -> bool {
    let Some((id, which)) = active(game) else {
        return false;
    };
    let s = st(game);
    if s[0] != vroom_tag(id) {
        return false;
    }
    match which {
        Minigame::Gallery => [8usize, 11]
            .iter()
            .any(|&o| s[o] != 0 && usize::from(s[o] - 1) == x),
        Minigame::Tower => s[8 + x] != 0,
    }
}

fn vroom_tag(id: VRoomId) -> u8 {
    (id.0 + 1) as u8
}

/// Per-frame work after [`Game::step`].
pub(crate) fn end_of_frame(game: &mut Game, opts: &ContentOpts) {
    if !(opts.virtual_rooms && opts.minigames) {
        return;
    }
    let Some((id, which)) = active(game) else {
        if game.enh_state.minigame != [0; 16] {
            game.enh_state.minigame = [0; 16];
        }
        return;
    };
    let entered = st(game)[0] != vroom_tag(id);
    if entered {
        game.enh_state.minigame = [0; 16];
        st_mut(game)[0] = vroom_tag(id);
    }
    match which {
        Minigame::Gallery => gallery_frame(game, id, entered),
        Minigame::Tower => tower_frame(game, entered),
    }
}

fn playing(game: &Game) -> bool {
    game.ram[MODE] == MODE_SIDEVIEW && game.ram[DIALOG] == 0 && game.ram[PANE] == 0
}

// ------------------------------------------------------------ enemy helpers

/// Spawn hp of enemy `id` in this world.
fn spawn_hp(game: &Game, id: u8) -> u8 {
    game.wram[HP_TABLE + usize::from(id & 0x3F)]
}

/// Room-pixel X of column `col`.
fn col_px(col: u8) -> u16 {
    u16::from(col) * 16
}

/// Link's room-pixel X.
fn link_px(game: &Game) -> u16 {
    u16::from(game.ram[LINK_PAGE] & 3) * 256 + u16::from(game.ram[LINK_X])
}

/// Spawn enemy `id` into free slot `slot` at column `col`, row `row`
/// through the ROM's spawn (`$D633`). Returns whether the slot came alive.
fn spawn_enemy(game: &mut Game, slot: usize, id: u8, col: u8, row: u8) -> bool {
    let eb = usize::from(content::WRAM_ENEMY_BUF - 0x6000);
    let page = (col >> 4) & 3;
    let c = col & 0x0F;
    let raw = row.saturating_sub(2) & 7;
    let saved_list = [game.wram[eb], game.wram[eb + 1], game.wram[eb + 2]];
    let saved_zp: [u8; 0x11] = std::array::from_fn(|i| game.ram[i]);
    let saved_ptr = [game.ram[0xD6], game.ram[0xD7]];
    let (a, x, y, p) = (game.cpu.a, game.cpu.x, game.cpu.y, game.cpu.p);
    let bank = game.mmc1.prg;
    game.wram[eb..eb + 3].copy_from_slice(&[3, (raw << 4) | c, (page << 6) | (id & 0x3F)]);
    let [lo, hi] = content::WRAM_ENEMY_BUF.to_le_bytes();
    game.ram[0xD6] = lo;
    game.ram[0xD7] = hi;
    game.ram[0x00] = page;
    game.ram[0x01] = c;
    game.ram[0x10] = slot as u8;
    game.ram[E_STATE + slot] = 0;
    game.cpu.x = slot as u8;
    game.cpu.y = 1;
    game.mmc1.prg = game.ram[SAVED_BANK];
    let ok = game.try_call_asm(SPAWN_ENTRY, 4000).is_ok();
    game.mmc1.prg = bank;
    game.cpu.a = a;
    game.cpu.x = x;
    game.cpu.y = y;
    game.cpu.p = p;
    game.ram[0xD6] = saved_ptr[0];
    game.ram[0xD7] = saved_ptr[1];
    game.ram[..0x11].copy_from_slice(&saved_zp);
    game.wram[eb..eb + 3].copy_from_slice(&saved_list);
    let alive = ok && game.ram[E_STATE + slot] == 1 && game.ram[E_ID + slot] == id;
    if alive {
        pin(game, slot, col, row);
    } else {
        game.ram[E_STATE + slot] = 0;
    }
    alive
}

/// Hold the enemy in `slot` on its spot.
fn pin(game: &mut Game, slot: usize, col: u8, row: u8) {
    let px = col_px(col);
    game.ram[E_X + slot] = (px & 0xFF) as u8;
    game.ram[E_PAGE + slot] = (px >> 8) as u8;
    game.ram[E_Y + slot] = row.wrapping_mul(16).wrapping_add(0x20);
    game.ram[E_XSPEED + slot] = 0;
    game.ram[E_YSPEED + slot] = 0;
}

/// A free enemy slot, scanning 5..0 like the ROM.
fn free_slot(game: &Game) -> Option<usize> {
    (0..SLOTS).rev().find(|&s| game.ram[E_STATE + s] == 0)
}

// ------------------------------------------------------------ gallery

fn gallery_frame(game: &mut Game, id: VRoomId, entered: bool) {
    let taken = content::taken_pages(game, id) != 0;
    if entered {
        st_mut(game)[1] = if gallery_won(game) || taken {
            Phase::Won as u8
        } else {
            Phase::Armed as u8
        };
        st_mut(game)[2..4].copy_from_slice(&mg::GALLERY_START_DELAY.to_le_bytes());
    }
    let phase = Phase::from(st(game)[1]);
    if phase != Phase::Won {
        hide_prize(game);
    }
    if !playing(game) {
        return;
    }
    match phase {
        Phase::Armed => {
            let left = gallery_clock(game).saturating_sub(1);
            set_clock(game, left);
            if left == 0 {
                let s = st_mut(game);
                s[1] = Phase::Running as u8;
                s[2..14].fill(0);
                game.ram[SFX_EE] = 0x08;
            }
        }
        Phase::Running => gallery_round(game, id),
        Phase::Won | Phase::Lost => {}
    }
}

fn set_clock(game: &mut Game, v: u16) {
    st_mut(game)[2..4].copy_from_slice(&v.to_le_bytes());
}

/// Remove the gallery's prize object (not won yet).
fn hide_prize(game: &mut Game) {
    for s in 0..SLOTS {
        if game.ram[E_STATE + s] != 0 && game.ram[E_ID + s] == ITEM_OBJECT {
            game.ram[E_STATE + s] = 0;
        }
    }
}

fn gallery_round(game: &mut Game, id: VRoomId) {
    let clock = gallery_clock(game).saturating_add(1);
    set_clock(game, clock);
    let hp0 = spawn_hp(game, mg::TARGET);
    let mut freed = false;
    for o in [8usize, 11] {
        let s = st(game)[o];
        if s == 0 {
            continue;
        }
        let slot = usize::from(s - 1);
        let spot = usize::from(st(game)[o + 1]);
        let live = game.ram[E_STATE + slot] != 0 && game.ram[E_ID + slot] == mg::TARGET;
        let hit = live && (game.ram[E_HP + slot] < hp0 || game.ram[E_STATE + slot] == 2);
        let life = st(game)[o + 2].saturating_sub(1);
        if hit {
            st_mut(game)[4] += 1;
        }
        if !live || hit || life == 0 {
            if live {
                game.ram[E_STATE + slot] = 0;
            }
            st_mut(game)[o..o + 3].fill(0);
            freed = true;
        } else {
            st_mut(game)[o + 2] = life;
            let (c, r) = mg::TARGET_SPOTS[spot % mg::TARGET_SPOTS.len()];
            pin(game, slot, c, r);
        }
    }
    if freed {
        st_mut(game)[6] = st(game)[6].max(mg::GALLERY_SPAWN_GAP as u8);
    }
    let live = gallery_targets(game).len();
    if st(game)[4] >= mg::GALLERY_HITS_NEEDED {
        clear_targets(game);
        st_mut(game)[1] = Phase::Won as u8;
        content::set_flag(game, Group::Minigames, FLAG_GALLERY_WON, true);
        reveal_prize(game, id);
        game.ram[SFX_EF] = 0x08;
        return;
    }
    let out = st(game)[5] >= mg::GALLERY_MAX_TARGETS && live == 0;
    if clock >= mg::GALLERY_TIME_LIMIT || out {
        clear_targets(game);
        st_mut(game)[1] = Phase::Lost as u8;
        game.ram[SFX_EC] = 0x02;
        return;
    }
    if st(game)[6] > 0 {
        st_mut(game)[6] -= 1;
        return;
    }
    if live < usize::from(mg::GALLERY_LIVE_TARGETS) && st(game)[5] < mg::GALLERY_MAX_TARGETS {
        spawn_target(game);
    }
}

/// Spot for the next target: among the spots within [`GALLERY_NEAR`]
/// pixels of Link (all of them when none is), free and not the last one
/// used, number `($12 + 3 * spawned) % count`.
fn pick_spot(game: &Game) -> Option<usize> {
    let s = st(game);
    let used: Vec<usize> = gallery_targets(game).iter().map(|&(_, p)| p).collect();
    let last = s[7].checked_sub(1).map(usize::from);
    let lx = link_px(game);
    let open: Vec<usize> = (0..mg::TARGET_SPOTS.len())
        .filter(|p| !used.contains(p) && Some(*p) != last)
        .collect();
    let near: Vec<usize> = open
        .iter()
        .copied()
        .filter(|&p| (col_px(mg::TARGET_SPOTS[p].0) + 8).abs_diff(lx) <= GALLERY_NEAR)
        .collect();
    let pool = if near.is_empty() { open } else { near };
    let n = pool.len();
    (n > 0).then(|| pool[(usize::from(game.ram[FRAME]) + 3 * usize::from(s[5])) % n])
}

fn spawn_target(game: &mut Game) {
    let Some(slot) = free_slot(game) else {
        return;
    };
    let Some(spot) = pick_spot(game) else {
        return;
    };
    let (c, r) = mg::TARGET_SPOTS[spot];
    if !spawn_enemy(game, slot, mg::TARGET, c, r) {
        return;
    }
    let o = if st(game)[8] == 0 { 8 } else { 11 };
    let s = st_mut(game);
    s[o] = slot as u8 + 1;
    s[o + 1] = spot as u8;
    s[o + 2] = mg::GALLERY_TARGET_LIFE as u8;
    s[5] += 1;
    s[7] = spot as u8 + 1;
    s[6] = mg::GALLERY_SPAWN_GAP as u8;
}

fn clear_targets(game: &mut Game) {
    for (slot, _) in gallery_targets(game) {
        game.ram[E_STATE + slot] = 0;
    }
    st_mut(game)[8..14].fill(0);
}

/// Put the prize's item object out (`LC9A5`'s fields) on its page.
fn reveal_prize(game: &mut Game, id: VRoomId) {
    if content::taken_pages(game, id) != 0 {
        return;
    }
    let Some(slot) = free_slot(game) else {
        return;
    };
    let (c, r) = mg::GALLERY_PRIZE;
    let px = col_px(c);
    let r8 = &mut game.ram;
    r8[E_PAGE + slot] = (px >> 8) as u8;
    r8[E_LIST + slot] = (r8[E_LIST + slot] >> 1) | 0x80;
    r8[E_X + slot] = ((px & 0xFF) as u8).wrapping_add(3);
    r8[E_Y + slot] = r.wrapping_mul(16).wrapping_add(0x20);
    r8[E_1A + slot] = 1;
    r8[E_STATE + slot] = 1;
    r8[E_ID + slot] = ITEM_OBJECT;
    r8[E_YSPEED + slot] = 1;
    r8[E_ITEM + slot] = HEART_CONTAINER;
}

// ------------------------------------------------------------ tower

fn tower_frame(game: &mut Game, entered: bool) {
    let hp0 = spawn_hp(game, mg::BOUNCER);
    if entered {
        // Adopt the Moas the room load spawned (list index -> spot).
        for s in 0..SLOTS {
            if game.ram[E_STATE + s] == 0 || game.ram[E_ID + s] != mg::BOUNCER {
                continue;
            }
            let idx = usize::from(game.ram[E_LIST + s]);
            let spot = idx.checked_sub(1).map(|i| i / 2);
            match spot.filter(|&k| idx % 2 == 1 && k < mg::BOUNCER_SPOTS.len()) {
                Some(k) => st_mut(game)[8 + s] = k as u8 + 1,
                None => game.ram[E_STATE + s] = 0,
            }
        }
    }
    block_list(game);
    let mut bounced = false;
    for s in 0..SLOTS {
        let spot = st(game)[8 + s];
        if spot == 0 {
            continue;
        }
        if game.ram[E_STATE + s] == 0 || game.ram[E_ID + s] != mg::BOUNCER {
            st_mut(game)[8 + s] = 0;
            continue;
        }
        if game.ram[E_HP + s] < hp0 {
            bounced = true;
        }
        let (c, r) = mg::BOUNCER_SPOTS[usize::from(spot - 1)];
        pin(game, s, c, r);
        game.ram[E_HP + s] = hp0;
        game.ram[E_FLASH + s] = 0;
        if game.ram[E_STATE + s] == 2 {
            game.ram[E_STATE + s] = 1;
        }
    }
    if bounced && game.ram[VSPEED] == 0xFE {
        game.ram[VSPEED] = TOWER_BOUNCE_VSPEED;
        game.ram[GRAV] = TOWER_BOUNCE_GRAV;
        game.ram[AIRBORNE] = 2;
    }
    manage_bouncers(game);
}

/// Mark every entry of the room's enemy list spawned, so the ROM's scroll
/// spawner leaves the bouncers to the runtime.
fn block_list(game: &mut Game) {
    let eb = usize::from(content::WRAM_ENEMY_BUF - 0x6000);
    let len = usize::from(game.wram[eb]).min(content::ENEMY_BUF_LEN);
    let mut i = 1;
    while i + 1 < len {
        game.wram[eb + i] |= 0x80;
        i += 2;
    }
}

/// Keep a bouncer on every spot near Link (nearest first, as slots allow);
/// drop the far ones.
fn manage_bouncers(game: &mut Game) {
    let lx = link_px(game);
    let near = |k: usize| {
        let x = col_px(mg::BOUNCER_SPOTS[k].0) + 8;
        x.abs_diff(lx) <= TOWER_RANGE
    };
    for s in 0..SLOTS {
        let spot = st(game)[8 + s];
        if spot != 0 && !near(usize::from(spot - 1)) {
            game.ram[E_STATE + s] = 0;
            st_mut(game)[8 + s] = 0;
        }
    }
    let held: Vec<usize> = (0..SLOTS)
        .filter_map(|s| st(game)[8 + s].checked_sub(1).map(usize::from))
        .collect();
    let mut want: Vec<usize> = (0..mg::BOUNCER_SPOTS.len())
        .filter(|&k| near(k) && !held.contains(&k))
        .collect();
    want.sort_by_key(|&k| (col_px(mg::BOUNCER_SPOTS[k].0) + 8).abs_diff(lx));
    for k in want {
        let Some(slot) = free_slot(game) else {
            break;
        };
        let (c, r) = mg::BOUNCER_SPOTS[k];
        if spawn_enemy(game, slot, mg::BOUNCER, c, r) {
            st_mut(game)[8 + slot] = k as u8 + 1;
        } else {
            break;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rooms_are_found() {
        let g = content::find_room("Target Gallery", 0).unwrap();
        let t = content::find_room("Pogo Tower", 0).unwrap();
        assert_eq!(minigame_of(g), Some(Minigame::Gallery));
        assert_eq!(minigame_of(t), Some(Minigame::Tower));
        assert_eq!(minigame_of(content::TEST_ROOM), None);
        assert!(content::flag_bit(Group::Minigames, FLAG_GALLERY_WON).is_some());
    }

    #[test]
    fn idle_outside_the_rooms() {
        let mut g = Game::new();
        let opts = ContentOpts {
            virtual_rooms: true,
            minigames: true,
            ..ContentOpts::default()
        };
        g.enh_state.minigame[3] = 9;
        end_of_frame(&mut g, &opts);
        assert_eq!(g.enh_state.minigame, [0; 16]);
        assert_eq!(gallery_phase(&g), None);
    }
}
