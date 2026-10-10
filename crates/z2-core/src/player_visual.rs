//! Read-only Link animation observations shared by native and web presenters.
//! Sprite drawing, collision and original NES animation timers are unchanged.
#[cfg(feature = "interp")]
use crate::coop;
use crate::game::Game;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlayerVisual {
    pub id: u8,
    pub character: &'static str,
    pub clip: &'static str,
    pub position: [f32; 2],
    pub flip_h: bool,
    pub oam_mask: u64,
    pub priority_oam: u8,
}

fn action(get: impl Fn(usize) -> u8) -> &'static str {
    if get(0x0494) != 0 {
        return "death";
    }
    if get(0x0013) != 0 {
        return "fairy_fly";
    }
    if get(0x050C) != 0 {
        return "hurt";
    }
    if get(0x049C) != 0 || get(0x049D) != 0 {
        return "item_get";
    }
    match get(0x0080) {
        4 => return "attack_windup",
        5 => {
            return if get(0x0479) != 0 {
                "jump_attack"
            } else {
                "attack"
            }
        }
        7 => return "crouch_attack",
        8 => return "up_thrust_hold",
        9 => return "down_thrust_hold",
        _ => {}
    }
    // Airborne first: the cartridge shows its duck frame `$80 = 6` while a
    // jump rises (bank 0 `$956C`), which is the HD pack's `jump_rise`.
    if get(0x0479) != 0 {
        return match get(0x057D) as i8 {
            v if v < 0 => "jump_rise",
            0 => "jump_apex",
            _ => "fall",
        };
    }
    // Grounded duck: Down held (`$9419`) or the hard-landing timer `$0497`,
    // both of which write `$80 = 6`. Not `$0495`: it latches the shield row
    // `$17` at each stab (`$9606`), so a standing stab sets it and only a
    // crouch stab clears it, and it stays set while Link stands and walks.
    if get(0x0497) != 0 || get(0x0080) == 6 {
        return "crouch_idle";
    }
    if get(0x0070) != 0 {
        "walk"
    } else {
        "idle"
    }
}

