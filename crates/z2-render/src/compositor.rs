//! RGBA compositor: the indexed frame at integer scale, with HD pack art
//! substituted per tile and sprite.
//!
//! ## Why the indexed frame is the ground truth
//!
//! The PPU already composited the NES image into [`IndexedFrame`], and
//! [`z2_ppu::render_wide_indexed`] does the same for widescreen margins. So
//! the base pass here is a plain nearest-neighbour upscale of that image
//! through the master palette: with no pack, scale-1 output equals
//! `indexed_to_rgba` of the frame byte for byte, and the widescreen centre
//! equals the non-wide composite — both by construction, not by re-deriving
//! pixels and hoping they agree.
//!
//! A line is only re-rendered where HD art applies: background slots whose
//! tile the pack replaces are painted from the pack, and then the line's
//! recorded sprites are replayed over them with the hardware mux. Replaying
//! is necessary because HD background art must land *under* the sprites the
//! flat frame already baked on top.
//!
//! ## Rules
//!
//! * HD alpha decides whether an HD pixel covers; **NES opacity decides
//!   priority** — background/sprite ordering, `behind`, and sprite-vs-sprite
//!   claiming follow `z2_ppu::render::render_sprites` exactly, so HD art never
//!   changes what is in front of what. A NES-transparent sprite pixel stays
//!   transparent (silhouette extension is not a v1 feature).
//! * The variant key for a tile is the three opaque NES colour indices of its
//!   palette group on that line: `palette_entry(pal * 4 + 1..=3)` for
//!   background, `0x10 + pal * 4 + 1..=3` for sprites. A sprite that misses
//!   (no variant, no default) borrows the variant of another sprite group
//!   loaded on the line, recoloured to its own colours ([`sprite_art`]), so
//!   enemies flashing after a hit keep their HD art.
//! * Slot geometry follows [`z2_ppu::wide`]: fetch slot `k` covers window
//!   pixels `[8k - fine_x, 8k - fine_x + 8)`. Window pixels `0..256` come
//!   from `record.tiles[k]` (outside a repainted edge strip); margins own only
//!   `wx < 0` and `wx >= 256`, so when `fine_x > 0` two slots straddle a
//!   margin boundary: slot 32 spills its first `fine_x` columns into the
//!   window and the rest into the right margin, and slot 0 spills its first
//!   `fine_x` columns into the *left margin*. Those leading columns are still
//!   slot 0 (the provider's `edge_left[0]`, else `record.tiles[0]`), not
//!   `margins.left`, so the HD pass substitutes them from the same tile.
//! * Every background pixel's tile identity, margins and edge strips
//!   included, comes from [`z2_ppu::wide::wide_bg_tile`] with the line's
//!   [`z2_ppu::wide::edge_fill`] — the same two functions
//!   [`z2_ppu::render_wide_indexed`] draws with, so the HD composite and the
//!   indexed wide frame cannot pick different tiles for the same pixel. In
//!   particular the edge strips ([`Margins::fill_left_clip`],
//!   [`Margins::fill_right_clip`]) take the provider's own edge identities
//!   where it set them (the overworld hides stale nametable columns there),
//!   and on a line the ROM masks with its opaque sprite column at x 248 the
//!   sprite replay leaves window pixels 248..256 alone, so they keep the
//!   pack's art for that tile (or the upscaled original where the pack has
//!   none).
//! * Widescreen margin sprites ([`Margins::sprites`], drawn into the wide
//!   indexed frame by [`z2_ppu::render_wide_indexed`]) are replayed after the
//!   window's sprites, through the same per-line expansion
//!   ([`z2_ppu::wide::margin_sprite_rows`]) and the same claim / `behind`
//!   rules against the margin background; they take pack art like any
//!   sprite (no bleed).
//! * Lines the pack cannot safely improve keep the exact NES image: greyscale
//!   lines (the greyscale bit rewrites every index), unrecorded lines,
//!   `pixel_split` lines, and [`LineRecord::split`] lines whose fine X moved,
//!   where the slot-to-pixel mapping does not hold for the whole line.
//! * Integer arithmetic only, no allocation in [`Compositor::compose`], no
//!   hash iteration: output is deterministic.
//!
//! ## Pack extensions
//!
//! A pack with `"sprite_alpha": "art"` or with layers that match the scene
//! takes a second sprite pass, [`Compositor::replay_sprites_shaped`]: every
//! NES sprite pixel on the line is first painted back to the true background
//! (layer, HD cell or NES colour), then the sprites are drawn front to back
//! with claims kept per *output* pixel. In `art` mode a replaced sprite covers
//! where its cell is opaque, whatever the NES pattern says, and a cell's
//! `bleed` is drawn around its box where nothing else claimed the pixel.
//! `behind` still tests NES background opacity.
//!
//! Layers ([`crate::pack::Layer`]) are sampled at
//! `window_x * scale - (x * scale - camera_x * scale * scroll / 100)`: a back
//! layer shows where the background is transparent (NES pattern 0 with no HD
//! cell, or a hole in the HD cell), a front layer covers the background.
//! Split lines have no usable tile identities, so there a back layer shows
//! where the indexed pixel equals the line's backdrop and front layers are
//! skipped. A pack without these keys composes exactly as before.
//!
//! The game draws its dialogue, sign and pause windows straight into the
//! nametable, over whatever tiles the scene had there. A front layer is
//! painted for the scene's own tiles (a town facade over the house tiles),
//! so on a line that crosses one of those windows no front layer is drawn
//! between the window's borders ([`text_box_span`]); the window then shows
//! as it does on the NES, over the facade instead of under it.

use z2_ppu::record::{BgTileId, FrameRecord, LineRecord, NO_PAGE, RECORD_TILES_PER_LINE};
use z2_ppu::wide::{
    chr_sub, edge_drops_window_sprite, edge_fill, left_sprite_fill, margin_sprite_paints,
    margin_sprite_rows, wide_bg_tile, wide_width, EdgeFill, Margins, WideFrame, MARGIN_SLOTS,
};
use z2_ppu::{
    IndexedFrame, SpriteRef, HEIGHT, PPUMASK_GRAYSCALE, PPUMASK_SHOW_BG, PPUMASK_SHOW_LEFT_BG,
    PPUMASK_SHOW_LEFT_SPRITES, PPUMASK_SHOW_SPRITES, WIDTH,
};

use crate::pack::{BackgroundTileSignature, CellRef, HdPack, LayerDepth, SceneView};
use crate::palette::MasterPalette;
use crate::MAX_SCALE;

/// Indexed input: the plain NES frame, or a widescreen frame.
#[derive(Debug, Clone, Copy)]
pub struct IndexedView<'a> {
    /// `width * 240` NES palette indices, row-major.
    pub pixels: &'a [u8],
    /// Pixels per row (256 plus `2 * 8 * margin_tiles`).
    pub width: usize,
}

impl<'a> IndexedView<'a> {
    /// The 256x240 NES frame.
    #[must_use]
    pub fn frame(frame: &'a IndexedFrame) -> Self {
        Self {
            pixels: frame.as_slice(),
            width: WIDTH,
        }
    }

    /// A widescreen indexed frame from [`z2_ppu::render_wide_indexed`].
    #[must_use]
    pub fn wide(wide: &'a WideFrame) -> Self {
        Self {
            pixels: &wide.pixels,
            width: wide.width,
        }
    }

    /// Margin width per side in pixels.
    #[must_use]
    pub fn margin_px(&self) -> usize {
        self.width.saturating_sub(WIDTH) / 2
    }
}

/// Everything one [`Compositor::compose`] call reads.
#[derive(Debug, Clone, Copy)]
pub struct ComposeInput<'a> {
    /// The composited indexed image (frame or wide frame).
    pub indexed: IndexedView<'a>,
    /// Record of the same frame.
    pub record: &'a FrameRecord,
    /// Full CHR image (32 x 4 KiB); missing bytes read as zero planes.
    pub chr_rom: &'a [u8],
    /// HD art, when a pack is loaded.
    pub pack: Option<&'a HdPack>,
    /// Margin tile identities; required for HD art in the margins.
    pub margins: Option<&'a Margins>,
    /// Display palette for indexed pixels.
    pub palette: &'a MasterPalette,
    /// The scene on screen, for the pack's layers. `None` hides every layer
    /// that names a scene.
    pub scene: Option<SceneView>,
    /// Emulated time for layer drift and animation: `(frame, frames per
    /// second)`. `None` is time 0 (still layers, no drift).
    pub time: Option<(u64, f64)>,
}

