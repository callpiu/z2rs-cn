//! The registry of authored areas for the `content` group: which static
//! areas ([`z2_content::StaticArea`]) each option turns on, global virtual
//! room ids, and the persistent-bit budget of each option.
//!
//! Areas come from `z2_content::areas::{parapa_annex, challenge_rooms,
//! minigames, hidden_caves, dark_isle}::statics()` plus the developer test
//! room defined here. Room ids ([`VRoomId`]) number every room of every
//! area of every group in [`GROUPS`] order, enabled or not, so an id does
//! not depend on the options.
//!
//! # Persistent bits
//!
//! Each group owns a fixed range of the 140-bit pool
//! ([`super::content_mem::PROGRESS_POOL`]): [`GroupInfo::bit_base`],
//! [`GroupInfo::bits`]. Inside it, tracked pages (a collectable or a
//! locked door, [`tracked_pages`]) take bits from the start, in area, room
//! and page order ([`page_bit`]); the last [`GroupInfo::flags`] bits are
//! free-form flags for engine code ([`flag_bit`], e.g. a minigame won).
//! Fixed ranges mean authoring one group never moves another group's bits.

use z2_content::{Dest, StaticArea, StaticRoom};

use super::content::ContentOpts;

/// One option of the content group that brings areas.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Group {
    /// `test_room`: the developer rooms (`z2_content::areas::dev_rooms`).
    TestRoom = 0,
    /// `parapa_annex`.
    ParapaAnnex = 1,
    /// `challenge_rooms`.
    ChallengeRooms = 2,
    /// `minigames`.
    Minigames = 3,
    /// `hidden_caves`.
    HiddenCaves = 4,
    /// `dark_isle`: eighth palace, monster town, southern isle.
    DarkIsle = 5,
}

/// Static facts about a [`Group`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GroupInfo {
    /// The group.
    pub group: Group,
    /// Option name in [`ContentOpts`].
    pub name: &'static str,
    /// First pool bit the group owns.
    pub bit_base: u16,
    /// Pool bits the group owns (tracked pages first, then flags).
    pub bits: u16,
    /// Of those, the last `flags` are free-form flags ([`flag_bit`]).
    pub flags: u16,
}

const fn gi(group: Group, name: &'static str, bit_base: u16, bits: u16, flags: u16) -> GroupInfo {
    GroupInfo {
        group,
        name,
        bit_base,
        bits,
        flags,
    }
}

/// Every group, in registration (and room id) order. The bit ranges tile
/// the 140-bit pool.
pub const GROUPS: [GroupInfo; 6] = [
    gi(Group::TestRoom, "test_room", 0, 4, 1),
    gi(Group::ParapaAnnex, "parapa_annex", 4, 24, 0),
    gi(Group::ChallengeRooms, "challenge_rooms", 28, 24, 4),
    gi(Group::Minigames, "minigames", 52, 24, 8),
    gi(Group::HiddenCaves, "hidden_caves", 76, 24, 0),
    gi(Group::DarkIsle, "dark_isle", 100, 40, 4),
];

impl Group {
    /// The group's static facts.
    #[must_use]
    pub const fn info(self) -> &'static GroupInfo {
        &GROUPS[self as usize]
    }
}

impl ContentOpts {
    /// Whether `g`'s areas are on (its option and the master switch).
    #[must_use]
    pub fn enables(&self, g: Group) -> bool {
        self.virtual_rooms
            && match g {
                Group::TestRoom => self.test_room,
                Group::ParapaAnnex => self.parapa_annex,
                Group::ChallengeRooms => self.challenge_rooms,
                Group::Minigames => self.minigames,
                Group::HiddenCaves => self.hidden_caves,
                Group::DarkIsle => self.dark_isle,
            }
    }
}

// ------------------------------------------------------------ developer rooms

pub use z2_content::areas::dev_rooms::{DEV_INN_FROM_MAP, DEV_PALACE_FROM_MAP, TEST_ROOM_FROM_MAP};

// ------------------------------------------------------------ registry

/// The static areas of group `g` (enabled or not).
#[must_use]
pub fn group_areas(g: Group) -> &'static [StaticArea] {
    use z2_content::areas as a;
    match g {
        Group::TestRoom => a::dev_rooms::statics(),
        Group::ParapaAnnex => a::parapa_annex::statics(),
        Group::ChallengeRooms => a::challenge_rooms::statics(),
        Group::Minigames => a::minigames::statics(),
        Group::HiddenCaves => a::hidden_caves::statics(),
        Group::DarkIsle => a::dark_isle::statics(),
    }
}

