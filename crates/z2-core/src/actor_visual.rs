//! Read-only enemy, NPC and boss observations for whole-image HD actors.
//!
//! Companion of [`crate::player_visual`]: each live object slot becomes one
//! [`ActorVisual`] (character key, clip, ground anchor, facing, OAM mask) that
//! a renderer can swap for a bone-animated image. Nothing here writes RAM,
//! CPU state, the mapper or OAM; gameplay and collision stay the cartridge's.
//!
//! # Character keys
//!
//! `$A1,x` (enemy code) only names a type within the enemy tables the area
//! load copies into WRAM (`bank7_Transfer_2A1_bytes_from_9400_or_A900_if_World_4_to_6D00`,
//! bank 7 `$CE77`): `$9400` of the area's PRG bank `$0769`, or `$A900` of
//! bank 4 when the world `$0707` is 4. The key is therefore
//! `<prefix>_<group>_<id as two hex digits>`:
//!
//! | group | `$0707` | `$0769` | tables | prefix |
//! |---|---|---|---|---|
//! | `b1` | 0 | 1 | West Hyrule, Death Mountain | `enemy` |
//! | `b2` | 0 | 2 | East Hyrule, Maze Island | `enemy` |
//! | `b3` | 1, 2 | 3 | towns | `npc` |
//! | `b4` | 3 | 4 | palaces 1, 2, 5 (`$9400`) | `enemy` |
//! | `b4b` | 4 | 4 | palaces 3, 4, 6 (`$A900`) | `enemy` |
//! | `b5` | 5 | 5 | Great Palace | `enemy` |
//!
//! e.g. `enemy_b1_14` (orange Moblin), `enemy_b4_20` (Horsehead),
//! `npc_b3_0f` (wise man). [`alias`] gives the human name and class.
//!
//! Where one id draws different characters, the key gets a qualifier
//! ([`qualifier`]) and the plain key stays the fallback
//! ([`ActorVisual::base_character`]): `enemy_b4_21_p2` (Helmethead) and
//! `enemy_b4_21_p5` (Gooma) by palace, and every townsperson by the
//! appearance index the town's table assigns that id (`$6DD5,id`), e.g.
//! `npc_b3_1d_34` (child), `npc_b3_1d_2c` (townsman), `npc_b3_1d_10`
//! (bearded man).
//!
//! # Displayed state
//!
//! The NMI copies OAM before the frame's logic runs, so the picture on screen
//! was drawn from the state the previous frame's logic left. Positions,
//! slot states and the clip therefore come from [`Game::shown_ram`] (RAM at
//! that OAM DMA) and the displayed OAM ([`Game::oam`]), not from the RAM the
//! logic has already moved. Each slot's block is the one its display routine
//! wrote: `$91,x` (OAM byte offset, rotated every frame through `LD58F` by
//! `$0485`, `bank7_code21` `$D5A7`).
//!
//! # Anchor
//!
//! `bank7_Display` (`$EF11`) hands each type's display routine `$00 = $2A,x`
//! and `$01 = $CD` (screen X), and the shared drawers `LF216`/`LF1F4`
//! (`$F216`/`$F1F4`) put each 16-pixel row as two 8x16 cells at `$01` and
//! `$01 + 8`, then step `$00` by 16 and the OAM pointer by two entries.
//!
//! The ground anchor is `[x0 + 8, y0 + 16 * rows]`: the centre of the
//! two-column body and the last displayed row of its bottom cell row (OAM Y
//! + 16, the same convention as Link's `[$CC + 16, $29 + 32]`). `rows` is the
//! type's body height ([`known_rows`]) even when the bottom cells are not
//! drawn this frame (clipped, or the death puff). Cells that do not sit on
//! that grid (a raised spear, an Iron Knuckle's blade) are still replaced but
//! do not move the anchor. Cells drawn with a blank pattern (the cartridge's
//! `$F5` "nothing" tile: teleport and edge-clipping frames), cells far from
//! the slot's own position (another slot's leftovers) and cells a type draws
//! beside its body (the Deeler's thread, [`body_entries`]) are never part of
//! a slot. Where no cell fits the grid, and for every [`ActorClass::Object`]
//! (doors, jars, the elevator), the anchor is the bottom centre of the
//! visible cells ([`AnchorRule::Bbox`]). Carock multiplexes two blocks on
//! alternate frames; its anchor X is the centre of its collision box
//! ([`AnchorRule::Box`]).
use crate::game::Game;

/// First OAM entry of the six enemy blocks (`LD58F`: `$50..$C8` / 4).
pub const ENEMY_OAM_FIRST: u8 = 0x14;
/// OAM entries per enemy slot (`$18` bytes between `LD58F` offsets).
pub const ENEMY_OAM_ENTRIES: usize = 6;
/// Renderer clock id of enemy slot 0 (players use 0 and 1).
pub const ENEMY_ID_BASE: u8 = 8;
/// One past the last enemy OAM entry.
const ENEMY_OAM_END: usize = ENEMY_OAM_FIRST as usize + 6 * ENEMY_OAM_ENTRIES;

/// Clip-selection family, from the type's display/AI routine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub enum ActorClass {
    /// Ground walker: `idle`, `walk`, `jump`.
    Walker,
    /// Hopping walker: as [`ActorClass::Walker`], hops are `jump`.
    Jumper,
    /// Throws a projectile while `$81,x != 0`: adds `throw`.
    Thrower,
    /// Melee swing while `$81,x != 0`: adds `attack`.
    Swordsman,
    /// Spell or energy shot while `$81,x != 0`: adds `cast`.
    Caster,
    /// Always airborne: `fly` (Ache and Acheman also `perch`, `land`).
    Flyer,
    /// Item, door, elevator, statue: `idle`.
    Object,
    /// Townsperson: `idle`, `walk`.
    Npc,
    /// Boss: `idle`, `walk`, `attack`, `jump`.
    Boss,
    /// Draws nothing (generators, invisible dialogue triggers, hidden items):
    /// never reported, never matched to OAM.
    Hidden,
}

/// How [`ActorVisual::position`] was measured.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub enum AnchorRule {
    /// From the `LF216` two-column grid (see the module docs).
    Pairs,
    /// Bottom centre of the visible cells (no cell fits the grid).
    Bbox,
    /// X from the centre of the slot's collision box; Y from the grid
    /// (Carock, whose body alternates between two blocks) or the box's
    /// bottom edge (Gooma, whose 3x3 body straddles two blocks).
    Box,
    /// Centre and bottom of the topmost cell row: Barba's head, whose neck
    /// below it grows and shrinks as he rises from the lava.
    Head,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct ActorVisual {
    /// Renderer clock id, [`ENEMY_ID_BASE`] + slot.
    pub id: u8,
    /// Enemy slot `x` (0-5).
    pub slot: u8,
    /// `$A1,x`.
    pub type_id: u8,
    /// Table group (`b1`, `b2`, `b3`, `b4`, `b4b`, `b5`).
    pub group: &'static str,
    /// The pack key to look up first: [`ActorVisual::base_character`] plus
    /// a [`qualifier`] where the id is ambiguous (`enemy_b4_21_p5`,
    /// `npc_b3_1d_34`), else the base key itself.
    pub character: String,
    /// `enemy_<group>_<id>` or `npc_b3_<id>`: the fallback when the pack has
    /// no clips for [`ActorVisual::character`].
    pub base_character: String,
    /// Human name (`?` where the disassembly has none).
    pub alias: &'static str,
    pub class: ActorClass,
    pub clip: &'static str,
    /// The older clip name to use when the pack has no `clip` for this
    /// character (`attack_high` -> `attack`, `perch` -> `fly`).
    pub clip_fallback: Option<&'static str>,
    /// A new throw/attack cycle started on this displayed frame while the
    /// clip stayed the same (chained swings, Guma's balls): restart the clip.
    pub restart: bool,
    /// The cartridge is flashing this slot (hit flash, `$040E,x`): the OAM
    /// palette (0-3) it drew with instead of the type's own.
    pub flash_palette: Option<u8>,
    /// Ground anchor in NES screen pixels (displayed OAM).
    pub position: [f32; 2],
    /// Mirror the clip (authored facing right): the cartridge drew the
    /// first cell without its horizontal-flip bit.
    pub flip_h: bool,
    /// The visible OAM entries this actor replaces.
    pub oam_mask: u64,
    /// The widescreen margin sprites this actor replaces: bit `i` is entry
    /// `i` of [`Game::margin_sprites_into`] (the side-view entries come
    /// first). These are the cells of columns the cartridge hid in OAM
    /// because they fell outside the 256-pixel window.
    pub margin_mask: u64,
    /// Lowest OAM entry the actor was drawn from (window cells and margin
    /// cells alike: a margin cell keeps the entry the cartridge hid).
    pub priority_oam: u8,
    /// Visible cells `[x, y, w, h]` in displayed pixels (y is OAM Y + 1).
    pub oam_bbox: [i32; 4],
    /// Cell rows on the anchor grid (0 for [`AnchorRule::Bbox`]).
    pub rows: u8,
    pub rule: AnchorRule,
}

/// The enemy-table group of the loaded side-view area.
#[must_use]
pub fn group(world: u8, prg_bank: u8) -> Option<&'static str> {
    Some(match world {
        0 if prg_bank == 2 => "b2",
        0 => "b1",
        1 | 2 => "b3",
        3 => "b4",
        4 => "b4b",
        5 => "b5",
        _ => return None,
    })
}

