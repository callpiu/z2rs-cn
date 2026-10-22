//! Compiling an [`Area`] to the game's byte formats, and the static,
//! allocation-free form the engine consumes.
//!
//! [`compile`] gives owned [`CompiledArea`]/[`CompiledRoom`] values (for
//! tools and tests). [`generate_source`] prints the same bytes as Rust
//! `static` items over [`StaticArea`]/[`StaticRoom`]; the output is checked
//! in as `src/generated.rs` and kept current by a test, so the engine's hot
//! path only ever reads `&'static [u8]` slices. [`generate_group_source`]
//! does the same for one of the option groups under [`crate::areas`]
//! (`const` items plus an `AREAS` slice).
//!
//! # Connection bytes
//!
//! Each room has four. Byte `i` is used when Link leaves through page `i`'s
//! side of the room, so the bytes mean (for a 4-page room): 0 = left (or a
//! fall on page 0), 1 = elevator down (or a fall on page 1), 2 = elevator up
//! (or a fall on page 2), 3 = right (or a fall on page 3). A narrower room
//! leaves right through byte `pages - 1`. A value is `map * 4 + page`
//! (where Link appears); [`OUTSIDE`] leads back to the overworld. Arrival
//! pages: through the left edge Link appears on the target's last page,
//! through the right edge on page 0, by elevator on the target's elevator
//! page, by falling on the page he fell from (the game keeps it). Unused bytes
//! hold [`OUTSIDE`]. This matches `z2-rando::palaces::write::connection_bytes`.
//!
//! Doorways ([`Dir::Door`]) are not a vanilla connection byte: they are
//! compiled into [`CompiledRoom::doors`] (one byte per page, same value
//! format, [`NO_DOOR`] when unused) for the engine to dispatch.
//!
//! # What the engine reads
//!
//! The engine runs every virtual room of a table in that table's one
//! reserved map slot, so a room-to-room exit cannot be a map number. It
//! follows the typed [`CompiledRoom::exits`] / [`CompiledRoom::door_exits`]
//! ([`Dest`]: an original connection byte, or room `i` of the same area on
//! a page), the area's [`StaticLink`]s (original exits redirected into the
//! area) and the [`CompiledEntrance::room`] / [`CompiledEntrance::page`] of
//! each overworld entrance. [`CompiledRoom::connections`] and
//! [`CompiledRoom::doors`] write room `i` as map `first_map + i`, the way a
//! ROM hack that gave every room its own map would; tools and tests use
//! them.

use std::fmt::Write as _;

use crate::enemies::{self, Enemy};
use crate::model::{Area, Dir, Exit, LinkVia, Object, Room, Target};
use crate::sideview::{Command, ObjectSet, Sideview, HEADER_LEN};
use crate::text;
use crate::ContentError;

/// Connection value that leads back to the overworld.
pub const OUTSIDE: u8 = 0xFC;
/// Door byte of a page without a doorway.
pub const NO_DOOR: u8 = 0xFF;
/// Highest map number a connection byte can name (`62 * 4 + 3 < $FC`).
pub const MAX_MAP: u8 = 62;

/// Where one exit of a virtual room leads, as the engine follows it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Dest {
    /// An original connection-format byte of the same table:
    /// `map * 4 + page`, or [`OUTSIDE`] (any value `>= $FC`) for the
    /// overworld.
    Byte(u8),
    /// Room `room` (index into the area's rooms) of the same area, Link
    /// appearing on `page`.
    Room {
        /// Room index in the area.
        room: u8,
        /// Arrival page (0-3).
        page: u8,
    },
}

/// One room in the game's byte formats.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompiledRoom {
    /// World (`$0707` value).
    pub world: u8,
    /// Map number in [`Self::connections`] form (`first_map + index`).
    pub map: u8,
    /// Sideview bytes (length byte first).
    pub sideview: Vec<u8>,
    /// Enemy list bytes (length byte first).
    pub enemies: Vec<u8>,
    /// Connection bytes (rooms of the area as `first_map + index`).
    pub connections: [u8; 4],
    /// Doorway destinations per page ([`NO_DOOR`] = none).
    pub doors: [u8; 4],
    /// The four exits, typed (what the engine follows).
    pub exits: [Dest; 4],
    /// Doorway exits per page, typed (`None` = no doorway).
    pub door_exits: [Option<Dest>; 4],
    /// The room's (first) collectable item byte.
    pub item: Option<u8>,
}