/// Every area of every group with its group, in id order.
pub fn all_areas() -> impl Iterator<Item = (Group, &'static StaticArea)> {
    GROUPS
        .iter()
        .flat_map(|g| group_areas(g.group).iter().map(move |a| (g.group, a)))
}

/// The areas `opts` turns on.
pub fn areas(opts: &ContentOpts) -> impl Iterator<Item = &'static StaticArea> {
    let opts = *opts;
    all_areas()
        .filter(move |(g, _)| opts.enables(*g))
        .map(|(_, a)| a)
}

/// A global virtual room id (index over every room of [`all_areas`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct VRoomId(pub u16);

/// The test room's id.
pub const TEST_ROOM: VRoomId = VRoomId(0);

/// One registered room and where it comes from.
#[derive(Debug, Clone, Copy)]
pub struct RoomRef {
    /// Its id.
    pub id: VRoomId,
    /// Group (option) it belongs to.
    pub group: Group,
    /// Its area.
    pub area: &'static StaticArea,
    /// Index in the area.
    pub index: u8,
    /// The room.
    pub room: &'static StaticRoom,
}

impl RoomRef {
    /// Side-view table set index (`content_mem::SETS`).
    #[must_use]
    pub fn table(&self) -> u8 {
        self.area.table
    }

    /// Id of room `index` of the same area.
    #[must_use]
    pub fn sibling(&self, index: u8) -> Option<VRoomId> {
        if usize::from(index) < self.area.rooms.len() {
            Some(VRoomId(
                self.id.0 - u16::from(self.index) + u16::from(index),
            ))
        } else {
            None
        }
    }
}

/// Every registered room, in id order.
pub fn rooms() -> impl Iterator<Item = RoomRef> {
    let mut next = 0u16;
    all_areas().flat_map(move |(group, area)| {
        let base = next;
        next += area.rooms.len() as u16;
        area.rooms.iter().enumerate().map(move |(i, room)| RoomRef {
            id: VRoomId(base + i as u16),
            group,
            area,
            index: i as u8,
            room,
        })
    })
}

/// Room `id`.
#[must_use]
pub fn room(id: VRoomId) -> Option<RoomRef> {
    let mut base = 0u16;
    for (group, area) in all_areas() {
        let n = area.rooms.len() as u16;
        if id.0 < base + n {
            let index = id.0 - base;
            return Some(RoomRef {
                id,
                group,
                area,
                index: index as u8,
                room: &area.rooms[usize::from(index)],
            });
        }
        base += n;
    }
    None
}

/// Number of registered rooms.
#[must_use]
pub fn room_count() -> u16 {
    all_areas().map(|(_, a)| a.rooms.len() as u16).sum()
}

/// Id of room `index` of `area` (the same `&'static` static the registry
/// holds, or an area with the same name and table).
#[must_use]
pub fn room_id(area: &StaticArea, index: u8) -> Option<VRoomId> {
    let mut base = 0u16;
    for (_, a) in all_areas() {
        if (std::ptr::eq(a, area) || (a.name == area.name && a.table == area.table))
            && usize::from(index) < a.rooms.len()
        {
            return Some(VRoomId(base + u16::from(index)));
        }
        base += a.rooms.len() as u16;
    }
    None
}

/// Id of room `index` of the area named `name`.
#[must_use]
pub fn find_room(name: &str, index: u8) -> Option<VRoomId> {
    let mut base = 0u16;
    for (_, a) in all_areas() {
        if a.name == name && usize::from(index) < a.rooms.len() {
            return Some(VRoomId(base + u16::from(index)));
        }
        base += a.rooms.len() as u16;
    }
    None
}

// ------------------------------------------------------------ pages and bits

/// Pages (bit `p` for page `p`) holding a collectable or, in a palace
/// world (3-5), a locked door. Walks the commands the way the loader does
/// (cursor steps in the low nibble, row 14 jumps to the start of page
/// `X & 3`, `B == $0F` on rows 0-12 and 15 is a 3-byte collectable, a
/// small object `$05` on rows 0-12 is a palace door).
#[must_use]
pub fn tracked_pages(world: u8, sideview: &[u8]) -> u8 {
    walk_pages(world, sideview, true)
}

