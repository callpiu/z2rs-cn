//! Camera zoom for the native present path (`DisplayEnh::camera_zoom`,
//! `--camera-zoom`): which point the camera follows, when it snaps or eases,
//! what stays unzoomed. The resampling itself is shared
//! ([`z2_render::camera`]). Presentation only: this reads RAM, OAM and CHR
//! after the frame and never writes anything the game sees.
//!
//! * **Target.** Side view: P1's body centre (`player_visuals`, the HD actor
//!   anchor 16 pixels up). Co-op: the midpoint of both Links while both are
//!   on screen and close enough to share the zoomed view, else P1.
//!   Overworld: P1's map sprite. Any other mode (title, file select,
//!   credits): no target, and the zoom eases back to 1.
//! * **Smoothing.** A deadzone of 12 x 24 NES pixels around the camera, then
//!   an eased catch-up per emulated frame ([`z2_render::camera::Camera`]),
//!   so walk animation and small hops do not move the view.
//! * **Snapping.** The first frame, a different room or scene (world, region,
//!   scene index, virtual room, or a mode change), a target jump of more
//!   than 48 pixels (a door, a fall into the next room) and a backwards frame
//!   count (a loaded state) all move the camera straight to the target, so a
//!   transition never shows a pan across the old room.
//! * **Unzoomed layers.** The side-view status bar (the top 32 lines) and the
//!   boss life meter (sprite cells `$6E`/`$70` at screen X 16) are drawn at
//!   their normal place and size over the zoomed view. Open dialogue boxes
//!   and the pause pane ease the zoom out to 1 so they are never cropped.
//! * **Clamping.** The view never leaves the composed frame (widescreen
//!   margins included), see [`z2_render::camera::view_rect`].
use z2_core::game::Game;
use z2_render::camera::{self, Camera, HudCell, Source, View};

/// Side-view status bar height in NES lines (the level starts below it).
pub const SIDEVIEW_HUD_ROWS: u32 = 32;
/// A target move larger than this in one frame is a cut, not a walk.
pub const SNAP_DISTANCE: f32 = 48.0;

const MODE_SIDEVIEW: u8 = 0x0B;
const MODE_OVERWORLD: u8 = 0x05;

/// Which room the camera is in; a change snaps it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SceneKey {
    mode: u8,
    world: u8,
    region: u8,
    scene: u8,
    vroom: u16,
}

/// What one frame asks of the camera.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Aim {
    /// Point to follow, in NES pixels of the composed frame.
    pub target: Option<[f32; 2]>,
    /// P1's own point (a jump of it is a cut; the co-op midpoint switching
    /// to P1 is not).
    pub p1: Option<[f32; 2]>,
    /// Unzoomed status-bar lines at the top.
    pub hud_rows: u32,
    /// The zoom may apply (gameplay without a dialogue or pane open).
    pub zoom_allowed: bool,
}

/// Where the camera should look this frame. `margin` is the widescreen
/// margin per side in NES pixels; `zoom` the requested zoom (for the co-op
/// midpoint test).
#[must_use]
pub fn aim(game: &Game, margin: u32, zoom: f32) -> Aim {
    let ram = game.ram();
    let m = margin as f32;
    let players = game.player_visuals();
    let mode = ram[0x0736];
    // `$0524` 1-6: dialogue / pause pane; `$074C == 2`: a dialogue box open.
    let menu = (1..=6).contains(&ram[0x0524]) || ram[0x074C] == 2;
    match mode {
        MODE_SIDEVIEW => {
            let body = |p: &z2_core::player_visual::PlayerVisual| {
                [p.position[0] + m, p.position[1] - 16.0]
            };
            let p1 = players[0].as_ref().map(body);
            let target = match (p1, players[1].as_ref().map(body)) {
                (Some(a), Some(b)) => {
                    // Both on screen: the midpoint while they fit one view
                    // with room to spare, else P1.
                    let view_w = (256.0 + 2.0 * m) / zoom.max(1.0);
                    let view_h = (240.0 - SIDEVIEW_HUD_ROWS as f32) / zoom.max(1.0);
                    if (a[0] - b[0]).abs() < view_w - 48.0 && (a[1] - b[1]).abs() < view_h - 48.0 {
                        Some([(a[0] + b[0]) / 2.0, (a[1] + b[1]) / 2.0])
                    } else {
                        Some(a)
                    }
                }
                (a, _) => a,
            };
            Aim {
                target,
                p1,
                hud_rows: SIDEVIEW_HUD_ROWS,
                zoom_allowed: !menu,
            }
        }
        MODE_OVERWORLD => {
            let p1 = players[0]
                .as_ref()
                .map(|p| [p.position[0] + m - 4.0, p.position[1] - 8.0]);
            Aim {
                target: p1,
                p1,
                hud_rows: 0,
                zoom_allowed: !menu,
            }
        }
        _ => Aim {
            target: None,
            p1: None,
            hud_rows: 0,
            zoom_allowed: false,
        },
    }
}