/// A talking person: which room, which entry of its enemy list, which
/// message.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompiledNpc {
    /// Index into [`CompiledArea::rooms`].
    pub room: u8,
    /// Index of their entry in the room's enemy list.
    pub enemy_index: u8,
    /// Index into [`CompiledArea::messages`].
    pub message: u8,
}

/// An overworld entrance, compiled.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompiledEntrance {
    /// Region (`$0706` value; 3 = Maze Island).
    pub region: u8,
    /// Tile row.
    pub row: u8,
    /// Tile column.
    pub col: u8,
    /// Tile look (0 cave, 1 town, 2 palace, 3 hidden).
    pub tile: u8,
    /// Connection-format value of where Link appears
    /// (`(first_map + room) * 4 + page`).
    pub target: u8,
    /// Room Link enters (index into the area's rooms).
    pub room: u8,
    /// Page he appears on.
    pub page: u8,
}

/// An overworld cell the area repaints, compiled
/// ([`crate::model::TerrainEdit`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CompiledTerrain {
    /// Region (`$0706` value; 3 = Maze Island).
    pub region: u8,
    /// Tile row.
    pub row: u8,
    /// Tile column.
    pub col: u8,
    /// Terrain code (0-15, [`crate::model::MapTile`]).
    pub tile: u8,
}

/// An original room's exit redirected into the area, compiled
/// ([`crate::model::VanillaLink`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct StaticLink {
    /// Original map of the area's table.
    pub map: u8,
    /// `true`: the doorway on page [`Self::index`] (door table); `false`:
    /// connection byte [`Self::index`].
    pub door: bool,
    /// Connection byte (0-3) or door page (0-3).
    pub index: u8,
    /// Room entered (index into the area's rooms).
    pub room: u8,
    /// Page Link appears on.
    pub page: u8,
}

/// A whole area in byte form.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompiledArea {
    /// Name.
    pub name: &'static str,
    /// World (`$0707` value).
    pub world: u8,
    /// Side-view table set ([`crate::model::Table::index`]).
    pub table: u8,
    /// Rooms in authoring order (room `i` is map `first_map + i`).
    pub rooms: Vec<CompiledRoom>,
    /// Messages (each with its end byte).
    pub messages: Vec<Vec<u8>>,
    /// Talking people.
    pub npcs: Vec<CompiledNpc>,
    /// Entrances.
    pub entrances: Vec<CompiledEntrance>,
    /// Original exits redirected into the area.
    pub links: Vec<StaticLink>,
    /// Overworld cells repainted.
    pub terrain: Vec<CompiledTerrain>,
}

/// The static form of [`CompiledRoom`] (what `src/generated.rs` holds and
/// the engine copies into its WRAM buffers).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StaticRoom {
    /// World (`$0707` value).
    pub world: u8,
    /// Map number in connection-byte form (`first_map + index`).
    pub map: u8,
    /// Sideview bytes.
    pub sideview: &'static [u8],
    /// Enemy list bytes.
    pub enemies: &'static [u8],
    /// Connection bytes.
    pub connections: [u8; 4],
    /// Doorway bytes.
    pub doors: [u8; 4],
    /// The four exits, typed.
    pub exits: [Dest; 4],
    /// Doorway exits per page, typed.
    pub door_exits: [Option<Dest>; 4],
    /// Collectable item.
    pub item: Option<u8>,
}

/// The static form of [`CompiledArea`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StaticArea {
    /// Name.
    pub name: &'static str,
    /// World.
    pub world: u8,
    /// Side-view table set ([`crate::model::Table::index`]).
    pub table: u8,
    /// Rooms.
    pub rooms: &'static [StaticRoom],
    /// Messages.
    pub messages: &'static [&'static [u8]],
    /// Talking people.
    pub npcs: &'static [CompiledNpc],
    /// Entrances.
    pub entrances: &'static [CompiledEntrance],
    /// Original exits redirected into the area.
    pub links: &'static [StaticLink],
    /// Overworld cells repainted.
    pub terrain: &'static [CompiledTerrain],
}