/// Pages holding a collectable (see [`tracked_pages`]).
#[must_use]
pub fn item_pages(sideview: &[u8]) -> u8 {
    walk_pages(0, sideview, false)
}

fn walk_pages(world: u8, sideview: &[u8], doors: bool) -> u8 {
    let len = sideview.first().map_or(0, |&b| usize::from(b));
    let data = &sideview[..len.min(sideview.len())];
    let palace = doors && (3..=5).contains(&world);
    let mut i = 4;
    let mut cursor: u16 = 0;
    let mut pages = 0u8;
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
        let page = 1 << ((cursor / 16) & 3);
        if row != 13 && b == 0x0F {
            pages |= page;
            i += 3;
        } else {
            if palace && row < 13 && b == 0x05 {
                pages |= page;
            }
            i += 2;
        }
    }
    pages
}

/// Pool bit of tracked page `page` of room `id` (`None` when the page is
/// not tracked or the group's budget is spent; [`validate`] rejects the
/// latter).
#[must_use]
pub fn page_bit(id: VRoomId, page: u8) -> Option<u16> {
    let r = room(id)?;
    let info = r.group.info();
    let mut n = 0u16;
    for x in rooms().filter(|x| x.group == r.group) {
        let t = tracked_pages(x.room.world, x.room.sideview);
        for p in 0..4u8 {
            if t & (1 << p) == 0 {
                continue;
            }
            if x.id == id && p == page {
                return (n < info.bits - info.flags).then_some(info.bit_base + n);
            }
            n += 1;
        }
        if x.id > id {
            break;
        }
    }
    None
}

/// Pool bit of free-form flag `i` of `group`.
#[must_use]
pub fn flag_bit(group: Group, i: u16) -> Option<u16> {
    let info = group.info();
    (i < info.flags).then_some(info.bit_base + info.bits - info.flags + i)
}

/// Tracked pages of `group` (bits it needs for pages).
#[must_use]
pub fn group_page_bits(group: Group) -> u16 {
    rooms()
        .filter(|x| x.group == group)
        .map(|x| tracked_pages(x.room.world, x.room.sideview).count_ones() as u16)
        .sum()
}

