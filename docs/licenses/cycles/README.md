# Cycles

Operandi renders with **Cycles**, the path-tracing renderer made by the
Blender Foundation and the Cycles contributors (see `AUTHORS`) as part of
the Blender project. Thank you to everyone who built it.

- Project: https://www.cycles-renderer.org
- Source: https://projects.blender.org/blender/cycles
- Version used: commit `3eb35f85996baa6c1b2e19bf297b22745c85e321`
  (main, 2026-10-26, Cycles 5.3.0 alpha), pinned in `scripts/build-cycles.sh`.
- Licence: Apache License 2.0 (`LICENSE`). Copyright 2011-2025 Blender
  Authors / Blender Foundation.
- Changes: none. Operandi builds Cycles unmodified and talks to it through
  its own C bridge in `crates/operandi-cycles/cycles/`.

Parts of Cycles carry their own licences (all kept in the source files
that ship with the app under `Contents/Resources/cycles/source/`):

| Part | Copyright | Licence |
|---|---|---|
| Most of Cycles | Blender Authors / Blender Foundation | Apache-2.0 |
| MikkTSpace tangents (`third_party/mikktspace`) | 2011 Morten S. Mikkelsen | Zlib |
| MD5 (`util/md5`) | 1999, 2002 Aladdin Enterprises | Zlib |
| Sky model (`third_party/sky`) | 2012-2013 Lukas Hosek and Alexander Wilkie; 2022 Fernando García Liñán | BSD-3-Clause / MIT |
| Voronoi texture | Blender Authors | MIT |
| Atomic hash set in MikkTSpace (simplified from Folly) | 2012-2021 Meta Platforms, Inc. | Apache-2.0 |
| Some closures, OSL shaders and sampling code | Sony Pictures Imageworks, Larry Gritz, NVIDIA, Intel, Blender Foundation | BSD-3-Clause |

The generic licence texts Cycles installs are in
`Contents/Resources/cycles/license/` inside the app.

## Code adapted into Operandi's own source

The viewport draws node graph layers with sums rewritten from Cycles'
kernel (MaterialX stage 5, 2026-11-20), so they are in every build, not
only `--features cycles` ones:

| Operandi file | Adapted from (Cycles, same commit) | Licence |
|---|---|---|
| `crates/operandi-core/src/cycles_noise.rs`, `shader_func.wgsl` | `util/hash.h`; `kernel/svm/noise.h` (Perlin noise, from Open Shading Language, © 2009-2010 Sony Pictures Imageworks Inc., et al.); `fractal_noise.h`, `noisetex.h`, `white_noise.h`, `checker.h`, `gradient.h`, `wave.h` | Apache-2.0 (`LICENSE`); `noise.h` BSD-3-Clause (`BSD-3-Clause-license.txt`) |
| `crates/operandi-core/src/cycles_voronoi.rs`, `cycles_voronoi.wgsl` | `kernel/svm/voronoi.h` (after Inigo Quilez, © 2013); PCG hashes in `util/hash.h` | MIT (`MIT-license.txt`) and Apache-2.0 |
| `crates/operandi-core/src/shader_func.rs`, `shader_func.wgsl` | `kernel/svm/math_util.h`, `color_util.h`, `hsv.h`, `map_range.h`, `mapping_util.h`, `fresnel.h`, `kernel/closure/bsdf_util.h`, `util/color.h`, `util/math_base.h`, `util/math_float3.h`, `util/transform.h` | Apache-2.0 |

Each file's header names its sources; the code is rewritten, not copied.
