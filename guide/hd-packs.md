# HD graphics packs

A pack is a directory of PNG sheets plus a `pack.json`. It replaces the game's 8x8 tiles and sprites with art drawn at 2x to 8x. The simplest way to make one is to paint over spritesheets of the game (next section). You can also start from your own ROM's graphics and paint the cells directly.

```sh
cargo xtask hdpack template --out ~/z2-art/mypack --scale 2   # start from your ROM's CHR
# ...paint the sheets...
cargo xtask hdpack check --pack ~/z2-art/mypack               # validate
cargo run --release -p z2-native -- --rom "$Z2_ROM" --hd-pack ~/z2-art/mypack --hd-scale 2
```

`--hd-record DIR` works the other way round. Play normally, and when you quit with `Esc` or by closing the window, z2rs writes a template pack that contains exactly the `(page, tile, palette)` combinations that session drew. That is a much smaller starting point than all 32 CHR pages. Only those two ways of quitting save the recording. A `kill` loses it.

In the browser, the folder picker loads the same packs in a build that has the `hd` feature. Choose the directory that holds `pack.json`. If the folder you pick holds two packs side by side, the browser refuses it and asks you to pick one, instead of choosing between them by name.

Template sheets are rendered from your ROM, so they must stay outside this repository. The tooling refuses to write them inside a git work tree (see [LEGAL.md](LEGAL.md)). `cargo xtask hdpack --help` covers the rest of the workflow, and the module docs in `crates/z2-render/src/pack.rs` describe the `pack.json` format.

## Painting over spritesheets

You can make a pack by painting over spritesheets of the game. Each character is laid out frame by frame and each scene's tileset metatile by metatile. When you are done, one command turns the painted sheets into a pack, so you never edit `pack.json` or look up CHR page numbers.

```sh
export Z2_ROM=/path/to/zelda2.nes   # your own dump, see LEGAL.md
make hd-sheets                      # capture the game and write the sheets (about 6 minutes once built, 1.7 GB of captures)
# paint over ~/z2-art/sheets/sheets/*.png in any image editor
make hd-pack                        # turn them into ~/z2-art/my-pack and validate it
make hd-play                        # play with it
```

