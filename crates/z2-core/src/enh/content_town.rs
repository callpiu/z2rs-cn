//! People, dialog and services in virtual town rooms for the `content`
//! group.
//!
//! The town people of an area ([`z2_content::TownNpc`]) are entries of its
//! rooms' enemy lists (the compiler appends them), so the game's own
//! loader spawns them and its own town code walks them, lets Link talk to
//! them and draws the box. This module only steps in while Link talks to
//! one of them in a virtual town room, and when he leaves a virtual house.
//!
//! # Dialog
//!
//! The dialog runs as the game's routine table (bank 3, `$B0AE`, `JSR
//! $D385` with `$0524` as the index, so every entry is a trap target). Three
//! entries are hooked; every one checks that bank 3 is mapped, the world is
//! a town world (1, 2) and Link is in the towns slot with a virtual room,
//! and that the person talking (`$048B`) is one of the room's town people
//! (`$BC,x` is the enemy-list offset `1 + 2 * index`), and otherwise just
//! runs what it replaced. Original towns never meet the conditions.
//!
//! * **Text pointer** (`$B480`, routine 5): the original picks a message
//!   from the town code `$056B` and the person's id, with per-id
//!   conditions that set quest bits (`$0796`/`$0797`, keyed on `$056B`).
//!   For a scripted person it runs with the id swapped for [`PLAIN_ID`]
//!   (the default condition, no side effects), then the person's encoded
//!   message is copied to [`TEXT_BUF`] in WRAM and the pointer
//!   `$0569`/`$056A` aimed at it; the typewriter reads the text through
//!   that pointer. The healer and magic ladies with their own role keep
//!   their original condition (it starts the walk to their door).
//! * **Typewriter** (`$B6A2`, routine 7) and **teardown** (`$B7B2`,
//!   routine 12): run with the same id swap, so the wise man's spell grant
//!   (`$B642`, `$077B + $056B`) and the townsfolk-to-Ache change
//!   (`$B7B2`, towns 2 and 5) never fire for a new person. After the
//!   teardown the role acts ([`z2_content::NpcRole`]): a healer starts the
//!   game's own refill (`$070D` life, `$070C` magic, `$FF` like
//!   `$B75B`), a sage sets his group flag and gives his gift once.
//!
//! Which message: the first talk of a visit says the message, later ones
//! [`z2_content::NpcScript::more`] in order (the game counts talks per
//! person in `$05A5,x`), the last repeating. A person whose
//! `requires` flag is not set says `locked` (or the message) and does not
//! act. A healer lady with the original walk says `more[0]` inside her
//! house (`$074C == 4`).
//!
//! # Door return
//!
//! The original game puts Link back in front of the door he came in by when
//! he walks out of a house: on a side exit (`$CF4C`) into a town map below
//! `$24` while the house depth `$075B` is not zero, the arrival code
//! `$075C` becomes `$075B + 4`, which restores the street's scroll and
//! Link's position saved when he took the door (`$C690`, from `$CFFC`).
//! Virtual streets run in the towns slot (map 62), so that branch never
//! runs for them. The side-exit hook (chained over `content`'s) does the
//! same after the exit when Link leaves a virtual town room for a town map
//! of `$24` or above: `$075C = $075B + 4`, `$075B - 1`, and on the last
//! level (outside town 7) the street music `$075F = 2` and sound `$EB =
//! $80`, as `$CFD3-$CFEA` do.
//!
//! # Doorways on bare street pages
//!
//! A [`z2_content::VanillaLink`] doorway may name a page of an original
//! street that has no door (the minigames' Mido and Darunia links). The
//! area-load hook (`$C4CB`, over `content`'s) runs [`paint_link_doorways`]
//! after the load: the town door's tiles ([`DOOR_TILES`]) go into the level
//! buffer at [`doorway_spot`] before the screen is drawn from it, so the
//! door shows, and its threshold tile makes the game's own door check
//! (`$E0C5`) fire on Up; `content`'s `$CFFC` hook then follows the link.
//!
//! # Memory
//!
//! [`TEXT_BUF`] (`$6C58-$6C97`, 64 bytes, inside the verified free range
//! `$6C58-$6CFF`): the message being typed. Scratch: rewritten on every
//! talk, never read outside a dialog. Everything else is the game's own
//! RAM; nothing is kept between frames.
//!
//! Called from `content::register` / `content::end_of_frame` only while
//! `ContentOpts::virtual_rooms` is on.

