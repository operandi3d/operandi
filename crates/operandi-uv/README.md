# operandi-uv

UV unwrapping and island packing for Operandi, ported to Rust from Blender
at commit `b4eed6fd498e76a7e93404bac2326e9f84deba22`:

| Operandi file | Blender source | Licence |
|---|---|---|
| `chart.rs`, `abf.rs`, `lscm.rs`, `stretch.rs` | `source/blender/geometry/intern/uv_parametrizer.cc` (and `blenlib/intern/rand.cc` for the generator) | GPL-2.0-or-later |
| `pack.rs` | `source/blender/geometry/intern/uv_pack.cc` | GPL-2.0-or-later |
| `boxpack.rs` | `source/blender/blenlib/intern/boxpack_2d.cc` | GPL-2.0-or-later |
| `geom.rs` | `source/blender/blenlib/intern/convexhull_2d.cc` and parametrizer helpers | GPL-2.0-or-later |
| `slim/mod.rs` | `intern/slim/intern/{geometry_data_retrieval,uv_initializer,least_squares_relocator,area_compensation,slim_parametrizer}.cpp` and the SLIM part of `uv_parametrizer.cc` | GPL-2.0-or-later |
| `slim/solver.rs` | `intern/slim/intern/{slim,flip_avoiding_line_search,doublearea}.cpp` (Michael Rabinovich, Alec Jacobson, Blender Authors) | MPL-2.0 |

`sparse.rs` (the faer wrapper), `lib.rs` and the tests are Operandi's own.
Each file's header names what it was adapted from. Operandi as a whole is
GPL-3.0-or-later; the GPL-2.0-or-later files are used under GPL-3.0, and
the MPL-2.0 file stays MPL-2.0 (text in `LICENSES/MPL-2.0.txt`).

Contributions to these files come in under their file's licence
(CONTRIBUTING.md).

## Differences from Blender

- Pointers and linked lists became index arenas; maths is f64 throughout.
- N-gons are triangulated by ear clipping with edge flips, not Blender's
  polyfill and beautify.
- The minimum-area rotation tests every hull edge instead of rotating
  calipers (same rectangles).
- SLIM's three-pin fit uses a proper Procrustes scale, and its two-pin fit
  turns the right way (Blender's code notes both).
- The shape packer's bitmap reads test only the span of each row the
  triangle covers (the bitmap is a distance field, so this keeps its
  decisions); about ten times faster on big islands.
- Cancellation and progress reporting from Blender's job system are left
  out.
