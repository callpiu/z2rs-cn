//! Checking an [`Area`] before it ships.
//!
//! Two levels:
//!
//! * [`validate`] needs nothing but the area. It checks the byte limits
//!   (sideview at most 255 bytes, 1-4 pages, rows 0-12), object and enemy
//!   bounds, enemy counts, that every exit resolves and its connection byte
//!   is free, that pits have somewhere to fall to, that dialog fits the
//!   box, and that with [`Area::needs`] every room and item can be reached
//!   from an entrance and every reached room can get back out (the area
//!   graph, [`reachable`]).
//! * [`validate_geometry`] also needs the game's floor/ceiling patterns
//!   (from the player's ROM, [`FloorTable::from_body`]). It replays each
//!   room on the solid-tile grid and checks, with a movement model ported
//!   from `z2-rando::palaces::reach`, that every entry reaches every exit
//!   and item with no more than the abilities the room declares. With
//!   [`Abilities::DOWNSTAB`] the cell of each of the room's enemies also
//!   counts as something to land on (a down-stab bounces Link off it), so
//!   a chain of enemies in the air can stand in for ledges.

use std::collections::VecDeque;
use std::fmt;

use crate::compile::{self, MAX_MAP};
use crate::enemies::Enemy;
use crate::model::{Abilities, Area, Dir, Object, Room, RoomId, Target};
use crate::sideview::{
    object_height, object_width, Cell, FloorTable, Grid, MAX_LEN, PAGE_COLS, ROWS,
};
use crate::text;

/// Most list entries (enemies, people) in one room.
pub const MAX_ENEMIES: usize = 24;
/// Most enemies plus items on one page: the game has six object slots.
pub const MAX_PER_PAGE: usize = 6;

/// One problem found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Issue {
    /// Room it is about, if any.
    pub room: Option<RoomId>,
    /// What is wrong.
    pub message: String,
}

impl fmt::Display for Issue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.room {
            Some(r) => write!(f, "room {r}: {}", self.message),
            None => write!(f, "{}", self.message),
        }
    }
}

struct Issues(Vec<Issue>);

impl Issues {
    fn room(&mut self, r: &Room, m: impl Into<String>) {
        self.0.push(Issue {
            room: Some(r.id),
            message: m.into(),
        });
    }
    fn area(&mut self, m: impl Into<String>) {
        self.0.push(Issue {
            room: None,
            message: m.into(),
        });
    }
    fn done(self) -> Result<(), Vec<Issue>> {
        if self.0.is_empty() {
            Ok(())
        } else {
            Err(self.0)
        }
    }
}

/// What the area graph gives with some abilities.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reach {
    /// Per room: reachable from an entrance.
    pub rooms: Vec<bool>,
    /// Per room: its items can be collected.
    pub items: Vec<bool>,
    /// Per room: a way back out (to the overworld or a vanilla map) exists.
    pub escapes: Vec<bool>,
}

