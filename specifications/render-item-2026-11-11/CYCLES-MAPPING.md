# Render item: Cycles mapping

| Row | Cycles setting | Notes |
|---|---|---|
| Render Camera | scene camera | existing |
| Frame Width / Height | output width / height | camera Override Render Resolution wins; Resolution Unit + DPI only change how pixels are worked out |
| DPI | written to saved image metadata | not a Cycles setting |
| Pixel Aspect Ratio | camera sensor aspect (pixel aspect x/y) | |
| Render Region + edges | render border (min/max x/y, 0…1) | Top/Bottom flip: Cycles border y counts from the bottom |
| Ray Tracing Engine | device: CPU / Metal / OptiX | unavailable → CPU + note |
| Maximum Quality | samples (adaptive sampling max) | replaces scene Samples |
| Noise Threshold | adaptive threshold = percent / 100 (10 % → 0.1); 0 → Cycles auto | replaces Adaptive Threshold. Adaptive sampling always on |
| Antialiasing Filter | pixel filter: Gaussian (width 1.5) / Box; Triangle → Blackman-Harris width 1.5 | close, not identical |
| Diffuse Depth | diffuse bounces | |
| Reflection Depth | glossy bounces | |
| Refraction Depth | transmission bounces (also transparent bounces = same value) | |
| (all three) | max bounces = largest of the three + 1 | so no single cap cuts a type short |
| Path Threshold | light sampling threshold / min-contribution termination | tier 3; nearest equivalent, not exact |
| Maximum Radiance | sample clamp indirect; clamp direct stays 0 | |
| Indirect Caustics | caustics reflective / refractive on or off | exact |
| Adaptive Subdivision + Rate | adaptive subdivision, dicing rate (px) | per-mesh level used when off |
| Micropoly Displacement | material displacement method = displacement; off = no displacement | off ignores displacement layers |
| Displacement Rate | dicing rate for displaced meshes | |
| Displacement as Bump | displacement method "displacement and bump" | |
| Convergence Target (Preview tab) | Preview stops at target × Maximum Quality samples | Preview session only |
| Max Render Time (Preview tab) | Preview session time limit (seconds = minutes × 60) | 0 = none; not used by F9 |
| Denoise (moved to Final Color output) | OIDN denoiser on/off | unchanged behaviour |

Saving: new scene-file channels for every built row; bump the native
format version. Old files: missing rows get defaults, except Samples,
Adaptive Threshold and Denoise, which are read from the old fields.

MCP bridge: the existing Samples / Denoise / Adaptive Threshold tools keep
working and now read and write the new rows (Maximum Quality, Noise
Threshold, Final Color Denoising). Any new command names go through the
command registry thread (UIC-160..165).