fn scene_key(game: &Game) -> SceneKey {
    let ram = game.ram();
    let (world, region, scene, vroom) = game
        .sideview_scene()
        .map_or((ram[0x0707], ram[0x0706], ram[0x0561], 0), |s| {
            (s.world, s.region, s.scene, s.vroom)
        });
    SceneKey {
        mode: ram[0x0736],
        world,
        region,
        scene,
        vroom,
    }
}

/// The boss life meter's sprite cells, with their opaque pixels from CHR,
/// in NES pixels of the composed frame.
#[must_use]
pub fn life_meter_cells(game: &Game, margin: u32) -> Vec<HudCell> {
    let oam = game.oam();
    let mut cells = Vec::new();
    if game.ram()[0x0736] != MODE_SIDEVIEW {
        return cells;
    }
    for c in oam.chunks(4) {
        if c[0] >= 0xEF || c[3] != 16 || !matches!(c[1], 0x6E | 0x70) {
            continue;
        }
        let (tile, attr) = (c[1], c[2]);
        let base = game.mmc1.map_chr4(tile & 1) * 0x1000 + usize::from(tile & 0xFE) * 16;
        let mut rows = [0u8; 16];
        for (r, row) in rows.iter_mut().enumerate() {
            let src = if attr & 0x80 != 0 { 15 - r } else { r };
            let at = base + (src / 8) * 16 + src % 8;
            let bits =
                game.chr.get(at).copied().unwrap_or(0) | game.chr.get(at + 8).copied().unwrap_or(0);
            *row = if attr & 0x40 != 0 {
                bits.reverse_bits()
            } else {
                bits
            };
        }
        cells.push(HudCell {
            x: i32::from(c[3]) + margin as i32,
            y: i32::from(c[0]) + 1,
            rows,
        });
    }
    cells
}

/// Per-display camera: the smoothed position plus what it last saw.
#[derive(Debug, Clone, Default)]
pub struct CameraState {
    camera: Camera,
    last_frame: Option<u64>,
    key: Option<SceneKey>,
    last_target: Option<[f32; 2]>,
    hud_rows: u32,
}

impl CameraState {
    /// Advance one emulated frame. Call once per stepped frame (repeat calls
    /// for the same frame are ignored), so the motion does not depend on how
    /// often frames are presented.
    pub fn step(&mut self, game: &Game, zoom: f32, margin: u32) {
        let frame = game.frame_count();
        if self.last_frame == Some(frame) {
            return;
        }
        let rewound = self.last_frame.is_some_and(|f| frame < f);
        self.last_frame = Some(frame);
        let a = aim(game, margin, zoom);
        self.hud_rows = a.hud_rows;
        let want = if a.zoom_allowed { zoom } else { 1.0 };
        let key = scene_key(game);
        let first = self.camera.center().is_none();
        let cut = rewound
            || self.key != Some(key)
            || match (a.p1, self.last_target) {
                (Some(t), Some(l)) => (t[0] - l[0]).hypot(t[1] - l[1]) > SNAP_DISTANCE,
                _ => false,
            };
        self.key = Some(key);
        if a.p1.is_some() {
            self.last_target = a.p1;
        }
        match a.target {
            Some(t) if first || cut => {
                let z = if first { want } else { self.camera.zoom() };
                self.camera.snap(t, z);
                self.camera.follow(None, want);
            }
            t => self.camera.follow(t, want),
        }
    }

    /// The current view over a composed frame `fw` NES pixels wide.
    #[must_use]
    pub fn view(&self, fw: f32) -> View {
        View {
            zoom: self.camera.zoom(),
            center: self.camera.center().unwrap_or([fw / 2.0, 120.0]),
            hud_rows: self.hud_rows,
        }
    }

    /// Resample the composed `src` (`w x h`, `scale` texels per NES pixel)
    /// into `dst` (`ow x oh`) through the current view.
    #[allow(clippy::too_many_arguments)]
    pub fn render(
        &self,
        src: &[u8],
        w: usize,
        h: usize,
        scale: u32,
        dst: &mut [u8],
        ow: usize,
        oh: usize,
        game: &Game,
        margin: u32,
    ) {
        let source = Source {
            rgba: src,
            width: w,
            height: h,
            scale,
        };
        let view = self.view(w as f32 / scale.max(1) as f32);
        let cells = if view.zoom > 1.0 {
            life_meter_cells(game, margin)
        } else {
            Vec::new()
        };
        camera::resample(&source, dst, ow, oh, &view, &cells);
    }
}