/// A layer's drift in whole pack pixels and its animation frame at emulated
/// `time` (see [`ComposeInput::time`]). Pure, so a replay, a state load or a
/// headless render at the same frame lands on the same pixels.
#[must_use]
pub fn layer_motion(
    layer: &crate::pack::Layer,
    pack_scale: u32,
    time: Option<(u64, f64)>,
) -> ([i64; 2], usize) {
    let Some((frame, hz)) = time.filter(|&(_, hz)| hz.is_finite() && hz > 0.0) else {
        return ([0, 0], 0);
    };
    let seconds = frame as f64 / hz;
    let ps = f64::from(pack_scale);
    let offset = |v: f64| (v * ps * seconds).floor() as i64;
    let n = layer.frames.len().max(1);
    let index = if n > 1 {
        ((seconds * layer.fps + 1e-9).floor() as u64 % n as u64) as usize
    } else {
        0
    };
    ([offset(layer.drift[0]), offset(layer.drift[1])], index)
}

/// Compose-time failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComposeError {
    /// Scale outside 1..=8.
    BadScale(u32),
    /// `out` is not [`Compositor::rgba_len`] bytes.
    BadOutLen { want: usize, got: usize },
    /// The indexed source is not `width * 240` pixels.
    BadSourceLen { want: usize, got: usize },
    /// The source row width does not match the configured margins.
    SourceWidth { want: usize, got: usize },
    /// The pack's cells cannot be point-sampled at this output scale.
    ScaleMismatch { pack: u32, output: u32 },
}

impl core::fmt::Display for ComposeError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            ComposeError::BadScale(s) => write!(f, "scale {s} out of range (1..={MAX_SCALE})"),
            ComposeError::BadOutLen { want, got } => {
                write!(f, "output buffer is {got} bytes, expected {want}")
            }
            ComposeError::BadSourceLen { want, got } => {
                write!(f, "indexed source is {got} pixels, expected {want}")
            }
            ComposeError::SourceWidth { want, got } => write!(
                f,
                "indexed source is {got} px wide, expected {want} for the configured margins"
            ),
            ComposeError::ScaleMismatch { pack, output } => write!(
                f,
                "output scale {output} does not divide pack scale {pack}: pick a scale that \
                 divides it (or the pack's own scale)"
            ),
        }
    }
}

impl std::error::Error for ComposeError {}

/// RGBA compositor at a fixed integer scale and margin width.
#[derive(Debug, Clone)]
pub struct Compositor {
    pub(crate) actors: Vec<crate::actors::ActorInstance>,
    scale: u32,
    margin_tiles: u8,
    src_width: usize,
    out_w: usize,
    out_h: usize,
    /// Scratch: NES background colour index per window pixel.
    bg_index: Vec<u8>,
    /// Scratch: NES background opacity per window pixel (sprite priority).
    bg_opaque: Vec<bool>,
    /// Scratch: sprite pixel claims per source column (window and margins).
    claimed: Vec<bool>,
    /// Scratch: sprite claims per output pixel of one line (shaped pass),
    /// indexed `(fy * src_width + source column) * scale + fx`.
    claimed_out: Vec<bool>,
    /// Scratch: background slot per source column of one line.
    col_id: Vec<BgTileId>,
    /// Scratch: pattern column of that slot per source column.
    col_sub: Vec<u8>,
    /// Scratch: NES background opacity per source column.
    col_opaque: Vec<bool>,
    /// Scratch: the pack layers shown this frame, in drawing order.
    layers: Vec<LayerPlan>,
    /// Scratch: byte offset of each layer's source row for each of the
    /// `scale` output rows of the current line, or `NO_ROW`.
    /// Indexed `plan * scale + fy`.
    layer_rows: Vec<u32>,
    /// Scratch: indices into `layers` of the back and the front layers, in
    /// drawing order, so a sample only walks its own depth.
    back_idx: Vec<u16>,
    front_idx: Vec<u16>,
    /// Whether any back / front layer reaches the line `prepare_layer_rows`
    /// was last called for.
    line_back: bool,
    line_front: bool,
    /// Source columns `first..=last` of the line `resolve_columns` was last
    /// called for that a game text window occupies ([`text_box_span`]):
    /// front layers are not drawn there.
    text_box: Option<(usize, usize)>,
}

/// Nametable tiles of the game's window frame (`bank3_Tables_for_dialog_box_rows`,
/// `$B407`): `$CA` corners, `$CB` the horizontal edges, `$CC` the vertical
/// edges. The dialogue, sign and pause windows all use them, and the font
/// sits beside them in every background CHR bank, so the numbers hold on any
/// page.
pub const TEXT_BOX_BORDER_TILES: [u8; 3] = [0xCA, 0xCB, 0xCC];

/// Columns `first..=last` of one line's background identities (`ids[x]` is
/// the tile under pixel column `x`) that a game text window covers: from the
/// first window-border column to the last, when at least two border cells
/// sit on the line (both sides of the window, or a top or bottom edge). A
/// front layer must not be drawn there: the game wrote the window over the
/// scene's tiles, and the layer was painted for those scene tiles.
#[must_use]
pub fn text_box_span(ids: &[BgTileId]) -> Option<(usize, usize)> {
    let border = |id: &BgTileId| {
        id.fetched && id.page != NO_PAGE && TEXT_BOX_BORDER_TILES.contains(&id.tile)
    };
    let first = ids.iter().position(border)?;
    let last = ids.iter().rposition(border)?;
    // Two cells at least: one 8-px border cell alone is not a window.
    (last >= first + 8).then_some((first, last))
}

/// `layer_rows` entry for "this layer does not reach that output row".
const NO_ROW: u32 = u32::MAX;

/// Most layers one pack may show at once (scratch is reserved for them).
pub const MAX_ACTIVE_LAYERS: usize = 64;

/// One active layer with its frame-constant placement already worked out, so
/// the camera and the `scroll` division stay out of the per-pixel path.
#[derive(Debug, Clone, Copy)]
struct LayerPlan {
    index: u16,
    sheet: u16,
    depth: LayerDepth,
    /// Left edge in HD pixels relative to window x 0, and top edge from the
    /// top of the screen.
    left: i32,
    top: i32,
    width: i32,
    height: i32,
    repeat_x: bool,
    /// The layer names `over_tiles`, so it needs the column's identity.
    restricted: bool,
}
/// Widest indexed source row (256 plus the widest margins).
const MAX_SRC_WIDTH: usize = WIDTH + 2 * 8 * MARGIN_SLOTS;

impl Compositor {
    /// Compositor at `scale` with no margins.
    ///
    /// # Errors
    /// [`ComposeError::BadScale`] outside 1..=8.
    pub fn new(scale: u32) -> Result<Self, ComposeError> {
        Self::with_margins(scale, 0)
    }

    /// Compositor at `scale` with `margin_tiles` 8-px tiles per side.
    ///
    /// # Errors
    /// [`ComposeError::BadScale`] outside 1..=8.
    pub fn with_margins(scale: u32, margin_tiles: u8) -> Result<Self, ComposeError> {
        let mut me = Self {
            actors: Vec::new(),
            scale: 1,
            margin_tiles: 0,
            src_width: WIDTH,
            out_w: WIDTH,
            out_h: HEIGHT,
            bg_index: vec![0; WIDTH],
            bg_opaque: vec![false; WIDTH],
            claimed: vec![false; MAX_SRC_WIDTH],
            claimed_out: vec![false; MAX_SRC_WIDTH * (MAX_SCALE * MAX_SCALE) as usize],
            col_id: vec![BgTileId::NONE; MAX_SRC_WIDTH],
            col_sub: vec![0; MAX_SRC_WIDTH],
            col_opaque: vec![false; MAX_SRC_WIDTH],
            layers: Vec::with_capacity(MAX_ACTIVE_LAYERS),
            layer_rows: vec![NO_ROW; MAX_ACTIVE_LAYERS * MAX_SCALE as usize],
            back_idx: Vec::with_capacity(MAX_ACTIVE_LAYERS),
            front_idx: Vec::with_capacity(MAX_ACTIVE_LAYERS),
            line_back: false,
            line_front: false,
            text_box: None,
        };
        me.reconfigure(scale, margin_tiles)?;
        Ok(me)
    }