/// The pack character key for type `id` in `group`.
#[must_use]
pub fn character_key(group: &str, id: u8) -> String {
    let prefix = if group == "b3" { "npc" } else { "enemy" };
    format!("{prefix}_{group}_{id:02x}")
}

/// Palace number 1-7 from the overworld region `$0706` and the area's
/// palace index `$056C` (West Hyrule holds palaces 1-3, Death Mountain and
/// Maze Island palace 4, East Hyrule palaces 5, 6 and the Great Palace).
#[must_use]
pub fn palace(region: u8, index: u8) -> Option<u8> {
    Some(match (region, index) {
        (0, i @ 0..=2) => i + 1,
        (1, 0) => 4,
        (2, i @ 0..=2) => i + 5,
        _ => return None,
    })
}

/// Key qualifier for ids that draw different characters: `p<palace>` for
/// the palace 2/5 boss id `enemy_b4_21` (Helmethead, Gooma), and for every
/// townsperson the appearance index `$6DD5,id` the town's table assigns
/// (the display routine draws tiles `$6ED4 + index`), as two hex digits.
/// `get` reads RAM, `wram` reads `$6000`-based WRAM offsets.
#[must_use]
pub fn qualifier(
    group: &str,
    id: u8,
    class: ActorClass,
    get: impl Fn(usize) -> u8,
    wram: impl Fn(usize) -> u8,
) -> Option<String> {
    match (group, id, class) {
        ("b4", 0x21, _) => palace(get(0x0706), get(0x056C)).map(|p| format!("p{p}")),
        ("b3", _, ActorClass::Npc) => Some(format!("{:02x}", wram(0x0DD5 + usize::from(id)))),
        _ => None,
    }
}

/// Human name and clip family. Names follow the per-bank display pointer
/// tables of the disassembly (`bank1/2/3/5_Pointer_table_for_Enemy_Routines*`,
/// `bank4_Pointer_table_for_Enemy_Routines0/1` for palaces 1, 2, 5); palaces
/// 3, 4, 6 reuse bank 4's code for most ids, and their unnamed ids are `?`.
#[must_use]
pub fn alias(group: &str, id: u8) -> (&'static str, ActorClass) {
    let (name, class) = named(group, id);
    let hidden =
        name.ends_with("_generator") || name.starts_with("invisible") || name.starts_with("hidden");
    (name, if hidden { ActorClass::Hidden } else { class })
}

fn named(group: &str, id: u8) -> (&'static str, ActorClass) {
    use ActorClass::*;
    let shared = |id: u8| -> (&'static str, ActorClass) {
        match id {
            0x00 => ("fairy", Object),
            0x01 => ("red_jar", Object),
            0x02 => ("locked_door", Object),
            0x03 => ("myu", Walker),
            0x04 => ("bot", Jumper),
            0x13 => ("elevator", Object),
            _ => ("?", Walker),
        }
    };
    match group {
        "b1" | "b2" => {
            let common = match id {
                0x05 => ("bit", Walker),
                0x06 => ("moa", Flyer),
                0x07 => ("ache", Flyer),
                0x0A => ("acheman", Flyer),
                0x0B => ("bubble_generator", Object),
                0x0C => ("rock_generator", Object),
                0x0D | 0x0E => ("deeler", Flyer),
                0x0F => ("bago_bago_generator", Object),
                0x10 => ("bago_bago", Flyer),
                0x11 => ("octorok_jumping", Thrower),
                0x12 => ("octorok", Thrower),
                _ => ("", Walker),
            };
            if !common.0.is_empty() {
                return common;
            }
            if group == "b1" {
                match id {
                    0x14 => ("moblin_orange", Thrower),
                    0x15 => ("moblin_red", Thrower),
                    0x16 => ("moblin_blue", Thrower),
                    0x17 => ("daira_orange", Swordsman),
                    0x18 => ("daira_red", Thrower),
                    0x19 => ("goriya_orange", Thrower),
                    0x1A => ("goriya_red", Thrower),
                    0x1B => ("goriya_blue", Thrower),
                    0x1C => ("lowder", Walker),
                    0x1D => ("moby_generator", Object),
                    0x1E => ("moby", Flyer),
                    0x1F => ("megmat", Jumper),
                    0x20 => ("geldarm", Walker),
                    0x21 => ("dumb_moblin_generator", Object),
                    0x22 => ("dumb_moblin", Walker),
                    _ => shared(id),
                }
            } else {
                match id {
                    0x14 => ("tektite", Jumper),
                    0x15 => ("girubokku", Flyer),
                    0x16 => ("leever", Jumper),
                    0x17 => ("boon", Flyer),
                    0x18 => ("zora", Thrower),
                    0x19 => ("aru_lowder", Walker),
                    0x1A => ("lizalfos_red", Thrower),
                    0x1B => ("lizalfos_orange", Swordsman),
                    0x1C => ("lizalfos_blue", Thrower),
                    0x1D => ("lizalfos_rock_tossing", Thrower),
                    _ => shared(id),
                }
            }
        }
        "b3" => match id {
            0x03 | 0x0A => ("ache", Flyer),
            0x04 | 0x0B => ("bit", Walker),
            0x05 => ("bot", Jumper),
            0x06 => ("moa", Flyer),
            0x07 => ("acheman", Flyer),
            0x08 | 0x09 => ("orange_lady", Npc),
            0x0C | 0x21 | 0x22 => ("invisible_dialog", Object),
            0x0D | 0x15 => ("blue_old_woman", Npc),
            0x0E | 0x19 | 0x1D => ("blue_lumberjack", Npc),
            0x0F => ("wise_man", Npc),
            0x10 => ("little_kid", Npc),
            0x11 | 0x14 => ("blue_lady_bowl", Npc),
            0x12 | 0x1B => ("red_old_woman", Npc),
            0x13 => ("red_lumberjack", Npc),
            0x16 => ("red_lady", Npc),
            0x17 => ("healing_lady", Npc),
            0x18 => ("magic_woman", Npc),
            0x1A => ("red_lady_bowl", Npc),
            0x1C | 0x20 => ("blue_lady", Npc),
            0x1E | 0x1F => ("red_young_lad", Npc),
            0x23 => ("townfolk_generator", Object),
            _ => match shared(id) {
                ("?", _) => ("?", Npc),
                known => known,
            },
        },
        "b4" | "b4b" => {
            let a = group == "b4";
            match id {
                0x05 => ("hidden_red_jar", Object),
                0x06 | 0x0E => ("bubble", Flyer),
                0x07 => ("moa", Flyer),
                0x08 => ("falling_block_generator", Object),
                0x09 => ("falling_block", Object),
                0x0A if a => ("ra", Flyer),
                // Palace 3's boss after losing its horse thrusts like an
                // Iron Knuckle (`$81,x` 1-4, same blade tiles).
                0x0A => ("rebonack_unhorsed", Swordsman),
                0x0B if a => ("tinsuit_generator", Object),
                0x0C => ("tinsuit", Jumper),
                0x0D => ("dripping_column", Object),
                0x0F if a => ("bago_bago_generator", Object),
                0x10 if a => ("bago_bago", Flyer),
                0x11 | 0x12 => ("rope", Thrower),
                0x14 => ("crystal_slot", Object),
                0x15 => ("crystal", Object),
                0x16 | 0x17 => ("energy_ball_shooter", Caster),
                0x18 => ("iron_knuckle_orange", Swordsman),
                0x19 => ("iron_knuckle_red", Swordsman),
                0x1A => ("iron_knuckle_blue", Swordsman),
                0x1B => ("mau_generator", Object),
                0x1C => ("mau", Jumper),
                0x1D if a => ("mago", Caster),
                0x1D => ("wizzrobe", Caster),
                0x1E if a => ("guma", Thrower),
                0x1E => ("doomknocker", Thrower),
                0x1F => ("stalfos", Swordsman),
                0x20 if a => ("horsehead", Boss),
                // Palace 2's Helmethead and palace 5's Gooma share this id;
                // `qualifier` tells them apart.
                0x21 if a => ("helmethead_gooma", Boss),
                0x22 if a => ("floating_helmet", Flyer),
                0x20 => ("rebonack", Boss),
                0x21 => ("barba", Boss),
                0x22 => ("carock", Boss),
                0x23 => ("stalfos_blue", Swordsman),
                _ => shared(id),
            }
        }
        "b5" => match id {
            0x05 => ("hidden_red_jar", Object),
            0x06 => ("moa", Flyer),
            0x07 => ("ache", Flyer),
            0x0A => ("acheman", Flyer),
            0x0D | 0x0E => ("deeler", Flyer),
            0x10 => ("fire_bago_bago", Flyer),
            0x11 => ("fire_rope_jumping", Thrower),
            0x12 => ("fire_rope", Thrower),
            0x14 => ("bubble_slow", Flyer),
            0x15 => ("bubble_fast", Flyer),
            0x16 => ("ra", Flyer),
            0x17 => ("big_bubble", Flyer),
            0x18 => ("fokka_orange", Swordsman),
            0x19 => ("fokka_red", Swordsman),
            0x1A => ("fokka_blue", Swordsman),
            0x1D => ("fokkeru", Caster),
            0x1E => ("giant_bot", Jumper),
            0x20 => ("electric_barrier", Object),
            0x22 => ("thunderbird", Boss),
            0x23 => ("dark_link", Boss),
            _ => shared(id),
        },
        _ => shared(id),
    }
}

