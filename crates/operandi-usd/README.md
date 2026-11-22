# operandi-usd

USD export and import through OpenUSD. For export the Rust side (`src/`)
turns an Operandi scene into a plan of prims and hands it to a small C++
bridge (`bridge/`), which is the only code that talks to OpenUSD. For
import (docs/BEHAVIOR-USD-IMPORT.md) the bridge's reader
(`bridge/operandi_usd_read.cpp`) walks a stage once and describes it as
JSON plus binary buffers (`src/stage.rs`); `src/import.rs` builds the
scene from that in plain Rust and `src/import_materials.rs` feeds each
material's shading network to Operandi's MaterialX reader. The reader is
Operandi's own code; nothing in it is adapted from Blender.
`scripts/build-usd.sh` builds the bridge against the OpenUSD in Blender's
precompiled library pack; `--features usd` links it.

## Licences

`src/`, `bridge/operandi_usd.h`, `bridge/operandi_usd.cpp`,
`bridge/operandi_usd_read.cpp` and `bridge/CMakeLists.txt` are Operandi's
own.

`bridge/blender/` is adapted from Blender's USD exporter
(<https://projects.blender.org/blender/blender>, commit
`e1bec63572c7092feb1af6719f8b4f76acce15fc`), copyright Blender Authors,
licensed GPL-2.0-or-later (`LICENSES/GPL-2.0-or-later.txt`). Each file keeps
Blender's notice and SPDX line. Changes to files there come in under
GPL-2.0-or-later (CONTRIBUTING.md).

| File | Adapted from |
| --- | --- |
| `bridge/blender/blender_usd.hh` | `source/blender/io/usd/intern/usd_utils.hh`, `usd_writer_camera.hh`, `usd_writer_light.hh`, `usd_writer_mesh.hh`, `usd_writer_material.hh`, `usd_writer_transform.hh` |
| `bridge/blender/usd_utils.cc` | `source/blender/io/usd/intern/usd_utils.cc` |
| `bridge/blender/usd_writer_camera.cc` | `source/blender/io/usd/intern/usd_writer_camera.cc` |
| `bridge/blender/usd_writer_light.cc` | `source/blender/io/usd/intern/usd_writer_light.cc` (and the dome part of `usd_light_convert.cc`) |
| `bridge/blender/usd_writer_material.cc` | `source/blender/io/usd/intern/usd_writer_material.cc` |
| `bridge/blender/usd_writer_mesh.cc` | `source/blender/io/usd/intern/usd_writer_mesh.cc` |
| `bridge/blender/usd_writer_transform.cc` | `source/blender/io/usd/intern/usd_writer_transform.cc` |

OpenUSD itself is TOST-1.0 (Apache-2.0 with a different trademark section);
see `docs/DEPENDENCIES.md`.

## Tests

`cargo test -p operandi-usd` tests the plans and the import's scene
builder without OpenUSD. `cargo test -p operandi-usd --features usd` (after
`scripts/build-usd.sh`) also writes and reads real files, including the
Blender-made USD-T1 scene in `fixtures/usd/usd_t1` (rebuild it with
`fixtures/usd/make_usd_t1.py`, see its header).