impl StaticArea {
    /// The room written as map `map` in connection-byte form.
    #[must_use]
    pub fn room_for_map(&self, map: u8) -> Option<&'static StaticRoom> {
        self.rooms.iter().find(|r| r.map == map)
    }
}

impl CompiledRoom {
    /// Same bytes as the static form?
    #[must_use]
    pub fn matches(&self, s: &StaticRoom) -> bool {
        self.world == s.world
            && self.map == s.map
            && self.sideview == s.sideview
            && self.enemies == s.enemies
            && self.connections == s.connections
            && self.doors == s.doors
            && self.exits == s.exits
            && self.door_exits == s.door_exits
            && self.item == s.item
    }
}

impl CompiledArea {
    /// Same bytes as the static form?
    #[must_use]
    pub fn matches(&self, s: &StaticArea) -> bool {
        self.name == s.name
            && self.world == s.world
            && self.table == s.table
            && self.rooms.len() == s.rooms.len()
            && self.rooms.iter().zip(s.rooms).all(|(a, b)| a.matches(b))
            && self.messages.len() == s.messages.len()
            && self.messages.iter().zip(s.messages).all(|(a, b)| a == b)
            && self.npcs == s.npcs
            && self.entrances == s.entrances
            && self.links == s.links
            && self.terrain == s.terrain
    }
}

/// Rows of a room (0-12).
const ROWS_IN_ROOM: u8 = 13;

/// The shaft an elevator at column `x` rides through, drawn right after
/// it: background columns (palace kind D, Great Palace kind E) through the
/// ceiling (rows 0-7) when the room has an `Up` exit and through the floor
/// (rows 10-12) when it has a `Down` exit, over both of its columns.
/// Measured: the cabin rests at `$2A = $98` (Link rides on rows 8-9), and
/// a solid ceiling or floor tile stops it short of the exit, so without
/// the shaft neither ride leaves the room. World 0 and towns get none
/// (no authored room there has an elevator).
#[must_use]
pub fn elevator_shaft(room: &Room, set: ObjectSet, x: u8) -> Vec<Command> {
    let kind = match set {
        ObjectSet::Palace => 0xD0,
        ObjectSet::GreatPalace => 0xE0,
        ObjectSet::Other => return Vec::new(),
    };
    let has = |d: Dir| room.exits.iter().any(|e| e.dir == d);
    let mut v = Vec::new();
    for (dir, y, rows) in [(Dir::Up, 0u8, 8u8), (Dir::Down, 10, 3)] {
        if has(dir) {
            for dx in 0..2 {
                v.push(Command::new(x + dx, y, kind | (rows - 1)));
            }
        }
    }
    v
}