/// Body height in 16-pixel cell rows for types whose display routine always
/// draws the same number of rows (measured over the gate captures; `None`
/// where a type changes height, e.g. Acheman's bat and devil forms, or is
/// unmeasured). The anchor uses it when the bottom row is not drawn.
#[must_use]
pub fn known_rows(group: &str, id: u8) -> Option<u8> {
    let shared = |id: u8| match id {
        0x03 | 0x04 => Some(1),
        _ => None,
    };
    match group {
        "b1" | "b2" => match id {
            0x05 | 0x06 | 0x07 | 0x0D | 0x0E | 0x10 | 0x11 | 0x12 => Some(1),
            _ if group == "b1" => match id {
                0x14..=0x1B | 0x22 => Some(2),
                0x1C | 0x1E | 0x1F | 0x20 => Some(1),
                _ => shared(id),
            },
            0x15 | 0x16 | 0x17 | 0x1D => Some(1),
            0x19..=0x1C => Some(2),
            _ => shared(id),
        },
        "b3" => match id {
            0x03..=0x06 | 0x0A | 0x0B => Some(1),
            0x08..=0x20 if alias(group, id).1 == ActorClass::Npc => Some(2),
            _ => None,
        },
        "b4" | "b4b" => {
            let a = group == "b4";
            match id {
                0x06 | 0x07 | 0x0E | 0x11 | 0x12 | 0x1C => Some(1),
                0x0A | 0x10 if a => Some(1),
                0x0A => Some(2),
                0x0C | 0x18..=0x1A | 0x1E | 0x1F | 0x23 => Some(2),
                0x20 if a => Some(3),
                0x20 => Some(2),
                0x22 if !a => Some(3),
                _ => shared(id),
            }
        }
        "b5" => match id {
            0x06 | 0x07 | 0x0D | 0x0E | 0x10..=0x12 | 0x14..=0x16 => Some(1),
            0x17..=0x1A | 0x1D | 0x23 => Some(2),
            0x22 => Some(3),
            _ => shared(id),
        },
        _ => shared(id),
    }
}

/// Which of a slot's six block entries belong to its body. The Deeler draws
/// its silk thread as entries 2-4 of its own block (`bank7_Enemy_Routines2_Deeler`,
/// `$F079`); those stay original sprites.
#[must_use]
pub fn body_entries(group: &str, id: u8) -> u8 {
    match (group, id) {
        ("b1" | "b2" | "b5", 0x0D | 0x0E) => 0b11,
        _ => 0x3F,
    }
}

/// Cells per 16-pixel row of the type's drawer: 3 for Dark Link, who is
/// drawn like Link (front column for the sword, two body columns), else 2.
#[must_use]
pub fn columns(group: &str, id: u8) -> usize {
    match (group, id) {
        ("b5", 0x23) => 3,
        _ => 2,
    }
}

/// Types drawn with a fixed OAM layout of their own rather than the slot's
/// rotated block (`$91,x`): every boss, and the Great Palace's King Bot,
/// whose 6x2 body always sits at entries 20-31. Their cells are found by
/// distance and claimed cell by cell, across block boundaries.
#[must_use]
pub fn fixed_layout(group: &str, id: u8, class: ActorClass) -> bool {
    class == ActorClass::Boss || (group, id) == ("b5", 0x1E)
}

/// Types drawn as one 8x16 sprite whose `$91,x` does not name their own
/// block: Helmethead's flying helmets (`enemy_b4_21`'s heads, id `0x22`),
/// whose OAM shares his six-entry group. Each is matched to its own nearest
/// sprite and anchored at its bottom centre.
#[must_use]
pub fn single_sprite(group: &str, id: u8) -> bool {
    (group, id) == ("b4", 0x22)
}

/// Farthest a single-sprite slot's cell may be from `($CD + 3, $2A,x)`
/// (a frame of flight plus the display's own offset).
const SINGLE_DISTANCE: i32 = 16;

/// Clip for one slot, from the cartridge's own state bytes:
/// `$B6,x == 2` dying (`bank7_monster_death`, `$E880`), `$040E,x` hit
/// stun (`$E726`), `$81,x` the display routines' animation frame (non-zero
/// is the throw/strike/cast pose, e.g. `bank1_Enemy_Routines2_Moblin`
/// `$9CA8`), `$057E,x` Y velocity (airborne), `$71,x` X velocity.
/// [`clip_for`] adds the per-type states on top.
#[must_use]
pub fn clip(class: ActorClass, get: impl Fn(usize) -> u8, slot: usize) -> &'static str {
    use ActorClass::*;
    if get(0x00B6 + slot) == 2 {
        return "death";
    }
    if get(0x040E + slot) != 0 {
        return "hurt";
    }
    let action = get(0x0081 + slot) != 0;
    let airborne = get(0x057E + slot) != 0;
    let moving = get(0x0071 + slot) != 0;
    match class {
        Flyer => return "fly",
        Object | Hidden => return "idle",
        Thrower if action => return "throw",
        Swordsman | Boss if action => return "attack",
        Caster if action => return "cast",
        Npc => return if moving { "walk" } else { "idle" },
        _ => {}
    }
    if airborne {
        "jump"
    } else if moving {
        "walk"
    } else {
        "idle"
    }
}

/// Types driven by the Iron Knuckle thrust code (`bank4_Enemy_Routines_Iron_Knuckle`,
/// `$9C8C`): `$81,x` 1-2 is a thrust at the top row, 3-4 at the second row.
fn thrusts_high_low(group: &str, id: u8) -> bool {
    matches!(
        (group, id),
        ("b4" | "b4b" | "b5", 0x18..=0x1A) | ("b4b", 0x0A)
    )
}

fn is_ache(group: &str, id: u8) -> bool {
    matches!(
        (group, id),
        ("b1" | "b2" | "b5", 0x07) | ("b3", 0x03 | 0x0A)
    )
}

fn is_acheman(group: &str, id: u8) -> bool {
    matches!((group, id), ("b1" | "b2" | "b5", 0x0A) | ("b3", 0x07))
}

/// The clip and its fallback for one slot. `now` reads the displayed RAM
/// ([`Game::shown_ram`]), `prev` the one before it, `wram` reads WRAM
/// offsets from `$6000`. On top of [`clip`]:
///
/// * Ache/Acheman hanging from the ceiling (`$81,x = 0`, drawn upside down,
///   `bank7_Ache` `$DB79`): `perch`, fallback `fly`. Acheman's devil form
///   (`$AF,x != 0`, `$DB5C`): `land`, fallback `fly`.
/// * Iron Knuckles, Fokkas and the unhorsed Rebonack: `attack_high`
///   (`$81,x` 1-2) and `attack_low` (3-4), fallback `attack`. Their display
///   runs before the thrust code, so the displayed pose is one frame behind
///   `$81,x`: the previous displayed value counts while the current one is 0.
/// * The Zora surfaced (`$0504,x` non-zero on this and the previous
///   displayed frame, drawn upright in two rows):
///   `surface` (fallback `idle`), and `surface_throw` (fallback `throw`)
///   while it readies a shot.
/// * Townspeople walk only while the display routine animates them
///   (`bank3_Enemy_Routines1_ManyNPC`, `$9AE8`): not for immobile types
///   (`$6DF9,id` bit 7), not while talking (`$05C3,x`), not during the
///   `$AF,x` 1-$1F pose, and only with `$71,x != 0`. Otherwise `idle`
///   (fallback `walk`, the clip older observers reported).
#[must_use]
pub fn clip_for(
    group: &str,
    id: u8,
    class: ActorClass,
    now: impl Fn(usize) -> u8,
    prev: impl Fn(usize) -> u8,
    wram: impl Fn(usize) -> u8,
    slot: usize,
) -> (&'static str, Option<&'static str>) {
    if now(0x00B6 + slot) == 2 {
        return ("death", None);
    }
    if now(0x040E + slot) != 0 {
        return ("hurt", None);
    }
    let frame = now(0x0081 + slot);
    if is_ache(group, id) || is_acheman(group, id) {
        if is_acheman(group, id) && now(0x00AF + slot) != 0 {
            return ("land", Some("fly"));
        }
        return if frame == 0 {
            ("perch", Some("fly"))
        } else {
            ("fly", None)
        };
    }
    if thrusts_high_low(group, id) {
        let shown = if frame == 0 {
            prev(0x0081 + slot)
        } else {
            frame
        };
        match shown {
            1 | 2 => return ("attack_high", Some("attack")),
            3 | 4 => return ("attack_low", Some("attack")),
            _ => {}
        }
        return (
            clip(class, |a| if a == 0x81 + slot { 0 } else { now(a) }, slot),
            None,
        );
    }
    if (group, id) == ("b2", 0x18) && now(0x0504 + slot) != 0 && prev(0x0504 + slot) != 0 {
        // Zora surfaced: while its `$0504,x` stay timer runs the display
        // (`bank2_Enemy_Routines2_Zora`, `$9E1A`) draws the upright two-row
        // body instead of the low swimming row, and the AI makes it
        // vulnerable (`$9A3F`). The display runs before the AI sets the
        // timer (`$9A6A`), so the pose on screen follows the timer one
        // displayed frame late: upright while both copies are non-zero.
        // `$81,x` counts down to the shot (`$9A70`).
        return if frame != 0 {
            ("surface_throw", Some("throw"))
        } else {
            ("surface", Some("idle"))
        };
    }
    if (group, id) == ("b4b", 0x1E) && now(0x0504 + slot) != 0 {
        // Doomknocker holds its mace up while the `$0504,x` cycle timer
        // runs and lets go at 1 (`LB0AD`, display `LAF26`); `$81,x` stays 0.
        return ("throw", None);
    }
    if class == ActorClass::Npc {
        let af = now(0x00AF + slot);
        let animates = wram(0x0DF9 + usize::from(id)) & 0x80 == 0
            && af < 0x40
            && now(0x05C3 + slot) & 0x7F == 0
            && (af == 0 || af >= 0x20)
            && now(0x0071 + slot) != 0;
        return if animates {
            ("walk", None)
        } else {
            ("idle", Some("walk"))
        };
    }
    (clip(class, now, slot), None)
}

