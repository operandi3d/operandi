use crate::{Result, Scene};
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::Path,
};
/// Largest scene file read. Writing has no limit. This was a fixed 64 MiB
/// for writing and reading alike, which refused scenes of a few hundred
/// thousand polygons; reading is now checked section by section instead
/// (packed array lengths, string and point indices, `Scene::validate`),
/// and this only stops a file far bigger than any real scene from being
/// read into memory whole.
const MAX_LOAD_BYTES: u64 = 4 * 1024 * 1024 * 1024;
/// Newest native version this build reads and writes.
// Version numbers: renumber here (and in the tests that name them) when
// branches merge in another order.
const CURRENT_VERSION: u32 = ANIMATION_VERSION;
/// Version 60: animation (`Scene::animation`: key curves, time settings,
/// markers). An older build would refuse the unknown field; a scene with
/// no animation keeps its older number.
const ANIMATION_VERSION: u32 = 60;
fn has_v60_animation(scene: &Scene) -> bool {
    !scene.animation.is_default()
}
/// Version 59: the scene's Guides (G9 GD-3, `Scene::guides`). An older
/// build would refuse the unknown field; a scene with default guides keeps
/// its older number.
const GUIDES_VERSION: u32 = 59;
fn has_v59_guides(scene: &Scene) -> bool {
    !scene.guides.is_default()
}
/// Version 58: Replicator items (G8 RP-1, `Item::replicator`). An older
/// build would refuse the unknown field; a scene without one keeps its
/// older number.
const REPLICATOR_VERSION: u32 = 58;
fn has_v58_replicators(scene: &Scene) -> bool {
    scene.items.iter().any(|i| i.replicator.is_some())
}
/// Version 57: live patch polygons (G7 PC-7, `PolygonRepresentation::Patch`)
/// and a mesh's Patch Subdivisions. An older build would refuse the
/// unknown polygon kind or field; a scene without either keeps its older
/// number.
const PATCH_POLYGON_VERSION: u32 = 57;
fn has_v57_patches(scene: &Scene) -> bool {
    scene.items.iter().any(|i| {
        i.subdivision
            .as_ref()
            .is_some_and(|s| s.patch_subdivisions != crate::patch_live::DEFAULT_CELLS)
            || (!i.mesh.is_packed() && crate::patch_live::has_patches(&i.mesh))
    })
}
/// Version 56: Material Groups' node views (free MaterialX nodes, wires,
/// box places; `ShaderTree::node_views`, node_view.rs). An older build
/// would refuse the unknown field; a scene without any keeps its older
/// number.
const NODE_VIEW_VERSION: u32 = 56;
fn has_v56_node_views(scene: &Scene) -> bool {
    !scene.shader_tree.node_views.is_empty()
}
/// Version 55: render outputs group D (Illumination / Diffuse Shading
/// (Unshadowed), Light Path Expressions and its expression, Shaded AA
/// Samples). Older builds refuse the unknown effect or field, a clean
/// refusal; a scene without any keeps its older number.
const RENDER_OUTPUTS_D_VERSION: u32 = 55;
fn has_v55_outputs(scene: &Scene) -> bool {
    use crate::shader_tree::NodeKind;
    let tree = &scene.shader_tree;
    tree.outputs.values().any(|s| s.needs_v55())
        || tree
            .nodes
            .iter()
            .any(|n| matches!(n.kind, NodeKind::RenderOutput(e) if e.needs_v55()))
}
/// Version 54: node graphs kept as MaterialX text (`StoredGraph::materialx`,
/// materials architecture slice 1). An older build would drop the text
/// without a word, so it must refuse the file; a scene without any keeps
/// its older number.
const MATERIALX_GRAPH_VERSION: u32 = 54;
fn has_v54_materialx_graphs(scene: &Scene) -> bool {
    scene
        .shader_tree
        .node_graphs
        .graphs
        .iter()
        .any(|g| g.materialx.is_some())
}
/// Version 52: material-value Render Outputs (`OutputEffect::material_value`,
/// material_values.rs). Older builds refuse the unknown effect, a clean
/// refusal; a scene without any keeps its older number.
const MATERIAL_VALUE_OUTPUTS_VERSION: u32 = 52;
fn has_v52_material_values(scene: &Scene) -> bool {
    scene.shader_tree.nodes.iter().any(|n| {
        matches!(
            n.kind,
            crate::shader_tree::NodeKind::RenderOutput(e) if e.material_value().is_some()
        )
    })
}
/// Version 53: the Cycles built-in Render Outputs (Illumination, UV
/// Coordinates, Segment ID, Cryptomatte) and the per-output Colorspace and
/// Cryptomatte settings (render_outputs.rs). Older builds refuse the
/// unknown effect or field, a clean refusal; a scene without any keeps its
/// older number.
const RENDER_OUTPUTS_V2_VERSION: u32 = 53;
fn has_v53_outputs(scene: &Scene) -> bool {
    use crate::shader_tree::NodeKind;
    let tree = &scene.shader_tree;
    tree.outputs.values().any(|s| s.needs_v53())
        || tree
            .nodes
            .iter()
            .any(|n| matches!(n.kind, NodeKind::RenderOutput(e) if e.needs_v53()))
}
/// Version 51: Mesh Instances (`Item::instance_of`, mesh_instance.rs).
/// Older builds refuse the unknown field, a clean refusal; a scene without
/// any keeps its older number.
const MESH_INSTANCE_VERSION: u32 = 51;
fn has_v51_instances(scene: &Scene) -> bool {
    scene.items.iter().any(|i| i.instance_of.is_some())
}
/// Version 50: MeshOps selection ops (`MeshOp::selops`,
/// meshop_selection.rs). Older builds refuse the unknown field, a clean
/// refusal; a scene without any keeps its older number.
const SELECTION_OPS_VERSION: u32 = 50;
fn has_v50_selops(scene: &Scene) -> bool {
    scene.items.iter().any(|i| {
        i.mesh_ops
            .as_ref()
            .is_some_and(|s| s.ops.iter().any(|o| !o.selops.is_empty()))
    })
}
/// Version 49 (master plan phase 0 doors): items of a kind a kit adds
/// (`Item::kit`, kit_item.rs; KIT-1) and the scene's time settings
/// (`Scene::time`, scene_time.rs; AR-02). Older builds refuse both as
/// unknown fields, which is a clean refusal; a scene with neither keeps
/// its older number.
const DOORS_VERSION: u32 = 49;
fn has_v49_doors(scene: &Scene) -> bool {
    !scene.time.is_default() || scene.items.iter().any(|i| i.kit.is_some())
}
/// Version 48: files inside the scene's project are saved as
/// `project:<relative/path>` (`project_paths.rs`, docs/PROJECT_SYSTEM.md
/// phase 2). Older builds would read those as missing files, so a scene
/// that has any is marked 48; one with only absolute paths keeps its
/// older number. Migration: none needed; older scenes' absolute paths load
/// unchanged and are rewritten only when saved inside a project.
const PROJECT_PATHS_VERSION: u32 = 48;
/// Version 47: meshes remember their selected UV map
/// (`MeshAttributes::current_uv_map`, UVE-1 section 1).
const UV_MAP_CURRENT_VERSION: u32 = 47;
fn has_v47_current_uv_map(scene: &Scene) -> bool {
    scene
        .items
        .iter()
        .any(|i| i.mesh.attributes.current_uv_map.is_some())
}
/// Version 46: the Render item's Direct Caustics section
/// (`RenderSettings::direct_caustics`). Files leave it out while it holds
/// its defaults.
const DIRECT_CAUSTICS_VERSION: u32 = 46;
fn has_v46_direct_caustics(scene: &Scene) -> bool {
    !scene.render.direct_caustics.is_default()
}
/// Version 45: meshes keep Solid Sketch nodes (`MeshAttributes::solid_sketch`).
const SOLID_SKETCH_VERSION: u32 = 45;
fn has_v45_solid_sketch(scene: &Scene) -> bool {
    scene
        .items
        .iter()
        .any(|i| i.mesh.attributes.solid_sketch.is_some())
}
/// Version 44: rotation order XYZ turns X first (Rz·Ry·Rx on column
/// vectors, the measured IT-O-H3 matrix, ETL-Q01); it turned Z first
/// before. Older items in XYZ whose angles the two readings turn
/// differently move to ZYX, which turns the old way
/// (`Scene::xyz_items_to_x_first`), so every saved scene looks the same.
/// Environment image locators set to Light Probe move back to Spherical
/// (`Scene::environment_light_probes_to_spherical`): Light Probe is a
/// mirror ball since v44 and drew as latitude-longitude before.
const XYZ_FIRST_VERSION: u32 = 44;
fn has_v44_xyz_first(scene: &Scene) -> bool {
    use crate::shader_tree::Projection;
    scene
        .items
        .iter()
        .any(|i| i.transform.xyz_meaning_changed())
        || scene.environment_image_locators().into_iter().any(|l| {
            scene
                .texture_locator(l)
                .is_some_and(|l| l.projection == Projection::LightProbe)
        })
}
/// Version 43: the environment image layer's Texture Locator. Item
/// transforms can hold the four rotation orders besides XYZ and ZXY
/// (`Transform::order`), Texture Locators hold U/V Tile Offset
/// (`TextureLocator::tile_offset`), and environment layers without a
/// locator get one on reading.
const ENV_LOCATOR_VERSION: u32 = 43;
fn has_v43_env_locators(scene: &Scene) -> bool {
    scene.items.iter().any(|i| {
        i.transform.order.is_some()
            || i.texture_locator()
                .is_some_and(|l| l.tile_offset != [0.0; 2])
    })
}
/// Version 42: an environment preset's item group and material group
/// remember the preset (`ItemCommon::from_preset`,
/// `LayerExtras::from_preset`).
const FROM_PRESET_VERSION: u32 = 42;
fn has_v42_from_preset(scene: &Scene) -> bool {
    scene.items.iter().any(|i| i.common.from_preset.is_some())
        || scene
            .shader_tree
            .nodes
            .iter()
            .any(|n| n.extras.from_preset.is_some())
}
/// Version 41: the OpenPBR material channels (`MaterialChannels::openpbr`,
/// openpbr.rs). New materials have them (OpenPBR's defaults), so a scene
/// with an edited shader tree saves as v41; older files read the legacy
/// values.
const OPENPBR_VERSION: u32 = 41;
fn has_v41_openpbr(scene: &Scene) -> bool {
    use crate::shader_tree::NodeKind;
    // An untouched default tree isn't written at all.
    !scene.shader_tree.is_default()
        && scene
            .shader_tree
            .nodes
            .iter()
            .any(|n| matches!(&n.kind, NodeKind::Material(m) if !m.openpbr.is_legacy()))
}
/// Version 40: MeshOps Mirror and Array saved as a Clone Effector with a
/// `pmodel.mirror` / `pmodel.array` generator (registry rev 3). Version 39
/// files saved them flat as `mirror.clone.item` / `array.clone.item`; the
/// reader still takes that form.
const CLONE_EFFECTOR_VERSION: u32 = 40;
fn has_v40_clone_effectors(scene: &Scene) -> bool {
    scene.items.iter().any(|i| {
        i.mesh_ops.as_ref().is_some_and(|s| {
            s.ops.iter().any(|o| {
                matches!(o.kind, crate::meshops::OpKind::Op { .. })
                    && matches!(o.kind.type_id(), "mirror.clone.item" | "array.clone.item")
            })
        })
    })
}
/// Version 39: MeshOps stacks on mesh items (`Item::mesh_ops`, meshops.rs).
const MESH_OPS_VERSION: u32 = 39;
fn has_v39_mesh_ops(scene: &Scene) -> bool {
    scene.items.iter().any(|i| i.mesh_ops.is_some())
}
/// Version 38: node graphs (the shader tree's Nodes section), Node
/// Material layers and texture layers driving their exposed inputs (Nodes
/// N1, NM-03).
const NODE_GRAPH_VERSION: u32 = 38;
fn has_v38_node_graphs(scene: &Scene) -> bool {
    use crate::shader_tree::NodeKind;
    let t = &scene.shader_tree;
    !t.node_graphs.is_empty()
        || t.nodes.iter().any(|n| match &n.kind {
            NodeKind::NodeMaterial(_) => true,
            NodeKind::Texture(x) => x.node_input.is_some(),
            _ => false,
        })
}
/// Version 37: a preset's licence and credit (`PresetHeader::source`,
/// converted materials). Scene files never carry it.
const PRESET_CREDIT_VERSION: u32 = 37;
/// Version 36: the Render item's rows (render item spec v1): frame size,
/// ray depths, Maximum Radiance, Indirect Caustics. Earlier files mean
/// Cycles' own values (`RenderSettings::default`).
const RENDER_ITEM_VERSION: u32 = 36;
fn has_v36_render_item(scene: &Scene) -> bool {
    scene.render.has_render_item_rows()
}
/// Version 35: an item's eye that also hides its children
/// (`Item::hides_children`, GV-01). Earlier files never set it.
const EYE_HIDES_CHILDREN_VERSION: u32 = 35;
fn has_v35_eyes(scene: &Scene) -> bool {
    scene.items.iter().any(|i| i.hides_children)
}
/// Version 34: big mesh arrays written packed (mesh_pack.rs). Readers
/// accept the packed and array forms at any version.
const PACKED_MESH_VERSION: u32 = 34;
fn has_v34_packed(scene: &Scene) -> bool {
    scene
        .items
        .iter()
        .any(|i| crate::mesh_pack::mesh_packs(&i.mesh))
}
/// Version 33: Environment items (`Item::environment`, environment.rs).
/// A scene with cameras or lights is written at 33 even without an
/// environment, so "no environment" survives: files before 33 that have a
/// camera or light (made in the app, which adds both) get the default
/// environment on load, so they keep a sky (coordinator's call,
/// 2026-11-09). Bare test scenes without lights stay at their old version.
const ENVIRONMENT_VERSION: u32 = 33;
fn has_v33_environments(scene: &Scene) -> bool {
    scene.items.iter().any(|i| i.environment.is_some()) || has_cameras_lights(scene)
}
/// Version 32: the shader layer menu's layer state (instances, the
/// Library and references; shader_layer_menu.rs).
const SHADER_MENU_VERSION: u32 = 32;
fn has_v32_shader_menu(scene: &Scene) -> bool {
    scene.shader_tree.nodes.iter().any(|n| !n.extras.is_empty())
}
/// Version 31: item and
/// shader layer Editor Colors (editor_color.rs).
const EDITOR_COLOR_VERSION: u32 = 31;
fn has_v31_editor_colors(scene: &Scene) -> bool {
    scene.items.iter().any(|i| i.common.editor_color.is_some())
        || scene
            .shader_tree
            .nodes
            .iter()
            .any(|n| n.layer.editor_color.is_some())
}
/// Version 30: Group Locator items (`Item::locator`, locator.rs).
const GROUP_LOCATOR_VERSION: u32 = 30;
fn has_v30_locators(scene: &Scene) -> bool {
    scene.items.iter().any(|i| i.locator.is_some())
}
/// Version 29: the other active meshes' component selections
/// (`Selection::others`, multi_active.rs).
const MULTI_ACTIVE_VERSION: u32 = 29;
fn has_v29_others(scene: &Scene) -> bool {
    !scene.selection.others.is_empty()
}
/// Version 28: locked vertices, edges and polygons (`Item::elements`,
/// element_state.rs). Hidden geometry is never saved.
const GEOMETRY_LOCK_VERSION: u32 = 28;
fn has_v28_locks(scene: &Scene) -> bool {
    scene.items.iter().any(|i| !i.elements.nothing_saved())
}
/// Version 25: Gabor Noise and Poisson Cells, and the locator's stored
/// Real World Size X/Y, Texture Replicator and Projection Camera.
const TEXTURES5_VERSION: u32 = 25;
fn has_v25_textures(scene: &Scene) -> bool {
    use crate::procedural::Pattern;
    scene.shader_tree.nodes.iter().any(|n| {
        matches!(
            &n.kind,
            crate::shader_tree::NodeKind::Texture(t)
                if matches!(&t.source, crate::shader_tree::TextureSource::Procedural(p)
                    if matches!(p.pattern, Pattern::Gabor { .. } | Pattern::Poisson { .. }))
        )
    }) || scene
        .items
        .iter()
        .filter_map(|i| i.texture_locator())
        .any(|l| {
            l.real_world_size_xy != [1., 1.]
                || l.replicator.is_some()
                || l.projection_camera.is_some()
        })
}
/// Version 24: Cellular, Wood, Ripples and Weave procedurals.
const MORE_PROCEDURALS_VERSION: u32 = 24;
fn has_v24_procedurals(scene: &Scene) -> bool {
    use crate::procedural::Pattern;
    scene.shader_tree.nodes.iter().any(|n| {
        matches!(
            &n.kind,
            crate::shader_tree::NodeKind::Texture(t)
                if matches!(&t.source, crate::shader_tree::TextureSource::Procedural(p)
                    if matches!(p.pattern, Pattern::Cellular { .. } | Pattern::Wood { .. }
                        | Pattern::Ripples { .. } | Pattern::Weave { .. }))
        )
    })
}
/// Version 23: Texture Locator option names from the full spec (Front
/// Culling, Random Offsets, Falloff, Tangent); older builds only knew the
/// stage 1 placeholders.
const LOCATOR_OPTIONS_VERSION: u32 = 23;
fn has_v23_locator_options(scene: &Scene) -> bool {
    use crate::texture_locator::{FalloffType, FrontCulling, RandomOffsets, TangentType};
    scene
        .items
        .iter()
        .filter_map(|i| i.texture_locator())
        .any(|l| {
            !matches!(l.front_culling, FrontCulling::ProjectThrough)
                || l.random_offsets != RandomOffsets::None
                || l.falloff_type != FalloffType::Cubic
                || l.tangent != TangentType::DpduDpdv
        })
}
/// Version 22: procedural texture layers (procedural.rs).
const PROCEDURAL_VERSION: u32 = 22;
fn has_v22_procedurals(scene: &Scene) -> bool {
    scene.shader_tree.nodes.iter().any(|n| {
        matches!(
            &n.kind,
            crate::shader_tree::NodeKind::Texture(t)
                if matches!(t.source, crate::shader_tree::TextureSource::Procedural(_))
        )
    })
}
/// Version 15 (shader tree Tier 1): Specular is an amount on a curve
/// (`material::specular_reflectance`); earlier files stored the
/// reflectance itself and are converted when read, so they look the same.
const SPECULAR_AMOUNT_VERSION: u32 = 15;
/// Version 16 (light properties Tier 1): portal lights and the Display /
/// Assembly settings every item shares (item_common.rs).
const ITEM_COMMON_VERSION: u32 = 16;
/// Version 18: item groups and Light Linking. (Both this and the Shading
/// Model were first numbered 17 on separate branches; groups moved up.)
const GROUPS_VERSION: u32 = 18;
/// Version 19: Texture Locator items. Older texture layers stored their
/// (never rendered) projection on the layer; reading one gives every such
/// layer a UV Map locator, so it looks as before.
const TEXTURE_LOCATOR_VERSION: u32 = 19;
fn has_v19_locators(scene: &Scene) -> bool {
    scene.items.iter().any(|i| i.texture.is_some())
}
/// Version 20 (camera properties): depth of field, Resolution Override,
/// the camera Background Image and Overlay, Set Focus Distance, Lens
/// Distortion and the projections beyond Perspective and Orthographic.
/// Older files load with those at their defaults.
const CAMERA_PROPS_VERSION: u32 = 20;
/// Whether `scene` needs version 20.
fn has_v20_cameras(scene: &Scene) -> bool {
    use crate::camera_light::{CameraSettings, Projection};
    let base = CameraSettings::default();
    scene
        .items
        .iter()
        .filter_map(|i| i.camera.as_ref())
        .any(|c| {
            !matches!(
                c.projection,
                Projection::Perspective | Projection::Orthographic
            ) || c.dof != base.dof
                || c.resolution != base.resolution
                || c.background != base.background
                || c.overlay.is_some()
                || c.focus_from_target
                || c.lens_distortion != 0.0
        })
}
/// Whether `scene` needs version 18.
fn has_v18_groups(scene: &Scene) -> bool {
    !scene.groups.is_empty()
        || scene
            .items
            .iter()
            .any(|i| i.light.as_ref().is_some_and(|l| l.linking.is_some()))
}
/// Version 21 (shader item v1): a Shader's control switches, multipliers,
/// light linking and fog (`ShaderChannels::more`). Older files load with
/// the starting values.
const SHADER_SETTINGS_VERSION: u32 = 21;
/// Version 27: the scene's render settings (`Scene::render`).
const RENDER_SETTINGS_VERSION: u32 = 27;
/// Version 26 (shader tree remaining, stages 2-6): Gradient, Vertex Map,
/// Variation, Process and Occlusion layers, group Scale, Render Output
/// settings and the newer Material settings.
const GRADIENT_VERSION: u32 = 26;
/// Whether `scene` needs version 26.
fn has_v26_gradients(scene: &Scene) -> bool {
    use crate::shader_tree::{NodeKind, OutputEffect};
    if !scene.shader_tree.outputs.is_empty() {
        return true;
    }
    scene.shader_tree.nodes.iter().any(|n| {
        match &n.kind {
            NodeKind::Group(g) if !g.scale.is_default() => return true,
            NodeKind::Material(m) if !m.more.is_default() => return true,
            NodeKind::Texture(t) if t.ignore_scale_group => return true,
            NodeKind::RenderOutput(e)
                if !matches!(e, OutputEffect::FinalColor | OutputEffect::Alpha) =>
            {
                return true;
            }
            _ => {}
        }
        matches!(
            &n.kind,
            crate::shader_tree::NodeKind::Texture(crate::shader_tree::TextureLayer {
                source: crate::shader_tree::TextureSource::Gradient(_)
                    | crate::shader_tree::TextureSource::VertexMap { .. }
                    | crate::shader_tree::TextureSource::Variation(_)
                    | crate::shader_tree::TextureSource::Process(_)
                    | crate::shader_tree::TextureSource::Occlusion(_),
                ..
            })
        )
    })
}
/// Whether `scene` needs version 21.
fn has_v21_shaders(scene: &Scene) -> bool {
    scene
        .shader_tree
        .nodes
        .iter()
        .any(|n| matches!(&n.kind, crate::shader_tree::NodeKind::Shader(s) if !s.more.is_default()))
}
/// Whether `scene` needs version 16.
fn has_v16_items(scene: &Scene) -> bool {
    scene.items.iter().any(|i| {
        !i.common.is_default()
            || i.light.as_ref().is_some_and(|l| {
                use crate::camera_light::LightKind;
                !matches!(
                    l.kind,
                    LightKind::Directional | LightKind::Point | LightKind::Spot | LightKind::Area
                ) || l.environment_only
                    || l.sun != Default::default()
                    || l.prototype.is_some()
                    || !l.use_prototype_transform
                    || !l.photometry_file.is_empty()
                    || l.length != crate::camera_light_defaults::CYLINDER_LENGTH
                    || l.volumetrics.is_some()
                    || !l.lpe_label.is_empty()
                    || !l.layers.is_empty()
                    || l.blend_mode != crate::camera_light::LightBlend::Normal
            })
    })
}
/// Version 17 (Cycles translation map v0.1): a Material's Shading Model
/// beyond Principled and Match Specular. Older files load as Principled
/// with Match Specular on.
const SHADING_MODEL_VERSION: u32 = 17;
/// Whether `scene` needs version 17.
fn has_v17_materials(scene: &Scene) -> bool {
    scene.shader_tree.nodes.iter().any(|n| match &n.kind {
        crate::shader_tree::NodeKind::Material(m) => {
            m.shading_model != crate::shader_tree::ShadingModel::Principled || !m.match_specular
        }
        _ => false,
    })
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Document {
    format: String,
    version: u32,
    /// Set only in preset files (preset.rs, PB-Q1): a preset is a cut-down
    /// scene file with this header. Builds from before presets refuse such
    /// a file as an unknown field, which is a clean refusal.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    preset: Option<crate::preset::PresetHeader>,
    scene: Scene,
}
/// Edge selection sets and selection set groups arrived in version 13.
fn has_v13_sets(mesh: &crate::Mesh) -> bool {
    !mesh.attributes.edge_sets.is_empty() || !mesh.attributes.set_groups.is_empty()
}
/// Camera and light items (CL-1) need native version 13.
fn has_cameras_lights(scene: &Scene) -> bool {
    scene.render_camera.is_some()
        || scene
            .items
            .iter()
            .any(|i| i.camera.is_some() || i.light.is_some())
}
pub fn encode(scene: &Scene) -> Result<Vec<u8>> {
    encode_document(scene, None)
}
fn encode_document(scene: &Scene, preset: Option<&crate::preset::PresetHeader>) -> Result<Vec<u8>> {
    scene.validate()?;
    let data = serde_json::to_vec_pretty(&Document {
        format: "Operandi".into(),
        preset: preset.cloned(),
        version: if has_v60_animation(scene) {
            ANIMATION_VERSION
        } else if has_v59_guides(scene) {
            GUIDES_VERSION
        } else if has_v58_replicators(scene) {
            REPLICATOR_VERSION
        } else if has_v57_patches(scene) {
            PATCH_POLYGON_VERSION
        } else if has_v56_node_views(scene) {
            NODE_VIEW_VERSION
        } else if has_v55_outputs(scene) {
            RENDER_OUTPUTS_D_VERSION
        } else if has_v54_materialx_graphs(scene) {
            MATERIALX_GRAPH_VERSION
        } else if has_v53_outputs(scene) {
            RENDER_OUTPUTS_V2_VERSION
        } else if has_v52_material_values(scene) {
            MATERIAL_VALUE_OUTPUTS_VERSION
        } else if has_v51_instances(scene) {
            MESH_INSTANCE_VERSION
        } else if has_v50_selops(scene) {
            SELECTION_OPS_VERSION
        } else if has_v49_doors(scene) {
            DOORS_VERSION
        } else if crate::project_paths::has_project_paths(scene) {
            PROJECT_PATHS_VERSION
        } else if has_v47_current_uv_map(scene) {
            UV_MAP_CURRENT_VERSION
        } else if has_v46_direct_caustics(scene) {
            DIRECT_CAUSTICS_VERSION
        } else if has_v45_solid_sketch(scene) {
            SOLID_SKETCH_VERSION
        } else if has_v44_xyz_first(scene) {
            XYZ_FIRST_VERSION
        } else if has_v43_env_locators(scene) {
            ENV_LOCATOR_VERSION
        } else if has_v42_from_preset(scene) {
            FROM_PRESET_VERSION
        } else if has_v41_openpbr(scene) {
            OPENPBR_VERSION
        } else if has_v40_clone_effectors(scene) {
            CLONE_EFFECTOR_VERSION
        } else if has_v39_mesh_ops(scene) {
            MESH_OPS_VERSION
        } else if has_v38_node_graphs(scene) {
            NODE_GRAPH_VERSION
        } else if preset.is_some_and(|p| p.source.is_some()) {
            PRESET_CREDIT_VERSION
        } else if has_v36_render_item(scene) {
            RENDER_ITEM_VERSION
        } else if has_v35_eyes(scene) {
            EYE_HIDES_CHILDREN_VERSION
        } else if has_v34_packed(scene) {
            PACKED_MESH_VERSION
        } else if has_v33_environments(scene) {
            ENVIRONMENT_VERSION
        } else if has_v32_shader_menu(scene) {
            SHADER_MENU_VERSION
        } else if has_v31_editor_colors(scene) {
            EDITOR_COLOR_VERSION
        } else if has_v30_locators(scene) {
            GROUP_LOCATOR_VERSION
        } else if has_v29_others(scene) {
            MULTI_ACTIVE_VERSION
        } else if has_v28_locks(scene) {
            GEOMETRY_LOCK_VERSION
        } else if !scene.render.is_default() {
            RENDER_SETTINGS_VERSION
        } else if has_v26_gradients(scene) {
            GRADIENT_VERSION
        } else if has_v25_textures(scene) {
            TEXTURES5_VERSION
        } else if has_v24_procedurals(scene) {
            MORE_PROCEDURALS_VERSION
        } else if has_v23_locator_options(scene) {
            LOCATOR_OPTIONS_VERSION
        } else if has_v22_procedurals(scene) {
            PROCEDURAL_VERSION
        } else if has_v21_shaders(scene) {
            SHADER_SETTINGS_VERSION
        } else if has_v20_cameras(scene) {
            CAMERA_PROPS_VERSION
        } else if has_v19_locators(scene) {
            TEXTURE_LOCATOR_VERSION
        } else if has_v18_groups(scene) {
            GROUPS_VERSION
        } else if has_v17_materials(scene) {
            SHADING_MODEL_VERSION
        } else if has_v16_items(scene) {
            ITEM_COMMON_VERSION
        } else if !scene.shader_tree.is_default() || !scene.materials.is_empty() {
            SPECULAR_AMOUNT_VERSION
        } else if scene.items.iter().any(|i| has_v13_sets(&i.mesh)) || has_cameras_lights(scene) {
            13
        } else if scene.smoothing != crate::shading::SmoothingSettings::default() {
            12
        } else if !scene.images.is_empty() || scene.items.iter().any(|i| i.backdrop.is_some()) {
            11
        } else if scene.items.iter().any(|i| i.subdivision.is_some()) {
            10
        } else if scene.items.iter().any(|i| i.mesh.has_polylines()) {
            9
        } else if scene
            .items
            .iter()
            .any(|i| !i.mesh.attributes.normal_map_order.is_empty())
        {
            8
        } else if scene
            .items
            .iter()
            .any(|i| i.mesh.attributes.has_typed_data())
        {
            7
        } else if scene.items.iter().any(|i| {
            !i.mesh.attributes.point_uvs.is_empty()
                || !i.mesh.attributes.corner_weights.is_empty()
                || !i.mesh.attributes.corner_colors.is_empty()
                || !i.mesh.attributes.extrude_edge_uses.is_empty()
        }) {
            6
        } else if scene.items.iter().any(|i| {
            !i.mesh.attributes.relative_morphs.is_empty()
                || !i.mesh.attributes.smoothing_labels.is_empty()
        }) {
            5
        } else {
            4
        },
        scene: scene.clone(),
    })
    .map_err(|e| e.to_string())?;
    Ok(data)
}
pub fn decode(bytes: &[u8]) -> Result<Scene> {
    match decode_document(bytes, false)? {
        (scene, None) => Ok(scene),
        (_, Some(_)) => {
            Err("This is an Operandi preset, not a scene; load it with File > Load Preset…".into())
        }
    }
}
/// A scene file or a preset file: the scene and the preset header, if any.
fn decode_document(
    bytes: &[u8],
    keep_broken_polygons: bool,
) -> Result<(Scene, Option<crate::preset::PresetHeader>)> {
    if bytes.len() as u64 > MAX_LOAD_BYTES {
        return Err(too_big());
    }
    let doc: Document = serde_json::from_slice(bytes).map_err(|e| format!("Invalid scene: {e}"))?;
    if doc.format != "Operandi" || !(1..=CURRENT_VERSION).contains(&doc.version) {
        return Err("Unsupported Operandi scene format/version; existing scene preserved".into());
    }
    if doc.version < MATERIAL_VALUE_OUTPUTS_VERSION && has_v52_material_values(&doc.scene) {
        return Err("Material-value Render Outputs require native version 52".into());
    }
    if doc.version < RENDER_OUTPUTS_D_VERSION && has_v55_outputs(&doc.scene) {
        return Err("These Render Outputs require native version 55".into());
    }
    if doc.version < ANIMATION_VERSION && has_v60_animation(&doc.scene) {
        return Err("Animation requires native version 60".into());
    }
    if doc.version < GUIDES_VERSION && has_v59_guides(&doc.scene) {
        return Err("Guides require native version 59".into());
    }
    if doc.version < REPLICATOR_VERSION && has_v58_replicators(&doc.scene) {
        return Err("Replicators require native version 58".into());
    }
    if doc.version < PATCH_POLYGON_VERSION && has_v57_patches(&doc.scene) {
        return Err("Live patches require native version 57".into());
    }
    if doc.version < NODE_VIEW_VERSION && has_v56_node_views(&doc.scene) {
        return Err("Material node views require native version 56".into());
    }
    if doc.version < MATERIALX_GRAPH_VERSION && has_v54_materialx_graphs(&doc.scene) {
        return Err("MaterialX graphs require native version 54".into());
    }
    if doc.version < RENDER_OUTPUTS_V2_VERSION && has_v53_outputs(&doc.scene) {
        return Err("These Render Outputs require native version 53".into());
    }
    if doc.version < MESH_INSTANCE_VERSION && has_v51_instances(&doc.scene) {
        return Err("Mesh Instances require native version 51".into());
    }
    if doc.version < SELECTION_OPS_VERSION && has_v50_selops(&doc.scene) {
        return Err("Mesh Stack selection operations require native version 50".into());
    }
    if doc.version < DOORS_VERSION && has_v49_doors(&doc.scene) {
        return Err("Kit items and scene time settings require native version 49".into());
    }
    if doc.version < UV_MAP_CURRENT_VERSION && has_v47_current_uv_map(&doc.scene) {
        return Err("A selected UV map requires native version 47".into());
    }
    if doc.version < DIRECT_CAUSTICS_VERSION && has_v46_direct_caustics(&doc.scene) {
        return Err("Direct Caustics settings require native version 46".into());
    }
    if doc.version < SOLID_SKETCH_VERSION && has_v45_solid_sketch(&doc.scene) {
        return Err("Solid Sketch data requires native version 45".into());
    }
    if doc.version < NODE_GRAPH_VERSION && has_v38_node_graphs(&doc.scene) {
        return Err("Node Materials require native version 38".into());
    }
    if doc.version < PRESET_CREDIT_VERSION
        && doc.preset.as_ref().is_some_and(|p| p.source.is_some())
    {
        return Err("Preset licence and credit require native version 37".into());
    }
    if doc.version < RENDER_ITEM_VERSION && doc.scene.render.has_render_item_rows() {
        return Err("Render item settings require native version 36".into());
    }
    if doc.version < EYE_HIDES_CHILDREN_VERSION && has_v35_eyes(&doc.scene) {
        return Err("An eye that hides its children requires native version 35".into());
    }
    if doc.version < ENVIRONMENT_VERSION && doc.scene.items.iter().any(|i| i.environment.is_some())
    {
        return Err("Environment items require native version 33".into());
    }
    if doc.version < SHADER_MENU_VERSION && has_v32_shader_menu(&doc.scene) {
        return Err("Shader layer instances and the Library require native version 32".into());
    }
    if doc.version < EDITOR_COLOR_VERSION && has_v31_editor_colors(&doc.scene) {
        return Err("Editor colors require native version 31".into());
    }
    if doc.version < GROUP_LOCATOR_VERSION && has_v30_locators(&doc.scene) {
        return Err("Group Locators require native version 30".into());
    }
    if doc.version < MULTI_ACTIVE_VERSION && has_v29_others(&doc.scene) {
        return Err("Selections on several active meshes require native version 29".into());
    }
    if doc.version < GEOMETRY_LOCK_VERSION && has_v28_locks(&doc.scene) {
        return Err("Locked geometry requires native version 28".into());
    }
    for item in &doc.scene.items {
        item.elements.validate(&item.mesh)?;
    }
    if doc.version < RENDER_SETTINGS_VERSION && !doc.scene.render.is_default() {
        return Err("Render settings require native version 27".into());
    }
    doc.scene.render.validate()?;
    if doc.version < GRADIENT_VERSION && has_v26_gradients(&doc.scene) {
        return Err("Gradient, Vertex Map, Variation, Process and Occlusion layers, group Scale, Render Outputs and the newer Material settings require native version 26".into());
    }
    if doc.version < TEXTURES5_VERSION && has_v25_textures(&doc.scene) {
        return Err(
            "Gabor Noise, Poisson Cells and these locator settings require native version 25"
                .into(),
        );
    }
    if doc.version < MORE_PROCEDURALS_VERSION && has_v24_procedurals(&doc.scene) {
        return Err("Cellular, Wood, Ripples and Weave require native version 24".into());
    }
    if doc.version < LOCATOR_OPTIONS_VERSION && has_v23_locator_options(&doc.scene) {
        // Stage 1 files hold the placeholder names, read through aliases.
        if doc.version < TEXTURE_LOCATOR_VERSION {
            return Err("Texture Locator options require native version 23".into());
        }
    }
    if doc.version < PROCEDURAL_VERSION && has_v22_procedurals(&doc.scene) {
        return Err("Procedural textures require native version 22".into());
    }
    if doc.version < SHADER_SETTINGS_VERSION && has_v21_shaders(&doc.scene) {
        return Err(
            "Shader control, light linking and fog settings require native version 21".into(),
        );
    }
    if doc.version < CAMERA_PROPS_VERSION && has_v20_cameras(&doc.scene) {
        return Err("Camera properties (depth of field, overlays, background images) require native version 20".into());
    }
    if doc.version < TEXTURE_LOCATOR_VERSION && has_v19_locators(&doc.scene) {
        return Err("Texture Locators require native version 19".into());
    }
    if doc.version < GROUPS_VERSION && has_v18_groups(&doc.scene) {
        return Err("Item groups and Light Linking require native version 18".into());
    }
    if doc.version < SHADING_MODEL_VERSION && has_v17_materials(&doc.scene) {
        return Err(
            "Material Shading Model and Match Specular settings require native version 17".into(),
        );
    }
    if doc.version < ITEM_COMMON_VERSION && has_v16_items(&doc.scene) {
        return Err("Light properties (portals, Tier 2 lights, Physical Sun, targets, item Display/Assembly) require native version 16".into());
    }
    if doc.version < 14 && !doc.scene.shader_tree.is_default() {
        return Err("Shader tree layers require native version 14".into());
    }
    if doc.version < 13 && doc.scene.items.iter().any(|i| has_v13_sets(&i.mesh)) {
        return Err("Edge selection sets and set groups require native version 13".into());
    }
    if doc.version < 13 && has_cameras_lights(&doc.scene) {
        return Err("Cameras and lights require native version 13".into());
    }
    if doc.version < 12 && doc.scene.smoothing != crate::shading::SmoothingSettings::default() {
        return Err("Smoothing settings require native version 12".into());
    }
    if doc.version < 11
        && (!doc.scene.images.is_empty() || doc.scene.items.iter().any(|i| i.backdrop.is_some()))
    {
        return Err("Backdrops require native version 11".into());
    }
    if doc.version < 10 && doc.scene.items.iter().any(|i| i.subdivision.is_some()) {
        return Err("Subdivision settings require native version 10".into());
    }
    if doc.version < 9 && doc.scene.items.iter().any(|i| i.mesh.has_polylines()) {
        return Err("Polyline semantics require native version 9".into());
    }
    if doc.version < 8
        && doc
            .scene
            .items
            .iter()
            .any(|i| !i.mesh.attributes.normal_map_order.is_empty())
    {
        return Err("Normal-map creation order requires native version 8".into());
    }
    if doc.version < 7
        && doc
            .scene
            .items
            .iter()
            .any(|i| i.mesh.attributes.has_typed_data())
    {
        return Err("Typed maps and named sets require native version 7".into());
    }
    if doc.version < 6
        && doc.scene.items.iter().any(|i| {
            !i.mesh.attributes.point_uvs.is_empty()
                || !i.mesh.attributes.corner_weights.is_empty()
                || !i.mesh.attributes.corner_colors.is_empty()
                || !i.mesh.attributes.extrude_edge_uses.is_empty()
        })
    {
        return Err("Extrude point/corner storage requires native version 6".into());
    }
    if doc.version < 5
        && doc.scene.items.iter().any(|i| {
            !i.mesh.attributes.relative_morphs.is_empty()
                || !i.mesh.attributes.smoothing_labels.is_empty()
        })
    {
        return Err("Relative morphs and smoothing labels require native version 5".into());
    }
    let mut scene = doc.scene;
    // First, so everything below reads the angles as they were saved.
    if doc.version < XYZ_FIRST_VERSION {
        scene.xyz_items_to_x_first();
    }
    scene.settle_environment_transforms();
    if doc.version < ENVIRONMENT_VERSION && has_cameras_lights(&scene) {
        scene.ensure_environment()?;
    }
    if doc.version < SPECULAR_AMOUNT_VERSION {
        scene.convert_specular_from_reflectance();
    }
    // Texture layers from before version 19 (or written without one) get
    // their locators.
    let mut next = scene.next_id;
    scene.give_layers_locators(&mut next)?;
    // Environment layers without a locator get one at identity (v43).
    scene.give_environment_layers_locators(&mut next)?;
    if doc.version < XYZ_FIRST_VERSION {
        scene.environment_light_probes_to_spherical();
    }
    // A scene written elsewhere may hold polygons with mixed winding or an
    // edge shared by three polygons without listing those edges. Record
    // them as the file importers do, rather than refusing to open it.
    for item in &mut scene.items {
        if keep_broken_polygons {
            crate::mesh_cleanup::keep_broken_polygons(&mut item.mesh);
        }
        if item.mesh.attributes.extrude_edge_uses.is_empty() {
            let exceptions = crate::interchange::edge_exceptions(&item.mesh);
            if !exceptions.is_empty() {
                item.mesh.attributes.extrude_edge_uses = exceptions;
            }
        }
        // Checked now, while the edge uses just found are still remembered
        // (the topology memo keeps only a few meshes); `Scene::validate`
        // below then skips this mesh.
        item.mesh.validate()?;
    }
    // Instances take their sources' values for every setting they don't
    // override (shader_instances.rs).
    scene.shader_tree.sync_instances(None)?;
    // Mesh Instances show their sources' meshes (mesh_instance.rs).
    scene.sync_instances(None);
    // A file whose op links loop opens with the closing ops turned off
    // (SAFE-07).
    crate::meshops::break_loops(&mut scene);
    scene.validate()?;
    // TK-42: a reopened scene shows its curves at the saved time; edits
    // that were never keyed don't survive closing it.
    let now = scene.time.current;
    crate::animation::pose(&mut scene, now);
    Ok((scene, doc.preset))
}
/// Read a scene file. `project:` references are resolved against the
/// project the file is in (`project_paths::resolve`), so in memory every
/// path is absolute again.
pub fn load(path: &Path) -> Result<Scene> {
    let mut scene = decode(&read_limited(path)?)?;
    crate::project_paths::resolve(&mut scene, path);
    Ok(scene)
}
/// `load` for the MCP test bridge (BRQ-60 checks): a scene whose meshes
/// hold 1- or 2-point, repeated-point or zero-area polygons opens as-is,
/// in their order and with their points, for Mesh Cleanup to fix
/// (`mesh_cleanup::keep_broken_polygons`). Never used by File > Open.
pub fn load_keeping_broken_polygons(path: &Path) -> Result<Scene> {
    let (mut scene, preset) = decode_document(&read_limited(path)?, true)?;
    if preset.is_some() {
        return Err("This is an Operandi preset, not a scene".into());
    }
    crate::project_paths::resolve(&mut scene, path);
    Ok(scene)
}
/// `load`, and when that refuses the file, `load_keeping_broken_polygons`
/// (the MCP test bridge; the first refusal is the one reported).
pub fn load_for_bridge(path: &Path) -> Result<Scene> {
    load(path).or_else(|strict| load_keeping_broken_polygons(path).map_err(|_| strict))
}
fn read_limited(path: &Path) -> Result<Vec<u8>> {
    let file = File::open(path).map_err(|e| e.to_string())?;
    let size = file.metadata().map_err(|e| e.to_string())?.len();
    if size > MAX_LOAD_BYTES {
        return Err(too_big());
    }
    let mut data = Vec::with_capacity(size as usize);
    file.take(MAX_LOAD_BYTES + 1)
        .read_to_end(&mut data)
        .map_err(|e| e.to_string())?;
    Ok(data)
}
fn too_big() -> String {
    format!(
        "The file is larger than {} GiB, more than any scene this build can open",
        MAX_LOAD_BYTES >> 30
    )
}
/// How much of an existing destination `guard_destination` reads.
const HEADER_BYTES: u64 = 64 * 1024;
/// The `format` and `version` among the leading top-level keys of a JSON
/// object, read from the start of a file. Scanning stops at the first
/// value that isn't a plain string or number (scene files write both keys
/// first), or wherever the prefix stops making sense.
fn leading_header(prefix: &[u8]) -> (Option<String>, Option<u64>) {
    let text = match std::str::from_utf8(prefix) {
        Ok(t) => t,
        // The prefix may end inside a multi-byte character.
        Err(e) => std::str::from_utf8(&prefix[..e.valid_up_to()]).unwrap_or(""),
    };
    let mut rest = text.trim_start();
    let (mut format, mut version) = (None, None);
    let Some(r) = rest.strip_prefix('{') else {
        return (None, None);
    };
    rest = r;
    // A JSON string at the start of `s`: its value and what follows it.
    fn string(s: &str) -> Option<(String, &str)> {
        let body = s.strip_prefix('"')?;
        let mut escaped = false;
        for (i, c) in body.char_indices() {
            match c {
                '\\' if !escaped => escaped = true,
                '"' if !escaped => {
                    let value = serde_json::from_str(&s[..i + 2]).ok()?;
                    return Some((value, &body[i + 1..]));
                }
                _ => escaped = false,
            }
        }
        None
    }
    for _ in 0..16 {
        let Some((key, r)) = string(rest.trim_start()) else {
            break;
        };
        let Some(r) = r.trim_start().strip_prefix(':') else {
            break;
        };
        let r = r.trim_start();
        if let Some((value, after)) = string(r) {
            if key == "format" {
                format = Some(value);
            }
            rest = after;
        } else {
            let end = r
                .find(|c: char| !(c.is_ascii_digit() || "+-.eE".contains(c)))
                .unwrap_or(r.len());
            if end == 0 {
                break;
            }
            if key == "version" {
                version = r[..end].parse().ok();
            }
            rest = &r[end..];
        }
        if format.is_some() && version.is_some() {
            break;
        }
        let Some(r) = rest.trim_start().strip_prefix(',') else {
            break;
        };
        rest = r;
    }
    (format, version)
}
/// Read a preset file (PB-50). A scene file is refused: scenes aren't
/// presets (PB-31 opens them as their own scene instead).
pub fn load_preset(path: &Path) -> Result<crate::preset::Preset> {
    let bytes = read_limited(path)?;
    match decode_document(&bytes, false)? {
        (mut scene, Some(header)) => {
            // A picture named relative to the preset (a content pack that
            // keeps its maps next to its presets, convert.rs) is found from
            // the preset's folder wherever the pack was copied.
            if let Some(folder) = path.parent() {
                for image in &mut scene.images {
                    let p = Path::new(&image.path);
                    if !image.path.is_empty() && p.is_relative() {
                        image.path = folder.join(p).to_string_lossy().into_owned();
                    }
                }
            }
            Ok(crate::preset::Preset { header, scene })
        }
        (_, None) => Err("That's a scene file, not a preset; open it with File > Open…".into()),
    }
}
/// Read only a preset file's header (kind and thumbnail) without reading
/// its scene, for the preset browser's listing (CD-10, PB-106). Operandi
/// writes the header before the scene, so reading stops there; a file
/// written some other way is read whole.
pub fn read_preset_header(path: &Path) -> Result<crate::preset::PresetHeader> {
    #[derive(Deserialize)]
    struct HeaderOnly {
        format: String,
        version: u32,
        #[serde(default)]
        preset: Option<crate::preset::PresetHeader>,
    }
    const PREFIX: u64 = 8 << 20;
    let file = File::open(path).map_err(|e| e.to_string())?;
    let mut prefix = Vec::new();
    file.take(PREFIX)
        .read_to_end(&mut prefix)
        .map_err(|e| e.to_string())?;
    let marker = b"\n  \"scene\":";
    if let Some(at) = prefix.windows(marker.len()).position(|w| w == marker) {
        let mut head = prefix[..at].to_vec();
        while head
            .last()
            .is_some_and(|c| c.is_ascii_whitespace() || *c == b',')
        {
            head.pop();
        }
        head.extend_from_slice(b"\n}");
        if let Ok(doc) = serde_json::from_slice::<HeaderOnly>(&head) {
            if doc.format != "Operandi" || !(1..=CURRENT_VERSION).contains(&doc.version) {
                return Err("Unsupported Operandi preset format/version".into());
            }
            return doc
                .preset
                .ok_or_else(|| "That's a scene file, not a preset".into());
        }
    }
    load_preset(path).map(|p| p.header)
}
/// Put `png` into a preset file's thumbnail slot (PB-52, PB-112's Replace
/// thumbnail), keeping everything else. `None` clears it.
pub fn set_preset_thumbnail(path: &Path, png: Option<&[u8]>) -> Result<()> {
    use base64::Engine;
    let mut preset = load_preset(path)?;
    preset.header.thumbnail = png.map(|p| base64::engine::general_purpose::STANDARD.encode(p));
    save_preset(path, &preset)
}
/// The PNG bytes in a preset header's thumbnail slot, if any.
pub fn preset_thumbnail(header: &crate::preset::PresetHeader) -> Option<Vec<u8>> {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD
        .decode(header.thumbnail.as_deref()?)
        .ok()
}
/// A preset file's bytes, as [`save_preset`] writes them.
pub fn encode_preset(preset: &crate::preset::Preset) -> Result<Vec<u8>> {
    encode_document(&preset.scene, Some(&preset.header))
}
/// Write a preset file, as atomically as a scene (PB-40..42). The path
/// must end in `.oppreset`.
pub fn save_preset(path: &Path, preset: &crate::preset::Preset) -> Result<()> {
    // An environment preset is .openv; .oppreset stays accepted for it,
    // as builds before .openv wrote.
    let want = crate::preset::extension_for(preset.header.kind);
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
    let fits = ext.eq_ignore_ascii_case(want)
        || (preset.header.kind == crate::preset::PresetKind::Environment
            && ext.eq_ignore_ascii_case(crate::preset::PRESET_EXTENSION));
    if !fits {
        return Err(format!("A preset of this kind needs a .{want} file name"));
    }
    let relative = relative_preset_images(path, preset);
    let scene = relative.as_ref().unwrap_or(&preset.scene);
    let data = encode_document(scene, Some(&preset.header))?;
    write_atomic(path, &data)
}
/// `preset`'s scene with every picture inside the preset file's folder
/// named relative to that folder, or `None` when nothing changes.
/// [`load_preset`] turns such names back into full paths, so a preset
/// rewritten after a load (a new thumbnail, a re-render) stays portable
/// with its folder instead of pinning this machine's paths. Pictures
/// outside the folder keep their paths.
fn relative_preset_images(path: &Path, preset: &crate::preset::Preset) -> Option<Scene> {
    let folder = path.parent().filter(|f| !f.as_os_str().is_empty())?;
    let rel = |image: &str| {
        Path::new(image)
            .strip_prefix(folder)
            .ok()
            .filter(|r| !r.as_os_str().is_empty())
            .map(|r| r.to_string_lossy().into_owned())
    };
    if !preset.scene.images.iter().any(|i| rel(&i.path).is_some()) {
        return None;
    }
    let mut scene = preset.scene.clone();
    for image in &mut scene.images {
        if let Some(r) = rel(&image.path) {
            image.path = r;
        }
    }
    Some(scene)
}
/// Same-directory create-new temporary, flush+sync, atomic rename. Errors retain
/// the previous destination. No existing temporary is overwritten.
///
/// Files inside the project enclosing `path` are written project-relative
/// (`project_paths`); the scene in memory is left as it is.
pub fn save(path: &Path, scene: &Scene) -> Result<()> {
    guard_destination(path, scene)?;
    let relative = crate::project_paths::project_root_for(path)
        .and_then(|root| crate::project_paths::to_project_relative(scene, &root));
    let data = encode(relative.as_ref().unwrap_or(scene))?;
    write_atomic(path, &data)
}
pub(crate) fn write_atomic(path: &Path, data: &[u8]) -> Result<()> {
    let name = path
        .file_name()
        .ok_or("Choose a scene file path")?
        .to_string_lossy();
    let temp = path.with_file_name(format!(".{name}.{}.tmp", std::process::id()));
    let mut created = false;
    let result = (|| {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp)
            .map_err(|e| format!("Cannot create save temporary: {e}"))?;
        created = true;
        file.write_all(data)
            .and_then(|_| file.sync_all())
            .map_err(|e| format!("Cannot write scene: {e}"))?;
        fs::rename(&temp, path).map_err(|e| format!("Cannot replace scene: {e}"))?;
        Ok(())
    })();
    if result.is_err() && created {
        let _ = fs::remove_file(&temp);
    }
    result
}