/// Walk the area graph from its entrances and links with `have`. An exit
/// can be taken from a reached room when `have` covers the room's and the
/// exit's requirements; [`Target::Outside`] and [`Target::Map`] count as
/// ways out.
#[must_use]
pub fn reachable(area: &Area, have: Abilities) -> Reach {
    let n = area.rooms.len();
    let mut rooms = vec![false; n];
    let mut q = VecDeque::new();
    let ways_in = area
        .entrances
        .iter()
        .map(|e| (e.requires, e.room))
        .chain(area.links.iter().map(|l| (l.requires, l.room)));
    for (requires, room) in ways_in {
        if requires.within(have) {
            if let Some(i) = area.room_index(room) {
                if !rooms[i] {
                    rooms[i] = true;
                    q.push_back(i);
                }
            }
        }
    }
    let usable = |r: &Room, e: &crate::model::Exit| r.requires.with(e.requires).within(have);
    while let Some(i) = q.pop_front() {
        let r = &area.rooms[i];
        for e in r.exits.iter().filter(|e| usable(r, e)) {
            if let Target::Room(t) = e.target {
                if let Some(j) = area.room_index(t) {
                    if !rooms[j] {
                        rooms[j] = true;
                        q.push_back(j);
                    }
                }
            }
        }
    }
    let items = area
        .rooms
        .iter()
        .enumerate()
        .map(|(i, r)| rooms[i] && r.requires.with(r.item_requires).within(have))
        .collect();
    // Ways out: fixpoint over "has an exit to outside or to a room that
    // escapes".
    let mut escapes = vec![false; n];
    loop {
        let mut changed = false;
        for (i, r) in area.rooms.iter().enumerate() {
            if escapes[i] {
                continue;
            }
            let out = r
                .exits
                .iter()
                .filter(|e| usable(r, e))
                .any(|e| match e.target {
                    Target::Outside | Target::OutsideAt(_) | Target::Map { .. } => true,
                    Target::Room(t) => area.room_index(t).is_some_and(|j| escapes[j]),
                });
            if out {
                escapes[i] = true;
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    Reach {
        rooms,
        items,
        escapes,
    }
}

fn check_room(area: &Area, r: &Room, out: &mut Issues) {
    let set = area.world.object_set();
    let width = r.width();
    if !(1..=4).contains(&r.pages) {
        out.room(r, format!("{} pages; rooms have 1-4", r.pages));
        return;
    }
    if r.floor > 15 || r.ground > 7 || r.palette > 31 || r.background > 7 {
        out.room(r, "floor/ground/palette/background out of range");
    }
    if r.objects.is_empty() {
        // The loader reads the first command slot even when the length
        // byte says there is none (measured: it read past an empty room's
        // header into stale bytes).
        out.room(
            r,
            "no objects; give the room at least one command (a floor command will do)",
        );
    }
    // Objects.
    let mut elevators = 0;
    let mut pit_pages = [false; 4];
    let mut items_per_page = [0usize; 4];
    for o in &r.objects {
        // (A wide pit compiles to one command per column; check one.)
        let one = match *o {
            Object::Pit { x, .. } => Object::Pit { x, width: 1 },
            o => o,
        };
        let c = match one.to_command(set) {
            Ok(c) => c,
            Err(e) => {
                out.room(r, e.to_string());
                continue;
            }
        };
        let x = usize::from(c.x);
        let w = match *o {
            Object::Pit { width, .. } => usize::from(width),
            _ => usize::from(object_width(&c, set)),
        };
        if x + w > width {
            out.room(r, format!("{o:?} runs past column {}", width - 1));
        }
        if !c.is_extended() && !c.is_floor() {
            let h = usize::from(object_height(&c, set));
            if usize::from(c.y) + h > ROWS {
                out.room(r, format!("{o:?} runs below row {}", ROWS - 1));
            }
        }
        if area.world == crate::model::World::Overworld
            && matches!(o, Object::Block { .. })
            && !r.alt_objects
        {
            // Measured: kind 2 is rock only with the alternate (cave)
            // object tiles; otherwise Link falls through it.
            out.room(
                r,
                format!("{o:?}: a world-0 block needs the cave object tiles (alt_objects)"),
            );
        }
        match o {
            Object::Elevator { .. } => elevators += 1,
            Object::Pit { x, width: pw } => {
                for col in usize::from(*x)..(usize::from(*x) + usize::from(*pw)).min(width) {
                    pit_pages[col / PAGE_COLS] = true;
                }
            }
            _ => {}
        }
        if c.is_collectable() && x < width {
            items_per_page[x / PAGE_COLS] += 1;
        }
    }
    if elevators > 1 {
        out.room(r, "more than one elevator");
    }
    match compile::room_sideview(r, area).map(|sv| sv.encode()) {
        Ok(Ok(bytes)) if bytes.len() <= MAX_LEN => {}
        Ok(Ok(bytes)) => out.room(r, format!("sideview is {} bytes", bytes.len())),
        Ok(Err(e)) => out.room(r, e.to_string()),
        Err(e) => out.room(r, e.to_string()),
    }
    // Enemies (and the people standing here).
    let list = compile::room_enemies(r, area);
    if list.len() > MAX_ENEMIES {
        out.room(
            r,
            format!("{} list entries; at most {MAX_ENEMIES}", list.len()),
        );
    }
    let mut per_page = items_per_page;
    for e in &list {
        if usize::from(e.x) >= width || e.id > 63 || !Enemy::encodable_row(e.y) {
            out.room(
                r,
                format!("enemy {e:?} is outside the room or not encodable"),
            );
            continue;
        }
        if e.y == 10 {
            // Measured (every world): a list entry on row 10 (raw row
            // code 8) never spawns. One row up spawns and drops.
            out.room(r, format!("enemy {e:?} on row 10 never spawns; use row 9"));
        }
        per_page[usize::from(e.x) / PAGE_COLS] += 1;
    }
    for (p, n) in per_page.iter().enumerate() {
        if *n > MAX_PER_PAGE {
            out.room(
                r,
                format!("page {p} has {n} enemies and items; at most {MAX_PER_PAGE}"),
            );
        }
    }
    // Exits.
    let mut drops = [false; 4];
    for e in &r.exits {
        match e.dir {
            Dir::Up | Dir::Down if elevators == 0 => {
                out.room(r, format!("{:?} exit without an elevator", e.dir));
            }
            Dir::Drop { page } if page >= r.pages => {
                out.room(r, format!("drop on page {page} of a {}-page room", r.pages));
            }
            Dir::Drop { page } => drops[usize::from(page)] = true,
            Dir::Door { x } if usize::from(x) >= width => {
                out.room(r, format!("doorway at column {x} is outside the room"));
            }
            _ => {}
        }
        match e.target {
            Target::Room(t) => match area.room(t) {
                None => out.room(r, format!("exit {:?} leads to missing room {t:?}", e.dir)),
                Some(tr) => match e.dir {
                    Dir::Up | Dir::Down if tr.elevator_page().is_none() => out.room(
                        r,
                        format!("{:?} exit into {t}, which has no elevator", e.dir),
                    ),
                    // A fall keeps its page: the room below must have it.
                    Dir::Drop { page } if page >= tr.pages => out.room(
                        r,
                        format!(
                            "drop from page {page} into {t}, which has {} pages",
                            tr.pages
                        ),
                    ),
                    _ => {}
                },
            },
            Target::Map { map, page } if map > MAX_MAP || page > 3 => {
                out.room(r, format!("vanilla map {map} page {page} out of range"));
            }
            _ => {}
        }
    }
    if let Err(e) = compile::room_connections(r, area) {
        out.room(r, e.to_string());
    }
    for (p, pit) in pit_pages.iter().enumerate() {
        if *pit && !drops[p] {
            out.room(r, format!("pit on page {p} but no drop exit for it"));
        }
    }
    if let Some(l) = r.landing {
        if l >= r.pages {
            out.room(r, format!("landing page {l} outside the room"));
        }
    }
    let has_door = r
        .objects
        .iter()
        .any(|o| matches!(o, Object::LockedDoor { .. }));
    let key_declared = r.requires.has(Abilities::KEY)
        || r.item_requires.has(Abilities::KEY)
        || r.exits.iter().any(|e| e.requires.has(Abilities::KEY));
    if has_door && !key_declared {
        out.room(r, "locked door but nothing declares KEY");
    }
    if r.exits.is_empty() {
        out.room(r, "no exits");
    }
}

/// Check `area` (see the module docs). Returns every problem found.
pub fn validate(area: &Area) -> Result<(), Vec<Issue>> {
    let mut out = Issues(Vec::new());
    if area.rooms.is_empty() {
        out.area("no rooms");
        return out.done();
    }
    if usize::from(area.first_map) + area.rooms.len() - 1 > usize::from(MAX_MAP) {
        out.area(format!(
            "maps {}..{} pass the last map slot {MAX_MAP}",
            area.first_map,
            usize::from(area.first_map) + area.rooms.len() - 1
        ));
    }
    for (i, r) in area.rooms.iter().enumerate() {
        if r.id.is_empty() {
            out.room(r, "empty id");
        }
        if area.rooms[..i].iter().any(|o| o.id == r.id) {
            out.room(r, "id used twice");
        }
        check_room(area, r, &mut out);
        if area.world == crate::model::World::Overworld
            && r.objects
                .iter()
                .any(|o| matches!(o, Object::Platform { .. }))
        {
            out.room(
                r,
                "Platform (large object kind 1) is not in the world-0 object set; it crashes the cave loader",
            );
        }
    }
    if area.entrances.is_empty() && area.links.is_empty() {
        out.area("no entrances and no links");
    }
    for l in &area.links {
        let (kind, index) = match l.via {
            crate::model::LinkVia::Byte(b) => ("connection byte", b),
            crate::model::LinkVia::Door(p) => ("door page", p),
        };
        if l.map > MAX_MAP || index > 3 {
            out.area(format!(
                "link from map {} {kind} {index} out of range",
                l.map
            ));
        }
        if matches!(l.via, crate::model::LinkVia::Door(_))
            && area.table() != crate::model::Table::Towns
        {
            out.area(format!(
                "door link from map {}: only towns have doors",
                l.map
            ));
        }
        match area.room(l.room) {
            None => out.area(format!("link into missing room {:?}", l.room)),
            Some(r) if l.page >= r.pages => {
                out.area(format!("link onto page {} of {}", l.page, r.id));
            }
            _ => {}
        }
        if area
            .links
            .iter()
            .filter(|o| (o.map, o.via) == (l.map, l.via))
            .count()
            > 1
        {
            out.area(format!("map {} {kind} {index} linked twice", l.map));
        }
    }
    for e in &area.entrances {
        if e.row >= crate::model::region_rows(e.region) || e.col >= crate::model::OVERWORLD_COLS {
            out.area(format!("entrance at ({}, {}) is off the map", e.row, e.col));
        }
        if area.world == crate::model::World::Overworld
            && crate::model::Table::of(area.world, e.region) != area.table()
        {
            out.area(format!(
                "entrance at ({}, {}): a cave is entered from its own region ({:?})",
                e.row, e.col, area.region
            ));
        }
        if area
            .entrances
            .iter()
            .filter(|o| o.region == e.region)
            .count()
            > 4
        {
            out.area(format!("more than four entrances in {:?}", e.region));
        }
        if area
            .entrances
            .iter()
            .filter(|o| (o.region, o.row, o.col) == (e.region, e.row, e.col))
            .count()
            > 1
        {
            out.area(format!("two entrances at ({}, {})", e.row, e.col));
        }
        match area.room(e.room) {
            None => out.area(format!("entrance into missing room {:?}", e.room)),
            Some(r) if e.page >= r.pages => {
                out.area(format!("entrance onto page {} of {}", e.page, r.id));
            }
            _ => {}
        }
    }
    for t in &area.terrain {
        if t.row >= crate::model::region_rows(t.region) || t.col >= crate::model::OVERWORLD_COLS {
            out.area(format!("terrain at ({}, {}) is off the map", t.row, t.col));
        }
    }
    for n in &area.npcs {
        if area.room(n.room).is_none() {
            out.area(format!("person in missing room {:?}", n.room));
        }
        if let Err(e) = text::validate_dialog(n.message.text, text::MAX_LINES)
            .and_then(|()| text::encode_message(n.message.text).map(|_| ()))
        {
            out.area(format!("message {:?}: {e}", n.message.text));
        }
    }
    if area.npcs.len() > 255 {
        out.area("more than 255 people");
    }
    let reach = reachable(area, area.needs);
    for (i, r) in area.rooms.iter().enumerate() {
        if !reach.rooms[i] {
            out.room(r, format!("not reachable with {:?}", area.needs));
        } else if !reach.escapes[i] {
            out.room(r, format!("no way back out with {:?}", area.needs));
        }
        if reach.rooms[i] && r.item().is_some() && !reach.items[i] {
            out.room(r, format!("item not reachable with {:?}", area.needs));
        }
    }
    out.done()
}

// ---------------------------------------------------------------------------
// Geometry (ported from z2-rando::palaces::reach, extended with locked
// doors, per-page drops, doorways and items).
// ---------------------------------------------------------------------------

/// Row Link's feet are on when he stands on an elevator cabin at rest.
const CABIN_ROW: usize = 9;

/// Rows Link clears with a plain jump, and with the jump spell.
const JUMP_ROWS: [usize; 2] = [2, 4];
/// Columns Link clears sideways with a plain jump, and with the spell.
const JUMP_COLS: [usize; 2] = [2, 4];

struct Mover<'a> {
    g: &'a Grid,
    glove: bool,
    jump: usize,
    fairy: bool,
    /// Locked-door columns that block (no key, no fairy).
    doors: Vec<usize>,
    elevator: Option<usize>,
    /// With the downward thrust: the cells of the room's enemies, which
    /// Link can land on (a down-stab bounces him off them) but still walks
    /// and jumps through.
    bounce: Vec<(usize, usize)>,
}

impl Mover<'_> {
    fn free(&self, x: usize, y: usize) -> bool {
        if self.doors.contains(&x) && y < ROWS - 1 {
            return false;
        }
        match self.g.cell(x, y) {
            Cell::Empty | Cell::FalseWall => true,
            Cell::Breakable => self.glove,
            _ => false,
        }
    }

    fn solid(&self, x: usize, y: usize) -> bool {
        y < ROWS
            && ((!self.free(x, y) && self.g.cell(x, y) != Cell::Lava)
                || self.bounce.contains(&(x, y)))
    }

    fn standable(&self, x: usize, y: usize) -> bool {
        y >= 1 && y + 1 < ROWS && self.free(x, y) && self.free(x, y - 1) && self.solid(x, y + 1)
    }

    fn on_elevator(&self, x: usize) -> bool {
        self.elevator.is_some_and(|ex| x == ex || x == ex + 1)
    }

    /// Spots Link can occupy in an elevator column (the elevator is his
    /// floor anywhere along the shaft).
    fn elevator_spots(&self, x: usize) -> Vec<(usize, usize)> {
        (1..ROWS - 1)
            .filter(|&y| self.free(x, y) && self.free(x, y - 1))
            .map(|y| (x, y))
            .collect()
    }

    fn fall(&self, x: usize, mut y: usize) -> Option<usize> {
        while y + 1 < ROWS {
            // The cabin at rest plugs the shaft: Link stands on it with his
            // feet on row 9 (cabin `$2A = $98`, measured).
            if self.on_elevator(x) && y >= CABIN_ROW {
                return Some(y);
            }
            if self.solid(x, y + 1) {
                return Some(y);
            }
            if self.g.cell(x, y + 1) == Cell::Lava {
                return None;
            }
            y += 1;
        }
        None
    }

    /// Spots reachable from `starts`, and per page whether Link can fall
    /// out of the bottom there.
    fn flood(&self, starts: &[(usize, usize)]) -> (Vec<bool>, [bool; 4]) {
        let w = self.g.width();
        let mut seen = vec![false; w * ROWS];
        let mut bottom = [false; 4];
        let mut q = VecDeque::new();
        for &(x, y) in starts {
            if x < w && y < ROWS && !seen[y * w + x] {
                seen[y * w + x] = true;
                q.push_back((x, y));
            }
        }
        let drop_out = |nx: usize, bottom: &mut [bool; 4]| {
            if self.g.cell(nx, ROWS - 1) == Cell::Empty {
                bottom[(nx / PAGE_COLS).min(3)] = true;
            }
        };
        while let Some((x, y)) = q.pop_front() {
            let mut next: Vec<(usize, usize)> = Vec::new();
            if self.fairy {
                for (dx, dy) in [(-1i32, 0i32), (1, 0), (0, -1), (0, 1)] {
                    let nx = x as i32 + dx;
                    let ny = y as i32 + dy;
                    if nx < 0 || ny < 0 || nx as usize >= w {
                        continue;
                    }
                    if ny as usize >= ROWS {
                        drop_out(x, &mut bottom);
                        continue;
                    }
                    if self.free(nx as usize, ny as usize) {
                        next.push((nx as usize, ny as usize));
                    }
                }
            }
            // Walk one column, then fall.
            for dx in [-1i32, 1] {
                let nx = x as i32 + dx;
                if nx < 0 || nx as usize >= w {
                    continue;
                }
                let nx = nx as usize;
                if self.free(nx, y) && self.free(nx, y.saturating_sub(1)) {
                    match self.fall(nx, y) {
                        Some(ly) => next.push((nx, ly)),
                        None => drop_out(nx, &mut bottom),
                    }
                }
            }
            // Jump: up to H rows, across up to W columns, then fall.
            let mut top = y;
            for _ in 0..JUMP_ROWS[self.jump] {
                if top >= 2 && self.free(x, top - 2) {
                    top -= 1;
                } else {
                    break;
                }
            }
            for dir in [-1i32, 1] {
                for step in 0..=JUMP_COLS[self.jump] as i32 {
                    let nx = x as i32 + dir * step;
                    if nx < 0 || nx as usize >= w {
                        break;
                    }
                    let nx = nx as usize;
                    if !(self.free(nx, top) && self.free(nx, top.saturating_sub(1))) {
                        break;
                    }
                    match self.fall(nx, top) {
                        Some(ly) => next.push((nx, ly)),
                        None => drop_out(nx, &mut bottom),
                    }
                }
            }
            if self.on_elevator(x) {
                next.extend(self.elevator_spots(x));
            }
            for (nx, ny) in next {
                let i = ny * w + nx;
                if !seen[i] {
                    seen[i] = true;
                    q.push_back((nx, ny));
                }
            }
        }
        (seen, bottom)
    }
}

/// Where an exit is, as spots Link stands on (empty for drops).
fn exit_spots(m: &Mover<'_>, dir: Dir) -> Vec<(usize, usize)> {
    let w = m.g.width();
    let column = |x: usize| -> Vec<(usize, usize)> {
        (1..ROWS - 1)
            .filter(|&y| m.standable(x, y) || (m.fairy && m.free(x, y)))
            .map(|y| (x, y))
            .collect()
    };
    match dir {
        Dir::Left => column(0),
        Dir::Right => column(w - 1),
        Dir::Up | Dir::Down => m
            .elevator
            .map(|ex| {
                let mut v = m.elevator_spots(ex);
                v.extend(m.elevator_spots(ex + 1));
                v
            })
            .unwrap_or_default(),
        Dir::Door { x } => column(usize::from(x).min(w - 1)),
        Dir::Drop { .. } => Vec::new(),
    }
}

fn mover<'a>(g: &'a Grid, r: &Room, have: Abilities) -> Mover<'a> {
    let pass_doors = have.has(Abilities::KEY) || have.has(Abilities::FAIRY);
    Mover {
        g,
        glove: have.has(Abilities::GLOVE),
        jump: usize::from(have.has(Abilities::JUMP)),
        fairy: have.has(Abilities::FAIRY),
        doors: if pass_doors {
            Vec::new()
        } else {
            g.doors.iter().map(|&d| usize::from(d)).collect()
        },
        elevator: g.elevator.map(usize::from).or_else(|| {
            r.elevator_page()
                .map(|p| usize::from(p) * PAGE_COLS + PAGE_COLS / 2 - 1)
        }),
        bounce: if have.has(Abilities::DOWNSTAB) {
            r.enemies
                .iter()
                .map(|e| (usize::from(e.x), usize::from(e.y)))
                .filter(|&(x, y)| x < g.width() && y < ROWS)
                .collect()
        } else {
            Vec::new()
        },
    }
}