/// Whether a new throw/attack cycle started between the previous displayed
/// frame (`prev`) and this one (`now`), for types whose clip name does not
/// change between cycles: the Daira's swings and the rock-tossing Lizalfos
/// (`$81,x` re-enters 1, `bank1` `$9A82`, `bank2` `$9730`), and Guma's and
/// Doomknocker's balls (the expired `$0504,x` cycle timer is reloaded,
/// `bank4` `$BC23`, `$B0BE`).
#[must_use]
pub fn cycle_restart(
    group: &str,
    id: u8,
    now: impl Fn(usize) -> u8,
    prev: impl Fn(usize) -> u8,
    slot: usize,
) -> bool {
    match (group, id) {
        ("b1", 0x17 | 0x18) | ("b2", 0x1D) => now(0x0081 + slot) == 1 && prev(0x0081 + slot) != 1,
        // A reload from an expired timer: Doomknocker reloads it on every
        // airborne frame of the same cycle, so 15 -> 16 is not a new one.
        ("b4", 0x1E) | ("b4b", 0x1E) => {
            now(0x0504 + slot) > prev(0x0504 + slot) && prev(0x0504 + slot) <= 1
        }
        _ => false,
    }
}

/// What [`measure_block`] reads off one block:
/// `(position, flip_h, visible mask bits 0-5, bbox, rows, rule)`.
pub type BlockMeasure = ([f32; 2], bool, u8, [i32; 4], u8, AnchorRule);

/// One visible cell of a block: `(entry 0-5, x, OAM y, attributes)`.
type Cell = (usize, i32, i32, u8);

/// Grid measurement: the [`BlockMeasure`] plus the grid origin `(x0, y0)`.
struct Measured {
    position: [f32; 2],
    flip_h: bool,
    mask: u8,
    bbox: [i32; 4],
    rows: u8,
    rule: AnchorRule,
    origin: (i32, i32),
    /// Every visible cell sits on the chosen grid.
    all_on_grid: bool,
}

/// Anchor of one displayed OAM block (six entries, 24 bytes).
/// Returns `(position, flip_h, visible mask bits 0-5, bbox, rows, rule)`.
///
/// `flip_h` reads the horizontal-flip bit of the left-column cells (the
/// right column of a symmetric type such as the Bot is always mirrored);
/// `facing_left` decides when no left-column cell is visible.
#[must_use]
pub fn measure_block(block: &[u8], facing_left: bool) -> Option<BlockMeasure> {
    let cells: Vec<Cell> = block
        .chunks(4)
        .take(ENEMY_OAM_ENTRIES)
        .enumerate()
        .filter(|(_, c)| c[0] < 0xEF)
        .map(|(i, c)| (i, i32::from(c[3]), i32::from(c[0]), c[2]))
        .collect();
    let m = measure_cells(&cells, facing_left, None)?;
    Some((m.position, m.flip_h, m.mask, m.bbox, m.rows, m.rule))
}

/// The grid that most visible cells sit on: entry `i` of an `LF216` block
/// sits at `(x0 + 8 (i % 2), y0 + 16 (i / 2))`. Ties go to the grid of the
/// lowest entry (the drawers start there), then to the one nearest `hint`
/// (the slot's displayed `($CD, $2A,x)`).
fn measure_cells(cells: &[Cell], facing_left: bool, hint: Option<(i32, i32)>) -> Option<Measured> {
    measure_grid(cells, facing_left, hint, 2)
}

/// [`measure_cells`] for a drawer that writes `cols` cells per 16-pixel row.
/// Three columns is the Link-style layout (Dark Link): a front column for
/// the sword or shield and two body columns behind it, so the body centre is
/// `x0 + 16` facing left and `x0 + 8` facing right.
fn measure_grid(
    cells: &[Cell],
    facing_left: bool,
    hint: Option<(i32, i32)>,
    cols: usize,
) -> Option<Measured> {
    let cols_i = cols as i32;
    if cells.is_empty() {
        return None;
    }
    let mask = cells.iter().fold(0u8, |m, c| m | 1 << c.0);
    let min_x = cells.iter().map(|c| c.1).min()?;
    let max_x = cells.iter().map(|c| c.1).max()?;
    let min_y = cells.iter().map(|c| c.2).min()?;
    let max_y = cells.iter().map(|c| c.2).max()?;
    let bbox = [min_x, min_y + 1, max_x + 8 - min_x, max_y + 16 - min_y];
    let origin = |c: &Cell| {
        (
            c.1 - 8 * (c.0 as i32 % cols_i),
            c.2 - 16 * (c.0 as i32 / cols_i),
        )
    };
    let on = |(x0, y0): (i32, i32)| -> Vec<&Cell> {
        cells
            .iter()
            .filter(|c| {
                c.1 == x0 + 8 * (c.0 as i32 % cols_i) && c.2 == y0 + 16 * (c.0 as i32 / cols_i)
            })
            .collect()
    };
    let distance = |(x0, y0): (i32, i32)| {
        hint.map_or(0, |(hx, hy)| {
            let dx = (x0 - hx).rem_euclid(256);
            dx.min(256 - dx) + (y0 - hy).abs()
        })
    };
    let (grid, on_grid) = cells
        .iter()
        .map(|c| (origin(c), c.0))
        .map(|(g, entry)| (g, on(g), entry))
        .max_by_key(|(g, on_grid, entry)| {
            (
                on_grid.len(),
                std::cmp::Reverse(*entry),
                std::cmp::Reverse(distance(*g)),
            )
        })
        .map(|(g, on_grid, _)| (g, on_grid))?;
    // A lone cell proves nothing about the grid unless it is in the block's
    // first row, which every drawer writes at `($01, $00)` and `($01 + 8, $00)`.
    let fits = on_grid.len() >= 2 || (on_grid.len() == 1 && on_grid[0].0 < cols);
    let flip_h = if cols == 2 {
        cells
            .iter()
            .find(|c| c.0 % 2 == 0)
            .map_or(facing_left, |c| c.3 & 0x40 == 0)
    } else {
        // Link-style drawers mirror the whole row: any cell's flip bit.
        cells.first().map_or(facing_left, |c| c.3 & 0x40 == 0)
    };
    let (x0, y0) = grid;
    let all_on_grid = on_grid.len() == cells.len();
    let centre = match (cols, flip_h) {
        (2, _) => x0 + 8,
        (_, true) => x0 + 16,
        (_, false) => x0 + 8,
    };
    Some(if fits {
        let rows = on_grid.iter().map(|c| c.0 / cols + 1).max().unwrap_or(1) as u8;
        Measured {
            position: [centre as f32, (y0 + 16 * i32::from(rows)) as f32],
            flip_h,
            mask,
            bbox,
            rows,
            rule: AnchorRule::Pairs,
            origin: grid,
            all_on_grid,
        }
    } else {
        Measured {
            position: [(min_x + max_x + 8) as f32 / 2.0, (max_y + 16) as f32],
            flip_h,
            mask,
            bbox,
            rows: 0,
            rule: AnchorRule::Bbox,
            origin: grid,
            all_on_grid,
        }
    })
}

/// One displayed sprite cell in window coordinates, as one object slot sees
/// it. Normally the OAM entry itself; in widescreen, a cell whose column fell
/// outside the window is the margin sprite the side-view observer
/// ([`crate::wide_sprites`]) drew for that slot and entry instead.
#[derive(Debug, Clone, Copy)]
struct Shown {
    /// Window X of the left column (outside `0..256` only in a margin).
    x: i32,
    /// OAM Y byte.
    y: u8,
    tile: u8,
    attr: u8,
    /// The visible OAM entry is this cell (for a margin cell: the routine
    /// left the column on screen, so OAM shows it wrapped to `x & 255`).
    oam: bool,
    /// For a margin cell, its [`Game::margin_sprites_into`] entries (bit per
    /// index): a display routine that runs twice before the OAM DMA leaves
    /// one copy per run, and all of them are replaced.
    margin: Option<u64>,
}