/// Sideview of `room` (objects in authoring order; an elevator is
/// followed by its [`elevator_shaft`]).
pub fn room_sideview(room: &Room, area: &Area) -> Result<Sideview, ContentError> {
    if !(1..=4).contains(&room.pages) {
        return Err(ContentError::Encode(format!(
            "room {}: {} pages, rooms have 1-4",
            room.id, room.pages
        )));
    }
    if room.floor > 15 || room.ground > 7 || room.palette > 31 || room.background > 7 {
        return Err(ContentError::Encode(format!(
            "room {}: header field out of range",
            room.id
        )));
    }
    let set = area.world.object_set();
    let header: [u8; HEADER_LEN] = [
        0,
        (u8::from(room.alt_objects) << 7) | ((room.pages - 1) << 5),
        (u8::from(room.no_ceiling) << 7) | (room.ground << 4) | room.floor,
        (room.palette << 3) | room.background,
    ];
    let mut commands = Vec::with_capacity(room.objects.len() + 4);
    for (k, o) in room.objects.iter().enumerate() {
        // A pit is one command per column; in a palace a column that a
        // later span (bridge, crumbling bridge, ledge) covers is cleared
        // only below it, so the span survives the column order below.
        if let Object::Pit { x, width } = *o {
            if set != ObjectSet::GreatPalace {
                for cx in x..x.saturating_add(width) {
                    let below = room.objects[k + 1..]
                        .iter()
                        .filter_map(|p| match *p {
                            Object::Bridge { x, y, len }
                            | Object::Crumble { x, y, len }
                            | Object::Platform { x, y, len }
                            | Object::Breakable { x, y, len }
                                if (x..x + len).contains(&cx) =>
                            {
                                Some(y + 1)
                            }
                            Object::Block { x, y, len }
                            | Object::BreakableBlock { x, y, len }
                            | Object::FalseWall { x, y, len }
                                if (x..x + len).contains(&cx) =>
                            {
                                Some(y + 2)
                            }
                            _ => None,
                        })
                        .max();
                    let c = match (set, below) {
                        (ObjectSet::Palace, Some(top)) if top < ROWS_IN_ROOM => {
                            Command::new(cx, top, 0xD0 | (ROWS_IN_ROOM - 1 - top))
                        }
                        (ObjectSet::Palace, Some(_)) => continue,
                        _ => Object::Pit { x: cx, width: 1 }
                            .to_command(set)
                            .map_err(|e| ContentError::Encode(format!("room {}: {e}", room.id)))?,
                    };
                    commands.push(c);
                }
                continue;
            }
        }
        let c = o
            .to_command(set)
            .map_err(|e| ContentError::Encode(format!("room {}: {e}", room.id)))?;
        commands.push(c);
        if let Object::Elevator { x } = *o {
            commands.extend(elevator_shaft(room, set, x));
        }
    }
    // The game builds the room in one sweep across the columns: an object
    // behind the sweep (after a page jump back) is only partly drawn
    // (measured: a crumbling span listed after the pit strips it covers
    // lost its first column). In column order, floors first, the rest in
    // authoring order, it draws as written.
    commands.sort_by_key(|c| (c.x, u8::from(!c.is_floor())));
    // (Rows of a room: 13.)
    const _: () = assert!(ROWS_IN_ROOM == 13);
    Ok(Sideview { header, commands })
}

/// Enemy list of `room`: its enemies, then the area's people standing in
/// it (in the order they are listed).
#[must_use]
pub fn room_enemies(room: &Room, area: &Area) -> Vec<Enemy> {
    room.enemies
        .iter()
        .chain(
            area.npcs
                .iter()
                .filter(|n| n.room == room.id)
                .map(|n| &n.spawn),
        )
        .map(|e| Enemy {
            x: e.x,
            y: e.y,
            id: e.id,
        })
        .collect()
}

fn index_of(area: &Area, id: &str) -> Result<u8, ContentError> {
    area.rooms
        .iter()
        .position(|r| r.id == id)
        .map(|i| i as u8)
        .ok_or_else(|| ContentError::Encode(format!("no room {id:?}")))
}

/// The page Link appears on when he enters `target` through `dir`.
fn arrival_page(r: &Room, dir: Dir) -> u8 {
    let page = match dir {
        Dir::Left => r.pages.clamp(1, 4) - 1,
        Dir::Right | Dir::Door { .. } => 0,
        Dir::Up | Dir::Down => r.elevator_page().unwrap_or(0),
        // A fall keeps the page Link fell on (measured: `LC67C` skips the
        // byte's page bits).
        Dir::Drop { page } => page,
    };
    page & 3
}

/// The typed destination of leaving through `dir` to `target`.
pub fn target_dest(area: &Area, dir: Dir, target: Target) -> Result<Dest, ContentError> {
    match target {
        Target::Outside => Ok(Dest::Byte(OUTSIDE)),
        Target::OutsideAt(n) if n <= 3 => Ok(Dest::Byte(OUTSIDE | n)),
        Target::OutsideAt(n) => Err(ContentError::Encode(format!(
            "outside offset {n} not in 0-3"
        ))),
        Target::Map { map, page } => {
            if map > MAX_MAP || page > 3 {
                return Err(ContentError::Encode(format!(
                    "map {map} page {page} out of range"
                )));
            }
            Ok(Dest::Byte((map << 2) | page))
        }
        Target::Room(id) => {
            let room = index_of(area, id)?;
            let r = &area.rooms[usize::from(room)];
            Ok(Dest::Room {
                room,
                page: arrival_page(r, dir),
            })
        }
    }
}

/// The connection-byte form of `d` (room `i` as map `first_map + i`).
#[must_use]
pub fn dest_byte(area: &Area, d: Dest) -> u8 {
    match d {
        Dest::Byte(b) => b,
        Dest::Room { room, page } => (area.first_map.wrapping_add(room) << 2) | (page & 3),
    }
}

