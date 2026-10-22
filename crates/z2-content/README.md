# z2-content

Authoring new areas for z2rs (the optional `content` enhancement group): an
extra palace, a monster town, challenge rooms, caves with heart and magic
pieces. The engine runs them as virtual rooms. It copies a room's sideview
bytes, enemy list and connection bytes into WRAM buffers that a reserved map
slot points at. This crate produces those bytes.

All level data here is original. Never copy room bytes, enemy lists or text
from the ROM or from another hack, and never commit ROM-derived data (see
`LEGAL.md`). The one game table that room
geometry depends on, the floor/ceiling patterns, is read from the player's
ROM at run time (`FloorTable::from_body`).

The crate has no dependencies, does not depend on `z2-core` or `z2-rando`
(so `z2-core` can depend on it), and builds for `wasm32-unknown-unknown`.

## Pipeline

```text
model::Area ──compile()──▶ CompiledArea (Vec<u8> per room)   tools, tests
            ──generate_source()──▶ src/generated.rs           engine (static, no allocation)
            ──validate()/validate_geometry()──▶ Ok | Vec<Issue>
```

`src/generated.rs` holds `pub static NAME: StaticArea` tables for every area
in `samples::all()`. A test fails when it is out of date. To regenerate it:

```sh
Z2_CONTENT_BLESS=1 cargo test -p z2-content generated
```

The areas the engine actually runs live in `src/areas/`, one module per
option of `z2_core::enh::ContentOpts` (`parapa_annex`, `challenge_rooms`,
`minigames`, `hidden_caves`, `dark_isle`, and `dev_rooms` for `test_room`).
Each has `areas() -> Vec<Area>` (fill this in) and `statics()`, read from
its `<module>_generated.rs`. The engine registers `statics()`
(`z2-core/src/enh/content_areas.rs`). Regenerate after editing:

```sh
Z2_CONTENT_BLESS=1 cargo test -p z2-content areas
```

## How the engine runs an area

Every virtual room of a table set (`Table`: West, Death Mountain, East,
Maze Island, towns, palace group A, palace group B, Great Palace) runs in
that table's one reserved map slot (28, 27, 28, 46, 62, 63, 63, 60). So:

* `first_map` only numbers rooms in the tool-facing `connections` /
  `doors` bytes. The engine never loads those maps; it follows the typed
  `exits` / `door_exits` (`Dest::Room { room, page }` for a room of the
  same area, `Dest::Byte` for an original map or outside).
* `Target::Map` must name an original map of the same table (not its
  slot). `Target::Outside` returns to the overworld at the location Link
  came in by, `Target::OutsideAt(n)` at the location `n` slots after it
  (multi-mouth caves put their mouths on consecutive slots).
* Ways in are overworld `Entrance`s (placed by the overworld engine) and
  `VanillaLink`s: an original room's connection byte (`LinkVia::Byte`) or
  town doorway (`LinkVia::Door`) redirected into the area while its option
  is on. Validation counts both as ways in.
* Every page with a collectable or a palace locked door gets one
  battery-saved bit from the option's budget, in area, room and page order:
  append new rooms at the end so existing saves keep their bits. Items
  `$13` / `$14` are heart / magic container pieces (four make a container).

## Authoring an area

1. Write a function returning an `Area` in `src/samples.rs` (or a new
   module), and add it to `samples::all()` with the identifier its static
   table should get.
2. Pick the `world`. It sets the object meanings, the enemy ID space and
   the tiles. Use `PalaceA`/`PalaceB` for palace rooms, `GreatPalace`,
   `WestTowns`/`EastTowns` for towns, and `Overworld` for caves and scenes.
3. Set `region` (which continent's table, for `Overworld` areas; ignored
   otherwise) and `first_map` (room `i` is written as map `first_map + i`
   in the tool-facing bytes; keep `first_map + rooms - 1 <= 62`).
4. Build each `Room` with `Room::new(id, pages, floor_pattern)`, then
   fill in:
   * `objects`, in drawing order (later objects draw over earlier ones).
     Columns run 0-63 across up to 4 pages of 16. Rows run 0-12 from the
     top. Sizes are 1-16. See the table on `model::Object` for which kinds
     exist in which world. `Object::Raw` covers anything without a name.
   * `enemies`: `EnemySpawn { x, y, id }`. Rows 1 and 3-17 can be stored.
     Each page holds at most 6 enemies plus items.
   * `exits`: `Exit::new(dir, target)`, plus `.needs(abilities)` when
     reaching that exit takes something.
     * `Left`/`Right` walk off an edge.
     * `Up`/`Down` ride the elevator, so the room needs an
       `Object::Elevator`.
     * `Drop { page }` falls through a pit. The target needs a
       `landing` page.
     * `Door { x }` is a town doorway. The engine handles it; it is not a
       vanilla connection byte.

     A target is a `Room(id)`, a vanilla `Map { map, page }`,
     `Outside` or `OutsideAt(n)`.
   * `requires` (needed to cross the room at all) and `item_requires`
     (needed to reach its items).
5. Add `entrances` (overworld region, row 0-74, column 0-63, tile look,
   arrival room and page), `links` (`VanillaLink::byte(map, byte, room,
   page)` / `VanillaLink::door(map, door_page, room, page)`) and any `npcs`
   (`TownNpc`: room, list entry, `Message`).