`make hd-sheets` needs the input movies described in [development.md](development.md#input-movies). It replays them into every town, palace, boss room and field, walks and attacks through each one, records which tile drew every pixel, and writes two kinds of sheet into `~/z2-art/sheets/sheets/`:

| sheet | one slot | one sheet |
|---|---|---|
| `fig-cXX-YY-ZZ.png` | a whole figure in one animation frame, such as Link mid-stab or a townsperson | one sprite palette, named by its three NES colours, which is usually one character |
| `tile-wW-rR-sSS.png` | one 16x16 metatile, four CHR tiles | one scene, with the most-used metatiles first |

The sheets are drawn from your ROM at 4x in the game's own colours, so each one looks like the game when you open it. Most editors show the transparent areas as a checkerboard.

When you paint:

- Paint in place. Keep every slot where it is and keep the image the same size. The dark gutters between slots are ignored.
- A slot you leave untouched keeps the original art. `make hd-pack` compares each slot with what `make hd-sheets` wrote and skips the ones that match, so you can paint one character, build, play, and come back for the rest.
- Figures can grow. Each figure slot has 8 NES pixels of empty margin around the sprite, and paint that goes into it is drawn in the game, so a longer sword or a cape works. Keep the figure standing where it is, because the hitboxes and the animation timing don't change.
- Transparency is on or off. Alpha below 128 is transparent and nothing is blended, so paint hard edges.
- Metatiles sit next to each other in the level, so match the edges of tiles you expect to touch.
- Tiles with no pixels at all are left off the sheets. That tile is the backdrop colour in every sky in the game. Paint a sky or distant hills as a layer instead (see below).
- If two poses share a CHR tile and you paint it two different ways, the version with more painted pixels wins.

`HD_ART` (default `~/z2-art`), `HD_SHEETS`, `HD_PACK` and `HD_SCALE` change where things go and the scale. `HD_PACK_NAME` names the pack. Keep all of them outside this repository: everything here is drawn from your ROM, and only art you painted yourself may be shared.

The targets run `tools/hd-sheets/specs.py` and the `article_shots` example with `--tilemap` to capture the game, then `tools/hd-sheets/paint_sheets.py build`. `make hd-pack` runs `paint_sheets.py cut` and `cargo xtask hdpack check`. Run `python3 tools/hd-sheets/paint_sheets.py build --help` to rename sheets (`--name c18-36-2A=link`) or to build from your own captures.

## Sprite shapes and layers

A pack can set `"sprite_alpha": "art"` in `pack.json`. A replaced sprite then takes its outline from the art's own alpha instead of the original sprite's pixels, and a `tiles[]` entry can carry `bleed: [left, top, right, bottom]`, up to 8 NES pixels of art drawn around the 8x8 box. Sprites still stack in the order the game draws them, and a sprite behind the background still hides behind opaque background pixels. Packs made with `make hd-pack` use both.

`layers[]` places whole images in one scene: a sky or far hills behind the background (`"depth": "back"`), or art over the background tiles (`"front"`). A layer can follow the camera at a fraction of its speed (`scroll`, a percentage), repeat horizontally, cover only chosen tiles (`over_tiles`), and apply to one scene through `when` (`world`, `region`, `scene`). `when.background_tile` (`x`, `y`, `page`, `tile`, and optionally `colors`) limits a layer to screens that show that background tile at that pixel of the NES picture, which keeps a title backdrop off the save menu and out of gameplay. Pick the anchor from captures of the screen you want and check that it is absent everywhere else. `paint_sheets.py cut --layer` adds them; its `--help` lists the options. A front layer is never drawn inside the game's text windows (dialogue, signs and the pause menu), so a house layer cannot hide the text. Older builds ignore these keys and draw every sprite inside its original outline.

## Actor animations

A pack can also replace the whole body of Link, a side-view enemy, a boss or a townsperson with its own animation frames instead of tile by tile. `actor_clips[]` in `pack.json` names a character, a clip, a frame rate and a list of transparent PNG frames with a pivot each. While a clip plays, the renderer draws that one image where the game draws that character, so poses that share a CHR tile no longer show seams. Tile replacements keep working for everything else, and a character or clip the pack leaves out falls back to the tile art.

```json
"actor_clips": [{
  "character": "link",
  "name": "walk",
  "fps": 60.0988,
  "looped": true,
  "frames": [
    { "file": "actors/walk/0000.png", "pivot": [28.5, 122] },
    { "file": "actors/walk/0001.png", "pivot": [29.5, 121] }
  ]
}]
```

The renderer draws each frame in pack pixels at the pack's scale and does not resize it. `pivot` is the point in each frame, measured from its top left corner, that lands on the same spot of Link's skeleton every frame; it may lie outside the image. Keep the pivots your export tool wrote rather than centering each crop. In side view that spot is Link's feet; on the overworld map it is the bottom center of the single visible 8x16 sprite cell, so author map clips at half the side-view height. Do not bake jump travel or knockback into the frames, because the game already moves Link.

`character` is `link` for player 1 and `link_red` for player 2. The side-view clip names are `idle`, `walk`, `attack_windup`, `attack`, `crouch_idle`, `crouch_attack`, `jump_rise`, `jump_apex`, `fall`, `jump_attack`, `up_thrust_hold`, `down_thrust_hold`, `hurt`, `death`, `fairy_fly` and `item_get`. Player 1 also uses `cast_spell`, `crystal_place` and `victory`. On the overworld the names are `overworld_walk_right`, `overworld_walk_left`, `overworld_walk_down`, `overworld_walk_up` and the four `overworld_idle_*` clips; player 2 is not drawn on the overworld in co-op, so it has no map clips. A pack may store clips under other names, but the game never picks them. The sword windup is its own state, so a thrust clip should start with the blade already out.

A clip's time comes from emulated frames at 60.0988 Hz, so a paused game holds the pose. Changing action or seeking backwards restarts the clip. `looped` defaults to `false`, which holds the last frame until the action changes. The original sprites still decide visibility, injury blinking, priority and what hides behind the background. Held items and sword beams stay separate sprites and come from the sheets.

The frame rate must be between 1 and 240, a pack may hold at most 4096 clips, each with 1 to 4096 frames, and a frame PNG has the same 4096 pixel edge limit as a sheet. Alpha is on or off, as for sheets. Frames are decoded when first shown and cached up to 512 MiB; characters used least recently give up their decoded frames when the cache fills. No single character may exceed that decoded budget. Compressed actor PNG files may total up to 1 GiB. `cargo xtask hdpack check` validates the files and reports the frame count of each clip.

A pack that uses `actor_clips` needs z2rs 0.5.1 or newer. Older builds ignore keys they do not know, so they load the pack without an error and draw Link from the sheets.

### Enemy and townsperson keys

Enemy keys have the form `enemy_<group>_<id>`, with a two-digit lowercase hexadecimal object id. Townspeople use `npc_b3_<id>`. The group identifies the area's enemy table:

| Group | Areas |
|---|---|
| `b1` | West Hyrule and Death Mountain |
| `b2` | East Hyrule and Maze Island |
| `b3` | towns, using the `npc` prefix |
| `b4` | palaces 1, 2 and 5 |
| `b4b` | palaces 3, 4 and 6 |
| `b5` | Great Palace |

For example, `enemy_b1_04` is a Bot, `enemy_b1_14` is an orange Moblin, `enemy_b4_20` is Horsehead and `npc_b3_0f` is a wise man. The observer adds qualifiers where one id has several appearances: `enemy_b4_21_p2` is Helmethead, `enemy_b4_21_p5` is Gooma, and `npc_b3_1d_34` is a child. A qualified key takes precedence over the plain key when a pack supplies both.

The available clips depend on the character. Use the `alias()` table and clip selection in `crates/z2-core/src/actor_visual.rs`, or run the native headless build with `--dump-actors actors.jsonl` to inspect characters in a replay. Author enemy frames facing right; the renderer mirrors them when needed. Character pivots follow the original drawn body or collision geometry. Barba's pivot belongs under his head, with the neck extending below it. Actors drawn outside the NES window use the same animation system in widescreen.

Native developer overlays can show the game's collision boxes and terrain probes, so you can check the art's placement against gameplay. Moving the art or its pivot never moves those collisions.

### Animated scenery and new rooms

A layer can use `frames` instead of `file` and loop those PNGs at `fps`. Every frame must have the same dimensions. Frame rates range from 1 to 240, with at most 256 frames in one layer. `drift_x` and `drift_y` move the layer at a constant speed in NES pixels per second; positive values move right and down. Horizontal drift can wrap with `repeat_x`.

```json
"layers": [{
  "frames": ["water/0.png", "water/1.png", "water/2.png"],
  "fps": 6,
  "x": 0,
  "y": 208,
  "repeat_x": true,
  "drift_x": -6,
  "when": { "world": 0, "region": 0, "scene": 12 }
}]
```

Animation and drift follow emulated time. Pausing holds them still, and loading a state or seeking returns to the picture for that game frame. The pack checker checks animation rates, dimensions and drift ranges.

Layers for optional new areas can add `when.vroom`. `0` matches only original rooms; a positive value matches virtual room id plus one. Omit it to match either kind of room. Virtual room ids belong to the authored area data and can change between builds, so check the current id in a capture before writing a filter.

Enemy and NPC animations, animated scenery and virtual-room filters require z2rs 0.6.0 or newer. Older builds ignore keys they do not recognize, which can leave original sprites visible or put a scenery layer in the wrong room.

## How packs work

Three things identify every tile the PPU draws: the CHR page it came from, its tile index, and the three opaque colours of the palette it was drawn in. A pack maps those keys to cells in a PNG sheet. Where a tile has no cell, z2rs scales up the original art instead, so a half-finished pack plays fine and you can repaint the game a few tiles at a time. A sprite drawn in colours that have no cell borrows the cell made for another sprite palette on the same line and is recoloured to match. That keeps an enemy in HD while it flashes after a hit, because the game cycles it through all four sprite palettes.

Replacement art never changes how the game behaves. The pack's alpha channel decides which HD pixels get painted, but the opacity of the original pixels still decides sprite priority, background priority and sprite-zero timing. The game logic keeps running on the ROM's own graphics while the screen shows yours.

`hdpack template` writes sheets for all 32 CHR pages of your ROM. `--hd-record` does the reverse: play, quit, and get a pack with exactly the tile and palette combinations that session drew.

## Limits

These apply to both the desktop app and the browser build.

- A background cell stays inside its 8x8 tile, and a sprite reaches at most 8 NES pixels past its box (`bleed`, with `"sprite_alpha": "art"`). Without that key, replaced sprites are also cut to their original opaque pixels. Art that belongs to no tile goes in a layer.
- Each tile identity has one cell. Every place the game draws that tile in those colours shows the same art. A layer with `over_tiles` can vary a repeated tile by position.
- Greyscale lines, lines with a mid-line `$2001` or palette write, and lines where fine X changed mid-line keep the original art, because the slot-to-pixel mapping doesn't hold for the whole line. In practice that is the row under the status bar and effects such as the palace-entry fade.
- Tile replacements are still images. Players, enemies, bosses and townspeople can animate through `actor_clips`, and scenery layers can use frame lists. There are no per-scanline palette effects.
- Layers are the most expensive part of a pack. The worst case (every tile replaced plus three layers) takes about 13.5 ms a frame at 4x widescreen on an Apple Silicon Mac, which still fits a 60 Hz frame. A pack without layers takes about 2 ms.