use super::content::{self, ContentOpts, VRoomId};
use super::content_areas::{room, Group, RoomRef};
use crate::game::Game;
use z2_content::{text, Gift, NpcRole, NpcScript, StaticArea};

/// WRAM home of the message being typed.
pub const TEXT_BUF: u16 = 0x6C58;
/// Bytes reserved at [`TEXT_BUF`] (a full box is 4 x 11 letters, 3 line
/// breaks and the end byte: 48).
pub const TEXT_BUF_LEN: usize = 0x40;

/// `bank3_Dialog_Routines_Set_text_pointer_according_to_Townfolk_type`.
pub const HOOK_TEXT_PTR: u16 = 0xB480;
/// `LB6A2`, the typewriter (and its end, `LB672`).
pub const HOOK_TYPEWRITER: u16 = 0xB6A2;
/// `LB7B2`, the dialog teardown (and the townsfolk-to-Ache change).
pub const HOOK_TEARDOWN: u16 = 0xB7B2;
/// Bank of the three dialog hooks.
pub const DIALOG_BANK: u8 = 3;
/// Id a scripted person wears while the dialog routines run: the
/// default condition (`$B5C7`), a normal typewriter and teardown.
pub const PLAIN_ID: u8 = 0x0C;

/// Towns table index ([`super::content_mem::SETS`]).
const TOWNS: u8 = 4;
/// First town map the original door return covers not (`CMP #$24`).
pub const DOOR_RETURN_MAP_LIMIT: u8 = 0x24;

const ADDR_TALK_SLOT: usize = 0x048B;
const ADDR_ID: usize = 0x00A1;
const ADDR_LIST_OFF: usize = 0x00BC;
const ADDR_CHAT: usize = 0x05A5;
const ADDR_DIALOG: usize = 0x074C;
const ADDR_TEXT_LO: usize = 0x0569;
const ADDR_TEXT_HI: usize = 0x056A;
const ADDR_REFILL_MAGIC: usize = 0x070C;
const ADDR_REFILL_LIFE: usize = 0x070D;
const ADDR_CONTAINERS_MAGIC: usize = 0x0783;
const ADDR_CONTAINERS_HEART: usize = 0x0784;
const ADDR_HOUSE_DEPTH: usize = 0x075B;
const ADDR_ARRIVAL: usize = 0x075C;
const ADDR_MUSIC: usize = 0x075F;
const ADDR_SOUND: usize = 0x00EB;
const ADDR_TOWN: usize = 0x056B;
const ADDR_MODE: usize = 0x0736;
const ADDR_WORLD: usize = 0x0707;
const ADDR_SCENE: usize = 0x0561;
/// `$074C` while the healer lady talks inside her house.
const DIALOG_INSIDE: u8 = 4;
/// `$0736` after a side exit to another side-view (`$CFF6`).
const MODE_SIDE_EXIT: u8 = 7;
/// Town code without the door-return music (`$CFDE`).
const TOWN_NO_RETURN_MUSIC: u8 = 7;

// ------------------------------------------------------------ register

/// Install the dialog and door-return hooks when the towns table has a
/// live slot and an enabled area there.
pub(crate) fn register(game: &mut Game, opts: &ContentOpts) {
    if content::slot_map(game, TOWNS).is_none() || !content::areas(opts).any(|a| a.table == TOWNS) {
        return;
    }
    let people = content::areas(opts).any(|a| a.table == TOWNS && !a.npcs.is_empty());
    if people {
        for (name, addr, func) in [
            (
                "enh_content_town_text",
                HOOK_TEXT_PTR,
                hook_text_ptr as crate::traps::TrapFn,
            ),
            ("enh_content_town_type", HOOK_TYPEWRITER, hook_typewriter),
            ("enh_content_town_close", HOOK_TEARDOWN, hook_teardown),
        ] {
            let cycles = if game.traps.get(addr).is_some() {
                None
            } else {
                Some(0)
            };
            super::hook(game, name, Some(DIALOG_BANK), addr, func, cycles);
        }
    }
    super::hook(
        game,
        "enh_content_town_door_return",
        Some(7),
        content::HOOK_SIDE_EXIT,
        hook_side_exit,
        None,
    );
    if content::areas(opts).any(|a| a.table == TOWNS && a.links.iter().any(|l| l.door)) {
        super::hook(
            game,
            "enh_content_town_link_doors",
            Some(7),
            content::HOOK_AREA_LOAD,
            hook_area_load,
            None,
        );
    }
}