    /// Change scale and margin width. Scratch buffers are size-stable, so
    /// this never allocates.
    ///
    /// # Errors
    /// [`ComposeError::BadScale`] outside 1..=8.
    pub fn reconfigure(&mut self, scale: u32, margin_tiles: u8) -> Result<(), ComposeError> {
        if scale == 0 || scale > MAX_SCALE {
            return Err(ComposeError::BadScale(scale));
        }
        self.scale = scale;
        self.margin_tiles = margin_tiles;
        self.src_width = wide_width(margin_tiles);
        self.out_w = self.src_width * scale as usize;
        self.out_h = HEIGHT * scale as usize;
        Ok(())
    }

    #[must_use]
    pub fn scale(&self) -> u32 {
        self.scale
    }

    #[must_use]
    pub fn margin_tiles(&self) -> u8 {
        self.margin_tiles
    }

    /// Output width in pixels.
    #[must_use]
    pub fn width(&self) -> usize {
        self.out_w
    }

    /// Output height in pixels.
    #[must_use]
    pub fn height(&self) -> usize {
        self.out_h
    }

    /// Bytes one composed frame needs.
    #[must_use]
    pub fn rgba_len(&self) -> usize {
        self.out_w * self.out_h * 4
    }

    /// Whether `pack`'s cells can be point-sampled at output `scale`.
    #[must_use]
    pub fn pack_supports_scale(pack: &HdPack, scale: u32) -> bool {
        scale > 0 && pack.scale().is_multiple_of(scale)
    }

    /// Compose one frame into `out` (row-major RGBA8, [`Compositor::rgba_len`] bytes).
    ///
    /// # Errors
    /// [`ComposeError`] on a buffer, source-size or scale mismatch.
    pub fn compose(&mut self, input: ComposeInput<'_>, out: &mut [u8]) -> Result<(), ComposeError> {
        if out.len() != self.rgba_len() {
            return Err(ComposeError::BadOutLen {
                want: self.rgba_len(),
                got: out.len(),
            });
        }
        if input.indexed.width != self.src_width {
            return Err(ComposeError::SourceWidth {
                want: self.src_width,
                got: input.indexed.width,
            });
        }
        if input.indexed.pixels.len() != self.src_width * HEIGHT {
            return Err(ComposeError::BadSourceLen {
                want: self.src_width * HEIGHT,
                got: input.indexed.pixels.len(),
            });
        }
        let step = match input.pack {
            Some(p) if !Self::pack_supports_scale(p, self.scale) => {
                return Err(ComposeError::ScaleMismatch {
                    pack: p.scale(),
                    output: self.scale,
                })
            }
            Some(p) => p.scale() / self.scale,
            None => 1,
        };

        self.layers.clear();
        if let Some(pack) = input.pack {
            let ps = pack.scale() as i32;
            let camera = input.scene.map_or(0, |s| s.camera_x);
            for (i, layer) in pack.layers().iter().enumerate() {
                if self.layers.len() >= MAX_ACTIVE_LAYERS
                    || !layer.matches(input.scene.as_ref())
                    || layer.background_tile.is_some_and(|signature| {
                        !Self::background_signature_matches(input.record, signature)
                    })
                {
                    continue;
                }
                let ([dx, dy], frame) = layer_motion(layer, pack.scale(), input.time);
                let sheet = layer.frames.get(frame).copied().unwrap_or(layer.sheet);
                let img = &pack.sheets()[usize::from(sheet)];
                let travelled =
                    (i64::from(camera) * i64::from(ps) * i64::from(layer.scroll)).div_euclid(100);
                let mut left = i64::from(layer.x) * i64::from(ps) - travelled + dx;
                if layer.repeat_x && img.width > 0 {
                    // Same picture, small numbers: the drift wraps.
                    left = left.rem_euclid(i64::from(img.width)) - i64::from(img.width);
                }
                self.layers.push(LayerPlan {
                    index: i as u16,
                    sheet,
                    depth: layer.depth,
                    left: left.clamp(i64::from(i32::MIN / 2), i64::from(i32::MAX / 2)) as i32,
                    top: (i64::from(layer.y) * i64::from(ps) + dy)
                        .clamp(i64::from(i32::MIN / 2), i64::from(i32::MAX / 2))
                        as i32,
                    width: img.width as i32,
                    height: img.height as i32,
                    repeat_x: layer.repeat_x,
                    restricted: !layer.over_tiles.is_empty(),
                });
            }
        }
        self.back_idx.clear();
        self.front_idx.clear();
        for (i, plan) in self.layers.iter().enumerate() {
            match plan.depth {
                LayerDepth::Back => self.back_idx.push(i as u16),
                LayerDepth::Front => self.front_idx.push(i as u16),
            }
        }
        let layered = !self.layers.is_empty();
        // Blink/visibility is still decided by the game's actual OAM. Hidden
        // actors must not leave an animated ghost during damage or transitions.
        // An actor out in a widescreen margin is visible through the margin
        // sprites that stand in for its hidden OAM entries.
        if !self.actors.is_empty() {
            let mut visible = 0u64;
            let mut visible_margin = 0u64;
            for y in 0..HEIGHT {
                for s in input.record.sprites_on(y) {
                    visible |= 1u64 << s.oam_index;
                }
                for r in input
                    .margins
                    .into_iter()
                    .flat_map(|m| margin_sprite_rows(m, input.record, y))
                    .filter(|r| r.index < 64)
                {
                    visible_margin |= 1u64 << r.index;
                }
            }
            self.actors
                .retain(|a| visible & a.oam_mask != 0 || visible_margin & a.margin_mask != 0);
            for a in &mut self.actors {
                let window = input
                    .record
                    .sprites
                    .iter()
                    .filter(|s| a.oam_mask & (1u64 << s.oam_index) != 0)
                    .min_by_key(|s| s.oam_index)
                    .map(|s| s.behind);
                let margin = || {
                    input.margins.and_then(|m| {
                        m.sprites
                            .iter()
                            .take(64)
                            .enumerate()
                            .find(|(i, _)| a.margin_mask & (1u64 << i) != 0)
                            .map(|(_, s)| s.attr & 0x20 != 0)
                    })
                };
                a.behind = window.or_else(margin).unwrap_or(false);
            }
        }

        for y in 0..HEIGHT {
            self.upscale_line(&input, y, out);
            let Some(pack) = input.pack else { continue };
            let rec = input.record.line(y);
            if !Self::line_uses_hd(rec) {
                if layered && rec.valid && rec.mask & PPUMASK_GRAYSCALE == 0 {
                    self.prepare_layer_rows(pack, step, y);
                    self.paint_layers_by_index(rec, &input, pack, step, y, out);
                }
                continue;
            }
            let hd_bg = self.paint_hd_background(rec, &input, pack, step, y, out);
            if layered || pack.sprite_alpha_art() || !self.actors.is_empty() {
                self.resolve_columns(rec, &input, y);
                self.prepare_layer_rows(pack, step, y);
                let painted = layered && self.paint_layers(rec, &input, pack, step, y, out);
                self.replay_sprites_shaped(rec, &input, pack, step, y, hd_bg || painted, out);
            } else {
                self.replay_sprites(rec, &input, pack, step, y, hd_bg, out);
            }
        }
        Ok(())
    }

    fn actor_owns(&self, slot: u8) -> bool {
        self.actors.iter().any(|a| a.oam_mask & (1u64 << slot) != 0)
    }

    /// Whether an actor replaces margin sprite `index` (`Margins::sprites`).
    fn actor_owns_margin(&self, index: usize) -> bool {
        index < 64
            && self
                .actors
                .iter()
                .any(|a| a.margin_mask & (1u64 << index) != 0)
    }

    fn actor_reaches(&self, actor: crate::actors::ActorInstance, pack: &HdPack, y: usize) -> bool {
        let frame = &pack.actor_clips()[actor.clip].frames[actor.frame];
        let top = (actor.position[1] * pack.scale() as f32 - frame.pivot[1]).round();
        let row = y as f32 * pack.scale() as f32;
        row + pack.scale() as f32 > top && row < top + frame.height as f32
    }

