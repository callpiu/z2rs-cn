//! Read-only geometry from the same mapped tables used by collision routines.
//! No scratch RAM, CPU cycles, mapper registers or animation timers are changed.
use crate::{coop, game::Game};

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub enum Kind {
    Body,
    Shield,
    Sword,
    Enemy(u8),
}

/// Screen-space rectangle. Width/height include the engine's inclusive endpoint.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub struct CollisionBox {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
    pub player: u8,
    pub kind: Kind,
}

#[derive(Debug, Default, PartialEq, Eq, serde::Serialize)]
pub struct Geometry {
    pub boxes: Vec<CollisionBox>,
    /// Actual level-test positions, including the centre foot/lava probe.
    pub terrain: Vec<[i32; 2]>,
}

impl Game {
    /// Observe current collision candidates; boxes can remain visible during
    /// invulnerability. These describe geometry, not a promise of damage.
    pub fn collision_geometry(&self) -> Geometry {
        let mut out = Geometry::default();
        if self.ram[0x736] != 0x0b || self.prg.is_empty() {
            return out;
        }
        let read = |addr: u16| -> u8 {
            match addr {
                0x6000..=0x7fff => self.wram[usize::from(addr - 0x6000)],
                0x8000..=0xffff => self.prg[self.mmc1.map_prg(addr, self.prg.len())],
                _ => 0,
            }
        };
        let mut player = |id, get: &dyn Fn(usize) -> u8| {
            let sx = get(0xcc);
            let ly = get(0x29);
            let shield = u16::from(get(0x17));
            let mut add = |x: u8, y: u8, w: u8, h: u8, kind| {
                out.boxes.push(CollisionBox {
                    x: i32::from(x),
                    y: i32::from(y),
                    w: i32::from(w) + 1,
                    h: i32::from(h) + 1,
                    player: id,
                    kind,
                });
            };
            let dx = read(0xe9d4u16.wrapping_add(u16::from(get(0x9f).wrapping_sub(1))));
            add(
                sx.wrapping_add(8).wrapping_add(dx).wrapping_sub(
                    u8::from(get(0x9f) == 1 && self.enhancements().fixes.shield_hitbox_symmetry)
                        * 2,
                ),
                ly.wrapping_add(read(0xe9d6 + shield)),
                5,
                12,
                Kind::Shield,
            );
            let x = sx.wrapping_add(9);
            let (y, carry) = ly.overflowing_add(read(0xe971 + shield));
            let w = if u16::from(x) + 13 + u16::from(carry) > 255 {
                !x
            } else {
                13
            };
            add(x, y, w, read(0xe973 + shield), Kind::Body);
            if get(0x480) < 0xf8 {
                let side = u16::from(get(0x47e) < sx);
                let row = u16::from(matches!(get(0x80), 8 | 9));
                add(
                    get(0x47e).wrapping_add(read(0xe99c + side)),
                    get(0x480).wrapping_add(read(0xe99e + row)),
                    read(0xe9b5),
                    read(0xe9a0 + row),
                    Kind::Sword,
                );
            }
            let fairy = get(0x13);
            let side = (get(0x5f) >> 1).wrapping_add(fairy);
            for index in [
                4u8.wrapping_add(fairy),
                5u8.wrapping_add(fairy),
                6u8.wrapping_add(fairy),
                7u8.wrapping_add(fairy),
                side,
                side.wrapping_add(2).wrapping_add(fairy),
                0x1d,
            ] {
                let dx = read(0xeaa0 + u16::from(index));
                let dy = read(0xeac0 + u16::from(index));
                out.terrain.push([
                    i32::from(sx) + i32::from(dx),
                    // Level-RAM row zero starts below the 32-pixel HUD.
                    i32::from(ly.wrapping_add(dy)) + 32,
                ]);
            }
        };
        player(0, &|a| self.ram[a]);
        if self.coop.enabled && self.coop.active {
            player(1, &|a| self.coop.block[coop::idx(a as u16)]);
        }
        for slot in 0..6 {
            if let Some([x, y, w, h]) = self.enemy_box_in(&self.ram, slot) {
                out.boxes.push(CollisionBox {
                    x,
                    y,
                    w,
                    h,
                    player: 0,
                    kind: Kind::Enemy(slot as u8),
                });
            }
        }
        out
    }

    /// Slot `slot`'s enemy collision box `[x, y, w, h]` (screen pixels,
    /// inclusive extents + 1) computed from `ram` (the live RAM, or a
    /// displayed-frame copy such as [`Game::shown_ram`]) and the mapped
    /// per-type tables. `None` for an empty slot or one off screen.
    pub(crate) fn enemy_box_in(&self, ram: &[u8; 0x800], slot: usize) -> Option<[i32; 4]> {
        let link_world = i32::from(ram[0x3b]) * 256 + i32::from(ram[0x4d]);
        let sx = i32::from(ram[0x3c + slot]) * 256 + i32::from(ram[0x4e + slot]) - link_world
            + i32::from(ram[0xcc]);
        self.enemy_box_at(ram, slot, sx)
    }

    /// [`Game::enemy_box_in`] for a slot at screen X `sx`.
    pub(crate) fn enemy_box_at(&self, ram: &[u8; 0x800], slot: usize, sx: i32) -> Option<[i32; 4]> {
        if ram[0xb6 + slot] == 0 || self.prg.is_empty() {
            return None;
        }
        let read = |addr: u16| -> u8 {
            match addr {
                0x6000..=0x7fff => self.wram[usize::from(addr - 0x6000)],
                0x8000..=0xffff => self.prg[self.mmc1.map_prg(addr, self.prg.len())],
                _ => 0,
            }
        };
        if !(0..256).contains(&sx) {
            return None;
        }
        let row = u16::from(read(0x6e1d + u16::from(ram[0xa1 + slot])).wrapping_mul(4));
        let x = (sx as u8).wrapping_add(read(0xe8fa + row));
        let (y, carry) = ram[0x2a + slot].overflowing_add(read(0xe8fc + row));
        let width = read(0xe8fb + row);
        let width = if u16::from(x) + u16::from(width) + u16::from(carry) > 255 {
            !x
        } else {
            width
        };
        Some([
            i32::from(x),
            i32::from(y),
            i32::from(width) + 1,
            i32::from(read(0xe8fd + row)) + 1,
        ])
    }
}