6. Set `needs`, the abilities the area is designed to be fully explored
   with.
7. Add the area to the validation tests. Every sample is already checked
   by `every_sample_area_validates`, by the synthetic geometry test, and,
   with `Z2_ROM` set, by `samples_pass_geometry_on_the_real_floor_patterns`.

### Connection-byte rules to keep in mind

A room has four connection bytes, and byte `i` serves page `i`'s side.
Left uses byte 0, elevator down byte 1, elevator up byte 2, right byte
`pages - 1`, and a fall on page `p` uses byte `p`. In a narrow room some
of these collide. A 1-page room's left and right exits share byte 0, and
a 2-page room's right exit shares byte 1 with down. Validation reports a
collision when the two exits need different values. Plan room widths
around this: a room with left, right and an elevator should be 4 pages
wide.

### Dialog

Messages use the game's alphabet: `A-Z 0-9`, space and `- . / , ! ? *`.
`\n` breaks a line and `~` pauses (it also counts as a line break). A
message holds at most 4 lines of 11 characters. `text::wrap` word-wraps
plain text to fit.

## Authored areas

`src/areas/` holds the areas that ship (the samples are examples only):

| option | areas |
|---|---|
| `parapa_annex` | Parapa Annex: a seven-room wing off Palace 1's entrance room, with an outdoor back door |
| `challenge_rooms` | Skyward Stair, Ember Run, Rubble Maze, Lizalfos Pit, Crumbling Causeway: one- or two-room trials entered from the overworld |
| `minigames` | Target Gallery (Mido), Pogo Tower (Darunia): behind new town doorways |
| `hidden_caves` | six small caves with the container pieces, a P-bag and a 1-up |
| `dark_isle` | Dark Isle Caves, Grimhollow (a monster town), Thornspire (an eighth palace) |
| `test_room` | the developer rooms (`dev_rooms`) |

Each module documents its rooms, attach points, abilities and rewards.
The overworld location slot of every entrance is handed out in one table,
`areas::SLOTS` (unique per continent, Death Mountain and Maze Island
counted as one layout; a multi-mouth area's mouths on consecutive slots).
The four heart pieces and four magic pieces (`$13` / `$14`) are spread
over `hidden_caves` and `minigames`: one extra container of each.

Tests: `src/areas/mod.rs` validates every area, keeps the generated files
current and checks the slot table; the ROM-gated `tests/areas_rom.rs`
checks every area on the real floor patterns, that every declared ability
is needed by the geometry, the vanilla palace links and every claimed
overworld slot and tile against the player's ROM;
`tests/caves_minigames_rom.rs` and `tests/dark_isle_rom.rs` hold the
area-specific checks. `z2-core/tests/content_areas_all.rs` checks all
options together against the engine registry (bit budgets, statics).

## What validation checks

`validate(&area)` needs only the area:

* Sizes: 1-4 pages, header fields in range, objects and enemies inside the
  room, rows 0-12, sideview at most 255 bytes, at most 24 list entries per
  room and 6 enemies plus items per page.
* Exits: every target room exists, map numbers fit, connection bytes do
  not collide, up/down exits have elevators at both ends, drop targets
  have a landing page, every pit has a drop exit, and a locked door comes
  with a declared `KEY`.
* Text: every message fits the font and the box.
* Objects: no `Platform` in an `Overworld` area (world 0 has no large
  object kind 1; the loader runs off the rails, measured).
* Links: maps and bytes in range, the target room and page exist, doors
  only in towns, no exit linked twice.
* Area graph (`reachable`): with `area.needs`, every room and every item
  can be reached from an entrance, and every room has a way back out.

`validate_geometry(&area, &floors)` also replays each room on its
solid-tile grid. It checks that every entry (the sides with exits, the
landing page, entrance arrivals) reaches every exit and item using only
the abilities declared for them. The movement model is ported from
`z2-rando::palaces::reach` and is conservative. A plain jump clears 2 rows
and 2 columns, and the jump spell clears 4. The glove breaks blocks, a
fairy flies through free cells and locked doors, and a key opens locked
doors. With the downward thrust (`DOWNSTAB`), every enemy in the room is
also something Link can land on and jump again from (a down-stab bounces
him off it), so a chain of enemies in the air can stand in for ledges. The
model also checks that every open-bottom page has a drop exit.

## Byte formats

| Format | Module | Notes |
|---|---|---|
| Sideview (room layout) | `sideview` | Header plus 2- and 3-byte commands, page jumps generated. Ported from `z2-rando::sideview`. |
| Solid-tile grid | `sideview::Grid` | Floors, objects, pits, lava, elevator, doors, items. |
| Enemy list | `enemies` | A length byte, then `YX`, `PI` pairs. Ported from `z2-rando::enemies::shuffle`. |
| Connection bytes | `compile::room_connections` | Same rules as `z2-rando::palaces::write::connection_bytes`. |
| Dialog text | `text` | Ported from `z2-rando::text`. |

The ROM-gated `tests/rom_crosscheck.rs` checks the ported decoders against
`z2-rando` on every vanilla room, enemy list and message, read from the
player's ROM at test time.
