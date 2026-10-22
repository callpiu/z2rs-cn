//! Whole-character, presentation-only animations. Frames keep a common pivot
//! after trimming, so limbs can cross the original CHR cell boundaries.
//!
//! Frames stay PNG-compressed in memory and are decoded the first time they
//! are shown ([`crate::Presenter::set_actors`]). Decoded frames are kept up
//! to [`MAX_DECODED_BYTES`]; past that, the characters used least recently
//! give theirs back. A pack may therefore hold more animation than fits
//! decoded at once, as long as no single character exceeds the budget.
use std::collections::HashMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::png_io::{decode_png_rgba, RgbaImage, MAX_SHEET_DIM};

/// Most unique character/clip pairs in one pack.
pub const MAX_ACTOR_CLIPS: usize = 4096;
/// Most frames in one clip.
pub const MAX_CLIP_FRAMES: usize = 4096;
/// Decoded actor frames kept in memory at once (RGBA8), and the most one
/// character's frames may decode to.
pub const MAX_DECODED_BYTES: usize = 512 * 1024 * 1024;
/// Most compressed PNG bytes all actor frames may hold.
pub const MAX_COMPRESSED_BYTES: usize = 1024 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActorFrameEntry {
    pub file: String,
    /// Origin in this cropped image, in pack pixels, from its top left.
    pub pivot: [f32; 2],
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActorClipEntry {
    pub character: String,
    pub name: String,
    pub fps: f64,
    #[serde(default)]
    pub looped: bool,
    pub frames: Vec<ActorFrameEntry>,
}

#[derive(Debug, Clone)]
pub struct ActorFrame {
    /// The frame's PNG file, shared by every clip that names it.
    pub file: String,
    png: Arc<[u8]>,
    pub width: u32,
    pub height: u32,
    pub pivot: [f32; 2],
    image: Option<RgbaImage>,
}

impl ActorFrame {
    /// The decoded frame, once [`crate::Presenter::set_actors`] has shown it.
    #[must_use]
    pub fn image(&self) -> Option<&RgbaImage> {
        self.image.as_ref()
    }

    /// Decoded size in bytes.
    #[must_use]
    pub fn decoded_bytes(&self) -> usize {
        self.width as usize * self.height as usize * 4
    }
}

#[derive(Debug, Clone)]
pub struct ActorClip {
    pub character: String,
    pub name: String,
    pub fps: f64,
    pub looped: bool,
    pub frames: Vec<ActorFrame>,
}

impl ActorClip {
    #[must_use]
    pub fn frame_index(&self, seconds: f64) -> usize {
        let index = (seconds.max(0.0) * self.fps + 1e-6).floor() as usize;
        if self.looped {
            index % self.frames.len()
        } else {
            index.min(self.frames.len() - 1)
        }
    }
}

/// Every clip of a pack plus the decode cache.
#[derive(Debug, Clone, Default)]
pub struct ActorLibrary {
    pub(crate) clips: Vec<ActorClip>,
    /// `character -> clip name -> index into clips`.
    index: HashMap<String, HashMap<String, usize>>,
    alpha: u8,
    /// Bytes of decoded frames currently held.
    resident: usize,
    /// Decoded bytes to keep before evicting ([`MAX_DECODED_BYTES`]).
    budget: usize,
    /// Per character: decoded bytes and the frame it was last shown.
    used: HashMap<String, (usize, u64)>,
}

impl ActorLibrary {
    /// The clip for `character`/`name`, if the pack has one.
    #[must_use]
    pub fn find(&self, character: &str, name: &str) -> Option<usize> {
        self.index.get(character)?.get(name).copied()
    }

    /// Keep at most `bytes` of decoded frames (capped at
    /// [`MAX_DECODED_BYTES`]) before giving back least recently shown
    /// characters, e.g. on a memory-constrained frontend. The character on
    /// screen is never evicted, so one character may exceed a small budget.
    pub fn set_decode_budget(&mut self, bytes: usize) {
        self.budget = bytes.min(MAX_DECODED_BYTES);
    }

    /// Decoded actor frame bytes currently held.
    #[must_use]
    pub fn resident_bytes(&self) -> usize {
        self.resident
    }

    /// Decode frame `frame` of clip `clip` if needed, marking its character
    /// used at emulated frame `now`, then give back the least recently used
    /// characters' frames while over [`MAX_DECODED_BYTES`]. `false` when the
    /// PNG does not decode (the actor then keeps its tile art).
    pub(crate) fn ensure(&mut self, clip: usize, frame: usize, now: u64) -> bool {
        let character = self.clips[clip].character.clone();
        let alpha = self.alpha;
        let entry = self.used.entry(character.clone()).or_insert((0, now));
        entry.1 = now;
        let f = &mut self.clips[clip].frames[frame];
        if f.image.is_some() {
            return true;
        }
        let Ok(mut image) = decode_png_rgba(&f.png) else {
            return false;
        };
        if image.width != f.width || image.height != f.height {
            return false;
        }
        for p in image.rgba.as_chunks_mut::<4>().0 {
            p[3] = if p[3] >= alpha { 255 } else { 0 };
        }
        let bytes = image.rgba.len();
        f.image = Some(image);
        self.resident += bytes;
        entry.0 += bytes;
        if self.resident > self.budget {
            self.evict(&character, now);
        }
        true
    }

    /// Drop decoded frames of characters not shown at `now`, oldest first,
    /// until the cache fits again; `keep` is never evicted. A character last
    /// shown *after* `now` belongs to a timeline a loaded save state or a
    /// rewind left behind, so it is not on screen either.
    fn evict(&mut self, keep: &str, now: u64) {
        let mut victims: Vec<(u64, String)> = self
            .used
            .iter()
            .filter(|(c, (bytes, last))| *bytes > 0 && *last != now && c.as_str() != keep)
            .map(|(c, (_, last))| (*last, c.clone()))
            .collect();
        victims.sort();
        for (_, character) in victims {
            if self.resident <= self.budget {
                break;
            }
            for clip in self.clips.iter_mut().filter(|c| c.character == character) {
                for f in &mut clip.frames {
                    if let Some(image) = f.image.take() {
                        self.resident -= image.rgba.len();
                    }
                }
            }
            if let Some(u) = self.used.get_mut(&character) {
                u.0 = 0;
            }
        }
    }

    /// Decode every distinct frame file once (without keeping it), so a pack
    /// check reports a broken PNG before it is played.
    ///
    /// # Errors
    /// The first frame that does not decode, or decodes to a size other
    /// than its header's.
    pub fn verify(&self) -> Result<(), String> {
        let mut seen = std::collections::HashSet::new();
        for clip in &self.clips {
            for f in &clip.frames {
                if !seen.insert(f.file.as_str()) {
                    continue;
                }
                let label = format!("actor_clips {}/{}: {}", clip.character, clip.name, f.file);
                let image = decode_png_rgba(&f.png).map_err(|e| format!("{label}: {e}"))?;
                if (image.width, image.height) != (f.width, f.height) {
                    return Err(format!("{label}: header size differs from the image"));
                }
            }
        }
        Ok(())
    }
}

/// Width and height from a PNG's `IHDR`, without decoding it.
fn png_size(bytes: &[u8]) -> Option<(u32, u32)> {
    const SIGNATURE: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
    if bytes.len() < 24 || bytes[..8] != SIGNATURE || &bytes[12..16] != b"IHDR" {
        return None;
    }
    let word =
        |at: usize| u32::from_be_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]]);
    Some((word(16), word(20)))
}

