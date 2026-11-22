# Performance snapshot, 2026-11-12 (main ccdee57e)

Measured on a Mac Studio, Apple M1 Max (10-core CPU, 32-core GPU), 64 GB,
Metal. Release build. Every frame is the app's whole per-frame work (input,
panels, viewport, picking, GPU upload and draw) at 1440 × 900 points, Retina
scale 2, with the GPU waited on each frame, no vsync. Scenes are generated
(no third-party assets).

Rerun:

```bash
OPERANDI_PERF_DIR=<folder> cargo test --release -p operandi-core --test all perf_snapshot:: -- --ignored --nocapture
OPERANDI_PERF_DIR=<folder> OPERANDI_PERF_SCENES=edit1m,objects10k,sub10m,flat10m,sub50m cargo test --release -p operandi-app perf_snapshot -- --ignored --nocapture --test-threads 1
```

## Scenes

| Scene | What | Cage polygons | Triangles drawn | File |
|---|---|---|---|---|
| sub10m | one sphere, Catmull-Clark level 2 | 313k | 10.0M | 28 MB |
| sub50m | one sphere, Catmull-Clark level 2 | 1.56M | 50.0M | 142 MB |
| flat10m | one sphere, no subdivision | 5.0M | 10.0M | 311 MB |
| edit1m | one sphere, no subdivision | 1.0M | 2.0M | 62 MB |
| objects10k | 10,000 spheres of 128 polygons | 1.28M | 2.2M | 518 MB |

## Results (avg ms per frame, fps in brackets)

| | sub10m | sub50m | flat10m | edit1m | objects10k |
|---|---|---|---|---|---|
| Orbit, Item mode | 9.3 (108) | 46.5 (21) | 104 (10) | 20.7 (48) | 181 (5.5) |
| Orbit, Polygon mode | 9.2 (109) | 59.8 (17) | 93 (11) | 19.9 (50) | 183 (5.5) |
| Pan | 9.1 | 46–55 | 90–95 | 19–20 | 180–283 |
| Open (file + first frame) | 0.6 s | 3.2 s | 10.6 s | 2.1 s | 4.1 s |
| Save (in app) | 0.06 s | 0.28 s | 0.70 s | 0.14 s | 1.2 s |
| Switch to Vertex / Edge / Polygon | 25–30 ms | 0.17–0.21 s | 0.38–0.59 s | 67–88 ms | 0.63–0.96 s |
| Switch to Item mode | 0.65 s | 5.0 s | 29 s | 2.2 s | 0.85 s |
| Switch Item → Polygon | 0.49 s | 3.8 s | 20 s | 1.7–2.0 s | 0.96 s |
| Click-select a polygon / item | 75 ms | 0.44 s | 1.1 s | 0.22 s | 3.2 s (item) |
| Drag 1 polygon (Move) | 87 (11.5) | 461 (2.2) | 1522 (0.7) | 280–295 (3.5) | — |
| Drag 10k polygons | 140 (7) | 683 (1.5) | 1478 (0.7) | 315 (3.2) | — |
| Drag 100 points | 98 (10) | 352 (2.8) | 1611 (0.6) | 260 (3.8) | — |
| Drag 1 item / all items | — | — | — | — | 602 (1.7) / 1380 (0.7) |
| Undo / redo a drag | 73–123 ms | 0.36–0.53 s | 1.1–1.3 s | 0.22–0.29 s | 0.45 s / 1.0 s |
| Peak memory (process) | 5.1 GB | 15.7 GB | 32.7 GB | — | 4.8 GB |

Core-only file times (no GUI): save 0.09 / 0.40 / 1.76 / 0.34 / 1.46 s,
load 0.04 / 0.24 / 0.62 / 0.11 / 0.92 s (same scene order as the table).

## Where the time goes (sampling profiler, 1M-polygon drag)

- About a third of each drag frame is whole-mesh edit bookkeeping in
  `operandi-core`: identity validation (`IdentityRuntime::validate_current`,
  `synchronize`), `Scene::validate` / `Mesh::validate` over every polygon,
  and a full copy of the mesh (`Shared<Mesh>` copy-on-write).
- About a quarter is GPU geometry: `Resident::item_placed` rebuilds the whole
  mesh several times per drag, ~49 MB is uploaded every frame for a
  one-polygon move, and `edge_bytes` / `whole_mesh_map` rebuild the
  wireframe and maps for the whole mesh.
- Switching to Item mode rebuilds all GPU geometry and wire (`item_placed`,
  `edge_bytes`), which is why it costs seconds.
- Plain (unsubdivided) meshes cost far more GPU data per triangle than the
  subdivided surface (~1.6 KB per cage polygon uploaded), so flat10m orbits
  10× slower than sub10m with the same triangle count.
- 10k objects: per-item per-frame work (~18 µs × 10k items) dominates orbit;
  the file also costs ~50 KB per item.
- First hover over a big mesh builds the picking surface (0.25 s on sub10m,
  0.76 s on edit1m, 6 s on flat10m).

## Size caps seen

- `Mesh::validate` refuses any mesh over 5,000,000 points or polygons, so a
  plain 10M-triangle mesh only fits as quads (flat10m is just under).
- Bevel, Extrude, Bridge and Spike refuse when the whole mesh would pass 5M
  polygons or points, plus their 8M generated-attribute and 1M generated
  geometry budgets; Extrude, Spike and action centres stop at 100,000
  selected corners.
- Smooth Shift refuses any mesh with more than 100,000 corners in total
  (about 25k quads), whatever is selected.
- The Sphere primitive stops at 250,000 polygons.