impl Game {
    /// Sideview players and the single overworld player. Actual OAM visibility (including
    /// injury blink) remains the renderer's gate; inputs describe their pose.
    pub fn player_visuals(&self) -> [Option<PlayerVisual>; 2] {
        if self.ram[0x0736] == 0x05 {
            let moving = self.ram[0x007D] != 0;
            let clip = match (self.ram[0x0562], moving) {
                (2, true) => "overworld_walk_left",
                (4, true) => "overworld_walk_down",
                (8, true) => "overworld_walk_up",
                (_, true) => "overworld_walk_right",
                (2, false) => "overworld_idle_left",
                (4, false) => "overworld_idle_down",
                (8, false) => "overworld_idle_up",
                (_, false) => "overworld_idle_right",
            };
            return [
                Some(PlayerVisual {
                    id: 0,
                    character: "link",
                    clip,
                    // Two 8x16 OAM cells; the first cell's Y is one less than
                    // its displayed row. The pack supplies a 16-pixel-tall map rig.
                    position: [
                        f32::from(self.ram[0x0203]) + 8.0,
                        f32::from(self.ram[0x0200]) + 17.0,
                    ],
                    flip_h: false,
                    oam_mask: 3,
                    priority_oam: 0,
                }),
                None,
            ];
        }
        if self.ram[0x0736] != 0x0B {
            return [None, None];
        }
        let get = |a: usize| self.ram[a];
        let player = |id, get: &dyn Fn(usize) -> u8, slots: &[u8], priority| PlayerVisual {
            id,
            character: if id == 0 { "link" } else { "link_red" },
            clip: {
                let clip = action(get);
                if id == 0
                    && matches!(clip, "idle" | "walk")
                    && self.ram[0x074B] & 0x3f != 0
                    && self.ram[0x074A] != 0
                    && self.ram[0x074F] & 0x80 != 0
                {
                    "cast_spell"
                } else if id == 0 && self.ram[0x0707] == 5 && matches!(self.ram[0x076C], 3 | 4) {
                    "victory"
                } else if id == 0
                    && self.ram[0x0707] >= 3
                    && self.ram[0x00DE] != 0
                    && self.ram[0x0767] != 0
                    && get(0x0080) == 3
                {
                    "crystal_place"
                } else {
                    clip
                }
            },
            position: [f32::from(get(0x00CC)) + 8.0, f32::from(get(0x0029)) + 32.0],
            flip_h: get(0x009F) == 2,
            // Keep the game's held item visible above the raised hand; the
            // body rig replaces weapon cells only while Link carries his sword.
            oam_mask: slots
                .iter()
                .take(if get(0x049C) != 0 || get(0x049D) != 0 {
                    6
                } else {
                    8
                })
                .fold(0, |mask, s| mask | (1u64 << s)),
            priority_oam: priority,
        };
        let p1 = player(0, &get, &[2, 3, 4, 5, 6, 7, 8, 9], 2);
        #[cfg(feature = "interp")]
        let p2 = (self.coop.enabled && self.coop.active).then(|| {
            let get = |a: usize| self.coop.block[coop::idx(a as u16)];
            player(1, &get, &coop::P2_OAM_SLOTS, 56)
        });
        #[cfg(not(feature = "interp"))]
        let p2 = None;
        [Some(p1), p2]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn actions_follow_gameplay_without_mutation() {
        let mut ram = [0; 0x800];
        assert_eq!(action(|a| ram[a]), "idle");
        ram[0x0080] = 9;
        assert_eq!(action(|a| ram[a]), "down_thrust_hold");
        ram[0x0013] = 8;
        assert_eq!(action(|a| ram[a]), "fairy_fly");
        ram[0x0013] = 0;
        // `$0495` left set by a standing stab: Link stands up again.
        ram[0x0080] = 3;
        ram[0x0495] = 1;
        assert_eq!(action(|a| ram[a]), "idle");
        ram[0x0070] = 0x10;
        assert_eq!(action(|a| ram[a]), "walk");
        ram[0x0080] = 6;
        assert_eq!(action(|a| ram[a]), "crouch_idle");
        // A rising jump also shows the duck frame `$80 = 6`.
        ram[0x0479] = 2;
        ram[0x057D] = 0xFC;
        assert_eq!(action(|a| ram[a]), "jump_rise");
        ram[0x0080] = 2;
        ram[0x057D] = 3;
        assert_eq!(action(|a| ram[a]), "fall");
        // Hard landing: `$0497` holds the duck pose on the ground.
        ram[0x0479] = 0;
        ram[0x0497] = 8;
        assert_eq!(action(|a| ram[a]), "crouch_idle");
        ram[0x0013] = 8;
        ram[0x0494] = 1;
        assert_eq!(action(|a| ram[a]), "death");
    }

    #[test]
    fn overworld_has_its_own_origin_and_stops_when_movement_stops() {
        let mut game = Game::new();
        game.ram[0x0736] = 5;
        game.ram[0x0562] = 2;
        game.ram[0x007D] = 4;
        game.ram[0x0203] = 120;
        game.ram[0x0200] = 99;
        let before = game.ram;
        let [Some(v), None] = game.player_visuals() else {
            panic!("one map player")
        };
        assert_eq!(v.clip, "overworld_walk_left");
        assert_eq!(v.position, [128.0, 116.0]);
        assert_eq!(v.oam_mask, 3);
        assert!(!v.flip_h);
        assert_eq!(game.ram, before);
        game.ram[0x007D] = 0;
        assert_eq!(
            game.player_visuals()[0].unwrap().clip,
            "overworld_idle_left"
        );
        game.ram[0x0736] = 3;
        assert_eq!(game.player_visuals(), [None, None]);
    }

    #[cfg(feature = "interp")]
    #[test]
    fn coop_players_keep_their_own_actions_and_item_slots() {
        let mut game = Game::new();
        game.ram[0x0736] = 0x0b;
        game.ram[0x0080] = 7;
        game.ram[0x009F] = 2;
        game.ram[0x00CC] = 40;
        game.coop.enabled = true;
        game.coop.active = true;
        for (a, v) in [(0x0080, 9), (0x00CC, 80), (0x0029, 100)] {
            game.coop.block[coop::idx(a)] = v;
        }
        let [Some(p1), Some(p2)] = game.player_visuals() else {
            panic!("two players")
        };
        assert_eq!(
            (p1.character, p1.clip, p1.flip_h),
            ("link", "crouch_attack", true)
        );
        assert_eq!(
            (p2.character, p2.clip, p2.position),
            ("link_red", "down_thrust_hold", [88.0, 132.0])
        );
        assert_eq!(p1.oam_mask & p2.oam_mask, 0);
        game.ram[0x049C] = 1;
        let v = game.player_visuals()[0].unwrap();
        assert_eq!(v.clip, "item_get");
        assert_eq!(v.oam_mask & ((1 << 8) | (1 << 9)), 0);
    }
}