/// Entries of a room: `(requirement of that side, spots)`.
fn entries(
    area: &Area,
    r: &Room,
    g: &Grid,
    have: Abilities,
) -> Vec<(Abilities, Vec<(usize, usize)>)> {
    let m = mover(g, r, have);
    let mut v = Vec::new();
    for e in &r.exits {
        if !matches!(e.dir, Dir::Drop { .. }) {
            v.push((e.requires, exit_spots(&m, e.dir)));
        }
    }
    let w = g.width();
    let fall_in = |page: u8| -> Vec<(usize, usize)> {
        let x = usize::from(page) * PAGE_COLS + PAGE_COLS / 2;
        if x >= w {
            return Vec::new();
        }
        m.fall(x, 1).map(|y| vec![(x, y)]).unwrap_or_default()
    };
    // Falls in from the rooms above, on the page they fell from.
    let falls = area
        .rooms
        .iter()
        .flat_map(|o| o.exits.iter())
        .filter_map(|e| match (e.dir, e.target) {
            (Dir::Drop { page }, Target::Room(t)) if t == r.id => Some(page),
            _ => None,
        });
    for p in r.landing.into_iter().chain(falls) {
        v.push((Abilities::NONE, fall_in(p)));
    }
    let arrivals = area
        .entrances
        .iter()
        .filter(|e| e.room == r.id)
        .map(|e| e.page)
        .chain(area.links.iter().filter(|l| l.room == r.id).map(|l| l.page));
    for page in arrivals {
        let spots = if page == 0 {
            exit_spots(&m, Dir::Left)
        } else if page + 1 == r.pages {
            exit_spots(&m, Dir::Right)
        } else {
            fall_in(page)
        };
        v.push((Abilities::NONE, spots));
    }
    v
}

