//! What town people do beyond their one message: roles, more messages,
//! flag gates.
//!
//! A [`crate::TownNpc`] says its [`crate::Message`] and acts the way its
//! enemy id does in the original game. An [`NpcScript`] adds to one person
//! of an area, by area name and index in [`crate::Area::npcs`], so the
//! existing structs stay as they are. The engine
//! (`z2_core::enh::content_town`) reads [`crate::areas::npc_scripts`].
//!
//! Without a script a person's role comes from the id ([`NpcRole::of_id`]):
//! the healer lady `$17` restores life, the magic lady `$18` magic, the wise
//! man `$0F` is a [`NpcRole::Sage`] without a flag, anyone else talks.
//!
//! Flags are the free-form persistent flags of the area's option group
//! (`z2_core::enh::content::flag`, battery-saved).

use crate::Message;

/// Healer lady (restores life): enemy id in the town worlds.
pub const ID_HEALER: u8 = 0x17;
/// Magic lady (restores magic).
pub const ID_MAGIC_LADY: u8 = 0x18;
/// Wise man.
pub const ID_WISE_MAN: u8 = 0x0F;
/// Lowest town id that talks at all (the game's dialog table starts here).
pub const ID_TALK_MIN: u8 = 0x0A;
/// Highest town id that talks.
pub const ID_TALK_MAX: u8 = 0x22;

/// What a sage hands over (once: the sage's flag records it).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Gift {
    /// Nothing but the flag.
    None,
    /// One more magic container (up to 8), magic refilled.
    MagicContainer,
    /// One more heart container (up to 8), life refilled.
    HeartContainer,
}

/// A town person's job.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NpcRole {
    /// Talks (message, then [`NpcScript::more`] on later talks).
    Talk,
    /// Refills life. With the healer lady's id the original walk to her door
    /// and the restore inside run; with any other id the refill starts as
    /// the box closes.
    HealLife,
    /// Refills magic (the magic lady's id: her original door walk).
    HealMagic,
    /// Talks, and when the box closes sets group flag `flag` (if any) and
    /// hands over `gift` the first time.
    Sage {
        /// Group flag to set.
        flag: Option<u8>,
        /// What he gives the first time.
        gift: Gift,
    },
}

impl NpcRole {
    /// The role an unscripted person with enemy id `id` has.
    #[must_use]
    pub const fn of_id(id: u8) -> NpcRole {
        match id {
            ID_HEALER => NpcRole::HealLife,
            ID_MAGIC_LADY => NpcRole::HealMagic,
            ID_WISE_MAN => NpcRole::Sage {
                flag: None,
                gift: Gift::None,
            },
            _ => NpcRole::Talk,
        }
    }
}

/// Extra behaviour for one town person.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NpcScript {
    /// [`crate::Area::name`] of the person's area.
    pub area: &'static str,
    /// Index in that area's [`crate::Area::npcs`].
    pub npc: u8,
    /// Job.
    pub role: NpcRole,
    /// Said on the second, third, ... talk of a visit (the last one
    /// repeats). For a healer with the original door walk: the line inside
    /// (first entry).
    pub more: &'static [Message],
    /// Group flag that must be set before the person does their job and
    /// says their messages.
    pub requires: Option<u8>,
    /// Said instead while `requires` is not met (default: the message).
    pub locked: Option<Message>,
}

impl NpcScript {
    /// A script for person `npc` of `area` with `role` and nothing else.
    #[must_use]
    pub const fn new(area: &'static str, npc: u8, role: NpcRole) -> NpcScript {
        NpcScript {
            area,
            npc,
            role,
            more: &[],
            requires: None,
            locked: None,
        }
    }

    /// Every message of the script.
    pub fn messages(&self) -> impl Iterator<Item = &Message> {
        self.more.iter().chain(self.locked.iter())
    }

    /// The flag the script sets, if any.
    #[must_use]
    pub const fn sets_flag(&self) -> Option<u8> {
        match self.role {
            NpcRole::Sage { flag, .. } => flag,
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roles_from_ids() {
        assert_eq!(NpcRole::of_id(ID_HEALER), NpcRole::HealLife);
        assert_eq!(NpcRole::of_id(ID_MAGIC_LADY), NpcRole::HealMagic);
        assert!(matches!(
            NpcRole::of_id(ID_WISE_MAN),
            NpcRole::Sage { flag: None, .. }
        ));
        assert_eq!(NpcRole::of_id(0x0C), NpcRole::Talk);
    }
}