/// Static checks over the registry: budgets, room ids fit `u16`, every
/// area's world and table agree, lengths fit the buffers, enemy lists are
/// a length byte plus pairs, typed exits name rooms of the same area on
/// pages 0-3, links name rooms and maps 0-62. Slot-dependent checks
/// (an exit byte naming the slot map) are in `content::validate`.
///
/// # Errors
/// The first problem found, in words.
pub fn validate() -> Result<(), String> {
    use super::content_mem::{PROGRESS_POOL_BITS, SETS, VROOM_DATA_STRIDE, VROOM_ENEMY_STRIDE};
    let mut end = 0u16;
    for g in &GROUPS {
        if g.bit_base != end || g.flags > g.bits {
            return Err(format!("group {}: bit range", g.name));
        }
        end += g.bits;
        let need = group_page_bits(g.group);
        if need + g.flags > g.bits {
            return Err(format!(
                "group {}: {need} tracked pages + {} flags, budget {}",
                g.name, g.flags, g.bits
            ));
        }
    }
    if end != PROGRESS_POOL_BITS {
        return Err("group bit ranges do not tile the pool".into());
    }
    if usize::from(room_count()) >= usize::from(u16::MAX) {
        return Err("too many rooms".into());
    }
    for (_, a) in all_areas() {
        let what = |m: String| format!("area {}: {m}", a.name);
        let set = SETS
            .get(usize::from(a.table))
            .ok_or_else(|| what(format!("table {}", a.table)))?;
        if a.rooms.is_empty() || a.rooms.len() > 255 {
            return Err(what("1-255 rooms".into()));
        }
        let worlds_ok = |w: u8| set.worlds.contains(&w);
        if !worlds_ok(a.world) {
            return Err(what(format!("world {} not in {}", a.world, set.name)));
        }
        let room_ok = |d: &Dest| match *d {
            Dest::Byte(b) => b >= 0xFC || b >> 2 < super::content_mem::MAPS,
            Dest::Room { room, page } => usize::from(room) < a.rooms.len() && page < 4,
        };
        for (i, r) in a.rooms.iter().enumerate() {
            let what = |m: &str| format!("area {} room {i}: {m}", a.name);
            if !worlds_ok(r.world) {
                return Err(what("world"));
            }
            let len = r.sideview.first().copied().unwrap_or(0);
            if len < 4
                || usize::from(len) != r.sideview.len()
                || r.sideview.len() > usize::from(VROOM_DATA_STRIDE)
            {
                return Err(what("side-view length byte"));
            }
            let elen = r.enemies.first().copied().unwrap_or(0);
            if elen < 1
                || usize::from(elen) != r.enemies.len()
                || r.enemies.len() > usize::from(VROOM_ENEMY_STRIDE)
                || elen % 2 != 1
            {
                return Err(what("enemy list length"));
            }
            if !r.exits.iter().all(room_ok) || !r.door_exits.iter().flatten().all(room_ok) {
                return Err(what("exit"));
            }
            if r.door_exits.iter().any(Option::is_some) && a.table != 4 {
                return Err(what("doorways outside the towns table"));
            }
        }
        for l in a.links {
            if usize::from(l.room) >= a.rooms.len()
                || l.map >= super::content_mem::MAPS
                || l.index > 3
                || l.page > 3
                || (l.door && a.table != 4)
            {
                return Err(what(format!("bad link {l:?}")));
            }
        }
        for e in a.entrances {
            if usize::from(e.room) >= a.rooms.len() || e.page > 3 {
                return Err(what(format!("bad entrance {e:?}")));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_is_valid_and_ids_round_trip() {
        validate().unwrap();
        let r = room(TEST_ROOM).unwrap();
        assert_eq!(r.group, Group::TestRoom);
        assert_eq!(r.area.name, "Test Room");
        assert_eq!(room_id(r.area, 0), Some(TEST_ROOM));
        assert_eq!(find_room("Test Room", 0), Some(TEST_ROOM));
        assert_eq!(find_room("Test Room", 1), None);
        for x in rooms() {
            let y = room(x.id).unwrap();
            assert!(std::ptr::eq(x.room, y.room));
            assert_eq!(room_id(x.area, x.index), Some(x.id));
            assert_eq!(x.sibling(x.index), Some(x.id));
        }
        assert_eq!(rooms().count(), usize::from(room_count()));
        assert!(room(VRoomId(room_count())).is_none());
    }

    #[test]
    fn pages_and_bits() {
        let sv = room(TEST_ROOM).unwrap().room.sideview;
        assert_eq!(item_pages(sv), 0b0010);
        assert_eq!(tracked_pages(5, sv), 0b0010);
        // Page jump to page 3, then an item two columns in.
        let sv = [
            0x0B, 0x60, 0x01, 0x08, 0xE3, 0x00, 0x92, 0x0F, 0x0A, 0x50, 0x13,
        ];
        assert_eq!(item_pages(&sv), 0b1000);
        assert_eq!(item_pages(&[0x04, 0x60, 0x01, 0x08]), 0);
        assert_eq!(item_pages(&[]), 0);
        // A palace locked door (small object 5) on page 1 is tracked in a
        // palace world only.
        let door = [0x08, 0x20, 0x01, 0x08, 0xE1, 0x00, 0x72, 0x05];
        assert_eq!(tracked_pages(3, &door), 0b0010);
        assert_eq!(tracked_pages(1, &door), 0);
        assert_eq!(page_bit(TEST_ROOM, 1), Some(0));
        assert_eq!(page_bit(TEST_ROOM, 0), None);
        assert_eq!(flag_bit(Group::Minigames, 0), Some(52 + 24 - 8));
        assert_eq!(flag_bit(Group::Minigames, 8), None);
        assert_eq!(flag_bit(Group::ParapaAnnex, 0), None);
    }

    #[test]
    fn options_gate_groups() {
        let mut o = ContentOpts {
            test_room: true,
            dark_isle: true,
            ..ContentOpts::default()
        };
        assert!(!o.enables(Group::TestRoom), "master off");
        o.virtual_rooms = true;
        assert!(o.enables(Group::TestRoom) && o.enables(Group::DarkIsle));
        assert!(!o.enables(Group::Minigames));
        assert_eq!(areas(&o).next().map(|a| a.name), Some("Test Room"));
    }
}