/// Each slot's view of the 64 displayed cells: the visible OAM entries, with
/// the margin sprites that slot drew (`margins`: `(slot, OAM entry, sprite)`
/// in [`Game::margin_sprites_into`] order) in place of their entries. Margin
/// cells are per slot because a later slot (a boss's fixed layout) may
/// overwrite the OAM entry another slot hid.
fn shown_cells(
    oam: &[u8; 256],
    margins: impl Iterator<Item = (u8, u8, z2_ppu::MarginSprite)>,
) -> Vec<[Option<Shown>; 64]> {
    let window: [Option<Shown>; 64] = std::array::from_fn(|i| {
        let c = &oam[i * 4..i * 4 + 4];
        (c[0] < 0xEF).then_some(Shown {
            x: i32::from(c[3]),
            y: c[0],
            tile: c[1],
            attr: c[2],
            oam: true,
            margin: None,
        })
    });
    let mut views = vec![window; 6];
    let mut seen = [0u64; 6];
    for (index, (slot, entry, m)) in margins.enumerate().take(64) {
        let (s, e) = (usize::from(slot), usize::from(entry));
        if s >= 6 || e >= 64 || m.y >= 0xEF {
            continue;
        }
        let x = i32::from(m.x);
        // The same cell is also in OAM when the routine left its column on
        // screen: wrapped round to `x & 255` (keep the margin position), or
        // drawn in the window by another display run a pixel or two away
        // (keep the window position). Either way both copies are this slot's.
        let same = window[e].filter(|o| {
            let d = (o.x - x).rem_euclid(256);
            o.tile == m.tile
                && (i32::from(o.y) - i32::from(m.y)).abs() <= 2
                && !(9..248).contains(&d)
        });
        let at = same.map_or(x, |o| if (o.x - x).abs() > 128 { x } else { o.x });
        let bit = 1u64 << index;
        if seen[s] & (1u64 << e) != 0 {
            if let Some(c) = &mut views[s][e] {
                c.margin = c.margin.map(|b| b | bit);
                if same.is_some() && !c.oam {
                    c.oam = true;
                    c.x = at;
                    c.y = window[e].map_or(c.y, |o| o.y);
                }
            }
            continue;
        }
        seen[s] |= 1u64 << e;
        views[s][e] = Some(Shown {
            x: at,
            y: same.map_or(m.y, |o| o.y),
            tile: m.tile,
            attr: m.attr,
            oam: same.is_some(),
            margin: Some(bit),
        });
    }
    views
}

/// A cell of the boss life meter (tiles `$6E`/`$70` stacked at X 16): never
/// part of any figure.
fn life_meter(c: &Shown) -> bool {
    c.x == 16 && matches!(c.tile, 0x6E | 0x70) && c.margin.is_none()
}

/// Distance from a slot's expected screen position to the nearest
/// displayed cell of `block` (`None` when the block is empty).
fn block_distance(view: &[Option<Shown>; 64], block: usize, sx: i32, sy: i32) -> Option<i32> {
    let at = usize::from(ENEMY_OAM_FIRST) + block * ENEMY_OAM_ENTRIES;
    view[at..at + ENEMY_OAM_ENTRIES]
        .iter()
        .flatten()
        .filter(|c| !life_meter(c))
        .map(|c| (c.x - sx).abs() + (i32::from(c.y) - sy).abs())
        .min()
}

/// Largest [`block_distance`] accepted as "this slot drew that block": one
/// frame of motion plus the cartridge's own small display offsets.
const MATCH_DISTANCE: i32 = 40;

/// Which `LD58F` rotation produced the displayed OAM, when no displayed-RAM
/// copy says (`$91,x`). Normally the one before `$0485` (the logic has
/// rotated since the copy); a lag frame keeps it, and boss rooms draw fixed
/// blocks. Choose the rotation whose blocks lie closest to the slots' screen
/// positions, preferring `$0485 - 1`, then `$0485`, on ties.
fn displayed_rotation(
    views: &[[Option<Shown>; 64]],
    current: usize,
    slots: &[(usize, i32, i32)],
) -> usize {
    let n = ENEMY_OAM_ENTRIES;
    let order = [n - 1, 0, 1, 2, 3, 4].map(|k| (current + k) % n);
    let cost = |rotation: usize| -> i32 {
        slots
            .iter()
            .filter(|&&(_, x, _)| (-16..272).contains(&x))
            .map(|&(s, x, y)| {
                block_distance(&views[s], (rotation + s) % n, x, y)
                    .unwrap_or(2 * MATCH_DISTANCE)
                    .min(2 * MATCH_DISTANCE)
            })
            .sum()
    };
    order
        .into_iter()
        .min_by_key(|&r| cost(r))
        .unwrap_or(order[0])
}

/// Whether cell `(x, y)` can belong to a slot whose display origin is
/// `(sx, sy)`: within the widest body plus its weapons and hanging parts.
/// `margin` cells are already in window coordinates.
fn near_slot(x: i32, y: i32, sx: i32, sy: i32, margin: bool) -> bool {
    let dx = if margin {
        x - sx
    } else {
        // A slot well off screen draws nothing on it (its 8-bit `$CD` would
        // wrap onto cells that belong to someone else).
        if !(-32..288).contains(&sx) {
            return false;
        }
        let dx = (x - sx).rem_euclid(256);
        if dx >= 128 {
            dx - 256
        } else {
            dx
        }
    };
    (-24..40).contains(&dx) && (-32..80).contains(&(y - sy))
}

impl Game {
    /// The cartridge draws an 8x16 cell with an all-zero pattern (its `$F5`
    /// "nothing" tile) to hide a slot without moving it off screen.
    fn blank_sprite_tile(&self, tile: u8) -> bool {
        #[cfg(feature = "interp")]
        {
            let bank = self.mmc1.map_chr4(tile & 1);
            let at = bank * 0x1000 + usize::from(tile & 0xFE) * 16;
            self.chr
                .get(at..at + 32)
                .is_some_and(|p| p.iter().all(|&b| b == 0))
        }
        #[cfg(not(feature = "interp"))]
        {
            let _ = tile;
            false
        }
    }

