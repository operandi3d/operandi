# operandi-remesh

Automatic Retopology's back end: QuadriFlow in C++, built with `cc` into one
static library and wrapped in safe Rust (`src/lib.rs`). Voxel Remesh and
Blocks / Smooth / Sharp are not here: they are Operandi's own Rust code in
`operandi-core/src/remesh_voxel.rs`.

## What's inside

| Path | What | Licence | From |
|---|---|---|---|
| `vendor/quadriflow/` | QuadriFlow (Huang et al. 2018) with Blender's patch (no Boost) | BSD-3-Clause (`LICENSE.txt`) | Blender `extern/quadriflow` at e1bec63572c7092feb1af6719f8b4f76acce15fc (upstream 27a6867) |
| `vendor/quadriflow/3rd/lemon-1.3.1/` | LEMON graph library | BSL-1.0 (`LICENSE`) | as above |
| `vendor/quadriflow/3rd/pcg32/` | PCG random numbers | Apache-2.0 (file header) | as above |
| `vendor/quadriflow/3rd/pss/` | Parallel stable sort | BSD-3-Clause (file header) | as above |
| `vendor/quadriflow/patches/boykov_kolmogorov_max_flow.hpp` | Max flow, adapted from Boost.Graph | MIT (SPDX header) | as above |
| `vendor/eigen/` | Eigen 3.4.0 headers | MPL-2.0 (`COPYING.MPL2`); `EIGEN_MPL2_ONLY` is set | gitlab.com/libeigen/eigen 3.4.0 |
| `cpp/quadriflow_capi.*` | QuadriFlow C API | GPL-2.0-or-later | Adapted from Blender `intern/quadriflow` at e1bec635 |
| `cpp/lemon/config.h`, `src/`, `build.rs` | Operandi's glue | GPL-2.0-or-later | Operandi |

The crate as a whole is distributed under GPL-2.0-or-later together with
the licences above (all compatible with Operandi's GPL-3.0-or-later).

## Operandi's changes to vendored files

- `vendor/quadriflow/src/parametrizer.hpp`, `parametrizer-mesh.cpp`: the
  crease angle for "preserve sharp" is a setting (`sharp_angle_degrees`,
  upstream fixes it at 60°), and open edges are skipped in that test instead
  of being compared with face 0.
- QuadriFlow's `main.cpp`, `Optimizer.cu` and `post-solver.cpp` (command
  line, CUDA, Boost) are left out; LEMON's CMake files are left out.
- `cpp/quadriflow_capi.cpp`: malloc instead of Blender's guarded allocator,
  `QFLOW_free_result`, a `sharp_angle` field, and C++ exceptions are caught
  so none reaches Rust.