/// The connection-format byte for arriving in `target` through `dir`.
pub fn target_byte(area: &Area, dir: Dir, target: Target) -> Result<u8, ContentError> {
    target_dest(area, dir, target).map(|d| dest_byte(area, d))
}

/// Which connection byte an exit uses (`None` for doorways).
#[must_use]
pub fn connection_slot(dir: Dir, pages: u8) -> Option<usize> {
    match dir {
        Dir::Left => Some(0),
        Dir::Right => Some(usize::from(pages.clamp(1, 4) - 1)),
        Dir::Down => Some(1),
        Dir::Up => Some(2),
        Dir::Drop { page } => Some(usize::from(page & 3)),
        Dir::Door { .. } => None,
    }
}

/// Typed exits of `room`: connection bytes 0-3 and doorways per page. Two
/// exits that need the same byte with different values are an error.
pub fn room_exits(
    room: &Room,
    area: &Area,
) -> Result<([Dest; 4], [Option<Dest>; 4]), ContentError> {
    let mut conn = [Dest::Byte(OUTSIDE); 4];
    let mut doors: [Option<Dest>; 4] = [None; 4];
    let mut owner: [Option<&Exit>; 4] = [None; 4];
    for e in &room.exits {
        let v = target_dest(area, e.dir, e.target)
            .map_err(|err| ContentError::Encode(format!("room {}: {err}", room.id)))?;
        match connection_slot(e.dir, room.pages) {
            Some(i) => {
                if let Some(prev) = owner[i] {
                    if conn[i] != v {
                        return Err(ContentError::Encode(format!(
                            "room {}: exits {:?} and {:?} both need connection byte {i}",
                            room.id, prev.dir, e.dir
                        )));
                    }
                }
                owner[i] = Some(e);
                conn[i] = v;
            }
            None => {
                let Dir::Door { x } = e.dir else {
                    unreachable!()
                };
                let p = usize::from(x / 16).min(3);
                if doors[p].is_some_and(|d| d != v) {
                    return Err(ContentError::Encode(format!(
                        "room {}: two doorways on page {p}",
                        room.id
                    )));
                }
                doors[p] = Some(v);
            }
        }
    }
    Ok((conn, doors))
}

/// Connection and door bytes of `room` ([`room_exits`] in byte form).
pub fn room_connections(room: &Room, area: &Area) -> Result<([u8; 4], [u8; 4]), ContentError> {
    let (conn, doors) = room_exits(room, area)?;
    Ok((
        conn.map(|d| dest_byte(area, d)),
        doors.map(|d| d.map_or(NO_DOOR, |d| dest_byte(area, d))),
    ))
}

/// Compile one room.
pub fn compile_room(room: &Room, area: &Area) -> Result<CompiledRoom, ContentError> {
    let idx = area
        .room_index(room.id)
        .ok_or_else(|| ContentError::Encode(format!("room {} is not in the area", room.id)))?;
    let sideview = room_sideview(room, area)?
        .encode()
        .map_err(|e| ContentError::Encode(format!("room {}: {e}", room.id)))?;
    let enemies = enemies::encode_list(&room_enemies(room, area))
        .map_err(|e| ContentError::Encode(format!("room {}: {e}", room.id)))?;
    let (exits, door_exits) = room_exits(room, area)?;
    let (connections, doors) = room_connections(room, area)?;
    Ok(CompiledRoom {
        world: area.world.id(),
        map: area.first_map + idx as u8,
        sideview,
        enemies,
        connections,
        doors,
        exits,
        door_exits,
        item: room.item(),
    })
}