    /// Visible side-view enemies, NPCs and bosses, one per object slot, in
    /// slot order. Slots without displayed cells, and slots showing their
    /// score (`$B6,x = 3`), are left out.
    #[must_use]
    pub fn actor_visuals(&self) -> Vec<ActorVisual> {
        let mut out = Vec::new();
        let ram = self.shown_ram();
        let prev = self.shown_prev_ram();
        if ram[0x0736] != 0x0B {
            return out;
        }
        let Some(group) = group(ram[0x0707], ram[0x0769]) else {
            return out;
        };
        // Screen X from the scroll position (`$072A`/`$072C`), the 16-bit
        // form of the display routines' `$CD = $4E,x - $072C`. (Link's own
        // `$CC` wraps when he stands left of the scroll's page.)
        let scroll = i32::from(ram[0x072A]) * 256 + i32::from(ram[0x072C]);
        let live: Vec<(usize, i32, i32)> = (0..6)
            .filter(|&s| {
                matches!(ram[0xB6 + s], 1 | 2)
                    && alias(group, ram[0xA1 + s]).1 != ActorClass::Hidden
            })
            .map(|s| {
                let world = i32::from(ram[0x3C + s]) * 256 + i32::from(ram[0x4E + s]);
                (s, world - scroll, i32::from(ram[0x2A + s]))
            })
            .collect();
        if live.is_empty() {
            return out;
        }
        // Hidden OAM entries of columns outside the window come back as the
        // widescreen margin sprites drawn for them (when that observer is on).
        #[cfg(feature = "interp")]
        let views = shown_cells(
            self.oam(),
            crate::wide_sprites::shown_sources(self)
                .iter()
                .zip(crate::wide_sprites::shown(self))
                .map(|(&(slot, entry), &m)| (slot, entry, m)),
        );
        #[cfg(not(feature = "interp"))]
        let views = shown_cells(self.oam(), std::iter::empty());
        // `$91,x` of the displayed state names each slot's block exactly;
        // without a DMA-time copy, search the rotation.
        let exact = self.shown_valid > 0;
        let rotation = if exact {
            0
        } else {
            displayed_rotation(&views, usize::from(ram[0x0485]) % ENEMY_OAM_ENTRIES, &live)
        };
        let get = |a: usize| ram[a];
        let before = |a: usize| prev[a];
        let wram = |a: usize| self.wram[a];
        // OAM entries and margin sprites already part of an actor.
        let mut claimed = 0u64;
        let mut claimed_margin = 0u64;
        // Blocks claimed through `$91,x` (single sprites do not reserve
        // their block).
        let mut claimed_blocks = 0u64;
        // Order: slots with their own block (`$91,x`), then single-sprite
        // slots (each takes its one nearest sprite), then bosses, whose
        // fixed blocks are found by distance among the blocks no slot drew.
        let mut order = live.clone();
        let fixed_at = |s: usize| {
            let id = ram[0xA1 + s];
            fixed_layout(group, id, alias(group, id).1)
        };
        let rank = |s: usize| {
            if single_sprite(group, ram[0xA1 + s]) {
                1
            } else if fixed_at(s) {
                2
            } else {
                0
            }
        };
        order.sort_by_key(|l| rank(l.0));
        for &(slot, sx, sy) in &order {
            let view = &views[slot];
            let boss_class = fixed_at(slot);
            let single = single_sprite(group, ram[0xA1 + slot]);
            let first = if single {
                // Its own sprite only, never `$91,x` (Helmethead's helmets
                // point it at his block): the nearest free drawn cell to
                // where the display routine puts it, `($CD + 3, $2A,x)`.
                let Some((_, entry)) = (usize::from(ENEMY_OAM_FIRST)..ENEMY_OAM_END)
                    .filter_map(|i| view[i].map(|c| (i, c)))
                    .filter(|(i, c)| {
                        !life_meter(c)
                            && !self.blank_sprite_tile(c.tile)
                            && !(c.oam && claimed & (1u64 << i) != 0)
                    })
                    .map(|(i, c)| ((c.x - sx - 3).abs() + (i32::from(c.y) - sy).abs(), i))
                    .filter(|&(d, _)| d <= SINGLE_DISTANCE)
                    .min()
                else {
                    continue;
                };
                entry
            } else if exact {
                let entry = usize::from(ram[0x91 + slot]) / 4;
                let valid = (usize::from(ENEMY_OAM_FIRST)..ENEMY_OAM_END).contains(&entry)
                    && (entry - usize::from(ENEMY_OAM_FIRST)) % ENEMY_OAM_ENTRIES == 0;
                if valid && !boss_class {
                    entry
                } else {
                    let Some(block) = (0..ENEMY_OAM_ENTRIES)
                        .filter(|b| {
                            claimed_blocks & (0x3Fu64 << (usize::from(ENEMY_OAM_FIRST) + b * 6))
                                == 0
                        })
                        .filter_map(|b| block_distance(view, b, sx, sy).map(|d| (d, b)))
                        .filter(|&(d, _)| d <= MATCH_DISTANCE)
                        .min()
                        .map(|(_, b)| b)
                    else {
                        continue;
                    };
                    usize::from(ENEMY_OAM_FIRST) + block * ENEMY_OAM_ENTRIES
                }
            } else {
                let block = (rotation + slot) % ENEMY_OAM_ENTRIES;
                if block_distance(view, block, sx, sy).is_none_or(|d| d > MATCH_DISTANCE) {
                    continue;
                }
                usize::from(ENEMY_OAM_FIRST) + block * ENEMY_OAM_ENTRIES
            };
            let type_id = ram[0xA1 + slot];
            let (mut alias, class) = alias(group, type_id);
            let boss = fixed_layout(group, type_id, class);
            let body = body_entries(group, type_id);
            let entries = if single { 1 } else { ENEMY_OAM_ENTRIES };
            let picked: Vec<(usize, Shown)> = (0..entries)
                .filter_map(|i| view[first + i].map(|c| (i, c)))
                .filter(|(i, c)| {
                    !(c.oam && claimed & (1u64 << (first + i)) != 0)
                        && !life_meter(c)
                        && body & (1 << i) != 0
                        && !self.blank_sprite_tile(c.tile)
                        && (boss || near_slot(c.x, i32::from(c.y), sx, sy, c.margin.is_some()))
                })
                .collect();
            let cells: Vec<Cell> = picked
                .iter()
                .map(|&(i, c)| (i, c.x, i32::from(c.y), c.attr))
                .collect();
            let Some(m) = measure_grid(
                &cells,
                ram[0x60 + slot] != 1,
                Some((sx, sy)),
                columns(group, type_id),
            ) else {
                continue;
            };
            let (mut position, mut rows, mut rule) = (m.position, m.rows, m.rule);
            // A Link-style (3-column) layout that is not on its grid is not
            // the body (Dark Link before the fight draws a one-row figure).
            let body_layout = columns(group, type_id) == 2 || m.all_on_grid;
            if rule == AnchorRule::Pairs && body_layout {
                if let Some(known) = known_rows(group, type_id) {
                    // The bottom row may be clipped, or replaced by the death
                    // puff: the body still stands on its full height.
                    rows = rows.max(known);
                    position[1] = (m.origin.1 + 16 * i32::from(rows)) as f32;
                }
            }
            if class == ActorClass::Object || single {
                // Measured override: objects draw their own layouts (the
                // elevator's platform cells hang below its first row), so
                // their anchor is the bottom centre of what is drawn.
                let [x, y, w, h] = m.bbox;
                position = [x as f32 + w as f32 / 2.0, (y + h - 1) as f32];
                rows = 0;
                rule = AnchorRule::Bbox;
            }
            #[cfg(feature = "interp")]
            if (group, type_id) == ("b4b", 0x22) {
                // Carock: its two body blocks alternate frames, so the slot's
                // own block is the left or the right half. The collision box
                // is centred on the whole body in both facings.
                if let Some([x, _, w, _]) = self.enemy_box_at(ram, slot, sx) {
                    position[0] = (x + w / 2) as f32;
                    rule = AnchorRule::Box;
                }
            }
            #[cfg(feature = "interp")]
            if (group, type_id) == ("b4", 0x21) && palace(ram[0x0706], ram[0x056C]) == Some(5) {
                // Gooma: a 3x3 body (plus flail) drawn across two blocks
                // from an unaligned entry. His collision box spans the body
                // and ends on his feet in both facings.
                if let Some([x, y, w, h]) = self.enemy_box_at(ram, slot, sx) {
                    position = [(x + w / 2) as f32, (y + h - 1) as f32];
                    rows = 3;
                    rule = AnchorRule::Box;
                }
            }
            let base = character_key(group, type_id);
            let character = match qualifier(group, type_id, class, get, wram) {
                Some(q) => format!("{base}_{q}"),
                None => base.clone(),
            };
            if (group, type_id) == ("b4", 0x21) {
                alias = match palace(ram[0x0706], ram[0x056C]) {
                    Some(2) => "helmethead",
                    Some(5) => "gooma",
                    _ => alias,
                };
            }
            let (clip, clip_fallback) = clip_for(group, type_id, class, get, before, wram, slot);
            let flash_palette = cells.first().and_then(|c| {
                let shown = c.3 & 3;
                let own = wram(0x0DD5 + usize::from(type_id)) >> 6;
                (ram[0x040E + slot] != 0 && class != ActorClass::Npc && shown != own)
                    .then_some(shown)
            });
            let mut mask = 0u64;
            let mut margin_mask = 0u64;
            for &(i, c) in &picked {
                if c.oam {
                    mask |= 1u64 << (first + i);
                }
                margin_mask |= c.margin.unwrap_or(0);
            }
            claimed |= mask;
            if !single {
                claimed_blocks |= mask;
            }
            claimed_margin |= margin_mask;
            out.push(ActorVisual {
                id: ENEMY_ID_BASE + slot as u8,
                slot: slot as u8,
                type_id,
                group,
                character,
                base_character: base,
                alias,
                class,
                clip,
                clip_fallback,
                restart: matches!(clip, "throw" | "attack" | "cast")
                    && cycle_restart(group, type_id, get, before, slot),
                flash_palette,
                position,
                flip_h: m.flip_h,
                oam_mask: mask,
                margin_mask,
                priority_oam: (u64::from(m.mask) << first).trailing_zeros() as u8,
                oam_bbox: m.bbox,
                rows,
                rule,
            });
        }
        out.sort_by_key(|v| v.slot);
        let cell_of = |v: &ActorVisual, i: usize| views[usize::from(v.slot)][i];
        let visible = |c: &Shown| !self.blank_sprite_tile(c.tile);
        // Whether cell `c` is free, and marking it as part of `v`: its OAM
        // entry `i` and/or its margin sprites.
        let free = |c: &Shown, i: usize, claimed: u64, claimed_margin: u64| {
            !(c.oam && claimed & (1u64 << i) != 0) && claimed_margin & c.margin.unwrap_or(0) == 0
        };
        let take = |v: &mut ActorVisual, c: &Shown, i: usize| {
            if c.oam {
                v.oam_mask |= 1u64 << i;
            }
            v.margin_mask |= c.margin.unwrap_or(0);
        };
        let owns = |v: &ActorVisual, i: usize| {
            cell_of(v, i).is_some_and(|c| {
                (c.oam && v.oam_mask & (1u64 << i) != 0)
                    || v.margin_mask & c.margin.unwrap_or(0) != 0
            })
        };
        // Bosses and King Bot draw fixed layouts of their own that cross block
        // boundaries (Horsehead's lance, Gooma's 3x3 body, Barba's neck):
        // claim, cell by cell, every unclaimed cell that touches the figure
        // grown so far, within reach of the slot's position. The boss life
        // meter (tiles `$6E`/`$70` at X 16) is never part of a figure.
        for v in out
            .iter_mut()
            .filter(|v| fixed_layout(v.group, v.type_id, v.class))
        {
            let s = usize::from(v.slot);
            let (_, sx, sy) = live.iter().copied().find(|l| l.0 == s).unwrap_or((s, 0, 0));
            // Barba's head and neck start at OAM entry 18 (`LB2C3`).
            let first = if (group, v.type_id) == ("b4b", 0x21) {
                18
            } else {
                usize::from(ENEMY_OAM_FIRST)
            };
            let view = &views[s];
            let cell = |i: usize| view[i].map_or((0, 0), |c| (c.x, i32::from(c.y) + 1));
            let mut bbox = v.oam_bbox;
            loop {
                let [x, y, w, h] = bbox;
                let next = (first..ENEMY_OAM_END).find(|&i| {
                    let Some(c) = view[i] else { return false };
                    let (cx, cy) = cell(i);
                    let dx = if c.margin.is_some() {
                        cx - sx
                    } else {
                        if !(-32..288).contains(&sx) {
                            return false;
                        }
                        let dx = (cx - sx).rem_euclid(256);
                        if dx >= 128 {
                            dx - 256
                        } else {
                            dx
                        }
                    };
                    free(&c, i, claimed, claimed_margin)
                        && visible(&c)
                        && !life_meter(&c)
                        && (-48..64).contains(&dx)
                        && (-64..160).contains(&(cy - sy))
                        && cx + 8 >= x - 8
                        && cx <= x + w + 8
                        && cy + 16 >= y - 8
                        && cy <= y + h + 8
                });
                let Some(i) = next else { break };
                let Some(c) = view[i] else { break };
                if c.oam {
                    claimed |= 1u64 << i;
                }
                claimed_margin |= c.margin.unwrap_or(0);
                take(v, &c, i);
                let (cx, cy) = cell(i);
                let (x1, y1) = (x.min(cx), y.min(cy));
                let (x2, y2) = ((x + w).max(cx + 8), (y + h).max(cy + 16));
                bbox = [x1, y1, x2 - x1, y2 - y1];
            }
            v.oam_bbox = bbox;
            v.priority_oam = (0..64).find(|&i| owns(v, i)).unwrap_or(64) as u8;
            let [x, y, w, h] = bbox;
            if (group, v.type_id) == ("b5", 0x1E) {
                // King Bot: one 6x2 figure; anchor its bottom centre.
                v.position = [x as f32 + w as f32 / 2.0, (y + h - 1) as f32];
                v.rows = 0;
                v.rule = AnchorRule::Bbox;
            } else if (group, v.type_id) == ("b4b", 0x21) {
                // Barba: the head is the topmost row; the neck below it
                // reaches down to the lava however high he has risen.
                let head: Vec<usize> = (0..64).filter(|&i| owns(v, i) && cell(i).1 == y).collect();
                let left = head.iter().map(|&i| cell(i).0).min().unwrap_or(x);
                let right = head.iter().map(|&i| cell(i).0).max().unwrap_or(x + w - 8);
                v.position = [(left + right + 8) as f32 / 2.0, (y + 15) as f32];
                v.rows = 0;
                v.rule = AnchorRule::Head;
            }
        }
        // A dying enemy draws its flashing body 16 bytes past its own block
        // (`LEEB2`, `$EEB6`), spilling into the next block: take those cells
        // too, so the death clip replaces the whole original figure.
        for v in out
            .iter_mut()
            .filter(|v| ram[0xB6 + usize::from(v.slot)] == 2)
        {
            let s = usize::from(v.slot);
            let (_, sx, sy) = live.iter().copied().find(|l| l.0 == s).unwrap_or((s, 0, 0));
            let own = usize::from(v.priority_oam).max(usize::from(ENEMY_OAM_FIRST));
            let end = (own + 2 * ENEMY_OAM_ENTRIES).min(ENEMY_OAM_END);
            for (i, c) in views[s].iter().enumerate().take(end).skip(own) {
                let Some(c) = *c else { continue };
                if free(&c, i, claimed, claimed_margin)
                    && visible(&c)
                    && near_slot(c.x, i32::from(c.y), sx, sy, c.margin.is_some())
                {
                    if c.oam {
                        claimed |= 1u64 << i;
                    }
                    claimed_margin |= c.margin.unwrap_or(0);
                    take(v, &c, i);
                }
            }
        }
        out
    }
}