/// Per-frame work after [`Game::step`]: none (every decision is made
/// inside the hooks from live RAM).
pub(crate) fn end_of_frame(game: &mut Game, opts: &ContentOpts) {
    let _ = (game, opts);
}

/// Identity bytes of every script (netplay: peers with other scripts must
/// not match).
#[must_use]
pub fn identity_bytes() -> Vec<u8> {
    let mut v = Vec::new();
    let msg = |v: &mut Vec<u8>, m: &z2_content::Message| {
        v.extend_from_slice(&(m.text.len() as u32).to_le_bytes());
        v.extend_from_slice(m.text.as_bytes());
    };
    for s in z2_content::areas::npc_scripts() {
        v.extend_from_slice(&(s.area.len() as u32).to_le_bytes());
        v.extend_from_slice(s.area.as_bytes());
        v.push(s.npc);
        match s.role {
            NpcRole::Talk => v.push(0),
            NpcRole::HealLife => v.push(1),
            NpcRole::HealMagic => v.push(2),
            NpcRole::Sage { flag, gift } => {
                v.extend_from_slice(&[3, flag.unwrap_or(0xFF), gift as u8]);
            }
        }
        v.push(s.requires.unwrap_or(0xFF));
        v.push(s.more.len() as u8);
        for m in s.messages() {
            msg(&mut v, m);
        }
    }
    v
}

// ------------------------------------------------------------ who talks

/// A town person of a virtual room Link is talking to.
#[derive(Debug, Clone, Copy)]
pub struct Talker {
    /// The room.
    pub room: VRoomId,
    /// Its area.
    pub area: &'static StaticArea,
    /// The area's option group (owner of its flags).
    pub group: Group,
    /// Index in the area's people.
    pub npc: u8,
    /// Object slot (`$048B`).
    pub slot: usize,
    /// Live enemy id (`$A1,x`).
    pub id: u8,
    /// The person's script, if any.
    pub script: Option<&'static NpcScript>,
}

impl Talker {
    /// The person's role (script, else from the id).
    #[must_use]
    pub fn role(&self) -> NpcRole {
        self.script.map_or(NpcRole::of_id(self.id), |s| s.role)
    }

    /// Whether the person's `requires` flag is met.
    #[must_use]
    pub fn unlocked(&self, game: &Game) -> bool {
        self.script
            .and_then(|s| s.requires)
            .is_none_or(|f| content::flag(game, self.group, u16::from(f)))
    }

    /// A healer or magic lady in her own role: her original door walk and
    /// restore run (only the text changes).
    #[must_use]
    pub fn original_healer(&self, game: &Game) -> bool {
        self.unlocked(game)
            && matches!(
                (self.role(), self.id),
                (NpcRole::HealLife, z2_content::npc::ID_HEALER)
                    | (NpcRole::HealMagic, z2_content::npc::ID_MAGIC_LADY)
            )
    }

    /// The encoded message (end byte included) the person says now.
    #[must_use]
    pub fn text(&self, game: &Game) -> Vec<u8> {
        let main = || {
            let n = self.area.npcs[usize::from(self.npc)];
            self.area
                .messages
                .get(usize::from(n.message))
                .map_or_else(|| vec![text::END], |m| m.to_vec())
        };
        let enc = |m: &z2_content::Message| {
            text::encode_message(m.text).unwrap_or_else(|_| vec![text::END])
        };
        let Some(s) = self.script else {
            return main();
        };
        if !self.unlocked(game) {
            return s.locked.as_ref().map_or_else(main, enc);
        }
        if self.original_healer(game) {
            return match s.more.first() {
                Some(m) if game.ram[ADDR_DIALOG] == DIALOG_INSIDE => enc(m),
                _ => main(),
            };
        }
        let talks = usize::from(game.ram[ADDR_CHAT + self.slot]);
        match talks.saturating_sub(1).min(s.more.len()) {
            0 => main(),
            i => enc(&s.more[i - 1]),
        }
    }
}