    /// The hit flash: the cartridge draws the actor in another sprite
    /// palette, so recolour the frame along that palette's three colours by
    /// brightness (dark, mid, light), as the NES would show its tiles.
    fn flash_colour(rgba: [u8; 4], ramp: &[[u8; 4]; 3]) -> [u8; 4] {
        let luma =
            |p: [u8; 4]| 299 * u32::from(p[0]) + 587 * u32::from(p[1]) + 114 * u32::from(p[2]);
        let t = luma(rgba) * 2 / 255; // 0..=2000
        let (a, b, f) = if t < 1000 {
            (ramp[0], ramp[1], t)
        } else {
            (ramp[1], ramp[2], t - 1000)
        };
        let mix = |i: usize| ((u32::from(a[i]) * (1000 - f) + u32::from(b[i]) * f) / 1000) as u8;
        [mix(0), mix(1), mix(2), rgba[3]]
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_actor_row(
        &mut self,
        actor: crate::actors::ActorInstance,
        pack: &HdPack,
        rec: &LineRecord,
        input: &ComposeInput<'_>,
        step: u32,
        y: usize,
        out: &mut [u8],
    ) {
        let frame = &pack.actor_clips()[actor.clip].frames[actor.frame];
        let Some(image) = frame.image() else {
            return;
        };
        let ramp = actor.flash_palette.map(|pal| {
            let mut ramp = [1u8, 2, 3].map(|sub| {
                input
                    .palette
                    .rgba(rec.palette_entry(0x10 + usize::from(pal & 3) * 4 + usize::from(sub)))
            });
            ramp.sort_by_key(|p| {
                299 * u32::from(p[0]) + 587 * u32::from(p[1]) + 114 * u32::from(p[2])
            });
            ramp
        });
        let p = pack.scale() as f32;
        let pivot_x = if actor.flip_h {
            image.width as f32 - frame.pivot[0]
        } else {
            frame.pivot[0]
        };
        let left = (actor.position[0] * p - pivot_x).round() as i32;
        let top = (actor.position[1] * p - frame.pivot[1]).round() as i32;
        let n = self.scale as usize;
        let margin = (self.src_width - WIDTH) / 2;
        let start = (left.div_euclid(step as i32) + (margin * n) as i32).max(0) as usize;
        let end = ((left + image.width as i32 + step as i32 - 1).div_euclid(step as i32)
            + (margin * n) as i32)
            .clamp(0, self.out_w as i32) as usize;
        for fy in 0..n {
            let iy = ((y * n + fy) * step as usize) as i32 - top;
            if iy < 0 || iy >= image.height as i32 {
                continue;
            }
            for dx in start..end {
                let sx = dx / n;
                let wx = sx as i32 - margin as i32;
                if (0..8).contains(&wx) && rec.mask & PPUMASK_SHOW_LEFT_SPRITES == 0 {
                    continue;
                }
                if Self::line_edge_fill(input, y).right && (248..256).contains(&wx) {
                    continue;
                }
                let ix = (dx as i32 - (margin * n) as i32) * step as i32 - left;
                if ix < 0 || ix >= image.width as i32 {
                    continue;
                }
                let ix = if actor.flip_h {
                    image.width as i32 - 1 - ix
                } else {
                    ix
                };
                let rgba = image.pixel(ix as u32, iy as u32);
                let claim = (fy * self.src_width + sx) * n + dx % n;
                if rgba[3] == 0 || self.claimed_out[claim] {
                    continue;
                }
                self.claimed_out[claim] = true;
                if actor.behind && self.col_opaque[sx] {
                    continue;
                }
                let rgba = match &ramp {
                    Some(ramp) => Self::flash_colour(rgba, ramp),
                    None => rgba,
                };
                let dst = ((y * n + fy) * self.out_w + dx) * 4;
                out[dst..dst + 4].copy_from_slice(&rgba);
            }
        }
    }

    /// Nearest-upscale one indexed line into its `scale` output rows.
    fn upscale_line(&self, input: &ComposeInput<'_>, y: usize, out: &mut [u8]) {
        let n = self.scale as usize;
        let src = &input.indexed.pixels[y * self.src_width..(y + 1) * self.src_width];
        let row_bytes = self.out_w * 4;
        let first = y * n;
        let (head, tail) = out.split_at_mut((first + 1) * row_bytes);
        let row0 = &mut head[first * row_bytes..];
        for (x, &idx) in src.iter().enumerate() {
            let rgba = input.palette.rgba(idx);
            for px in row0[x * n * 4..(x * n + n) * 4].as_chunks_mut::<4>().0 {
                px.copy_from_slice(&rgba);
            }
        }
        for r in 0..n.saturating_sub(1) {
            tail[r * row_bytes..(r + 1) * row_bytes].copy_from_slice(row0);
        }
    }

    /// Checked once per layer per frame, before any scanline/pixel work.
    fn background_signature_matches(record: &FrameRecord, s: BackgroundTileSignature) -> bool {
        let Some(rec) = record.lines.get(s.y as usize) else {
            return false;
        };
        if s.x >= WIDTH as u64
            || !Self::line_uses_hd(rec)
            || rec.mask & PPUMASK_SHOW_BG == 0
            || (s.x < 8 && rec.mask & PPUMASK_SHOW_LEFT_BG == 0)
        {
            return false;
        }
        let (id, _) = Self::window_tile(rec, s.x as i32);
        id.fetched
            && u64::from(id.page) == s.page
            && u64::from(id.tile) == s.tile
            && s.colors
                .is_none_or(|colors| bg_colors(rec, id.pal).map(|c| u64::from(c & 0x3f)) == colors)
    }

    /// Can HD art be applied to this line at all?
    ///
    /// A fetch-state write that leaves fine X alone does not disturb the
    /// slot-to-pixel mapping: every recorded tile still carries the identity
    /// it was fetched with. Zelda II makes one such write on most sideview
    /// frames (`$2000` when its frame logic ends, on whatever line that is),
    /// so refusing those lines showed as a flickering row of original art.
    fn line_uses_hd(rec: &LineRecord) -> bool {
        rec.valid
            && rec.mask & PPUMASK_GRAYSCALE == 0
            && !rec.pixel_split
            && (!rec.split || rec.fine_x == rec.fine_x_end)
    }

    /// Tile identity of window pixel `wx`'s slot (window pixels only).
    fn window_tile(rec: &LineRecord, wx: i32) -> (BgTileId, u8) {
        let fine_x = i32::from(rec.fine_x & 7);
        let k = (wx + fine_x).div_euclid(8);
        let sub_x = (wx + fine_x).rem_euclid(8) as u8;
        let id = if (0..RECORD_TILES_PER_LINE as i32).contains(&k) {
            rec.tiles[k as usize]
        } else {
            BgTileId::NONE
        };
        (id, sub_x)
    }

    /// Paint every background slot the pack replaces. Returns true when any
    /// pixel was painted (the line's sprites then need replaying).
    fn paint_hd_background(
        &mut self,
        rec: &LineRecord,
        input: &ComposeInput<'_>,
        pack: &HdPack,
        step: u32,
        y: usize,
        out: &mut [u8],
    ) -> bool {
        if rec.mask & PPUMASK_SHOW_BG == 0 {
            return false;
        }
        let left_px = (self.src_width.saturating_sub(WIDTH) / 2) as i32;
        let fine_x = i32::from(rec.fine_x & 7);
        let fill = Self::line_edge_fill(input, y);
        // Every slot that reaches the source row, margins included. Slot 0's
        // leading `fine_x` columns land at `wx < 0` and slot 32 spans the
        // right boundary; `wide_bg_tile` picks each pixel's identity.
        let first = (-left_px + fine_x).div_euclid(8);
        let last = (WIDTH as i32 + left_px - 1 + fine_x).div_euclid(8);
        let mut painted = false;
        for k in first..=last {
            painted |= self.paint_slot(rec, input, fill, pack, step, y, k, out);
        }
        painted
    }

    /// The edge strips line `y` repaints: [`z2_ppu::wide::edge_fill`], the
    /// same gate `render_wide_indexed` uses, or none without margins.
    fn line_edge_fill(input: &ComposeInput<'_>, y: usize) -> EdgeFill {
        input.margins.map_or(EdgeFill::NONE, |m| {
            edge_fill(input.record, m, y, input.chr_rom)
        })
    }

    /// Paint the HD cells of slot `k`'s pixels. Each pixel's tile identity
    /// comes from [`wide_bg_tile`], so a slot that straddles an edge strip or
    /// the window boundary can take two identities. Returns whether it
    /// painted.
    #[allow(clippy::too_many_arguments)]
    fn paint_slot(
        &self,
        rec: &LineRecord,
        input: &ComposeInput<'_>,
        fill: EdgeFill,
        pack: &HdPack,
        step: u32,
        y: usize,
        k: i32,
        out: &mut [u8],
    ) -> bool {
        let n = self.scale as usize;
        let left_px = (self.src_width.saturating_sub(WIDTH) / 2) as i32;
        let fine_x = i32::from(rec.fine_x & 7);
        let ml = input.margins.map(|m| &m.lines[y]);
        let backdrop = input.palette.rgba(rec.backdrop);
        let ps = pack.scale();
        let mut painted = false;
        let mut cached: Option<(BgTileId, Option<CellRef>)> = None;

        for px in 0..8i32 {
            let wx = 8 * k + px - fine_x;
            if !(-left_px..WIDTH as i32 + left_px).contains(&wx) {
                continue;
            }
            let (id, sub_x) = wide_bg_tile(rec, ml, fill, wx);
            debug_assert_eq!(i32::from(sub_x), px);
            if !id.fetched || id.page == NO_PAGE {
                continue;
            }
            let cell = match cached {
                Some((c_id, cell)) if c_id == id => cell,
                _ => {
                    let cell = pack.lookup(id.page, id.tile, bg_colors(rec, id.pal));
                    cached = Some((id, cell));
                    cell
                }
            };
            let Some(cell) = cell else { continue };
            let cellpx = pack.cell_pixels(cell);
            let ox = (wx + left_px) as usize;
            for fy in 0..n {
                let hd_y = u32::from(id.fine_y & 7) * ps + (fy as u32) * step;
                let row = cellpx.row(hd_y);
                let oy = y * n + fy;
                for fx in 0..n {
                    let sidx = ((px as u32 * ps + fx as u32 * step) as usize) * 4;
                    let dst = (oy * self.out_w + ox * n + fx) * 4;
                    if row[sidx + 3] != 0 {
                        out[dst..dst + 4].copy_from_slice(&[
                            row[sidx],
                            row[sidx + 1],
                            row[sidx + 2],
                            0xFF,
                        ]);
                    } else {
                        // Transparent HD background == transparent NES
                        // background: the line's backdrop colour.
                        out[dst..dst + 4].copy_from_slice(&backdrop);
                    }
                }
            }
            painted = true;
        }
        painted
    }

    /// Replay the line's recorded sprites over the (possibly HD) background.
    ///
    /// Needed when HD background art was painted (it would otherwise cover
    /// sprites the flat frame already had on top) or when a sprite itself has
    /// HD art. The mux is `render_sprites`': OAM order claims pixels, and a
    /// `behind` sprite loses to opaque background. Widescreen margin sprites
    /// follow the window's sprites with the same rules, against the margin
    /// background.
    #[allow(clippy::too_many_arguments)]
    fn replay_sprites(
        &mut self,
        rec: &LineRecord,
        input: &ComposeInput<'_>,
        pack: &HdPack,
        step: u32,
        y: usize,
        hd_bg: bool,
        out: &mut [u8],
    ) {
        if rec.mask & PPUMASK_SHOW_SPRITES == 0 {
            return;
        }
        let sprites = input.record.sprites_on(y);
        let margin_rows = || {
            input
                .margins
                .into_iter()
                .flat_map(move |m| margin_sprite_rows(m, input.record, y))
        };
        let has_margin = margin_rows().next().is_some();
        if sprites.is_empty() && !has_margin {
            return;
        }
        // Widescreen right-edge recovery: when the line is covered by the
        // ROM's edge-mask sprite column and the fill is on, the wide indexed
        // frame already shows the background there, so the replay must not
        // put the mask (or anything else) back over those columns. Same gate
        // as `z2_ppu::wide::render_wide_indexed`, via the same helper.
        let fill = Self::line_edge_fill(input, y);
        let hd_of = |s: &SpriteRef| sprite_art(pack, rec, input.palette, s);
        let any_hd_sprite = sprites.iter().any(|s| hd_of(s).is_some())
            || margin_rows().any(|r| hd_of(&r.sprite).is_some());
        if !hd_bg && !any_hd_sprite {
            return;
        }

        self.resolve_background_layer(rec, input, y);
        if has_margin {
            self.resolve_columns(rec, input, y);
        }
        let margin_px = self.src_width.saturating_sub(WIDTH) / 2;
        let show_left_spr = rec.mask & PPUMASK_SHOW_LEFT_SPRITES != 0;
        let left_fill = input.margins.is_some_and(|m| left_sprite_fill(m, rec));
        for c in self.claimed[..self.src_width].iter_mut() {
            *c = false;
        }
        // Window sprites (real OAM) first, then the margin sprites. Where the
        // right strip is recovered the edge-mask column is gone, exactly as in
        // `z2_ppu::wide::render_wide_indexed`.
        let rows = sprites
            .iter()
            .filter(|s| !edge_drops_window_sprite(fill, s, sprites, input.chr_rom))
            .map(|s| (*s, i32::from(s.x), true))
            .chain(margin_rows().map(|r| (r.sprite, r.x, false)));
        for (s, left, window) in rows {
            let hd = hd_of(&s);
            let id = sprite_tile_id(&s);
            for dx in 0..8i32 {
                let wx = left + dx;
                if window {
                    let x = wx as usize;
                    if x >= WIDTH {
                        break;
                    }
                    if x < 8 && !show_left_spr && !fill.left {
                        continue;
                    }
                } else if !margin_sprite_paints(wx, left, margin_px as i32, left_fill, fill.right) {
                    continue;
                }
                let sx = (wx + margin_px as i32) as usize;
                let col = if s.flip_h { 7 - dx } else { dx } as usize;
                let sub = chr_sub(input.chr_rom, id, col as u8).unwrap_or(0);
                if sub == 0 {
                    continue; // NES-transparent: no coverage, no claim.
                }
                if self.claimed[sx] {
                    continue;
                }
                self.claimed[sx] = true;
                let (bg_opaque, bg_index) = if window {
                    (self.bg_opaque[wx as usize], self.bg_index[wx as usize])
                } else {
                    self.column_bg(rec, input, sx)
                };
                if s.behind && bg_opaque {
                    continue;
                }
                match hd {
                    None => {
                        let rgba = input.palette.rgba(
                            rec.palette_entry(0x10 + usize::from(s.pal & 3) * 4 + usize::from(sub)),
                        );
                        self.fill_block(sx, y, &rgba, out);
                    }
                    Some(art) => {
                        let bg = input.palette.rgba(bg_index);
                        self.draw_hd_sprite_pixel(pack, art, &s, col, step, sx, y, &bg, out);
                    }
                }
            }
        }
    }

    /// One NES pixel (`col` of sprite row `s`) of HD sprite art into its
    /// `scale x scale` block at source column `sx`; holes show `bg`.
    #[allow(clippy::too_many_arguments)]
    fn draw_hd_sprite_pixel(
        &self,
        pack: &HdPack,
        art: SpriteArt,
        s: &SpriteRef,
        col: usize,
        step: u32,
        sx: usize,
        y: usize,
        bg: &[u8; 4],
        out: &mut [u8],
    ) {
        let n = self.scale as usize;
        let cellpx = pack.cell_pixels(art.cell);
        let ps = pack.scale();
        for fy in 0..n {
            let sub_row = fy as u32 * step;
            let hd_y =
                u32::from(s.fine_row & 7) * ps + if s.flip_v { ps - 1 - sub_row } else { sub_row };
            let row = cellpx.row(hd_y);
            let oy = y * n + fy;
            for fx in 0..n {
                let sub_col = fx as u32 * step;
                let mirrored = if s.flip_h { ps - 1 - sub_col } else { sub_col };
                let sidx = ((col as u32 * ps + mirrored) as usize) * 4;
                let dst = (oy * self.out_w + sx * n + fx) * 4;
                if row[sidx + 3] != 0 {
                    let px = [row[sidx], row[sidx + 1], row[sidx + 2], 0xFF];
                    let px = art.tint.map_or(px, |t| t.apply(px));
                    out[dst..dst + 4].copy_from_slice(&px);
                } else {
                    // Hole in HD sprite art shows the background.
                    out[dst..dst + 4].copy_from_slice(bg);
                }
            }
        }
    }

    /// NES background opacity and colour index of source column `sx`, from
    /// [`Compositor::resolve_columns`] (margins included).
    fn column_bg(&self, rec: &LineRecord, input: &ComposeInput<'_>, sx: usize) -> (bool, u8) {
        if !self.col_opaque[sx] {
            return (false, rec.backdrop);
        }
        let id = self.col_id[sx];
        let sub = chr_sub(input.chr_rom, id, self.col_sub[sx]).unwrap_or(0);
        (
            true,
            rec.palette_entry(usize::from(id.pal & 3) * 4 + usize::from(sub)),
        )
    }

    /// NES background colour index and opacity per window pixel, exactly as
    /// `render_background` computes them (needed for sprite priority and for
    /// the colour under a hole in HD sprite art).
    fn resolve_background_layer(&mut self, rec: &LineRecord, input: &ComposeInput<'_>, y: usize) {
        let show_bg = rec.mask & PPUMASK_SHOW_BG != 0;
        let show_left_bg = rec.mask & PPUMASK_SHOW_LEFT_BG != 0;
        // Repainted edge pixels take the recovered background's opacity.
        let fill = Self::line_edge_fill(input, y);
        let ml = input.margins.map(|m| &m.lines[y]);
        for x in 0..WIDTH {
            let (idx, opaque) = if !show_bg {
                (rec.backdrop, false)
            } else {
                let repainted = fill.repaints(x as i32);
                let (id, sub_x) = if repainted {
                    wide_bg_tile(rec, ml, fill, x as i32)
                } else {
                    Self::window_tile(rec, x as i32)
                };
                match chr_sub(input.chr_rom, id, sub_x) {
                    None | Some(0) => (rec.backdrop, false),
                    Some(sub) if x < 8 && !show_left_bg && !repainted => {
                        let _ = sub;
                        (rec.backdrop, false)
                    }
                    Some(sub) => (
                        rec.palette_entry(usize::from(id.pal & 3) * 4 + usize::from(sub)),
                        true,
                    ),
                }
            };
            self.bg_index[x] = idx;
            self.bg_opaque[x] = opaque;
        }
    }

    /// Background slot, pattern column and NES opacity of every source
    /// column of line `y` (margins included), per
    /// [`z2_ppu::wide::wide_bg_tile`].
    fn resolve_columns(&mut self, rec: &LineRecord, input: &ComposeInput<'_>, y: usize) {
        let margin_px = (self.src_width.saturating_sub(WIDTH) / 2) as i32;
        let fill = Self::line_edge_fill(input, y);
        let ml = input.margins.map(|m| &m.lines[y]);
        for sx in 0..self.src_width {
            let (id, sub_x) = wide_bg_tile(rec, ml, fill, sx as i32 - margin_px);
            let usable = id.fetched && id.page != NO_PAGE;
            self.col_id[sx] = if usable { id } else { BgTileId::NONE };
            self.col_sub[sx] = sub_x;
            self.col_opaque[sx] =
                usable && chr_sub(input.chr_rom, id, sub_x).is_some_and(|v| v != 0);
        }
        self.text_box = text_box_span(&self.col_id[..self.src_width]);
    }

    /// Find each layer's source row for every output row of line `y`, once
    /// per line, so the per-pixel path is a subtraction and an index.
    fn prepare_layer_rows(&mut self, pack: &HdPack, step: u32, y: usize) {
        let n = self.scale as usize;
        let ps = pack.scale() as i32;
        let (mut back, mut front) = (false, false);
        for (i, plan) in self.layers.iter().enumerate() {
            let mut any = false;
            for fy in 0..n {
                let ly = y as i32 * ps + (fy as i32) * step as i32 - plan.top;
                let hit = (0..plan.height).contains(&ly);
                any |= hit;
                self.layer_rows[i * n + fy] = if hit {
                    (ly as u32) * (plan.width as u32) * 4
                } else {
                    NO_ROW
                };
            }
            match (any, plan.depth) {
                (true, LayerDepth::Back) => back = true,
                (true, LayerDepth::Front) => front = true,
                _ => {}
            }
        }
        (self.line_back, self.line_front) = (back, front);
    }

    /// Topmost layer of `depth` opaque at window HD column `hwx` of output
    /// sub-row `fy`. A front layer restricted by `over_tiles` only counts
    /// over tile `id`. Needs [`Compositor::prepare_layer_rows`] for the line.
    #[inline]
    fn sample_layers(
        &self,
        pack: &HdPack,
        depth: LayerDepth,
        id: BgTileId,
        hwx: i32,
        fy: usize,
    ) -> Option<[u8; 4]> {
        let (list, any) = match depth {
            LayerDepth::Back => (&self.back_idx, self.line_back),
            LayerDepth::Front => (&self.front_idx, self.line_front),
        };
        if !any {
            return None;
        }
        let n = self.scale as usize;
        list.iter().rev().find_map(|&li| {
            let i = usize::from(li);
            let plan = &self.layers[i];
            let row = self.layer_rows[i * n + fy];
            if row == NO_ROW {
                return None;
            }
            // A restricted layer needs a known tile; an unfetched slot has none.
            if plan.restricted {
                let layer = &pack.layers()[usize::from(plan.index)];
                if !(id.fetched && layer.covers_tile(id.page, id.tile)) {
                    return None;
                }
            }
            let mut lx = hwx - plan.left;
            if plan.repeat_x {
                lx = lx.rem_euclid(plan.width);
            } else if lx < 0 || lx >= plan.width {
                return None;
            }
            let at = (row + lx as u32 * 4) as usize;
            let px = &pack.sheets()[usize::from(plan.sheet)].rgba[at..at + 4];
            (px[3] != 0).then(|| [px[0], px[1], px[2], px[3]])
        })
    }

    /// The background without sprites at output sub-pixel `(fx, fy)` of
    /// source column `sx`: front layer, HD cell, NES colour, back layer,
    /// backdrop. Needs [`Compositor::resolve_columns`] and
    /// [`Compositor::prepare_layer_rows`] for the line.
    #[allow(clippy::too_many_arguments)]
    fn background_pixel(
        &self,
        rec: &LineRecord,
        input: &ComposeInput<'_>,
        pack: &HdPack,
        cell: Option<CellRef>,
        step: u32,
        sx: usize,
        fx: u32,
        fy: u32,
    ) -> [u8; 4] {
        let ps = pack.scale();
        let margin_px = (self.src_width.saturating_sub(WIDTH) / 2) as i32;
        let id = self.col_id[sx];
        let hwx = (sx as i32 - margin_px) * ps as i32 + (fx * step) as i32;
        let in_text_box = self
            .text_box
            .is_some_and(|(first, last)| (first..=last).contains(&sx));
        if !in_text_box {
            if let Some(px) = self.sample_layers(pack, LayerDepth::Front, id, hwx, fy as usize) {
                return [px[0], px[1], px[2], 0xFF];
            }
        }
        let mut transparent = !self.col_opaque[sx];
        if let Some(cell) = cell {
            let px = pack.cell_pixels(cell).pixel(
                u32::from(self.col_sub[sx]) * ps + fx * step,
                u32::from(id.fine_y & 7) * ps + fy * step,
            );
            if px[3] != 0 {
                return [px[0], px[1], px[2], 0xFF];
            }
            transparent = true;
        }
        if !transparent {
            let sub = chr_sub(input.chr_rom, id, self.col_sub[sx]).unwrap_or(0);
            return input
                .palette
                .rgba(rec.palette_entry(usize::from(id.pal & 3) * 4 + usize::from(sub)));
        }
        match self.sample_layers(pack, LayerDepth::Back, id, hwx, fy as usize) {
            Some(px) => [px[0], px[1], px[2], 0xFF],
            None => input.palette.rgba(rec.backdrop),
        }
    }

    /// HD cell of source column `sx`'s background slot, if the pack has one.
    fn column_cell(&self, rec: &LineRecord, pack: &HdPack, sx: usize) -> Option<CellRef> {
        let id = self.col_id[sx];
        if !id.fetched {
            return None;
        }
        pack.lookup(id.page, id.tile, bg_colors(rec, id.pal))
    }

    /// Paint the active layers into line `y`. Returns true when a pixel
    /// changed (the line's sprites then need replaying).
    fn paint_layers(
        &mut self,
        rec: &LineRecord,
        input: &ComposeInput<'_>,
        pack: &HdPack,
        step: u32,
        y: usize,
        out: &mut [u8],
    ) -> bool {
        let n = self.scale as usize;
        let any_front = self.line_front;
        // Skip the line when no layer reaches it.
        if !any_front && !self.line_back {
            return false;
        }
        let mut painted = false;
        for sx in 0..self.src_width {
            let cell = self.column_cell(rec, pack, sx);
            // A plain opaque NES pixel can only change under a front layer.
            if !any_front && cell.is_none() && self.col_opaque[sx] {
                continue;
            }
            for fy in 0..n {
                for fx in 0..n {
                    let px = self
                        .background_pixel(rec, input, pack, cell, step, sx, fx as u32, fy as u32);
                    let dst = ((y * n + fy) * self.out_w + sx * n + fx) * 4;
                    if out[dst..dst + 4] != px {
                        out[dst..dst + 4].copy_from_slice(&px);
                        painted = true;
                    }
                }
            }
        }
        painted
    }

    /// Layers on a line without usable tile identities (a split line): a
    /// back layer shows where the indexed pixel is the line's backdrop.
    fn paint_layers_by_index(
        &self,
        rec: &LineRecord,
        input: &ComposeInput<'_>,
        pack: &HdPack,
        step: u32,
        y: usize,
        out: &mut [u8],
    ) {
        let n = self.scale as usize;
        let ps = pack.scale() as i32;
        if !self.line_back {
            return;
        }
        let margin_px = (self.src_width.saturating_sub(WIDTH) / 2) as i32;
        let src = &input.indexed.pixels[y * self.src_width..(y + 1) * self.src_width];
        for (sx, &idx) in src.iter().enumerate() {
            if idx != rec.backdrop {
                continue;
            }
            for fy in 0..n {
                for fx in 0..n {
                    let hwx = (sx as i32 - margin_px) * ps + (fx as i32) * step as i32;
                    if let Some(px) =
                        self.sample_layers(pack, LayerDepth::Back, BgTileId::NONE, hwx, fy)
                    {
                        let dst = ((y * n + fy) * self.out_w + sx * n + fx) * 4;
                        out[dst..dst + 4].copy_from_slice(&[px[0], px[1], px[2], 0xFF]);
                    }
                }
            }
        }
    }

    /// Sprite pass for packs with `"sprite_alpha": "art"` or active layers
    /// (see the module docs). Needs [`Compositor::resolve_columns`].
    #[allow(clippy::too_many_arguments)]
    fn replay_sprites_shaped(
        &mut self,
        rec: &LineRecord,
        input: &ComposeInput<'_>,
        pack: &HdPack,
        step: u32,
        y: usize,
        bg_changed: bool,
        out: &mut [u8],
    ) {
        if rec.mask & PPUMASK_SHOW_SPRITES == 0 {
            return;
        }
        let art = pack.sprite_alpha_art();
        let reach = if art {
            usize::from(pack.max_bleed_v())
        } else {
            0
        };
        let sprites = input.record.sprites_on(y);
        let hd_cell = |s: &SpriteRef, line: &LineRecord| sprite_art(pack, line, input.palette, s);
        let margin_rows = || {
            input
                .margins
                .into_iter()
                .flat_map(move |m| margin_sprite_rows(m, input.record, y))
        };
        let own = sprites.iter().any(|s| hd_cell(s, rec).is_some())
            || margin_rows().any(|r| hd_cell(&r.sprite, rec).is_some())
            || self.actors.iter().any(|a| self.actor_reaches(*a, pack, y))
            || sprites.iter().any(|s| self.actor_owns(s.oam_index))
            || margin_rows().any(|r| self.actor_owns_margin(r.index));
        // A neighbouring line's sprite whose bleed reaches this line.
        let reaching = |d: usize, below: bool| {
            let ly = if below { y + d } else { y.wrapping_sub(d) };
            (ly < HEIGHT).then(|| (ly, input.record.line(ly)))
        };
        let mut neighbour = false;
        for d in 1..=reach {
            for below in [true, false] {
                let Some((ly, line)) = reaching(d, below) else {
                    continue;
                };
                let edge_row = if below { 0 } else { 7 };
                neighbour |= input.record.sprites_on(ly).iter().any(|s| {
                    s.row_in_sprite % 8 == edge_row
                        && hd_cell(s, line).is_some_and(|a| {
                            let [_, t, _, b] = a.cell.bleed;
                            let (top, bottom) = if s.flip_v { (b, t) } else { (t, b) };
                            usize::from(if below { top } else { bottom }) >= d
                        })
                });
            }
        }
        if !bg_changed && !own && !neighbour {
            return;
        }

        self.resolve_background_layer(rec, input, y);
        let n = self.scale as usize;
        let ps = pack.scale();
        let margin_px = self.src_width.saturating_sub(WIDTH) / 2;
        let show_left_spr = rec.mask & PPUMASK_SHOW_LEFT_SPRITES != 0;
        let fill = Self::line_edge_fill(input, y);
        // Columns a sprite may reach on this line: inside the window, not in a
        // left edge the game clipped and the margins did not recover.
        let drawable = |x: usize| x < WIDTH && (x >= 8 || show_left_spr || fill.left);
        // Source column of pixel `dx` of a sprite row whose left column is at
        // window x `left`: window sprites keep to `drawable`, margin sprites to
        // `margin_sprite_paints`.
        let left_fill = input.margins.is_some_and(|m| left_sprite_fill(m, rec));
        let column = |left: i32, dx: i32, window: bool| {
            let wx = left + dx;
            let ok = if window {
                drawable(wx as usize)
            } else {
                margin_sprite_paints(wx, left, margin_px as i32, left_fill, fill.right)
            };
            ok.then_some((wx + margin_px as i32) as usize)
        };
        // Window sprites (real OAM) first, then the margin sprites. The
        // third field is `None` for a window sprite, else the margin index.
        let rows = || {
            sprites
                .iter()
                .filter(|s| !edge_drops_window_sprite(fill, s, sprites, input.chr_rom))
                .map(|s| (*s, i32::from(s.x), None))
                .chain(margin_rows().map(|r| (r.sprite, r.x, Some(r.index))))
        };
        let src_width = self.src_width;

        // 1. Take the baked NES sprites off the line (margin sprites too: the
        //    wide indexed frame has them baked in).
        for (s, left, index) in rows() {
            let window = index.is_none();
            let id = sprite_tile_id(&s);
            for dx in 0..8i32 {
                let Some(sx) = column(left, dx, window) else {
                    continue;
                };
                let col = if s.flip_h { 7 - dx } else { dx };
                if chr_sub(input.chr_rom, id, col as u8).unwrap_or(0) == 0 {
                    continue;
                }
                let cell = self.column_cell(rec, pack, sx);
                for fy in 0..n {
                    for fx in 0..n {
                        let px = self.background_pixel(
                            rec, input, pack, cell, step, sx, fx as u32, fy as u32,
                        );
                        let dst = ((y * n + fy) * self.out_w + sx * n + fx) * 4;
                        out[dst..dst + 4].copy_from_slice(&px);
                    }
                }
            }
        }

        // 2. Sprite cores, front to back, claiming output pixels.
        for c in self.claimed_out[..src_width * n * n].iter_mut() {
            *c = false;
        }
        let mut actor_drawn = 0u32;
        for (s, left, index) in rows() {
            let window = index.is_none();
            for i in 0..self.actors.len() {
                if actor_drawn & (1 << i) == 0
                    && (self.actors[i].priority_oam <= s.oam_index || !window)
                {
                    self.draw_actor_row(self.actors[i], pack, rec, input, step, y, out);
                    actor_drawn |= 1 << i;
                }
            }
            let owned = match index {
                None => self.actor_owns(s.oam_index),
                Some(index) => self.actor_owns_margin(index),
            };
            if owned {
                continue;
            }
            let s = &s;
            let hd = hd_cell(s, rec);
            let id = sprite_tile_id(s);
            for dx in 0..8i32 {
                let Some(sx) = column(left, dx, window) else {
                    continue;
                };
                let col = if s.flip_h { 7 - dx } else { dx } as usize;
                let sub = chr_sub(input.chr_rom, id, col as u8).unwrap_or(0);
                if sub == 0 && !(art && hd.is_some()) {
                    continue;
                }
                // `behind` tests NES background opacity: the window's own
                // layer inside it, the margin tiles outside.
                let hidden = s.behind
                    && if window {
                        self.bg_opaque[sx - margin_px]
                    } else {
                        self.col_opaque[sx]
                    };
                let nes_rgba = input
                    .palette
                    .rgba(rec.palette_entry(0x10 + usize::from(s.pal & 3) * 4 + usize::from(sub)));
                let cellpx = hd.map(|a| (pack.cell_pixels(a.cell), a.tint));
                // Without art alpha the NES pixel claims its whole block.
                let block_claim = !(art && hd.is_some());
                for fy in 0..n {
                    for fx in 0..n {
                        let claim = (fy * src_width + sx) * n + fx;
                        if self.claimed_out[claim] {
                            continue;
                        }
                        let rgba = match &cellpx {
                            None => Some(nes_rgba),
                            Some((cp, tint)) => {
                                let sub_col = fx as u32 * step;
                                let sub_row = fy as u32 * step;
                                let hx = col as u32 * ps
                                    + if s.flip_h { ps - 1 - sub_col } else { sub_col };
                                let hy = u32::from(s.fine_row & 7) * ps
                                    + if s.flip_v { ps - 1 - sub_row } else { sub_row };
                                let px = cp.pixel(hx, hy);
                                (px[3] != 0).then(|| {
                                    let px = [px[0], px[1], px[2], 0xFF];
                                    tint.map_or(px, |t| t.apply(px))
                                })
                            }
                        };
                        if block_claim || rgba.is_some() {
                            self.claimed_out[claim] = true;
                        }
                        if let (Some(rgba), false) = (rgba, hidden) {
                            let dst = ((y * n + fy) * self.out_w + sx * n + fx) * 4;
                            out[dst..dst + 4].copy_from_slice(&rgba);
                        }
                    }
                }
            }
        }
        for i in 0..self.actors.len() {
            if actor_drawn & (1 << i) == 0 {
                self.draw_actor_row(self.actors[i], pack, rec, input, step, y, out);
            }
        }
        if !art {
            return;
        }

        // 3. Bleed: art around a sprite's box, where nothing claimed the pixel.
        // A box is found once per line: on the line itself when `y` is inside
        // it, else on its first row (box below `y`) or last row (box above).
        let p = ps as i32;
        for d in 0..=reach {
            for below in [true, false] {
                if d == 0 && !below {
                    continue;
                }
                let Some((ly, line)) = reaching(d, below) else {
                    continue;
                };
                for s in input.record.sprites_on(ly) {
                    if self.actor_owns(s.oam_index) {
                        continue;
                    }
                    let row_in_box = i32::from(s.row_in_sprite % 8);
                    if d > 0 && row_in_box != if below { 0 } else { 7 } {
                        continue;
                    }
                    let Some(SpriteArt { cell, tint }) = hd_cell(s, line) else {
                        continue;
                    };
                    if cell.bleed == [0; 4] {
                        continue;
                    }
                    // Screen row of line `y` in the box, then the pattern row.
                    let q = y as i32 - (ly as i32 - row_in_box);
                    let prow = if s.flip_v { 7 - q } else { q };
                    let [bl, bt, br, bb] = cell.bleed.map(i32::from);
                    if prow < -bt || prow >= 8 + bb {
                        continue;
                    }
                    let (left, right) = if s.flip_h { (br, bl) } else { (bl, br) };
                    let cp = pack.cell_pixels(cell);
                    let box_x = i32::from(s.x);
                    for xi in box_x - left..box_x + 8 + right {
                        let core = (0..8).contains(&q) && (box_x..box_x + 8).contains(&xi);
                        if core || xi < 0 || !drawable(xi as usize) {
                            continue;
                        }
                        let x = xi as usize;
                        if s.behind && self.bg_opaque[x] {
                            continue;
                        }
                        let qx = xi - box_x;
                        let pcol = if s.flip_h { 7 - qx } else { qx };
                        for fy in 0..n {
                            for fx in 0..n {
                                let claim = (fy * src_width + x + margin_px) * n + fx;
                                if self.claimed_out[claim] {
                                    continue;
                                }
                                let sub_col = (fx as u32 * step) as i32;
                                let sub_row = (fy as u32 * step) as i32;
                                let hx =
                                    pcol * p + if s.flip_h { p - 1 - sub_col } else { sub_col };
                                let hy =
                                    prow * p + if s.flip_v { p - 1 - sub_row } else { sub_row };
                                let Some(px) = cp.pixel_rel(hx, hy) else {
                                    continue;
                                };
                                if px[3] == 0 {
                                    continue;
                                }
                                self.claimed_out[claim] = true;
                                let dst =
                                    ((y * n + fy) * self.out_w + (x + margin_px) * n + fx) * 4;
                                let px = [px[0], px[1], px[2], 0xFF];
                                let px = tint.map_or(px, |t| t.apply(px));
                                out[dst..dst + 4].copy_from_slice(&px);
                            }
                        }
                    }
                }
            }
        }
    }

    /// Fill one NES pixel's `scale x scale` output block with `rgba`.
    fn fill_block(&self, ox: usize, y: usize, rgba: &[u8; 4], out: &mut [u8]) {
        let n = self.scale as usize;
        for fy in 0..n {
            let oy = y * n + fy;
            for fx in 0..n {
                let dst = (oy * self.out_w + ox * n + fx) * 4;
                out[dst..dst + 4].copy_from_slice(rgba);
            }
        }
    }
}

/// Pattern identity of a recorded sprite row (for [`chr_sub`]).
fn sprite_tile_id(s: &SpriteRef) -> BgTileId {
    BgTileId {
        page: s.page,
        tile: s.tile,
        pal: s.pal,
        fine_y: s.fine_row & 7,
        fetched: true,
        ..BgTileId::NONE
    }
}

/// Variant key for a background tile: its palette group's 3 opaque colours.
#[must_use]
pub fn bg_colors(rec: &LineRecord, pal: u8) -> [u8; 3] {
    let base = usize::from(pal & 3) * 4;
    [
        rec.palette_entry(base + 1),
        rec.palette_entry(base + 2),
        rec.palette_entry(base + 3),
    ]
}

/// Variant key for a sprite: its palette group's 3 opaque colours.
#[must_use]
pub fn sprite_colors(rec: &LineRecord, pal: u8) -> [u8; 3] {
    let base = 0x10 + usize::from(pal & 3) * 4;
    [
        rec.palette_entry(base + 1),
        rec.palette_entry(base + 2),
        rec.palette_entry(base + 3),
    ]
}

/// HD art for one sprite row: the cell, plus a recolour when the cell was
/// borrowed from another palette group ([`sprite_art`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpriteArt {
    pub cell: CellRef,
    pub tint: Option<SpriteTint>,
}

