//! The `content` minigames (`z2_core::enh::content_minigames`) in a real
//! run, driven by pad input only.
//!
//! * Target Gallery (behind Mido's street door, map 10 page 2): an idle
//!   player loses (no prize); a scripted player wins (12 hits inside the
//!   30-second round), the prize appears only then, the win flag and the
//!   taken prize survive a battery save and a reload, and a won gallery is
//!   not re-armed.
//! * Pogo Tower (behind Darunia's street door, map 16 page 1): a scripted
//!   climber reaches the plateau and the prize with chained down stabs;
//!   with the same script and no downward thrust he never gets there.
//!
//! The scripted players read RAM (Link's and the enemies' positions) to
//! decide what to press, like a person watching the screen. The gallery
//! player tries a few strikes on each live target in a throwaway copy of
//! the game (save state) and plays the quickest one that scores.
//!
//! ROM-gated (self-skips unless `Z2_ROM` names a file). With
//! `Z2_MINIGAME_SHOTS=DIR` (outside the work tree: frames come from the
//! ROM) it saves PNGs for eyeballing.

mod common;

use z2_content::areas::minigames as mg;
use z2_core::enh::content::{self, ContentOpts};
use z2_core::enh::content_minigames::{self as m, Phase};
use z2_core::enh::Enhancements;
use z2_core::game::{
    Game, INPUT_A, INPUT_B, INPUT_DOWN, INPUT_LEFT, INPUT_RIGHT, INPUT_SELECT, INPUT_START,
};

const A: u8 = 1 << INPUT_A;
const B: u8 = 1 << INPUT_B;
const DOWN: u8 = 1 << INPUT_DOWN;
const SELECT: u8 = 1 << INPUT_SELECT;
const START: u8 = 1 << INPUT_START;
const LEFT: u8 = 1 << INPUT_LEFT;
const RIGHT: u8 = 1 << INPUT_RIGHT;

const MODE: usize = 0x0736;
const MODE_SIDEVIEW: u8 = 0x0B;
const SCENE: usize = 0x0561;
const WORLD: usize = 0x0707;
const PAGE: usize = 0x003B;
const LINK_X: usize = 0x004D;
const LINK_Y: usize = 0x0029;
const AIRBORNE: usize = 0x0479;
const THRUSTS: usize = 0x0796;
const DOWN_THRUST: u8 = 0x10;
const XP_HI: usize = 0x0775;
const XP_LO: usize = 0x0776;
const LIFE: usize = 0x0774;
const STAGE: usize = 0x076C;
const MENU_SEL: usize = 0x0488;
const LIVES: usize = 0x0700;

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