pub(crate) fn load_clips(
    entries: &[ActorClipEntry],
    alpha: u8,
    read: &dyn Fn(&str) -> Result<Vec<u8>, String>,
) -> Result<ActorLibrary, String> {
    let mut library = ActorLibrary {
        alpha,
        budget: MAX_DECODED_BYTES,
        ..ActorLibrary::default()
    };
    let mut files: HashMap<String, (Arc<[u8]>, u32, u32)> = HashMap::new();
    let mut compressed = 0usize;
    let mut per_character: HashMap<&str, usize> = HashMap::new();
    if entries.len() > MAX_ACTOR_CLIPS {
        return Err(format!(
            "actor_clips: at most {MAX_ACTOR_CLIPS} character/clip pairs"
        ));
    }
    for e in entries {
        let label = format!("actor_clips {}/{}", e.character, e.name);
        if e.character.is_empty()
            || e.name.is_empty()
            || library.find(&e.character, &e.name).is_some()
        {
            return Err(format!("{label}: empty or duplicate character/clip name"));
        }
        if !e.fps.is_finite()
            || !(1.0..=240.0).contains(&e.fps)
            || e.frames.is_empty()
            || e.frames.len() > MAX_CLIP_FRAMES
        {
            return Err(format!(
                "{label}: fps must be 1..240 and frames 1..{MAX_CLIP_FRAMES}"
            ));
        }
        let mut frames = Vec::new();
        for f in &e.frames {
            // Same rules as sheet and layer files: a manifest written on
            // Windows (`actors\link\walk\0000.png`) or with `./` prefixes
            // names the same file; absolute, drive-qualified and `..` paths
            // are refused.
            let path = crate::pack::normalize_rel_path(&f.file)
                .filter(|p| !p.contains(':'))
                .filter(|_| f.pivot.iter().all(|x| x.is_finite() && x.abs() <= 8192.0))
                .ok_or_else(|| {
                    format!("{label}: invalid relative frame path or pivot: {}", f.file)
                })?;
            let (png, width, height) = match files.get(&path) {
                Some(known) => known.clone(),
                None => {
                    let bytes = read(&path).map_err(|err| format!("{label}: {}: {err}", f.file))?;
                    let (width, height) = png_size(&bytes)
                        .ok_or_else(|| format!("{label}: {}: not a PNG", f.file))?;
                    if width == 0 || height == 0 || width > MAX_SHEET_DIM || height > MAX_SHEET_DIM
                    {
                        return Err(format!(
                            "{label}: {}: image is {width}x{height}; frames may be at most \
                             {MAX_SHEET_DIM}x{MAX_SHEET_DIM}",
                            f.file
                        ));
                    }
                    compressed += bytes.len();
                    if compressed > MAX_COMPRESSED_BYTES {
                        return Err(format!(
                            "actor_clips: frame files exceed {} MiB",
                            MAX_COMPRESSED_BYTES >> 20
                        ));
                    }
                    let known = (Arc::<[u8]>::from(bytes), width, height);
                    files.insert(path.clone(), known.clone());
                    known
                }
            };
            let decoded = per_character.entry(e.character.as_str()).or_default();
            *decoded += width as usize * height as usize * 4;
            if *decoded > MAX_DECODED_BYTES {
                return Err(format!(
                    "{label}: the character's frames decode to more than {} MiB",
                    MAX_DECODED_BYTES >> 20
                ));
            }
            frames.push(ActorFrame {
                file: path,
                png,
                width,
                height,
                pivot: f.pivot,
                image: None,
            });
        }
        library
            .index
            .entry(e.character.clone())
            .or_default()
            .insert(e.name.clone(), library.clips.len());
        library.clips.push(ActorClip {
            character: e.character.clone(),
            name: e.name.clone(),
            fps: e.fps,
            looped: e.looped,
            frames,
        });
    }
    Ok(library)
}