/// Palette swap of HD art: each pixel takes the shift from its nearest base
/// colour to the matching colour of the group the sprite is drawn with, so
/// the base variant's own colours land exactly on the drawn colours and the
/// HD shading in between is kept.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpriteTint {
    base: [[u8; 3]; 3],
    shift: [[i16; 3]; 3],
}

impl SpriteTint {
    /// Recolour `base` (the borrowed variant's colours) to `drawn`.
    #[must_use]
    pub fn new(palette: &MasterPalette, base: [u8; 3], drawn: [u8; 3]) -> Self {
        let base = base.map(|c| palette.rgb(c));
        let drawn = drawn.map(|c| palette.rgb(c));
        let shift = core::array::from_fn(|k| {
            core::array::from_fn(|i| i16::from(drawn[k][i]) - i16::from(base[k][i]))
        });
        Self { base, shift }
    }

    /// `px` (opaque RGBA) recoloured.
    #[must_use]
    pub fn apply(&self, px: [u8; 4]) -> [u8; 4] {
        let dist = |b: &[u8; 3]| -> i32 {
            (0..3)
                .map(|i| {
                    let d = i32::from(px[i]) - i32::from(b[i]);
                    d * d
                })
                .sum()
        };
        let mut k = 0;
        for j in 1..3 {
            if dist(&self.base[j]) < dist(&self.base[k]) {
                k = j;
            }
        }
        let ch = |i: usize| (i16::from(px[i]) + self.shift[k][i]).clamp(0, 255) as u8;
        [ch(0), ch(1), ch(2), px[3]]
    }
}

