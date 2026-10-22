//! Enemy lists: the per-room spawn list the game copies to WRAM `$7000+`.
//!
//! Format (ported from `z2-rando::enemies::shuffle`; spawn side documented
//! in `z2-core::sideview_spawn`): a length byte (the whole list, itself
//! included, so `1 + 2 * n`), then two bytes per enemy:
//!
//! * `YX`: Y row code in the high nibble, column bits 3-0 in the low nibble;
//! * `PI`: column bits 5-4 (the page) in bits 7-6, enemy ID in bits 5-0.
//!
//! The row code is stored compressed: raw 0 is row 1 (ceiling flyers), raw
//! `n >= 1` is row `n + 2`, so rows 1 and 3-17 are exact. The game edits
//! its WRAM copy while spawning, so the engine must copy an authored list
//! fresh into its buffer every time the room loads.

use crate::ContentError;

/// Longest list the one-byte length allows (127 enemies).
pub const MAX_LIST_LEN: usize = 255;

/// One enemy of a list.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Enemy {
    /// Column 0-63.
    pub x: u8,
    /// Row (decoded: raw 0 is row 1, raw n is row n + 2).
    pub y: u8,
    /// Enemy ID (0-63; meaning depends on the world's enemy group).
    pub id: u8,
}

impl Enemy {
    /// Decode one 2-byte entry.
    #[must_use]
    pub const fn decode(b0: u8, b1: u8) -> Enemy {
        let raw_y = b0 >> 4;
        Enemy {
            x: ((b1 >> 6) << 4) | (b0 & 0x0F),
            y: if raw_y == 0 { 1 } else { raw_y + 2 },
            id: b1 & 0x3F,
        }
    }

    /// Encode; rows below 3 become raw 0 (row 1), rows past 17 clamp.
    #[must_use]
    pub const fn encode(self) -> [u8; 2] {
        let raw_y = if self.y < 3 {
            0
        } else if self.y - 2 > 15 {
            15
        } else {
            self.y - 2
        };
        [
            (raw_y << 4) | (self.x & 0x0F),
            (((self.x >> 4) & 3) << 6) | (self.id & 0x3F),
        ]
    }

    /// Can row `y` be stored exactly?
    #[must_use]
    pub const fn encodable_row(y: u8) -> bool {
        y == 1 || (y >= 3 && y <= 17)
    }
}

/// Encode a whole list (length byte first).
pub fn encode_list(list: &[Enemy]) -> Result<Vec<u8>, ContentError> {
    let len = 1 + 2 * list.len();
    if len > MAX_LIST_LEN {
        return Err(ContentError::Encode(format!(
            "enemy list of {} enemies is {len} bytes, the length byte holds {MAX_LIST_LEN}",
            list.len()
        )));
    }
    let mut out = Vec::with_capacity(len);
    out.push(len as u8);
    for e in list {
        if e.x > 63 || e.id > 63 || !Enemy::encodable_row(e.y) {
            return Err(ContentError::Encode(format!(
                "enemy {e:?} cannot be encoded"
            )));
        }
        out.extend_from_slice(&e.encode());
    }
    Ok(out)
}

/// Decode a list (`bytes` may be longer than the list; the length byte
/// decides). A length below 3 is an empty list.
pub fn decode_list(bytes: &[u8]) -> Result<Vec<Enemy>, ContentError> {
    let len = usize::from(
        *bytes
            .first()
            .ok_or_else(|| ContentError::Decode("empty enemy list".into()))?,
    );
    if len < 3 {
        return Ok(Vec::new());
    }
    if bytes.len() < len {
        return Err(ContentError::Decode(format!(
            "enemy list length {len} but only {} bytes",
            bytes.len()
        )));
    }
    Ok(bytes[1..len]
        .as_chunks::<2>()
        .0
        .iter()
        .map(|&[a, b]| Enemy::decode(a, b))
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entry_round_trip_every_encodable_value() {
        for x in 0..64 {
            for y in (1..=17).filter(|&y| Enemy::encodable_row(y)) {
                for id in [0, 1, 0x13, 0x3F] {
                    let e = Enemy { x, y, id };
                    let [a, b] = e.encode();
                    assert_eq!(Enemy::decode(a, b), e);
                }
            }
        }
        assert!(!Enemy::encodable_row(2));
        assert!(!Enemy::encodable_row(18));
    }

    #[test]
    fn list_round_trip_and_limits() {
        let l = vec![Enemy { x: 5, y: 9, id: 3 }, Enemy { x: 40, y: 1, id: 6 }];
        let raw = encode_list(&l).unwrap();
        assert_eq!(raw, vec![5, 0x75, 0x03, 0x08, 0x86]);
        assert_eq!(decode_list(&raw).unwrap(), l);
        assert_eq!(decode_list(&[1]).unwrap(), vec![]);
        assert!(decode_list(&[5, 0, 0]).is_err());
        assert!(encode_list(&[Enemy { x: 64, y: 9, id: 0 }]).is_err());
        assert!(encode_list(&[Enemy { x: 0, y: 9, id: 0 }; 128]).is_err());
        assert!(encode_list(&[Enemy { x: 0, y: 9, id: 0 }; 127]).is_ok());
    }
}