/// Compile a whole area. Does not validate it (see
/// [`crate::validate::validate`]); it fails only when something has no
/// byte encoding at all.
pub fn compile(area: &Area) -> Result<CompiledArea, ContentError> {
    let rooms = area
        .rooms
        .iter()
        .map(|r| compile_room(r, area))
        .collect::<Result<Vec<_>, _>>()?;
    let mut messages = Vec::new();
    let mut npcs = Vec::new();
    for n in &area.npcs {
        let room = area
            .room_index(n.room)
            .ok_or_else(|| ContentError::Encode(format!("person in missing room {}", n.room)))?;
        let before = area.rooms[room].enemies.len()
            + area
                .npcs
                .iter()
                .take_while(|m| !std::ptr::eq(*m, n))
                .filter(|m| m.room == n.room)
                .count();
        messages.push(text::encode_message(n.message.text)?);
        npcs.push(CompiledNpc {
            room: room as u8,
            enemy_index: before as u8,
            message: (messages.len() - 1) as u8,
        });
    }
    let entrances = area
        .entrances
        .iter()
        .map(|e| {
            let room = index_of(area, e.room)?;
            Ok(CompiledEntrance {
                region: e.region.id(),
                row: e.row,
                col: e.col,
                tile: e.tile as u8,
                target: (area.first_map.wrapping_add(room) << 2) | (e.page & 3),
                room,
                page: e.page & 3,
            })
        })
        .collect::<Result<Vec<_>, ContentError>>()?;
    let links = area
        .links
        .iter()
        .map(|l| {
            let room = index_of(area, l.room)?;
            let (door, index) = match l.via {
                LinkVia::Byte(b) => (false, b),
                LinkVia::Door(p) => (true, p),
            };
            if l.map > MAX_MAP || index > 3 || l.page > 3 {
                return Err(ContentError::Encode(format!(
                    "link from map {} exit {index} page {} out of range",
                    l.map, l.page
                )));
            }
            Ok(StaticLink {
                map: l.map,
                door,
                index,
                room,
                page: l.page,
            })
        })
        .collect::<Result<Vec<_>, ContentError>>()?;
    let terrain = area
        .terrain
        .iter()
        .map(|t| CompiledTerrain {
            region: t.region.id(),
            row: t.row,
            col: t.col,
            tile: t.tile.code(),
        })
        .collect();
    Ok(CompiledArea {
        name: area.name,
        world: area.world.id(),
        table: area.table().index(),
        rooms,
        messages,
        npcs,
        entrances,
        links,
        terrain,
    })
}

fn bytes(out: &mut String, b: &[u8]) {
    out.push('[');
    for (i, v) in b.iter().enumerate() {
        if i > 0 {
            out.push_str(", ");
        }
        let _ = write!(out, "0x{v:02X}");
    }
    out.push(']');
}

fn dest_src(d: Dest) -> String {
    match d {
        Dest::Byte(b) => format!("Dest::Byte(0x{b:02X})"),
        Dest::Room { room, page } => format!("Dest::Room {{ room: {room}, page: {page} }}"),
    }
}

/// One `StaticArea { .. }` literal (the value after `=`), indented for a
/// top-level item.
fn area_literal(s: &mut String, c: &CompiledArea) {
    let _ = write!(
        s,
        "StaticArea {{\n    name: {:?},\n    world: {},\n    table: {},\n    rooms: &[\n",
        c.name, c.world, c.table
    );
    for r in &c.rooms {
        let _ = write!(
            s,
            "        StaticRoom {{\n            world: {},\n            map: {},\n            sideview: &",
            r.world, r.map
        );
        bytes(s, &r.sideview);
        s.push_str(",\n            enemies: &");
        bytes(s, &r.enemies);
        s.push_str(",\n            connections: ");
        bytes(s, &r.connections);
        s.push_str(",\n            doors: ");
        bytes(s, &r.doors);
        s.push_str(",\n            exits: [");
        for (i, d) in r.exits.iter().enumerate() {
            if i > 0 {
                s.push_str(", ");
            }
            s.push_str(&dest_src(*d));
        }
        s.push_str("],\n            door_exits: [");
        for (i, d) in r.door_exits.iter().enumerate() {
            if i > 0 {
                s.push_str(", ");
            }
            match d {
                Some(d) => {
                    let _ = write!(s, "Some({})", dest_src(*d));
                }
                None => s.push_str("None"),
            }
        }
        let _ = write!(
            s,
            "],\n            item: {},\n        }},\n",
            match r.item {
                Some(i) => format!("Some(0x{i:02X})"),
                None => "None".into(),
            }
        );
    }
    s.push_str("    ],\n    messages: &[");
    for m in &c.messages {
        s.push_str("\n        &");
        bytes(s, m);
        s.push(',');
    }
    if !c.messages.is_empty() {
        s.push_str("\n    ");
    }
    s.push_str("],\n    npcs: &[");
    for n in &c.npcs {
        let _ = write!(
            s,
            "\n        CompiledNpc {{\n            room: {},\n            enemy_index: {},\n            message: {},\n        }},",
            n.room, n.enemy_index, n.message
        );
    }
    if !c.npcs.is_empty() {
        s.push_str("\n    ");
    }
    s.push_str("],\n    entrances: &[");
    for e in &c.entrances {
        let _ = write!(
            s,
            "\n        CompiledEntrance {{\n            region: {},\n            row: {},\n            col: {},\n            tile: {},\n            target: 0x{:02X},\n            room: {},\n            page: {},\n        }},",
            e.region, e.row, e.col, e.tile, e.target, e.room, e.page
        );
    }
    if !c.entrances.is_empty() {
        s.push_str("\n    ");
    }
    s.push_str("],\n    links: &[");
    for l in &c.links {
        let _ = write!(
            s,
            "\n        StaticLink {{\n            map: {},\n            door: {},\n            index: {},\n            room: {},\n            page: {},\n        }},",
            l.map, l.door, l.index, l.room, l.page
        );
    }
    if !c.links.is_empty() {
        s.push_str("\n    ");
    }
    s.push_str("],\n    terrain: &[");
    for t in &c.terrain {
        let _ = write!(
            s,
            "\n        CompiledTerrain {{ region: {}, row: {}, col: {}, tile: {} }},",
            t.region, t.row, t.col, t.tile
        );
    }
    if !c.terrain.is_empty() {
        s.push_str("\n    ");
    }
    s.push_str("],\n}");
}