/// Resolve HD art for a recorded sprite row.
///
/// [`HdPack::lookup`] first (exact variant, then the default). When that
/// misses, the tile is tried with the colours of the other three sprite
/// groups loaded on the line, and a hit is drawn recoloured to the sprite's
/// own colours ([`SpriteTint`]).
///
/// This is what keeps enemies in HD while they flash after a hit: the ROM
/// (`LF1B5`, bank 7) replaces a stunned enemy's palette bits with
/// `($040E,x >> 1) & 3`, cycling the sprite through all four groups every
/// two frames, so three frames in four carry colours its variant was not
/// registered for. Its own colours stay loaded in one of the other groups.
#[must_use]
pub fn sprite_art(
    pack: &HdPack,
    rec: &LineRecord,
    palette: &MasterPalette,
    s: &SpriteRef,
) -> Option<SpriteArt> {
    if s.page == NO_PAGE {
        return None;
    }
    let drawn = sprite_colors(rec, s.pal);
    if let Some(cell) = pack.lookup(s.page, s.tile, drawn) {
        return Some(SpriteArt { cell, tint: None });
    }
    (1..4u8).find_map(|d| {
        let base = sprite_colors(rec, (s.pal + d) & 3);
        pack.lookup_variant(s.page, s.tile, base)
            .map(|cell| SpriteArt {
                cell,
                tint: (base != drawn).then(|| SpriteTint::new(palette, base, drawn)),
            })
    })
}