/// Check extension and source identity before creating any output temporary.
pub fn guard_destination(path: &Path, scene: &Scene) -> Result<()> {
    if !path
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("opscene"))
    {
        return Err("Save requires a separate .opscene path; use File > Export for LXO".into());
    }
    if path.exists() {
        let file = File::open(path).map_err(|e| format!("Cannot inspect destination: {e}"))?;
        // Only the header is read: a scene file can be hundreds of MB, and
        // parsing all of it here cost more time and memory than the save.
        let mut prefix = Vec::new();
        file.take(HEADER_BYTES)
            .read_to_end(&mut prefix)
            .map_err(|e| format!("Cannot inspect destination: {e}"))?;
        // Preserve the established replacement workflow for damaged/non-native
        // destinations; only recognized native headers establish a version guard.
        let (format, version) = leading_header(&prefix);
        if format.as_deref() == Some("Operandi")
            && !version.is_some_and(|v| (1..=u64::from(CURRENT_VERSION)).contains(&v))
        {
            return Err("Unsupported destination native version; existing file preserved".into());
        }
    }
    if let Some(source) = &scene.import_source {
        let original = Path::new(&source.path);
        if path == original
            || (path.canonicalize().ok().is_some()
                && path.canonicalize().ok() == original.canonicalize().ok())
        {
            return Err(
                "The imported LXO source is read-only; choose a separate native file".into(),
            );
        }
        #[cfg(unix)]
        if let Ok(meta) = path.metadata() {
            use std::os::unix::fs::MetadataExt;
            if source.device == Some(meta.dev()) && source.inode == Some(meta.ino()) {
                return Err("Destination aliases the read-only LXO source".into());
            }
        }
    }
    Ok(())
}