/// Whether switchable `bank` is mapped at `$8000-$BFFF`.
fn bank_mapped(game: &Game, bank: u8) -> bool {
    game.prg.len() >= 0x8000
        && game.mmc1.map_prg(0x8000, game.prg.len()) == usize::from(bank) * 0x4000
}

/// The virtual town room Link is in (towns slot loaded), if any.
#[must_use]
pub fn town_room(game: &Game) -> Option<RoomRef> {
    if !matches!(game.ram[ADDR_WORLD], 1 | 2) || content::current_table(game) != Some(TOWNS) {
        return None;
    }
    let slot = content::slot_map(game, TOWNS)?;
    let r = room(content::current_room(game)?)?;
    (r.table() == TOWNS && game.ram[ADDR_SCENE] == slot).then_some(r)
}

/// The town person Link is talking to (`$048B`), when it is one of the
/// current virtual room's people.
#[must_use]
pub fn talker(game: &Game) -> Option<Talker> {
    let r = town_room(game)?;
    let slot = usize::from(game.ram[ADDR_TALK_SLOT]);
    if slot >= 6 {
        return None;
    }
    let off = game.ram[ADDR_LIST_OFF + slot];
    if off == 0xFF || off.is_multiple_of(2) {
        return None;
    }
    let index = (off - 1) / 2;
    let npc = r
        .area
        .npcs
        .iter()
        .position(|n| n.room == r.index && n.enemy_index == index)?;
    let npc = npc as u8;
    Some(Talker {
        room: r.id,
        area: r.area,
        group: r.group,
        npc,
        slot,
        id: game.ram[ADDR_ID + slot],
        script: z2_content::areas::npc_scripts().find(|s| s.area == r.area.name && s.npc == npc),
    })
}

/// [`talker`] while bank 3 runs (the hooks' guard).
fn active(game: &Game) -> Option<Talker> {
    if bank_mapped(game, DIALOG_BANK) {
        talker(game)
    } else {
        None
    }
}

// ------------------------------------------------------------ hooks

/// Run the routine at `addr` with the talker's id swapped for
/// [`PLAIN_ID`].
fn run_plain(game: &mut Game, t: &Talker, addr: u16) {
    let at = ADDR_ID + t.slot;
    let id = game.ram[at];
    game.ram[at] = PLAIN_ID;
    super::call_original(game, addr);
    game.ram[at] = id;
}

/// Copy `bytes` (ending in the end byte) to [`TEXT_BUF`] and aim the text
/// pointer at it.
pub fn stage_text(game: &mut Game, bytes: &[u8]) {
    let n = bytes.len().min(TEXT_BUF_LEN);
    let at = usize::from(TEXT_BUF - 0x6000);
    game.wram[at..at + n].copy_from_slice(&bytes[..n]);
    game.wram[at + n.max(1) - 1] = text::END;
    let [lo, hi] = TEXT_BUF.to_le_bytes();
    game.ram[ADDR_TEXT_LO] = lo;
    game.ram[ADDR_TEXT_HI] = hi;
}

/// `$B480` hook: original condition (plain id, or the lady's own), then
/// the person's own text.
fn hook_text_ptr(game: &mut Game) {
    let Some(t) = active(game) else {
        super::call_original(game, HOOK_TEXT_PTR);
        return;
    };
    if t.original_healer(game) {
        super::call_original(game, HOOK_TEXT_PTR);
    } else {
        run_plain(game, &t, HOOK_TEXT_PTR);
    }
    let text = t.text(game);
    stage_text(game, &text);
}