/// Inputs are observations; the renderer never writes gameplay or OAM state.
#[derive(Debug, Clone, Copy, Default)]
pub struct ActorInput<'a> {
    pub id: u8,
    pub character: &'a str,
    pub clip: &'a str,
    /// Character to try when the pack has no clips for `character` (the
    /// plain key of a qualified one); empty for none.
    pub character_fallback: &'a str,
    /// Clip to try when the pack has no `clip` for the character (the older
    /// name of a newer, finer state); empty for none.
    pub clip_fallback: &'a str,
    /// A new cycle of the same action started: restart the clip.
    pub restart: bool,
    /// Tint the frame with this sprite palette (0-3): the cartridge is
    /// flashing the actor in another palette (hit flash).
    pub flash_palette: Option<u8>,
    /// Skeleton origin in NES screen pixels (before widescreen margins).
    pub position: [f32; 2],
    pub flip_h: bool,
    /// The original OAM entries this full-character image replaces.
    pub oam_mask: u64,
    /// The widescreen margin sprites it replaces (bit `i` is
    /// `Margins::sprites[i]`): the columns the game hid in OAM because they
    /// fell outside the window.
    pub margin_mask: u64,
    pub priority_oam: u8,
}

impl ActorInput<'_> {
    /// The pack's clip for this input: `character`/`clip`, then
    /// `character`/`clip_fallback`, then the same with `character_fallback`.
    #[must_use]
    pub fn resolve(&self, library: &ActorLibrary) -> Option<usize> {
        let characters = [self.character, self.character_fallback];
        let clips = [self.clip, self.clip_fallback];
        characters
            .iter()
            .filter(|c| !c.is_empty())
            .flat_map(|c| {
                clips
                    .iter()
                    .filter(|n| !n.is_empty())
                    .map(move |n| (*c, *n))
            })
            .find_map(|(c, n)| library.find(c, n))
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct ActorInstance {
    pub clip: usize,
    pub frame: usize,
    pub position: [f32; 2],
    pub flip_h: bool,
    pub oam_mask: u64,
    pub margin_mask: u64,
    pub priority_oam: u8,
    pub behind: bool,
    pub flash_palette: Option<u8>,
}

#[derive(Debug, Clone)]
pub(crate) struct ActorClock {
    pub id: u8,
    pub clip: usize,
    pub began: u64,
    pub last: u64,
}
