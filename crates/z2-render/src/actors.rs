//! Whole-character, presentation-only animations. Frames keep a common pivot
//! after trimming, so limbs can cross the original CHR cell boundaries.
use serde::{Deserialize, Serialize};

use crate::png_io::{decode_png_rgba, RgbaImage};

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
    pub image: RgbaImage,
    pub pivot: [f32; 2],
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

pub(crate) fn load_clips(
    entries: &[ActorClipEntry],
    alpha: u8,
    read: &dyn Fn(&str) -> Result<Vec<u8>, String>,
) -> Result<Vec<ActorClip>, String> {
    let mut clips = Vec::new();
    let mut names = std::collections::BTreeSet::new();
    let mut decoded_bytes = 0usize;
    if entries.len() > 512 {
        return Err("actor_clips: at most 512 clips".into());
    }
    for e in entries {
        let label = format!("actor_clips {}/{}", e.character, e.name);
        if e.character.is_empty() || e.name.is_empty() || !names.insert((&e.character, &e.name)) {
            return Err(format!("{label}: empty or duplicate character/clip name"));
        }
        if !e.fps.is_finite()
            || !(1.0..=240.0).contains(&e.fps)
            || e.frames.is_empty()
            || e.frames.len() > 4096
        {
            return Err(format!("{label}: fps must be 1..240 and frames 1..4096"));
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
            let bytes = read(&path).map_err(|err| format!("{label}: {}: {err}", f.file))?;
            let mut image =
                decode_png_rgba(&bytes).map_err(|err| format!("{label}: {}: {err}", f.file))?;
            decoded_bytes += image.rgba.len();
            if decoded_bytes > 512 * 1024 * 1024 {
                return Err("actor_clips: decoded frames exceed 512 MiB".into());
            }
            for p in image.rgba.as_chunks_mut::<4>().0 {
                p[3] = if p[3] >= alpha { 255 } else { 0 };
            }
            frames.push(ActorFrame {
                image,
                pivot: f.pivot,
            });
        }
        clips.push(ActorClip {
            character: e.character.clone(),
            name: e.name.clone(),
            fps: e.fps,
            looped: e.looped,
            frames,
        });
    }
    Ok(clips)
}

/// Inputs are observations; the renderer never writes gameplay or OAM state.
#[derive(Debug, Clone, Copy, Default)]
pub struct ActorInput<'a> {
    pub id: u8,
    pub character: &'a str,
    pub clip: &'a str,
    /// Skeleton origin in NES screen pixels (before widescreen margins).
    pub position: [f32; 2],
    pub flip_h: bool,
    /// The original OAM entries this full-character image replaces.
    pub oam_mask: u64,
    pub priority_oam: u8,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct ActorInstance {
    pub clip: usize,
    pub frame: usize,
    pub position: [f32; 2],
    pub flip_h: bool,
    pub oam_mask: u64,
    pub priority_oam: u8,
    pub behind: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct ActorClock {
    pub id: u8,
    pub clip: usize,
    pub began: u64,
    pub last: u64,
}