/// `$B6A2` hook: the typewriter and its end with a plain id.
fn hook_typewriter(game: &mut Game) {
    match active(game) {
        Some(t) if !t.original_healer(game) => run_plain(game, &t, HOOK_TYPEWRITER),
        _ => super::call_original(game, HOOK_TYPEWRITER),
    }
}

/// `$B7B2` hook: the teardown with a plain id, then the role's effect.
fn hook_teardown(game: &mut Game) {
    let Some(t) = active(game) else {
        super::call_original(game, HOOK_TEARDOWN);
        return;
    };
    if t.original_healer(game) {
        super::call_original(game, HOOK_TEARDOWN);
        return;
    }
    run_plain(game, &t, HOOK_TEARDOWN);
    if t.unlocked(game) {
        act(game, &t);
    }
}

/// What a role does as the box closes.
fn act(game: &mut Game, t: &Talker) {
    match t.role() {
        NpcRole::Talk => {}
        NpcRole::HealLife => game.ram[ADDR_REFILL_LIFE] = 0xFF,
        NpcRole::HealMagic => game.ram[ADDR_REFILL_MAGIC] = 0xFF,
        NpcRole::Sage { flag, gift } => {
            let Some(f) = flag.map(u16::from) else {
                return;
            };
            if content::flag(game, t.group, f) {
                return;
            }
            content::set_flag(game, t.group, f, true);
            give(game, gift);
        }
    }
}

/// Hand over a sage's gift: one container (up to 8) and a full meter.
fn give(game: &mut Game, gift: Gift) {
    let (count, refill) = match gift {
        Gift::None => return,
        Gift::MagicContainer => (ADDR_CONTAINERS_MAGIC, ADDR_REFILL_MAGIC),
        Gift::HeartContainer => (ADDR_CONTAINERS_HEART, ADDR_REFILL_LIFE),
    };
    if game.ram[count] < 8 {
        game.ram[count] += 1;
    }
    game.ram[refill] = 0xFF;
}

/// `$CF4C` hook (over `content`'s): door return out of a virtual house
/// into a town map the original branch does not cover.
fn hook_side_exit(game: &mut Game) {
    let from_virtual = town_room(game).is_some();
    let depth = game.ram[ADDR_HOUSE_DEPTH];
    let world = game.ram[ADDR_WORLD];
    super::call_original(game, content::HOOK_SIDE_EXIT);
    if !from_virtual
        || depth == 0
        || game.ram[ADDR_MODE] != MODE_SIDE_EXIT
        || game.ram[ADDR_WORLD] != world
        || game.ram[ADDR_SCENE] < DOOR_RETURN_MAP_LIMIT
    {
        return;
    }
    game.ram[ADDR_ARRIVAL] = depth + 4;
    game.ram[ADDR_HOUSE_DEPTH] = depth - 1;
    if depth == 1 && game.ram[ADDR_TOWN] != TOWN_NO_RETURN_MUSIC {
        game.ram[ADDR_SOUND] = 0x80;
        game.ram[ADDR_MUSIC] = 2;
    }
}

// ------------------------------------------------------------ link doorways

/// The town door object's tiles, top to bottom (small object 1 of the town
/// object set, measured in the level buffer): lintel, two dark rows, and
/// the threshold the door check reads under Link's feet.
pub const DOOR_TILES: [u8; 4] = [0x8C, 0x91, 0x91, 0x54];
/// Tiles the game's door check takes for a doorway (bank 3
/// `$851A-$851C`, compared at `$E0CA-$E0D2`).
pub const DOOR_TRIGGER_TILES: [u8; 3] = [0x54, 0xEA, 0x58];
/// Bytes per page of the side-view level buffer (`$6000`, 13 rows of 16).
const PAGE_BYTES: usize = 0xD0;
const ROWS: usize = 13;
const COLS: usize = 16;

/// Level-buffer index of `page`, `row`, `col`.
const fn cell(page: usize, row: usize, col: usize) -> usize {
    page * PAGE_BYTES + row * COLS + col
}