const USES: &str = "#[allow(unused_imports)]\n\
     use crate::compile::{\n    CompiledEntrance, CompiledNpc, CompiledTerrain, Dest, StaticArea, StaticLink, StaticRoom,\n};\n";

/// Rust source for `areas` as `pub static <IDENT>: StaticArea` items
/// (the content of `src/generated.rs`). Deterministic: same areas, same
/// text.
pub fn generate_source(areas: &[(&str, &Area)]) -> Result<String, ContentError> {
    let mut s = String::new();
    s.push_str(
        "//! Generated by `z2_content::compile::generate_source` from the areas in\n\
         //! `src/samples.rs`. Do not edit by hand: run\n\
         //! `Z2_CONTENT_BLESS=1 cargo test -p z2-content generated` to refresh.\n\
         //! All bytes are original authored content (no ROM data).\n\n",
    );
    s.push_str(USES);
    for (ident, area) in areas {
        let c = compile(area)?;
        let _ = write!(
            s,
            "\n/// `{}` ({} rooms).\npub static {ident}: StaticArea = ",
            c.name,
            c.rooms.len()
        );
        area_literal(&mut s, &c);
        s.push_str(";\n");
    }
    Ok(s)
}

/// Rust source for one option group under [`crate::areas`]: a
/// `pub const <IDENT>: StaticArea` per area and `pub static AREAS`, the
/// slice the engine registers (`origin` names the authoring module in the
/// header). Deterministic.
pub fn generate_group_source(
    areas: &[(&str, &Area)],
    origin: &str,
) -> Result<String, ContentError> {
    let mut s = String::new();
    let _ = write!(
        s,
        "//! Generated by `z2_content::compile::generate_group_source` from\n\
         //! `{origin}`. Do not edit by hand: run\n\
         //! `Z2_CONTENT_BLESS=1 cargo test -p z2-content areas` to refresh.\n\
         //! All bytes are original authored content (no ROM data).\n\n"
    );
    s.push_str(USES);
    for (ident, area) in areas {
        let c = compile(area)?;
        let _ = write!(
            s,
            "\n/// `{}` ({} rooms).\npub const {ident}: StaticArea = ",
            c.name,
            c.rooms.len()
        );
        area_literal(&mut s, &c);
        s.push_str(";\n");
    }
    s.push_str("\n/// Every area of this group, in registration order.\npub static AREAS: &[StaticArea] = &[");
    for (i, (ident, _)) in areas.iter().enumerate() {
        if i > 0 {
            s.push_str(", ");
        }
        s.push_str(ident);
    }
    s.push_str("];\n");
    Ok(s)
}