/// The solid-tile grid of `r` in `area`.
pub fn room_grid(area: &Area, r: &Room, floors: &FloorTable) -> Result<Grid, String> {
    let sv = compile::room_sideview(r, area).map_err(|e| e.to_string())?;
    Ok(sv.grid(area.world.object_set(), floors))
}

/// Can every entry of `r` reach `goal` (exit index, or `None` for the
/// items) with the declared abilities?
fn geometry_room(area: &Area, r: &Room, floors: &FloorTable, out: &mut Issues) {
    let g = match room_grid(area, r, floors) {
        Ok(g) => g,
        Err(e) => {
            out.room(r, e);
            return;
        }
    };
    let w = g.width();
    // Goals: every exit, then the items.
    let mut goals: Vec<(String, Abilities, Option<Dir>)> = r
        .exits
        .iter()
        .map(|e| (format!("{:?} exit", e.dir), e.requires, Some(e.dir)))
        .collect();
    if !g.items.is_empty() {
        goals.push(("items".into(), r.item_requires, None));
    }
    for (name, need, dir) in goals {
        // Entries are checked one at a time with the entry's own
        // requirement added (see `Exit::requires`).
        let base = r.requires.with(need);
        let all = entries(area, r, &g, base.with(Abilities::ALL));
        for (k, (side_need, _)) in all.iter().enumerate() {
            let have = base.with(*side_need);
            let m = mover(&g, r, have);
            let starts = &entries(area, r, &g, have)[k].1;
            if starts.is_empty() {
                continue; // this side cannot be stood on with `have`
            }
            let (seen, bottom) = m.flood(starts);
            let at = |x: usize, y: usize| x < w && y < ROWS && seen[y * w + x];
            let ok = match dir {
                Some(Dir::Drop { page }) => bottom[usize::from(page).min(3)],
                Some(d) => exit_spots(&m, d).iter().any(|&(x, y)| at(x, y)),
                None => g.items.iter().all(|&(ix, iy, _)| {
                    let (ix, iy) = (usize::from(ix), usize::from(iy));
                    (ix.saturating_sub(1)..=ix + 1)
                        .any(|x| (iy..=(iy + 1).min(ROWS - 1)).any(|y| at(x, y)))
                }),
            };
            if !ok {
                out.room(
                    r,
                    format!("{name} not reachable from entry {k} with {have:?}"),
                );
            }
        }
    }
    // Every side with an exit must be standable with every ability.
    let m = mover(&g, r, Abilities::ALL);
    for e in &r.exits {
        if !matches!(e.dir, Dir::Drop { .. }) && exit_spots(&m, e.dir).is_empty() {
            out.room(r, format!("{:?} exit has nowhere to stand", e.dir));
        }
    }
    // Open bottoms need a drop exit.
    let drops: Vec<u8> = r
        .exits
        .iter()
        .filter_map(|e| match e.dir {
            Dir::Drop { page } => Some(page),
            _ => None,
        })
        .collect();
    for p in g.drop_pages() {
        if !drops.contains(&p) {
            out.room(
                r,
                format!("page {p} is open at the bottom but has no drop exit"),
            );
        }
    }
}

/// [`validate`] plus the per-room movement check on the real floor
/// patterns.
pub fn validate_geometry(area: &Area, floors: &FloorTable) -> Result<(), Vec<Issue>> {
    let mut out = Issues(match validate(area) {
        Ok(()) => Vec::new(),
        Err(v) => v,
    });
    for r in &area.rooms {
        geometry_room(area, r, floors, &mut out);
    }
    out.done()
}