/// The column of `page` where a doorway fits (the ground's top row under
/// it and three uniform rows of open wall above), with the ground row:
/// in a house front (a wall that is not the page's sky, row 0) if there is
/// one, else anywhere, the middle of the page first.
#[must_use]
pub fn doorway_spot(wram: &[u8], page: usize) -> Option<(usize, usize)> {
    let order = [8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13];
    let fits = |col: usize, in_house: bool| {
        let t = |row| wram[cell(page, row, col)];
        let ground = t(ROWS - 1);
        let mut top = ROWS - 1;
        while top > 4 && t(top - 1) == ground {
            top -= 1;
        }
        let wall = t(top - 1);
        let open = (top - 3..top).all(|r| t(r) == wall) && wall != ground;
        (open && (!in_house || wall != t(0))).then_some((col, top))
    };
    [true, false]
        .into_iter()
        .find_map(|house| order.into_iter().find_map(|col| fits(col, house)))
}

/// Whether `page` of the level buffer already has a doorway.
fn page_has_door(wram: &[u8], page: usize) -> bool {
    (0..ROWS * COLS).any(|i| DOOR_TRIGGER_TILES.contains(&wram[page * PAGE_BYTES + i]))
}

/// Give every page of the original town map just loaded that a
/// [`z2_content::VanillaLink`] doorway of an enabled area names, and that
/// has no doorway of its own, a door: [`DOOR_TILES`] painted into the
/// level buffer at [`doorway_spot`], before the screen is drawn from it.
/// The threshold tile makes the game's own door check fire on Up (house
/// depth `$075B` + 1, mode `$16`, then `$CFFC`), and `content`'s door hook
/// sends that doorway into the link's room. Returns the painted
/// `(page, column)`s.
pub fn paint_link_doorways(game: &mut Game) -> Vec<(u8, u8)> {
    let mut painted = Vec::new();
    if !matches!(game.ram[ADDR_WORLD], 1 | 2) || content::current_table(game) != Some(TOWNS) {
        return painted;
    }
    let map = game.ram[ADDR_SCENE];
    if content::slot_map(game, TOWNS) == Some(map) {
        return painted;
    }
    let opts = game.enh.content;
    let pages: Vec<u8> = content::areas(&opts)
        .filter(|a| a.table == TOWNS)
        .flat_map(|a| a.links.iter())
        .filter(|l| l.door && l.map == map)
        .map(|l| l.index & 3)
        .collect();
    for page in pages {
        let p = usize::from(page);
        if painted.iter().any(|&(q, _)| q == page) || page_has_door(&game.wram, p) {
            continue;
        }
        let Some((col, ground)) = doorway_spot(&game.wram, p) else {
            continue;
        };
        for (k, &tile) in DOOR_TILES.iter().enumerate() {
            game.wram[cell(p, ground + k - 3, col)] = tile;
        }
        painted.push((page, col as u8));
    }
    painted
}

/// `$C4CB` hook (over `content`'s): the load, then [`paint_link_doorways`].
fn hook_area_load(game: &mut Game) {
    super::call_original(game, content::HOOK_AREA_LOAD);
    paint_link_doorways(game);
}

#[cfg(test)]
mod tests {
    use super::*;
    use z2_content::areas::dev_rooms::{inn_npcs, INN_TALKER_LINES};

    fn inn() -> RoomRef {
        room(content::find_room("Dev Inn", 0).unwrap()).unwrap()
    }

    fn talker_for(g: &mut Game, npc: u8, id: u8) -> Talker {
        let r = inn();
        let n = r.area.npcs[usize::from(npc)];
        g.ram[ADDR_TALK_SLOT] = 2;
        g.ram[ADDR_LIST_OFF + 2] = 1 + 2 * n.enemy_index;
        g.ram[ADDR_ID + 2] = id;
        Talker {
            room: r.id,
            area: r.area,
            group: r.group,
            npc,
            slot: 2,
            id,
            script: z2_content::areas::npc_scripts()
                .find(|s| s.area == r.area.name && s.npc == npc),
        }
    }