/// Machine-readable alignment record of the current frame: players and
/// enemies with their character keys, clips, ground anchors, displayed OAM
/// boxes and the cartridge's collision boxes. The art pipeline measures each
/// character's collision contract from it (`article_shots --collision-boxes`
/// sidecars, `z2-native --headless --dump-actors`).
///
/// Anchors and `oam_bbox` describe the picture on screen (OAM copied before
/// this frame's logic); `collision_box` comes from the RAM that logic has
/// just written, so a moving actor's box can lead its picture by one frame.
#[cfg(feature = "interp")]
#[must_use]
pub fn actor_report(game: &Game) -> serde_json::Value {
    use crate::collision_debug::Kind;
    let geometry = game.collision_geometry();
    let players: Vec<_> = game
        .player_visuals()
        .iter()
        .flatten()
        .map(|v| {
            serde_json::json!({
                "id": v.id, "character": v.character, "clip": v.clip,
                "position": v.position, "flip_h": v.flip_h,
                "oam_mask": format!("{:#018x}", v.oam_mask),
            })
        })
        .collect();
    let actors: Vec<_> = game
        .actor_visuals()
        .into_iter()
        .map(|v| {
            let collision_box = geometry
                .boxes
                .iter()
                .find(|b| b.kind == Kind::Enemy(v.slot))
                .map(|b| [b.x, b.y, b.w, b.h]);
            serde_json::json!({
                "slot": v.slot, "id": v.id, "type_id": format!("{:02x}", v.type_id),
                "group": v.group, "character": v.character,
                "base_character": v.base_character, "alias": v.alias,
                "class": v.class, "clip": v.clip, "clip_fallback": v.clip_fallback,
                "restart": v.restart, "flash_palette": v.flash_palette,
                "anchor": v.position,
                "flip_h": v.flip_h, "rule": v.rule, "rows": v.rows,
                "oam_bbox": v.oam_bbox, "oam_mask": format!("{:#018x}", v.oam_mask),
                "margin_mask": format!("{:#018x}", v.margin_mask),
                "priority_oam": v.priority_oam, "collision_box": collision_box,
            })
        })
        .collect();
    serde_json::json!({
        "frame": game.frame_count(),
        "mode": game.ram[0x0736],
        "world": game.ram[0x0707],
        "region": game.ram[0x0706],
        "players": players,
        "actors": actors,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cell(y: u8, x: u8, attr: u8) -> [u8; 4] {
        [y, 0x31, attr, x]
    }

    #[test]
    fn grid_anchor_is_column_centre_and_bottom_row() {
        let mut block = [0xF8u8; 24];
        for (i, c) in [
            cell(160, 175, 2),
            cell(160, 183, 2),
            cell(176, 175, 2),
            cell(176, 183, 2),
            cell(192, 175, 2),
            cell(192, 183, 2),
        ]
        .iter()
        .enumerate()
        {
            block[i * 4..i * 4 + 4].copy_from_slice(c);
        }
        let (pos, flip, mask, bbox, rows, rule) = measure_block(&block, true).unwrap();
        assert_eq!((pos, rows, rule), ([183.0, 208.0], 3, AnchorRule::Pairs));
        assert!(flip, "unflipped NES cells face left; clips face right");
        assert_eq!(mask, 0x3F);
        assert_eq!(bbox, [175, 161, 16, 48]);
        // The cartridge's mirror swaps tiles, not positions: same anchor.
        for c in block.chunks_mut(4) {
            c[2] |= 0x40;
        }
        let (mirrored, flip, ..) = measure_block(&block, true).unwrap();
        assert_eq!(mirrored, pos);
        assert!(!flip);
        // Off-grid extras (a raised spear) and a hidden first cell.
        block[0] = 0xF8;
        block[16..20].copy_from_slice(&cell(140, 170, 0x42));
        let (pos, _, mask, _, rows, rule) = measure_block(&block, true).unwrap();
        assert_eq!((pos, rows, rule), ([183.0, 208.0], 3, AnchorRule::Pairs));
        assert_eq!(mask, 0x3E);
    }

    #[test]
    fn off_grid_blocks_fall_back_to_the_visible_bottom_centre() {
        let mut block = [0xF8u8; 24];
        block[8..12].copy_from_slice(&cell(100, 40, 0));
        block[12..16].copy_from_slice(&cell(90, 70, 0));
        let (pos, _, mask, _, rows, rule) = measure_block(&block, true).unwrap();
        assert_eq!((pos, rows, rule), ([59.0, 116.0], 0, AnchorRule::Bbox));
        assert_eq!(mask, 0b1100);
        // A lone right-column cell of the first row (left cell clipped).
        let mut block = [0xF8u8; 24];
        block[4..8].copy_from_slice(&cell(100, 48, 0x40));
        let (pos, flip, _, _, rows, rule) = measure_block(&block, false).unwrap();
        assert_eq!((pos, rows, rule), ([48.0, 116.0], 1, AnchorRule::Pairs));
        assert!(!flip, "no left-column cell: the slot's facing decides");
        assert!(measure_block(&[0xF8; 24], true).is_none());
    }

    #[test]
    fn the_grid_most_cells_share_wins_then_the_first_entry() {
        // Death: the puff row at the origin (entries 0, 1) and the flashing
        // body's first row drawn at entries 4, 5 on the same line. Both
        // grids hold two cells; the puff's, from entry 0, wins.
        let cells = [
            (0, 40, 100, 0),
            (1, 48, 100, 0),
            (4, 40, 100, 0),
            (5, 48, 100, 0),
        ];
        let m = measure_cells(&cells, true, Some((40, 100))).unwrap();
        assert_eq!(
            (m.origin, m.rows, m.position),
            ((40, 100), 1, [48.0, 116.0])
        );
        let m = measure_cells(&cells, true, Some((40, 68))).unwrap();
        assert_eq!(m.origin, (40, 100), "the drawer starts at entry 0");
        // Cells of a wobbling Bot that share no grid: the first entry's
        // grid, not the one nearest the hint.
        let cells = [(0, 40, 100, 0), (1, 49, 100, 0x40)];
        let m = measure_cells(&cells, true, Some((41, 100))).unwrap();
        assert_eq!(m.origin, (40, 100));
    }

    #[test]
    fn keys_and_clips_follow_state_bytes() {
        assert_eq!(group(0, 1), Some("b1"));
        assert_eq!(group(0, 2), Some("b2"));
        assert_eq!(group(4, 4), Some("b4b"));
        assert_eq!(character_key("b1", 0x14), "enemy_b1_14");
        assert_eq!(character_key("b3", 0x0F), "npc_b3_0f");
        assert_eq!(alias("b4", 0x20), ("horsehead", ActorClass::Boss));
        assert_eq!(alias("b3", 0x19).1, ActorClass::Npc);
        assert_eq!(
            alias("b4b", 0x0A),
            ("rebonack_unhorsed", ActorClass::Swordsman)
        );
        assert_eq!(alias("b4b", 0x22), ("carock", ActorClass::Boss));
        let mut ram = [0u8; 0x800];
        let class = ActorClass::Thrower;
        assert_eq!(clip(class, |a| ram[a], 2), "idle");
        ram[0x71 + 2] = 0xF8;
        assert_eq!(clip(class, |a| ram[a], 2), "walk");
        ram[0x57E + 2] = 0xFC;
        assert_eq!(clip(class, |a| ram[a], 2), "jump");
        ram[0x81 + 2] = 1;
        assert_eq!(clip(class, |a| ram[a], 2), "throw");
        assert_eq!(clip(ActorClass::Caster, |a| ram[a], 2), "cast");
        assert_eq!(clip(ActorClass::Flyer, |a| ram[a], 2), "fly");
        ram[0x40E + 2] = 0x30;
        assert_eq!(clip(class, |a| ram[a], 2), "hurt");
        ram[0xB6 + 2] = 2;
        assert_eq!(clip(class, |a| ram[a], 2), "death");
    }

    #[test]
    fn per_type_states_have_fallbacks() {
        let wram = [0u8; 0x2000];
        let w = |a: usize| wram[a];
        let mut ram = [0u8; 0x800];
        let mut prev = [0u8; 0x800];
        // Ache: perched while `$81,x = 0`.
        let ache = |ram: &[u8; 0x800]| {
            clip_for("b1", 0x07, ActorClass::Flyer, |a| ram[a], |a| ram[a], w, 1)
        };
        assert_eq!(ache(&ram), ("perch", Some("fly")));
        ram[0x81 + 1] = 1;
        assert_eq!(ache(&ram), ("fly", None));
        // Acheman's devil form.
        ram[0xAF + 1] = 0x20;
        let acheman = clip_for("b1", 0x0A, ActorClass::Flyer, |a| ram[a], |a| ram[a], w, 1);
        assert_eq!(acheman, ("land", Some("fly")));
        // Iron Knuckle: high and low thrusts; the last displayed thrust frame
        // still counts after `$81,x` is cleared.
        let mut ram = [0u8; 0x800];
        let ik = |ram: &[u8; 0x800], prev: &[u8; 0x800]| {
            clip_for(
                "b4",
                0x18,
                ActorClass::Swordsman,
                |a| ram[a],
                |a| prev[a],
                w,
                0,
            )
        };
        ram[0x81] = 2;
        assert_eq!(ik(&ram, &prev), ("attack_high", Some("attack")));
        ram[0x81] = 4;
        assert_eq!(ik(&ram, &prev), ("attack_low", Some("attack")));
        prev[0x81] = 4;
        ram[0x81] = 0;
        assert_eq!(ik(&ram, &prev), ("attack_low", Some("attack")));
        prev[0x81] = 0;
        assert_eq!(ik(&ram, &prev), ("idle", None));
        assert_eq!(
            clip_for(
                "b4b",
                0x0A,
                alias("b4b", 0x0A).1,
                |a| ram[a],
                |a| prev[a],
                w,
                0
            )
            .0,
            "idle"
        );
        // Zora: upright once the stay timer has run for a displayed frame.
        let mut now = [0u8; 0x800];
        let mut before = [0u8; 0x800];
        let zora = |now: &[u8; 0x800], before: &[u8; 0x800]| {
            clip_for(
                "b2",
                0x18,
                ActorClass::Thrower,
                |a| now[a],
                |a| before[a],
                w,
                2,
            )
        };
        now[0x504 + 2] = 0x50;
        assert_eq!(zora(&now, &before), ("idle", None), "drawn low this frame");
        before[0x504 + 2] = 0x50;
        assert_eq!(zora(&now, &before), ("surface", Some("idle")));
        now[0x81 + 2] = 5;
        assert_eq!(zora(&now, &before), ("surface_throw", Some("throw")));
        // Townspeople: `$71,x` alone is not walking.
        let mut ram = [0u8; 0x800];
        let mut wram = [0u8; 0x2000];
        ram[0x71 + 3] = 0xF8;
        let npc = |ram: &[u8; 0x800], wram: &[u8; 0x2000]| {
            clip_for(
                "b3",
                0x11,
                ActorClass::Npc,
                |a| ram[a],
                |a| ram[a],
                |a| wram[a],
                3,
            )
        };
        assert_eq!(npc(&ram, &wram), ("walk", None));
        wram[0x0DF9 + 0x11] = 0x83; // immobile type
        assert_eq!(npc(&ram, &wram), ("idle", Some("walk")));
    }

    #[test]
    fn cycles_restart_on_the_cartridges_own_edges() {
        let mut now = [0u8; 0x800];
        let mut prev = [0u8; 0x800];
        // Rock-tossing Lizalfos: `$81,x` 2 -> 1 starts the next rock.
        prev[0x81] = 2;
        now[0x81] = 1;
        assert!(cycle_restart("b2", 0x1D, |a| now[a], |a| prev[a], 0));
        prev[0x81] = 1;
        assert!(!cycle_restart("b2", 0x1D, |a| now[a], |a| prev[a], 0));
        // Guma: the cycle timer reloads.
        prev[0x504] = 0;
        now[0x504] = 15;
        assert!(cycle_restart("b4", 0x1E, |a| now[a], |a| prev[a], 0));
        prev[0x504] = 15;
        now[0x504] = 14;
        assert!(!cycle_restart("b4", 0x1E, |a| now[a], |a| prev[a], 0));
        assert!(!cycle_restart("b4", 0x18, |a| now[a], |a| prev[a], 0));
    }

    #[test]
    fn qualifiers_split_shared_ids() {
        let mut ram = [0u8; 0x800];
        let mut wram = [0u8; 0x2000];
        let q = |ram: &[u8; 0x800], wram: &[u8; 0x2000], g, id, class| {
            qualifier(g, id, class, |a| ram[a], |a| wram[a])
        };
        ram[0x0706] = 0;
        ram[0x056C] = 1;
        assert_eq!(
            q(&ram, &wram, "b4", 0x21, ActorClass::Boss).as_deref(),
            Some("p2")
        );
        ram[0x0706] = 2;
        ram[0x056C] = 0;
        assert_eq!(
            q(&ram, &wram, "b4", 0x21, ActorClass::Boss).as_deref(),
            Some("p5")
        );
        assert_eq!(q(&ram, &wram, "b4", 0x20, ActorClass::Boss), None);
        wram[0x0DD5 + 0x1D] = 0x34;
        assert_eq!(
            q(&ram, &wram, "b3", 0x1D, ActorClass::Npc).as_deref(),
            Some("34")
        );
        assert_eq!(palace(1, 0), Some(4));
        assert_eq!(palace(2, 2), Some(7));
    }

    #[test]
    fn observer_reads_displayed_oam_without_mutation() {
        let mut game = Game::new();
        game.ram[0x0736] = 0x0B;
        game.ram[0x0707] = 3;
        game.ram[0x0769] = 4;
        game.ram[0x0485] = 1; // the copy on screen used rotation 0
        game.ram[0xB6 + 2] = 1;
        game.ram[0xA1 + 2] = 0x20;
        game.ram[0xCC] = 100;
        game.ram[0x4D] = 100;
        game.ram[0x4E + 2] = 140;
        game.ram[0x2A + 2] = 160;
        let first = usize::from(ENEMY_OAM_FIRST) + 2 * ENEMY_OAM_ENTRIES;
        game.oam = [0xF8; 256];
        // Drawn tiles have pixels; an all-zero pattern is the blank tile.
        game.chr = vec![0x55; 0x2000];
        for (i, (y, x)) in [(158, 138), (158, 146)].into_iter().enumerate() {
            game.oam[(first + i) * 4..(first + i) * 4 + 4].copy_from_slice(&cell(y, x, 0x42));
        }
        let ram = game.ram;
        let v = game.actor_visuals();
        assert_eq!(game.ram, ram);
        assert_eq!(v.len(), 1);
        let v = &v[0];
        assert_eq!(v.character, "enemy_b4_20");
        assert_eq!(v.base_character, "enemy_b4_20");
        assert_eq!((v.id, v.alias, v.clip), (10, "horsehead", "idle"));
        // Horsehead's body is three rows, whatever this frame drew.
        assert_eq!(v.position, [146.0, 206.0]);
        assert_eq!(v.rows, 3);
        assert_eq!(v.oam_mask, 3 << first);
        assert_eq!(v.priority_oam, first as u8);
        assert!(!v.flip_h);
        // A slot drawn only with blank tiles (teleporting, clipped) is
        // invisible: no figure.
        game.chr[0x1000 + 0x30 * 16..0x1000 + 0x32 * 16].fill(0);
        assert!(game.actor_visuals().is_empty());
        game.chr.fill(0x55);
        // Score popups (`$B6,x = 3`) keep their original sprites.
        game.ram[0xB6 + 2] = 3;
        assert!(game.actor_visuals().is_empty());
        game.ram[0xB6 + 2] = 1;
        game.ram[0x0736] = 0x05;
        assert!(game.actor_visuals().is_empty());
    }
}
