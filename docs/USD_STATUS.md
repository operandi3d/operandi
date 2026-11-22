# USD status

Audit of the code and tests on main `99c49012` (2026-11-19), not of the
docs, updated for branch usd-gaps-2b (the four gap jobs: foreign data,
curves, PointInstancer export, untested rows) and branch usd-gaps-3
(node graphs, render settings, mesh lights, variants; 2026-11-20). Code lives in `crates/operandi-usd`; paths below are relative to its
`src/` (and `bridge/`). Tests named are in `tests.rs`, `plan_tests.rs`,
`import_tests.rs`, `export_gaps_tests.rs`, `lights.rs`,
`import_lights.rs`, `untested_rows_tests.rs`, `foreign_tests.rs`,
`curves.rs`, `curves_tests.rs`, `point_instancer_tests.rs`,
`node_graph_usd_tests.rs`, `render_usd_tests.rs`, `mesh_lights_tests.rs`
and `import_variants.rs`. "untested" = the code path exists but no test reaches it.

D = done, P = partial, M = missing, n/a = no such thing on that side.

| Item | Export | Import | Notes and evidence |
| --- | --- | --- | --- |
| Meshes, quads and n-gons | D | D | plan.rs `mesh_prim`; `a_box_round_trips_through_openusd`, `polygons_and_their_maps_come_across` |
| Normals | D | D | Export: corner normals of plain polygons, shading set only (other normal maps noted, plan.rs `extra_normal_maps`). Import: `normals` map + smoothing (import.rs `NORMAL_MAP`) |
| UV sets | D | D | Primary map is `st`, others by name; `the_texture_map_is_st_and_other_maps_keep_their_names`, `uvs_are_indexed_primvars_and_tags_are_subsets` |
| Weight maps | D | D | float primvars with Operandi customData (plan_extras.rs). Import: Operandi's, and other programs' float primvars per point or corner (Blender vertex groups and attributes, Houdini attributes), USD-IQ23; `blender_vertex_groups_shape_keys_and_attributes_come_across`, `houdini_style_attributes_and_groups_come_across` |
| Typed maps | D | D | Import: other programs' ints, float2/3/4 and normal/point/vector primvars become typed maps by width and role (USD-IQ23); same tests |
| Colour maps | D | D | color4f primvars, first one also `displayColor`. Import takes any vertex/faceVarying colour primvar; constant ones dropped and noted |
| Morphs / blend shapes | D | D | UsdSkelBlendShape + BindingAPI. Import: every bound BlendShape, Blender shape keys included (offsets and pointIndices); inbetweens left out |
| Selection sets | D | D | Polygon/point sets as GeomSubsets, edge sets in customData; set groups dropped and noted (`what_stays_behind_is_reported`). Import: Operandi families and any other family but materialBind (face, point and edge subsets; Houdini groups) |
| Parts | D | D | GeomSubset family `operandi:part`; round trip test above |
| Subdivision and creases | D | D | Catmull-Clark scheme, boundary and FV rules, creases ×10; `subdivision_cage_writes_scheme_and_creases`, `subdivision_and_creases_come_across`. Import drops corner sharpness (noted) |
| Curves (BasisCurves / NurbsCurves) | D | D | curves.rs (USD-IQ24). Export exact for every kind: linear, bezier, periodic bspline, clamped B-Splines as NurbsCurves, catmullRom; widths from a `widths` typed map. Import every basis and wrap; NURBS other than order 2 and clamped uniform cubics are sampled (reported). `every_curve_kind_round_trips`, `other_programs_curves_come_in_as_the_curves_they_draw`, `blender_curves_round_trip` |
| Instances (instanceable) | D | D | Mesh Instances as instanceable refs to `/root/_prototypes`; `mesh_instances_are_usd_instances`, `instances_or_copies` |
| PointInstancer | D | D | Export: 16+ plain Mesh Instances of one source under one parent (point_instancer.rs, USD-IQ25), names and eyes in customData; `a_thousand_copy_scatter_round_trips_as_a_point_instancer` (63 KB vs 116 KB, 0.017 s vs 0.097 s to import). Import: copies or Mesh Instances; invisibleIds hide, inactiveIds skip (the mask used to misalign instances) |
| Cameras (persp / ortho / DOF) | D | D | lights.rs `camera_prim`, import_lights.rs `camera_settings`; perspective, `an_orthographic_camera_keeps_its_view_size`, `depth_of_field_keeps_its_f_stop_and_focus_distance`; cylindrical written as perspective |
| SphereLight | D | D | Point/spot/photometric; `cameras_and_lights_are_usd_cameras_and_lights_in_their_items`, `blender_lights_keep_their_cycles_power` |
| DiskLight | D | D | Round area lights (ellipses noted as disks); `a_round_area_light_is_a_disk_light_both_ways` |
| RectLight | D | D | tested both ways |
| DistantLight | D | D | `light_power_follows_blenders_usd_units`, `usd_t1_cameras_lights_and_the_dome` |
| CylinderLight | D | D | `a_cylinder_light_keeps_its_size_and_radiance` |
| DomeLight | P | D | Export bakes the Environment into a 256×128 .hdr (image layers lose detail, noted); a Dome light item gets intensity only, no picture. Import: picture becomes the environment (`usd_t1_cameras_lights_and_the_dome`) |
| Mesh / Geometry light | D | D | mesh_lights.rs: UsdLux MeshLightAPI on the prototype's Mesh prim (not normalized: intensity is the radiance; `noMaterialResponse`), light name in customData. Lights with Use Prototype Transform off, no mesh or a Mesh Instance mesh are reported. Import: the mesh plus a Mesh light using it (exposure folded in); a MeshLightAPI mesh used to be taken for a light lens. `a_mesh_light_is_mesh_light_api_on_its_mesh_both_ways` |
| PortalLight | D | D | `a_portal_light_keeps_its_size` (the import used to drop its width and height; fixed 2026-11-19) |
| ShapingAPI | D | D | Cone (`exposure_temperature_and_spot_cones`) and IES file both ways: a SphereLight with an IES file is a photometric light (`a_photometric_light_writes_and_reads_its_ies_file`). The IES path is written as it is, not copied beside the file |
| UsdPreviewSurface | D | D | materials.rs; `materials_are_preview_surfaces_bound_to_meshes_and_tags`, `the_preview_surface_option_reads_usd_preview_surface`. Procedurals written as averages (noted) |
| MaterialX | D | D | materialx_out.rs (U6). Graph layers go out as they are (materialx_graph_out.rs): nodegraphs as NodeGraph prims with interface inputs and outputs, multi-output nodes, spare nodes, xpos/ypos as `ui:nodegraph:node:pos`; a Material Group's free nodes (node editing) all go out. Operandi's networks (customData `operandi:materialxGraph`) import as written (import_mtlx_graph.rs), others resolved. `u6_exports_materialx_and_round_trips`, `a_materialx_graph_comes_back_node_for_node` (topology, values, wires, places identical; Blender 5.2.2 loads it), `free_nodes_go_out_and_come_back` |
| Textures | D | D | Copied to `<name>_textures/` or packed; `image_maps_are_copied_beside_the_file_and_read_through_st`, `usd_t5_a_missing_picture_is_reported_and_kept` |
| UV transforms | D | D | UsdTransform2d repeat/rotation (usd_writer_material.cc); import maps it into MaterialX (import_materials.rs header) |
| Node Material graphs (Blender) | P | D | Export (cycles_mtlx.rs): the graph as the MaterialX nodes its Cycles nodes map to (Principled and Emission onto open_pbr_surface, Image, Math, Mix, Mapping, Invert, Separate/Combine RGB, tangent Normal Map, Value, RGB, Texture Coordinate); Noise, Voronoi, Wave, Gradient, Color Ramp, Bump, Fresnel, Layer Weight and closures other than the two are reported, and a surface with no map keeps Preview Surface only. No sidecar is written. Import reads `<file>_graphs.json` (`usd_t4_the_graphs_sidecar_brings_node_materials`). `a_blender_graph_goes_out_as_the_materialx_it_maps_to` |
| Transforms and hierarchy | D | D | `/root` Xform, items nest; `children_nest_under_their_parents`, `a_z_up_hierarchy_keeps_local_transforms_turned_to_y_up` |
| Xform op orders | D | D | All 6 rotate orders (`world_placement_matches_operandi_for_both_rotation_orders`); import reads plain TRS, else the matrix (shear dropped, resetXformStack read as local, both noted) |
| Animation / time samples | M | M | Nothing writes time samples; the reader samples `EarliestTime` only (operandi_usd_read.cpp `TIME`). Operandi has scene time but no keyframes yet |
| Visibility and purpose | D | D | Inherited visibility per GV-01 (`hidden_items_and_hidden_hierarchies`). Export writes no purpose (all default). Import drops guide/proxy, keeps render (`usda_details_come_across`) |
| Units and up-axis | D | D | metersPerUnit, Y/Z up, X up via a root matrix; `z_up_scenes_say_so`, `up_axis_follows_the_scene`, `units_and_up_axis_can_be_overridden` |
| Kits | M | n/a | Kit items skipped, noted per kind (plan.rs `Skipped`) |
| Locators | P | D | Export: Group Locators as Xforms; other locators only when they hold exported children. Import: transforms become Group Locators |
| Solid Sketch | P | n/a | Polygons export, sketch nodes don't (noted) |
| Render settings | D | D | render_usd.rs: UsdRender RenderSettings `/Render/Settings` (resolution, pixel aspect 1, render camera; renderSettingsPrimPath) and RenderProduct `/Render/Products/Final` (the camera's Resolution Override), Cycles' rows as customData `operandi:render`. Import: resolution is the frame, a product of another size the camera's override, Operandi's rows exactly; other programs' renderer attributes not read; non-square pixels reported. `the_render_item_round_trips_as_render_settings`, `other_programs_render_settings_give_the_frame`, `the_render_settings_camera_becomes_the_render_camera` |
| Variants and payloads | M | D | Export writes neither. Import: OpenUSD's composed stage (default variant selections, payloads loaded by `UsdStage::Open`), reported per prim (import_variants.rs); no variant picker. `the_default_variant_and_payloads_come_in` (fixtures/usd/variants) |
| .usdz | D | D | `usdz_packs_the_layer_and_its_pictures`, `usd_t2_the_usdz_gives_the_same_scene` |
| .usdc / .usda | D | D | `usda_is_text_and_usd_is_binary`; Blender-made `usd_t1.usdc` fixture |

## Score

A partial row counts half.

- Export: 33 done, 4 partial, 3 missing of 40 rows: **88%** (was 81%).
- Import: 37 done, 0 partial, 1 missing of 38 rows (2 n/a): **97%** (was 92%).
- Overall: 72 of 78: **92%** (was 87%).

The nine "done" rows that rested on untested code (ortho/DOF cameras,
Disk, Cylinder and Portal lights, each both ways, and IES export) have
tests since 2026-11-19 (`untested_rows_tests.rs`). They found two gaps,
both fixed: a PortalLight came in with the default size, and an IES file
was written but not read back.

## Samples sweep

`round_trip_the_samples_folder` (opt-in) imports every file in a folder
of public samples kept outside the repo, exports it, imports that and
compares counts and the world box. On 2026-11-20 (usd-gaps-3, 5001 files,
about 2 hours):

| Set | Pass | Skip (no geometry) | Fail |
| --- | --- | --- | --- |
| usd-wg assets (284 files) | 183 | 96 | 5, all expected (below) |
| Pixar Kitchen Set (230 files) | 230 | 0 | 0 |
| Apple USDZ gallery (4 files) | 4 | 0 | 0 |
| Animal Logic ALab 2.3 (4483 files) | 2508 | 1974 | 1 (below) |

The usd-wg failures: two framesPerSecond test files OpenUSD itself
refuses; two timeCodesPerSecond 0 / -1 files, refused with a reason (0
crashed inside OpenUSD before, in Blender's OpenUSD too); and
pointsTypes.usda, a schema test of wrongly typed `points` whose broken
meshes come in empty and aren't exported again. The earlier sweep found
two bugs, both fixed: Houdini's million-unit camera far plane failed the
whole import, and the timeCodesPerSecond crash.

ALab's one failure, the stoat geocache `payload.usdc`: all 285,340
polygons round-trip, but three whisker items (two meshes with no
polygons and their group) come in empty and aren't exported again, the
same way as pointsTypes.usda. Not fixed yet.

## Top 3 missing for real users

1. **Animation / time samples.** Nothing writes or reads them beyond the
   first sample; it should come with keyframes.
2. **Variants and payloads on export, and a variant picker on import.**
   The import takes the default selections only.
3. **Blender graph nodes with no MaterialX counterpart** (procedural
   textures, Color Ramp, Bump) and the DomeLight picture of a Dome light
   item.