    #[test]
    fn messages_advance_per_talk_and_stay_on_the_last() {
        let mut g = Game::new();
        let t = talker_for(&mut g, inn_npcs::TALKER, 0x22);
        for (talks, line) in [(0, 0), (1, 0), (2, 1), (3, 2), (9, 2)] {
            g.ram[ADDR_CHAT + 2] = talks;
            assert_eq!(
                text::decode(&t.text(&g)),
                INN_TALKER_LINES[line],
                "talk {talks}"
            );
        }
    }

    #[test]
    fn locked_people_say_their_locked_line_until_the_flag() {
        let mut g = Game::new();
        let t = talker_for(&mut g, inn_npcs::HINT, 0x22);
        assert!(!t.unlocked(&g));
        assert!(text::decode(&t.text(&g)).starts_with("SEE THE"));
        content::set_flag(&mut g, Group::TestRoom, 0, true);
        assert!(t.unlocked(&g));
        assert!(text::decode(&t.text(&g)).starts_with("THE SAGE"));
    }

    #[test]
    fn roles_act_once() {
        let mut g = Game::new();
        let sage = talker_for(&mut g, inn_npcs::SAGE, 0x0F);
        assert!(matches!(sage.role(), NpcRole::Sage { flag: Some(0), .. }));
        act(&mut g, &sage);
        assert!(content::flag(&g, Group::TestRoom, 0));
        let heal = talker_for(&mut g, inn_npcs::HEALER, 0x19);
        act(&mut g, &heal);
        assert_eq!(g.ram[ADDR_REFILL_LIFE], 0xFF);
        assert!(!heal.original_healer(&g), "not the healer lady's id");
        let lady = talker_for(&mut g, inn_npcs::HEALER, z2_content::npc::ID_HEALER);
        assert!(lady.original_healer(&g));
        give(&mut g, Gift::MagicContainer);
        assert_eq!(g.ram[ADDR_CONTAINERS_MAGIC], 1);
        assert_eq!(g.ram[ADDR_REFILL_MAGIC], 0xFF);
    }

    #[test]
    fn staged_text_ends_and_is_pointed_at() {
        let mut g = Game::new();
        stage_text(&mut g, &[0xDA, 0xDB, text::END]);
        let at = usize::from(TEXT_BUF - 0x6000);
        assert_eq!(&g.wram[at..at + 3], &[0xDA, 0xDB, text::END]);
        assert_eq!(
            u16::from_le_bytes([g.ram[ADDR_TEXT_LO], g.ram[ADDR_TEXT_HI]]),
            TEXT_BUF
        );
        // Too long: cut, still ended.
        stage_text(&mut g, &[0xDA; 100]);
        assert_eq!(g.wram[at + TEXT_BUF_LEN - 1], text::END);
        assert!(super::super::content_mem::WRAM_FREE
            .iter()
            .any(|f| f.contains(TEXT_BUF) && f.contains(TEXT_BUF + TEXT_BUF_LEN as u16 - 1)));
    }

    #[test]
    fn doorway_spots_prefer_a_house_front() {
        // Sky (row 0) down to row 9, ground rows 10-12.
        let mut w = vec![0x42u8; 0x2000];
        for col in 0..16 {
            for row in 10..13 {
                w[cell(1, row, col)] = 0x55;
            }
        }
        assert_eq!(doorway_spot(&w, 1), Some((8, 10)), "middle, open sky");
        // A house front on columns 4-6, rows 6-9.
        for col in 4..7 {
            for row in 6..10 {
                w[cell(1, row, col)] = 0x30;
            }
        }
        assert_eq!(doorway_spot(&w, 1), Some((6, 10)), "in the house");
        assert!(!page_has_door(&w, 1));
        w[cell(1, 10, 6)] = DOOR_TILES[3];
        assert!(page_has_door(&w, 1));
    }

    #[test]
    fn every_script_flag_fits_its_group() {
        for s in z2_content::areas::npc_scripts() {
            let (g, _) = content::all_areas()
                .find(|(_, a)| a.name == s.area)
                .expect("script area registered");
            for f in s.requires.iter().chain(s.sets_flag().iter()) {
                assert!(
                    super::super::content_areas::flag_bit(g, u16::from(*f)).is_some(),
                    "{}: flag {f} past the group's flags",
                    s.area
                );
            }
        }
    }
}