fn opts(minigames: bool) -> Enhancements {
    Enhancements {
        content: ContentOpts {
            virtual_rooms: true,
            minigames,
            ..ContentOpts::default()
        },
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

fn step(g: &mut Game, frames: usize, pad: u8) {
    for _ in 0..frames {
        g.step(pad);
    }
}

/// Title -> file select -> name -> load -> side-view play.
fn reach_gameplay(g: &mut Game) {
    step(g, 30, 0);
    step(g, 5, START);
    step(g, 20, 0);
    step(g, 5, START);
    step(g, 20, 0);
    for _ in 0..8 {
        step(g, 2, A);
        step(g, 8, 0);
    }
    for _ in 0..3 {
        step(g, 2, SELECT);
        step(g, 8, 0);
    }
    step(g, 5, START);
    step(g, 30, 0);
    step(g, 5, START);
    step(g, 170, 0);
    assert_eq!(g.ram[MODE], MODE_SIDEVIEW, "reached side-view play");
}

/// Magic topped up (life is left alone: the tests check the Moas never
/// hurt Link).
fn top_up(g: &mut Game) {
    g.ram[0x0773] = 0xFF;
}

fn step_until(g: &mut Game, max: usize, f: impl Fn(&Game) -> bool) -> bool {
    for _ in 0..max {
        if f(g) {
            return true;
        }
        g.step(0);
    }
    f(g)
}

/// Load map `map` of town world `world` through the game's own loader.
fn load_scene(g: &mut Game, region: u8, world: u8, key: u8, map: u8, page: u8) -> bool {
    g.ram[0x0706] = region;
    g.ram[WORLD] = world;
    g.ram[0x070A] = 0;
    g.ram[0x0748] = key;
    g.ram[0x056C] = key.wrapping_sub(0x34);
    g.ram[SCENE] = map;
    g.ram[0x075C] = page;
    g.ram[0x0701] = page & 1;
    g.ram[0x0709] = 0;
    g.ram[0x0704] = 0;
    g.ram[MODE] = 0;
    step(g, 150, 0);
    step_until(g, 600, |g| g.ram[MODE] == MODE_SIDEVIEW)
}

/// Call the ROM's door routine (`$CFFC`) the way the game does when Link
/// walks into a doorway on page `page` (a `JSR` from scratch WRAM, so the
/// hooked trap fires).
fn take_door(g: &mut Game, page: u8) {
    g.ram[PAGE] = page;
    let at = 0x0900; // $6900, engine scratch (content_mem::VROOM_SCRATCH)
    g.wram[at..at + 4].copy_from_slice(&[0x20, 0xFC, 0xCF, 0x60]);
    g.call_asm(0x6900);
}

/// Through the street door into minigame `area` (`ATTACH` entry `i`).
fn enter_minigame(g: &mut Game, i: usize) {
    let at = mg::ATTACH[i];
    let (region, world, key) = match i {
        0 => (0, 1, 0x33),
        _ => (2, 2, 0x2F),
    };
    assert!(
        load_scene(g, region, world, key, at.street, at.door_slot),
        "{} street",
        at.town
    );
    take_door(g, at.door_slot);
    let id = content::find_room(at.area, 0).unwrap();
    assert!(
        step_until(g, 600, |g| g.ram[MODE] == MODE_SIDEVIEW
            && content::current_room(g) == Some(id)),
        "{} entered",
        at.area
    );
}

fn shot(g: &Game, name: &str) {
    if let Some(dir) = std::env::var_os("Z2_MINIGAME_SHOTS") {
        let png = z2_ppu::encode_indexed_png(g.frame_indexed());
        let path = std::path::Path::new(&dir).join(format!("{name}.png"));
        std::fs::write(path, png).expect("write shot");
    }
}

fn xp(g: &Game) -> u16 {
    u16::from_be_bytes([g.ram[XP_HI], g.ram[XP_LO]])
}

/// Link's room-pixel X (centre).
fn link_px(g: &Game) -> i32 {
    i32::from(g.ram[PAGE] & 3) * 256 + i32::from(g.ram[LINK_X]) + 8
}

/// Enemy slot `s`'s room-pixel X (centre).
fn enemy_px(g: &Game, s: usize) -> i32 {
    i32::from(g.ram[0x3C + s] & 3) * 256 + i32::from(g.ram[0x4E + s]) + 8
}

/// Item objects (`$A1,x = 1`) alive.
fn item_objects(g: &Game) -> usize {
    (0..6)
        .filter(|&s| g.ram[0xB6 + s] != 0 && g.ram[0xA1 + s] == 1)
        .count()
}

fn gallery() -> content::VRoomId {
    content::find_room("Target Gallery", 0).unwrap()
}

fn tower() -> content::VRoomId {
    content::find_room("Pogo Tower", 0).unwrap()
}

// ------------------------------------------------------------ gallery player

const LINK_HSPEED: usize = 0x0070;
const LINK_FACING: usize = 0x009F;

/// Strike on target `slot`: walk to the striking distance on the side Link
/// is on, stop facing it, then a crouching stab (a target on the floor
/// row) or a jump with a late stab (a target two rows up). Returns the
/// frames used when the hit counter went up, `None` when the target left
/// or the strike missed.
fn strike_target(g: &mut Game, slot: usize, max: usize) -> Option<usize> {
    #[derive(Clone, Copy, PartialEq)]
    enum St {
        Approach,
        Settle,
        Strike(usize),
    }
    let hits0 = m::gallery_hits(g);
    let mut st = St::Approach;
    for f in 0..max {
        if m::gallery_hits(g) > hits0 {
            return Some(f);
        }
        if !m::gallery_targets(g).iter().any(|&(s, _)| s == slot) {
            return None;
        }
        top_up(g);
        let tx = enemy_px(g, slot);
        let low = g.ram[0x2A + slot] >= 0xB0;
        let lx = link_px(g);
        let side = if lx < tx { -1 } else { 1 };
        let toward = if side < 0 { RIGHT } else { LEFT };
        let grounded = g.ram[AIRBORNE] == 0;
        let goal = tx + side * if low { 0x12 } else { 0x1E };
        let tol = if low { 6 } else { 4 };
        // Link slides about speed^2 / 34 px after letting go.
        let sp = i32::from(g.ram[LINK_HSPEED] as i8);
        let slide = sp * sp / 34;
        let pad = match st {
            St::Approach => {
                let dx = goal - lx;
                let closing = (dx > 0) == (sp > 0);
                if dx.abs() <= tol / 2 || (closing && dx.abs() <= slide + 1) {
                    st = St::Settle;
                    0
                } else if dx > 0 {
                    RIGHT
                } else {
                    LEFT
                }
            }
            St::Settle => {
                if !grounded || g.ram[LINK_HSPEED] != 0 {
                    0
                } else if (goal - lx).abs() > tol {
                    st = St::Approach;
                    0
                } else if g.ram[LINK_FACING] != if side < 0 { 1 } else { 2 } {
                    toward
                } else {
                    st = St::Strike(f);
                    if low {
                        DOWN
                    } else {
                        A
                    }
                }
            }
            St::Strike(s0) => {
                let k = f - s0;
                if k > 36 {
                    return None;
                }
                if low {
                    match k {
                        1 => DOWN | B,
                        2..=14 => DOWN,
                        _ => 0,
                    }
                } else {
                    let mut p = if k < 26 { A } else { 0 };
                    if k == 19 {
                        p |= B;
                    }
                    p
                }
            }
        };
        g.step(pad);
    }
    None
}

/// Play the gallery round to its end: at each decision, try a strike on
/// each live target in a throwaway copy (save state) and play the quickest
/// one that scores; with none, wait a few frames. Returns the hits scored.
fn play_gallery(g: &mut Game, rom: &[u8]) -> u8 {
    let mut sim = game(rom, opts(true));
    let mut guard = 0;
    while m::gallery_phase(g) == Some(Phase::Running) {
        guard += 1;
        assert!(guard < 2000, "gallery player stuck");
        let snap = g.save_state();
        let mut best: Option<(usize, usize)> = None;
        for (slot, _) in m::gallery_targets(g) {
            // Only worth a try when the target is close.
            if (enemy_px(g, slot) - link_px(g)).abs() > 0x70 {
                continue;
            }
            sim.load_state(&snap);
            if let Some(f) = strike_target(&mut sim, slot, 160) {
                if best.is_none_or(|b| f < b.0) {
                    best = Some((f, slot));
                }
            }
        }
        match best {
            Some((_, slot)) => {
                let hits = m::gallery_hits(g);
                strike_target(g, slot, 160);
                assert_eq!(m::gallery_hits(g), hits + 1, "replayed strike scores");
            }
            None => {
                // Nothing in reach: head for the nearest target (or the
                // middle of the room) for a few frames.
                let lx = link_px(g);
                let to = m::gallery_targets(g)
                    .iter()
                    .map(|&(s, _)| enemy_px(g, s))
                    .min_by_key(|&x| (x - lx).abs())
                    .unwrap_or(0x100);
                let pad = if (to - lx).abs() < 0x20 {
                    0
                } else if to > lx {
                    RIGHT
                } else {
                    LEFT
                };
                for _ in 0..6 {
                    top_up(g);
                    g.step(pad);
                }
            }
        }
    }
    m::gallery_hits(g)
}

/// Walk with `pad` until `f` holds (at most `max` frames).
fn walk_until(g: &mut Game, pad: u8, max: usize, f: impl Fn(&Game) -> bool) -> bool {
    for _ in 0..max {
        if f(g) {
            return true;
        }
        top_up(g);
        g.step(pad);
    }
    f(g)
}

/// Leave the room by its left exit, back onto the street.
fn leave_left(g: &mut Game) {
    assert!(
        walk_until(g, LEFT, 900, |g| content::current_room(g).is_none()),
        "left the room"
    );
    assert!(step_until(g, 600, |g| g.ram[MODE] == MODE_SIDEVIEW));
}

/// SAVE on the game-over screen, then a fresh game on that battery RAM
/// with file 1 loaded from the menu.
fn battery_round_trip(g: &mut Game, rom: &[u8]) -> Game {
    g.ram[LIVES] = 1;
    g.ram[STAGE] = 2;
    assert!(
        step_until(g, 3000, |g| g.ram[STAGE] == 2 && g.ram[MODE] == 9),
        "game-over screen"
    );
    g.ram[MENU_SEL] = 1;
    step(g, 2, START);
    step(g, 300, 0);
    let sram = *g.wram();
    let mut h = Game::from_ines(rom).expect("ROM");
    register_default_groups(&mut h);
    h.set_enhancements(opts(true));
    h.wram = sram;
    h.reset();
    step(&mut h, 30, 0);
    step(&mut h, 5, START);
    step(&mut h, 20, 0);
    step(&mut h, 5, START);
    assert!(
        step_until(&mut h, 600, |g| g.ram[MODE] == MODE_SIDEVIEW),
        "file loaded"
    );
    h
}

// ------------------------------------------------------------ gallery tests

#[test]
fn gallery_idle_player_loses_and_the_round_rearms() {
    let t = "gallery_idle_player_loses_and_the_round_rearms";
    let Some(rom) = common::rom_bytes(t) else {
        return;
    };
    let mut g = game(&rom, opts(true));
    reach_gameplay(&mut g);
    enter_minigame(&mut g, 0);
    assert_eq!(m::gallery_phase(&g), Some(Phase::Armed));
    let life = g.ram[LIFE];
    // Mid-round save state (rollback): a copy loaded from it plays on
    // exactly like the original.
    assert!(step_until(&mut g, 300, |g| !m::gallery_targets(g).is_empty()));
    let mid = g.save_state();
    let mut twin = game(&rom, opts(true));
    twin.load_state(&mid);
    for _ in 0..300 {
        g.step(RIGHT);
        twin.step(RIGHT);
    }
    assert_eq!(g.enh_state, twin.enh_state, "minigame state replays");
    assert_eq!(g.ram, twin.ram);
    g.load_state(&mid);
    // Standing still at the door: targets come and go, nothing is hit.
    assert!(
        walk_until(&mut g, 0, 2400, |g| {
            m::gallery_phase(g) == Some(Phase::Lost)
        }),
        "round over"
    );
    let seen = m::gallery_spawned(&g);
    shot(&g, "gallery-lost");
    assert!(seen > 0, "targets were spawned");
    assert_eq!(m::gallery_hits(&g), 0);
    assert!(m::gallery_clock(&g) >= mg::GALLERY_TIME_LIMIT || seen == mg::GALLERY_MAX_TARGETS);
    assert!(!m::gallery_won(&g), "no win flag");
    assert_eq!(item_objects(&g), 0, "no prize after a loss");
    assert!(m::gallery_targets(&g).is_empty(), "targets cleared");
    assert_eq!(g.ram[LIFE], life, "nothing hurt Link");
    step(&mut g, 120, 0);
    assert_eq!(item_objects(&g), 0, "still no prize");
    // Out and back in: armed again.
    leave_left(&mut g);
    take_door(&mut g, mg::ATTACH[0].door_slot);
    assert!(step_until(&mut g, 600, |g| g.ram[MODE] == MODE_SIDEVIEW
        && content::current_room(g) == Some(gallery())));
    assert_eq!(m::gallery_phase(&g), Some(Phase::Armed), "re-armed");
    assert!(step_until(&mut g, 200, |g| m::gallery_phase(g) == Some(Phase::Running)));
    assert_eq!(m::gallery_hits(&g), 0);
    assert_eq!(g.exec_errors, 0);
}

#[test]
fn gallery_is_off_without_the_option() {
    let t = "gallery_is_off_without_the_option";
    let Some(rom) = common::rom_bytes(t) else {
        return;
    };
    // Content on, minigames off: the street door is an ordinary wall.
    let mut g = game(&rom, opts(false));
    reach_gameplay(&mut g);
    let at = mg::ATTACH[0];
    assert!(load_scene(&mut g, 0, 1, 0x33, at.street, at.door_slot));
    step(&mut g, 300, 0);
    assert_eq!(content::current_room(&g), None);
    assert_eq!(m::gallery_phase(&g), None);
    assert_eq!(g.enh_state.minigame, [0; 16]);
}

#[test]
fn gallery_scripted_player_wins_and_the_prize_persists() {
    let t = "gallery_scripted_player_wins_and_the_prize_persists";
    let Some(rom) = common::rom_bytes(t) else {
        return;
    };
    let mut g = game(&rom, opts(true));
    reach_gameplay(&mut g);
    // The next level far away: any XP would show.
    g.ram[0x0770] = 0x0F;
    g.ram[0x0771] = 0xFF;
    enter_minigame(&mut g, 0);
    let id = gallery();
    assert_eq!(m::gallery_phase(&g), Some(Phase::Armed));
    assert!(!m::gallery_won(&g));
    assert!(step_until(&mut g, 200, |g| m::gallery_phase(g) == Some(Phase::Running)));
    assert_eq!(item_objects(&g), 0, "prize hidden while armed and running");
    let (life0, xp0) = (g.ram[LIFE], xp(&g));
    let hits = play_gallery(&mut g, &rom);
    eprintln!(
        "gallery: {hits} hits in {} frames, {} targets",
        m::gallery_clock(&g),
        m::gallery_spawned(&g)
    );
    assert_eq!(m::gallery_phase(&g), Some(Phase::Won));
    assert!(hits >= mg::GALLERY_HITS_NEEDED);
    assert!(m::gallery_clock(&g) <= mg::GALLERY_TIME_LIMIT);
    assert!(m::gallery_won(&g), "win flag set");
    assert_eq!(g.ram[LIFE], life0, "targets do no damage");
    assert_eq!(xp(&g), xp0, "targets give no XP");
    assert!(m::gallery_targets(&g).is_empty());
    assert_eq!(item_objects(&g), 1, "prize out after the win");
    step(&mut g, 60, 0);
    shot(&g, "gallery-won");
    assert_eq!(item_objects(&g), 1, "prize stays out");

    // Won but not taken: out and back in, the prize is there from the
    // start and no round runs.
    let won = g.save_state();
    leave_left(&mut g);
    assert!(m::gallery_won(&g));
    take_door(&mut g, mg::ATTACH[0].door_slot);
    assert!(step_until(&mut g, 600, |g| g.ram[MODE] == MODE_SIDEVIEW
        && content::current_room(g) == Some(id)));
    step(&mut g, 120, 0);
    assert_eq!(m::gallery_phase(&g), Some(Phase::Won));
    assert_eq!(item_objects(&g), 1, "prize shown on re-entry");
    assert_eq!(m::gallery_spawned(&g), 0, "no round");

    // Take it (the normal pickup): the page bit is recorded.
    g.load_state(&won);
    let prize_x = i32::from(mg::GALLERY_PRIZE.0) * 16 + 8;
    let pad = if link_px(&g) < prize_x { RIGHT } else { LEFT };
    assert!(
        walk_until(&mut g, pad, 600, |g| content::taken_pages(g, id) != 0),
        "prize taken"
    );
    assert_eq!(item_objects(&g), 0);
    step(&mut g, 60, 0);

    // Battery: the flag and the taken prize survive a save and a reload,
    // and the gallery stays won (no round, no prize).
    let mut h = battery_round_trip(&mut g, &rom);
    assert!(m::gallery_won(&h), "win flag reloaded");
    assert_ne!(content::taken_pages(&h, id), 0, "prize still taken");
    step(&mut h, 120, 0);
    enter_minigame(&mut h, 0);
    step(&mut h, 200, 0);
    assert_eq!(m::gallery_phase(&h), Some(Phase::Won));
    assert_eq!(m::gallery_spawned(&h), 0, "won gallery is not re-armed");
    assert_eq!(item_objects(&h), 0, "no second prize");
    assert_eq!(g.exec_errors + h.exec_errors, 0);
}

// ------------------------------------------------------------ tower climber

const VSPEED: usize = 0x057D;

/// Room-pixel X (centre) of bouncer spot `k`.
fn spot_px(k: usize) -> i32 {
    i32::from(mg::BOUNCER_SPOTS[k].0) * 16 + 8
}

/// Link stands on the floor (row 9).
fn on_floor(g: &Game) -> bool {
    g.ram[AIRBORNE] == 0 && g.ram[LINK_Y] >= 0xA0
}

/// Climb one flight: from the floor, jump at the chain's first bouncer,
/// hold the down stab while falling onto each bouncer in turn (steering
/// toward the next one), then steer onto the landing at room X `land`.
/// Retries from the floor after a miss. Returns whether Link stands on
/// something above the floor near `land` within `max` frames.
fn climb_flight(g: &mut Game, chain: &[usize], land: i32, max: usize) -> bool {
    let mut next = 0usize;
    let mut jump_hold = 0;
    let mut prev_vy = 0u8;
    for _ in 0..max {
        top_up(g);
        let lx = link_px(g);
        let grounded = g.ram[AIRBORNE] == 0;
        if grounded && !on_floor(g) && (lx - land).abs() < 0x30 {
            return true;
        }
        if on_floor(g) {
            next = 0;
        }
        let vy = g.ram[VSPEED];
        let pad = if grounded && jump_hold == 0 {
            // Line up left of the first bouncer, then jump toward it.
            let goal = spot_px(chain[0]) - 0x18;
            if (lx - goal).abs() <= 3 {
                jump_hold = 22;
                A | RIGHT
            } else if lx < goal {
                RIGHT
            } else {
                LEFT
            }
        } else {
            let tx = if next < chain.len() {
                spot_px(chain[next])
            } else {
                land
            };
            let mut p = 0;
            if jump_hold > 0 {
                jump_hold -= 1;
                p |= A;
            }
            if lx < tx - 2 {
                p |= RIGHT;
            } else if lx > tx + 2 {
                p |= LEFT;
            }
            if (vy as i8) >= 0 && next < chain.len() {
                p |= DOWN;
            }
            p
        };
        g.step(pad);
        let vy_now = g.ram[VSPEED];
        if vy_now == m::TOWER_BOUNCE_VSPEED && prev_vy != m::TOWER_BOUNCE_VSPEED {
            next += 1;
            jump_hold = 0;
        }
        prev_vy = vy_now;
    }
    false
}

/// The whole climb: flight 1 to the wall top, down into the trough,
/// flight 2 to the plateau. Returns whether Link stands on the plateau.
fn climb_tower(g: &mut Game, max: usize) -> bool {
    let wall = i32::from(mg::BOUNCER_SPOTS[2].0) * 16 + 0x38;
    if !climb_flight(g, &[0, 1, 2], wall, max) {
        return false;
    }
    // Off the wall top's right edge into the trough.
    if !walk_until(g, RIGHT, 300, on_floor) {
        return false;
    }
    let plateau = 39 * 16 + 8;
    climb_flight(g, &[6, 7, 8], plateau, max) && on_plateau(g)
}

fn on_plateau(g: &Game) -> bool {
    g.ram[AIRBORNE] == 0 && link_px(g) >= 38 * 16 && g.ram[LINK_Y] < 0xA0
}

// ------------------------------------------------------------ tower tests

#[test]
fn tower_is_climbed_with_chained_down_stabs() {
    let t = "tower_is_climbed_with_chained_down_stabs";
    let Some(rom) = common::rom_bytes(t) else {
        return;
    };
    let mut g = game(&rom, opts(true));
    reach_gameplay(&mut g);
    g.ram[THRUSTS] |= DOWN_THRUST;
    g.ram[0x0770] = 0x0F;
    g.ram[0x0771] = 0xFF;
    enter_minigame(&mut g, 1);
    let id = tower();
    let (life0, xp0) = (g.ram[LIFE], xp(&g));
    // Bouncers held on their spots.
    step(&mut g, 30, 0);
    let held: Vec<(usize, i32)> = (0..6)
        .filter(|&s| g.ram[0xB6 + s] != 0 && g.ram[0xA1 + s] == mg::BOUNCER)
        .map(|s| (s, enemy_px(&g, s)))
        .collect();
    assert!(held.len() >= 3, "bouncers up: {held:?}");
    shot(&g, "tower-flight1");
    // Drawn without the cross (the East towns hide Moas otherwise): OAM
    // has sprites at the row-7 and row-5 bouncers' heights (Link stands
    // lower, on the floor).
    assert_eq!(g.ram[0x078A], 0, "no cross");
    let oam = *g.oam();
    for y in [0x90u8, 0x70] {
        assert!(
            oam.chunks(4).any(|e| e[0].abs_diff(y.wrapping_sub(1)) <= 3),
            "a bouncer sprite near Y {y:#x}"
        );
    }
    step(&mut g, 30, 0);
    for &(s, x) in &held {
        assert_eq!(enemy_px(&g, s), x, "bouncer {s} pinned");
    }
    let snap = g.save_state();
    assert!(climb_tower(&mut g, 3000), "plateau reached");
    shot(&g, "tower-plateau");
    assert_eq!(g.ram[LIFE], life0, "bouncers do no damage");
    assert_eq!(xp(&g), xp0, "bouncers give no XP");
    assert!(
        walk_until(&mut g, RIGHT, 600, |g| content::taken_pages(g, id) != 0),
        "prize taken"
    );
    assert_eq!(g.exec_errors, 0);

    // The same climber without the downward thrust never gets up.
    let mut n = game(&rom, opts(true));
    n.load_state(&snap);
    n.ram[THRUSTS] &= !DOWN_THRUST;
    assert!(!climb_tower(&mut n, 3000), "no plateau without the thrust");
    assert!(!on_plateau(&n));
    assert_eq!(content::taken_pages(&n, id), 0);
}
