//! The icon studio: Operandi's own UI icons rendered from small 3D scenes
//! with Cycles, so every rendered icon shares one camera, one light rig
//! and one set of materials.
//!
//! The studio scene ([`studio_scene`], saved as
//! `tools/icon-studio/studio.opscene`) holds what every icon shares:
//!
//! - an orthographic camera looking down the (1, 1, 1) diagonal at the
//!   origin, the same 30° isometric view as the vector icon generator
//!   (`tools/operandi-icons`), scaled so a 1 m cube centred on the origin
//!   fills the picture as the generator's unit cube fills its 32 px canvas;
//! - a key light from above and to the left and a soft 4-colour sky, tuned
//!   so a clay cube's top, left and right faces come out near the
//!   generator's top, left and right greys;
//! - one material per [`MATERIALS`] entry, picked by polygon tag, in the
//!   generator's palette (yellow-orange is always "selected", cyan new
//!   geometry, red/green/blue X/Y/Z).
//!
//! An icon scene is a copy of the studio with geometry added (File > Save
//! As in Operandi). At render time only its geometry is used:
//! [`render_scene`] puts the icon's items into a fresh copy of the studio,
//! so the studio's camera, lights, sky and materials always win and an
//! icon can't drift from the shared look by editing its own copy.
use crate::shader_tree::{GroupMask, MaterialChannels, NewLayer, NodeKind, TagMask, TagType};
use crate::{Mesh, Result, Scene, Transform};
use glam::DVec3;

/// The pixel sizes every icon is rendered at, each also at 2× for
/// high-density screens.
pub const SIZES: [u32; 4] = [16, 20, 24, 32];

/// Half the picture's height (and width) in metres: a 1 m cube seen along
/// the diagonal spans 2·√(2/3) m from top to bottom, and the generator draws
/// that span 25.8 px tall on its 32 px canvas (`UNIT` = 12.9 px per edge).
pub const HALF_SIZE: f64 = 1.0127;

/// How far from the origin the camera sits along the diagonal, metres.
/// Anything outside the icon box is far closer, so nothing is clipped.
pub const CAMERA_DISTANCE: f64 = 10.;

/// The polygon tag of the material untagged polygons fall back to.
pub const CLAY: &str = "Clay";

/// One shared material: the polygon tag that picks it and its colour as
/// sRGB hex (the generator's palette, `conventions.py`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StudioMaterial {
    pub tag: &'static str,
    pub hex: u32,
    pub roughness: f64,
    pub metallic: f64,
    /// Glow strength (Luminous Intensity); 0 for everything but Glow.
    pub luminous: f64,
}

/// The shared materials, the Base Material (Clay) first. Clay is a light
/// neutral that the light rig shades into the generator's face greys.
pub const MATERIALS: [StudioMaterial; 8] = [
    StudioMaterial {
        tag: CLAY,
        hex: 0xE4E9EE,
        roughness: 0.6,
        metallic: 0.,
        luminous: 0.,
    },
    StudioMaterial {
        tag: "Selected",
        hex: 0xFFD470,
        roughness: 0.5,
        metallic: 0.,
        luminous: 0.,
    },
    StudioMaterial {
        tag: "New",
        hex: 0x56B4D3,
        roughness: 0.5,
        metallic: 0.,
        luminous: 0.,
    },
    StudioMaterial {
        tag: "Axis X",
        hex: 0xD85C58,
        roughness: 0.5,
        metallic: 0.,
        luminous: 0.,
    },
    StudioMaterial {
        tag: "Axis Y",
        hex: 0x4EAA73,
        roughness: 0.5,
        metallic: 0.,
        luminous: 0.,
    },
    StudioMaterial {
        tag: "Axis Z",
        hex: 0x4C8CD6,
        roughness: 0.5,
        metallic: 0.,
        luminous: 0.,
    },
    // Lit things (lights, lamps): a warm white that glows, so it reads as
    // light rather than as "selected" yellow-orange (Jason, 2026-11-20).
    StudioMaterial {
        tag: "Glow",
        hex: 0xFFE4A0,
        roughness: 0.5,
        metallic: 0.,
        luminous: 0.45,
    },
    StudioMaterial {
        tag: "Ink",
        hex: 0x263442,
        roughness: 0.7,
        metallic: 0.,
        luminous: 0.,
    },
];

/// The ink colour the renderer outlines silhouettes with (sRGB hex), for
/// the set used on light interface colours.
pub const INK: u32 = 0x263442;
/// The light outline, for the set used on dark interface colours: the
/// generator's foreground on its dark previews.
pub const LIGHT_INK: u32 = 0xDCE3E9;

/// The direction toward the key light: mostly from above, tipped toward
/// +Z so the left (+Z) face is lit less than the top and the right (+X)
/// face sees only the sky, as in the generator's shading.
pub fn key_direction() -> DVec3 {
    DVec3::new(-0.08, 0.95, 0.37).normalize()
}

/// The key light's irradiance, W/m², and colour (linear). Provisional:
/// tuned so a clay cube's faces land within a few levels of the
/// generator's face greys (see the icon studio README).
pub const KEY_INTENSITY: f64 = 2.54;
pub const KEY_COLOR: [f64; 3] = [1.0, 0.97, 0.86];
/// The key's angular size in degrees: soft shadow edges.
pub const KEY_SPREAD_DEGREES: f64 = 3.;

/// Samples per pixel the studio renders with. Icons are tiny, so plenty
/// of samples are cheap, and they anti-alias edges at render time.
pub const SAMPLES: u32 = 512;

/// An sRGB hex colour as linear RGB.
pub fn linear_from_hex(hex: u32) -> [f64; 3] {
    let channel = |shift: u32| {
        let s = f64::from((hex >> shift) & 0xFF) / 255.;
        if s <= 0.04045 {
            s / 12.92
        } else {
            ((s + 0.055) / 1.055).powf(2.4)
        }
    };
    [channel(16), channel(8), channel(0)]
}

fn set_material(m: &mut MaterialChannels, look: &StudioMaterial) {
    m.diffuse_color = linear_from_hex(look.hex);
    m.roughness = look.roughness;
    m.metallic = look.metallic;
    if look.luminous > 0. {
        m.luminous_color = linear_from_hex(look.hex);
        m.luminous_intensity = look.luminous;
    }
    m.smoothing = 1.;
    m.smoothing_angle = 40.;
}

/// Writes `look` onto the shader tree's Base Material.
fn set_base_material(scene: &mut Scene, look: &StudioMaterial) -> Result<()> {
    let base = scene
        .shader_tree
        .nodes
        .iter()
        .find(|n| matches!(n.kind, NodeKind::Material(_)))
        .ok_or("The studio's shader tree has no Base Material")?;
    let mut node = base.clone();
    if let NodeKind::Material(m) = &mut node.kind {
        set_material(m, look);
    }
    scene.shader_tree.replace(node)
}

/// A material group for polygons tagged `look.tag`, holding one material.
fn add_tag_material(scene: &mut Scene, look: &StudioMaterial) -> Result<()> {
    let group = scene.shader_tree.add(NewLayer::Group, None)?;
    let mut node = scene.shader_tree.get(group).ok_or("no group")?.clone();
    node.name = look.tag.into();
    node.kind = NodeKind::Group(GroupMask {
        tag_type: TagType::Material,
        tag: TagMask::Name(look.tag.into()),
        ..GroupMask::default()
    });
    scene.shader_tree.replace(node)?;
    let material = scene.shader_tree.add(NewLayer::Material, Some(group))?;
    let mut node = scene
        .shader_tree
        .get(material)
        .ok_or("no material")?
        .clone();
    node.parent = Some(group);
    node.name = format!("{} Material", look.tag);
    if let NodeKind::Material(m) = &mut node.kind {
        set_material(m, look);
    }
    scene.shader_tree.replace(node)
}

/// The camera's settings: orthographic, framing [`HALF_SIZE`] metres
/// either side of the centre on a square picture.
pub fn camera_settings() -> crate::camera_light::CameraSettings {
    let mut camera = crate::camera_light::new_scene_camera_settings();
    camera.projection = crate::camera_light::Projection::Orthographic;
    let (_, half_tan) = camera.half_tangents(1.);
    // Orthographic size is the frame a perspective lens would see at the
    // Target Distance (CL-Q02), so the distance sets the size.
    camera.target_distance = HALF_SIZE / half_tan;
    camera.focus_distance = CAMERA_DISTANCE;
    camera.clipping = false;
    camera
}

fn aimed(eye: DVec3, target: DVec3) -> Transform {
    Transform {
        translation: eye,
        rotation: crate::camera_light::look_at_rotation(eye, target),
        ..Default::default()
    }
}

/// The studio: camera, key light, sky and shared materials, no geometry.
pub fn studio_scene() -> Result<Scene> {
    let mut scene = Scene::default();
    let eye = DVec3::ONE.normalize() * CAMERA_DISTANCE;
    let camera = scene.add_camera(camera_settings(), aimed(eye, DVec3::ZERO))?;
    scene.set_render_camera(camera)?;
    if let Some(item) = scene.items.iter_mut().find(|i| i.id == camera) {
        item.name = "Icon Camera".into();
    }

    let key = crate::camera_light::LightSettings {
        intensity: KEY_INTENSITY,
        color: KEY_COLOR,
        spread_angle: KEY_SPREAD_DEGREES.to_radians(),
        ..Default::default()
    };
    let key_at = key_direction() * CAMERA_DISTANCE;
    let light = scene.add_light(key, aimed(key_at, DVec3::ZERO))?;
    if let Some(item) = scene.items.iter_mut().find(|i| i.id == light) {
        item.name = "Key Light".into();
    }

    let sky = scene.add_environment()?;
    if let Some(env) = scene
        .items
        .iter_mut()
        .find(|i| i.id == sky)
        .and_then(|i| i.environment.as_mut())
    {
        // A soft, slightly cool sky: the faces the key doesn't reach still
        // read, cooler than the lit ones. The camera never sees it
        // (renders are transparent).
        let m = &mut env.material.settings;
        // About half the fill the first, softer rig had, with a brighter
        // key and crisper shadows (Jason chose the more dramatic look,
        // 2026-11-20).
        m.zenith = [0.194, 0.225, 0.252];
        m.sky = [0.167, 0.194, 0.239];
        m.ground = [0.117, 0.140, 0.171];
        m.nadir = [0.081, 0.095, 0.122];
        m.sky_exponent = 2.;
        m.ground_exponent = 2.;
    }
    if let Some(item) = scene.items.iter_mut().find(|i| i.id == sky) {
        item.name = "Icon Sky".into();
    }

    set_base_material(&mut scene, &MATERIALS[0])?;
    for look in &MATERIALS[1..] {
        add_tag_material(&mut scene, look)?;
    }

    scene.render.frame = [32, 32];
    scene.render.samples = SAMPLES;
    scene.render.denoise = false;
    Ok(scene)
}

/// A sample icon the studio can write, to prove the pipeline.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sample {
    /// The cube primitive: a clay cube.
    Cube,
    /// Polygon selection: a clay cube with its top face selected.
    SelectedFace,
    /// The sphere primitive.
    Sphere,
    /// The cylinder primitive.
    Cylinder,
    /// Edge Bevel: a cube with its top-left edge cut, the new face cyan.
    EdgeBevel,
    /// A point light: a glowing bulb on a dark screw base.
    PointLight,
    Cone,
    Torus,
    Capsule,
    Ellipsoid,
    /// Vertex selection mode: the cube's visible corners selected.
    SelectedVertices,
    /// Edge selection mode: the cube's visible edges selected.
    SelectedEdges,
    /// Polygon Bevel: the cube's upper right edge cut, the cut cyan.
    Bevel,
    /// Extrude: a box whose top grew upward, new sides cyan, top selected.
    Extrude,
    /// Spike: a cyan pyramid rising from a box's top.
    Spike,
    /// Curve: a cyan curve through four light points.
    Curve,
    /// Bezier Curve: a cyan curve with a selected tangent handle.
    BezierCurve,
    /// B-Spline: a cyan curve pulled toward a cage of light points.
    BSpline,
    /// Pen: a flat polygon with its points, the newest edge cyan.
    Pen,
    /// Tube: a pipe swept through cyan points.
    Tube,
    /// Arc: a cyan quarter arc of edges and points round a centre.
    Arc,
    /// Bridge: two blocks joined by a cyan bridge.
    Bridge,
    /// Thicken: a thin plate whose new sides are cyan.
    Thicken,
    /// Smooth Shift: top and front pushed out together, cyan walls between.
    SmoothShift,
    /// Edge Extrude: the top right edge pulled out into a cyan ridge.
    EdgeExtrude,
    /// Edge Chamfer: the front edge cut into one flat cyan strip.
    EdgeChamfer,
    /// Vertex Bevel: the upper right corner cut off by a cyan triangle.
    VertexBevel,
    /// Vertex Extrude: the upper right corner pulled out into a cyan point.
    VertexExtrude,
    /// Loop Slice: three evenly spaced cyan loops round the cube.
    LoopSlice,
    /// Inset: a selected inner top face joined to the cube's corners.
    Inset,
    /// Triple: the cube's faces split into triangles by cyan diagonals.
    Triple,
    /// A mesh item: a small cube.
    MeshItem,
    /// A scene: a cube standing on a ground plate.
    Scene,
    /// Copy: a cube and its copy with a cyan top.
    Copy,
    /// Clone: a cube and two copies with cyan tops in a line.
    Clone,
    /// Array: a cube and its copies with cyan tops in a three by three grid.
    Array,
    /// Radial Array: a cube and its copies with cyan tops round a circle.
    RadialArray,
    /// Dicer: the cube cut into four blocks, pulled apart.
    Dicer,
    /// Julienne: the cube cut into three slabs, pulled apart.
    Julienne,
    /// Taper: a column narrowing toward its top.
    Taper,
    /// Bulge: a column swollen round its middle.
    Bulge,
    /// Flare: a column opening out toward its top.
    Flare,
    /// Twist: a square column wrung about its axis.
    Twist,
    /// Bend: a straight bar bent up through a quarter turn.
    Bend,
    /// Shear: a block whose top slides along a red X arrow.
    Shear,
    /// Text: an extruded capital T.
    Text,
    /// Twist Bevel: a tapering cyan tube curling up from the top face, its tip selected.
    TwistBevel,
    /// Poly Haul: the selected top face with an arrow pushing it out.
    PolyHaul,
    /// Shift: the selected top and front faces moved out on their own, the box left open.
    Shift,
    /// Scale Offset: the selected top face shrunk, the sides leaning in to follow.
    ScaleOffset,
    /// Scale Offset Detach: a smaller selected face lifted off the top on new cyan points.
    ScaleOffsetDetach,
    /// Add Loop: one cyan edge loop round the cube.
    AddLoop,
    /// Add Point: a new cyan point on the cube's front top edge.
    AddPoint,
    /// Edge Extend: a plate extended past its selected border edge by a cyan row.
    EdgeExtend,
    /// Edge Crease: the selected front edge with cyan sharpness marks.
    EdgeCrease,
    /// Vertex Tear: the upper right corner torn off and lifted, selected.
    VertexTear,
    /// Merge: two blocks joined at a cyan seam, arrows pointing in.
    Merge,
    /// Unmerge: a block split in two, arrows pointing apart.
    Unmerge,
    /// Ground Align: a selected block over its cyan footprint on the ground.
    GroundAlign,
    /// Mirror: a step block and its cyan-topped mirror image either side of a cyan plane.
    Mirror,
    /// Boolean: a cyan ball sunk through the cube's top.
    Boolean,
    /// Sketch Extrude: a selected square profile and a cyan path to sweep along.
    SketchExtrude,
    /// Curve Extrude: a selected square profile and a cyan curve with knots.
    CurveExtrude,
    /// Bezier Extrude: a selected profile and a cyan Bezier curve with end handles.
    BezierExtrude,
    /// B-Spline Extrude: a selected profile and a cyan B-spline in its cage.
    BsplineExtrude,
    /// Pen Extrude: a selected profile and a cyan path of straight segments.
    PenExtrude,
    /// Curve Clone: a cube and cyan-topped copies along a curve with knots.
    CurveClone,
    /// Bezier Clone: cubes along a Bezier curve with end handles.
    BezierClone,
    /// B-Spline Clone: cubes along a B-spline in its cage.
    BsplineClone,
    /// Pen Clone: cubes along straight segments between knots.
    PenClone,
    /// Scatter Clone: cubes dropped at loose spots on a flat patch.
    ScatterClone,
    /// Chamfer: the upper right edge rounded into cyan rows.
    Chamfer,
    /// Edge Slide: a selected edge on the top with a two-way arrow.
    EdgeSlide,
    /// Edge Relax: a jagged chain smoothed into a cyan curve.
    EdgeRelax,
    /// Vertex Merge: three selected points drawn into one.
    VertexMerge,
    /// Vertex Weld: a selected point moving onto the cube's corner.
    VertexWeld,
    /// Vertex Slide: a selected point on an edge with a two-way arrow.
    VertexSlide,
    /// Detriangulate: faint diagonals fading from the cube's faces.
    Detriangulate,
    /// Symmetrize: one half kept, the other mirrored from it in cyan, welded
    /// along a selected seam.
    Symmetrize,
    /// Topology Pen: a flat strip of quads with a new cyan quad pulled out
    /// of its end edge, the two new points as dots.
    TopologyPen,
    /// Link off (an X/Y/Z group's link button, UIC-56): a broken chain,
    /// two links pulled apart.
    LinkIndependent,
    /// Link: same value. Two equal links joined.
    LinkCopy,
    /// Link: scale together. A big link joined to a small one.
    LinkProportional,
    /// Link: add together. Two equal links joined, with a cyan plus.
    LinkRelative,
    /// Fix Black Dots: a dark pinch dot on a plate, ringed in cyan.
    FixBlackDots,
    /// Edge Zip: the rows either side of a selected loop pulled onto it.
    EdgeZip,
    /// Diagonal Re-Direct: two selected quads meeting at a corner, a cyan diagonal through it.
    DiagonalRedirect,
    /// Contour: a cyan ring loop laid round a background ball.
    Contour,
    /// Mesh Paint: cyan copies brushed onto a plate inside a selected brush ring.
    MeshPaint,
    /// Mesh Paint Instance: a selected source above, linked to the cyan copies on the plate.
    MeshPaintInstance,
    /// Clone as a Replicator: the copies sit on dark points (a point cloud).
    ReplicaClone,
    /// CurveClone as a Replicator: the copies sit on dark points (a point cloud).
    CurveReplica,
    /// BezierClone as a Replicator: the copies sit on dark points (a point cloud).
    BezierReplica,
    /// BsplineClone as a Replicator: the copies sit on dark points (a point cloud).
    BsplineReplica,
    /// PenClone as a Replicator: the copies sit on dark points (a point cloud).
    PenReplicate,
    /// ScatterClone as a Replicator: the copies sit on dark points (a point cloud).
    ReplicaScatter,
    /// Array as a Replicator: the copies sit on dark points (a point cloud).
    ReplicaArray,
    /// RadialArray as a Replicator: the copies sit on dark points (a point cloud).
    ReplicaRadialArray,
    /// Mirror as a Replicator: the copies sit on dark points (a point cloud).
    ReplicaMirror,
    /// MeshPaint as a Replicator: the copies sit on dark points (a point cloud).
    MeshPaintReplica,
    /// Clone as instances: the copies linked to the source by dashed lines.
    InstanceClone,
    /// CurveClone as instances: the copies linked to the source by dashed lines.
    CurveInstance,
    /// BezierClone as instances: the copies linked to the source by dashed lines.
    BezierInstance,
    /// BsplineClone as instances: the copies linked to the source by dashed lines.
    BsplineInstance,
    /// PenClone as instances: the copies linked to the source by dashed lines.
    PenInstance,
    /// ScatterClone as instances: the copies linked to the source by dashed lines.
    InstanceScatter,
    /// Array as instances: the copies linked to the source by dashed lines.
    InstanceArray,
    /// RadialArray as instances: the copies linked to the source by dashed lines.
    InstanceRadialArray,
    /// Mirror as instances: the copies linked to the source by dashed lines.
    InstanceMirror,
    /// Vertex Normal Tools: a ball with cyan normal arrows.
    VertexNormalTools,
    /// Recalculate Vertex Normals: a cube with a cyan normal out of each face.
    RecalculateNormals,
    /// Harden: a selected hard edge, its two faces' normals split apart.
    HardenNormals,
    /// Soften: a rounded edge with one cyan normal across it.
    SoftenNormals,
    /// Select Hard Edges: a cube with its hard edges selected.
    SelectHardEdges,
    /// Transfer Normals: a source ball's normals copied onto a plate.
    TransferNormals,
    /// Spin Polygon Vertex Indices: a selected quad with a cyan turning arrow.
    SpinVertexIndices,
    /// Guides: light cyan guide lines across a plate, a point snapped to one.
    Guides,
    /// Timeline: a clock face.
    TimelineClock,
    /// First Frame: a bar and a triangle pointing to it.
    FirstFrame,
    /// Last Frame: a triangle pointing to a bar.
    LastFrame,
    /// Previous Frame: a small triangle stepping left off a frame tile.
    PreviousFrame,
    /// Next Frame: a small triangle stepping right off a frame tile.
    NextFrame,
    /// Play: a triangle.
    Play,
    /// Stop: a square.
    Stop,
    /// Previous Key: a triangle pointing left to a cyan key.
    PreviousKey,
    /// Next Key: a triangle pointing right to a cyan key.
    NextKey,
    /// First Key: a bar beside a cyan key.
    FirstKey,
    /// Last Key: a cyan key beside a bar.
    LastKey,
    /// Key Selected Channels: a cyan key on three channel lines.
    KeyChannels,
    /// Key Selected Items: a cyan key over a cube.
    KeyItems,
    /// Insert Frame: a cyan plus over a frame tile.
    InsertFrame,
    /// Remove Frame: a dark minus over a frame tile.
    RemoveFrame,
    /// Apply: a cyan key with a cyan tick.
    ApplyKeys,
    /// Discard: a cyan key with a dark cross.
    DiscardKeys,
    /// Auto Key: a cyan key ringed by a turning arrow.
    AutoKey,
    /// Graph Editor: a dark curve with cyan keys on a tile.
    GraphEditor,
    /// Key Keeping Shape: a new cyan key set on a curve that keeps its shape.
    KeyKeepingShape,
    /// Break: a key whose two selected handles point apart.
    BreakTangents,
    /// Unify: a key whose selected handles line up.
    UnifyTangents,
    /// Pre behaviour: a curve repeated before its first key.
    PreBehaviour,
    /// Post behaviour: a curve repeated after its last key.
    PostBehaviour,
    /// Cycle: a wave with a turning arrow.
    CycleCurve,
    /// Shadow copy: a curve and its grey shadow.
    ShadowCopy,
    /// Shadow swap: a curve and its shadow with a two-way arrow.
    ShadowSwap,
    /// Normalised: a wave held between two bars.
    Normalised,
    /// Stacked: three curves one above another.
    Stacked,
    /// Isolate: one cyan curve among grey ones.
    IsolateCurve,
    /// Velocity: a curve with an arrow along its slope.
    Velocity,
    /// Link Related: two keys joined by a link.
    LinkRelated,
    /// Box Scale: keys inside a selected box with corner handles.
    BoxScale,
    /// Retime: keys on a curve over a two-way time arrow.
    Retime,
    /// Speed: a dial with a cyan-tipped needle.
    CurveSpeed,
    /// In slope: a key with its incoming handle selected.
    InSlope,
    /// Out slope: a key with its outgoing handle selected.
    OutSlope,
    /// Presets: three stacked cards with curve shapes.
    CurvePresets,
    /// Animation Editors: a panel with a curve above a row of keys.
    AnimationEditors,
    /// Constrain: a cyan cone aimed at a cube along a dashed line.
    Constrain,
    /// Hierarchy: a parent cube above two children, joined by dark links.
    Hierarchy,
    /// Channel Links: two channel sliders joined by a cyan link.
    ChannelLinks,
    /// Schematic: three node tiles wired together in cyan.
    Schematic,
    /// Bake: a curve set with a cyan key at every frame.
    BakeKeys,
    /// Setup: a skeleton standing in its straight rest pose on a selected base.
    SetupPose,
    /// Skeleton: a joint chain with its newest bone and joint cyan.
    SkeletonTool,
    /// Skin: a bent mesh tube bound to the cyan bones beside it.
    SkinMesh,
    /// Apply IK (Two-Bone): a bent two-bone limb reaching a cyan goal.
    IkTwoBone,
    /// Apply IK (Chain): a curving bone chain reaching a cyan goal.
    IkChain,
    /// IK Up Vector: a bent limb whose knee points at a selected pole.
    IkUpVector,
    /// IK Goal: a cyan target with a bone tip in it.
    IkGoal,
}

impl Sample {
    pub const ALL: [Sample; 168] = [
        Sample::Cube,
        Sample::SelectedFace,
        Sample::Sphere,
        Sample::Cylinder,
        Sample::EdgeBevel,
        Sample::PointLight,
        Sample::Cone,
        Sample::Torus,
        Sample::Capsule,
        Sample::Ellipsoid,
        Sample::SelectedVertices,
        Sample::SelectedEdges,
        Sample::Bevel,
        Sample::Extrude,
        Sample::Spike,
        Sample::Curve,
        Sample::BezierCurve,
        Sample::BSpline,
        Sample::Pen,
        Sample::Tube,
        Sample::Arc,
        Sample::Bridge,
        Sample::Thicken,
        Sample::SmoothShift,
        Sample::EdgeExtrude,
        Sample::EdgeChamfer,
        Sample::VertexBevel,
        Sample::VertexExtrude,
        Sample::LoopSlice,
        Sample::Inset,
        Sample::Triple,
        Sample::MeshItem,
        Sample::Scene,
        Sample::Copy,
        Sample::Clone,
        Sample::Array,
        Sample::RadialArray,
        Sample::Dicer,
        Sample::Julienne,
        Sample::Taper,
        Sample::Bulge,
        Sample::Flare,
        Sample::Twist,
        Sample::Bend,
        Sample::Shear,
        Sample::Text,
        Sample::TwistBevel,
        Sample::PolyHaul,
        Sample::Shift,
        Sample::ScaleOffset,
        Sample::ScaleOffsetDetach,
        Sample::AddLoop,
        Sample::AddPoint,
        Sample::EdgeExtend,
        Sample::EdgeCrease,
        Sample::VertexTear,
        Sample::Merge,
        Sample::Unmerge,
        Sample::GroundAlign,
        Sample::Mirror,
        Sample::Boolean,
        Sample::SketchExtrude,
        Sample::CurveExtrude,
        Sample::BezierExtrude,
        Sample::BsplineExtrude,
        Sample::PenExtrude,
        Sample::CurveClone,
        Sample::BezierClone,
        Sample::BsplineClone,
        Sample::PenClone,
        Sample::ScatterClone,
        Sample::Chamfer,
        Sample::EdgeSlide,
        Sample::EdgeRelax,
        Sample::VertexMerge,
        Sample::VertexWeld,
        Sample::VertexSlide,
        Sample::Detriangulate,
        Sample::Symmetrize,
        Sample::TopologyPen,
        Sample::LinkIndependent,
        Sample::LinkCopy,
        Sample::LinkProportional,
        Sample::LinkRelative,
        Sample::FixBlackDots,
        Sample::EdgeZip,
        Sample::DiagonalRedirect,
        Sample::Contour,
        Sample::MeshPaint,
        Sample::MeshPaintInstance,
        Sample::ReplicaClone,
        Sample::CurveReplica,
        Sample::BezierReplica,
        Sample::BsplineReplica,
        Sample::PenReplicate,
        Sample::ReplicaScatter,
        Sample::ReplicaArray,
        Sample::ReplicaRadialArray,
        Sample::ReplicaMirror,
        Sample::MeshPaintReplica,
        Sample::InstanceClone,
        Sample::CurveInstance,
        Sample::BezierInstance,
        Sample::BsplineInstance,
        Sample::PenInstance,
        Sample::InstanceScatter,
        Sample::InstanceArray,
        Sample::InstanceRadialArray,
        Sample::InstanceMirror,
        Sample::VertexNormalTools,
        Sample::RecalculateNormals,
        Sample::HardenNormals,
        Sample::SoftenNormals,
        Sample::SelectHardEdges,
        Sample::TransferNormals,
        Sample::SpinVertexIndices,
        Sample::Guides,
        Sample::TimelineClock,
        Sample::FirstFrame,
        Sample::LastFrame,
        Sample::PreviousFrame,
        Sample::NextFrame,
        Sample::Play,
        Sample::Stop,
        Sample::PreviousKey,
        Sample::NextKey,
        Sample::FirstKey,
        Sample::LastKey,
        Sample::KeyChannels,
        Sample::KeyItems,
        Sample::InsertFrame,
        Sample::RemoveFrame,
        Sample::ApplyKeys,
        Sample::DiscardKeys,
        Sample::AutoKey,
        Sample::GraphEditor,
        Sample::KeyKeepingShape,
        Sample::BreakTangents,
        Sample::UnifyTangents,
        Sample::PreBehaviour,
        Sample::PostBehaviour,
        Sample::CycleCurve,
        Sample::ShadowCopy,
        Sample::ShadowSwap,
        Sample::Normalised,
        Sample::Stacked,
        Sample::IsolateCurve,
        Sample::Velocity,
        Sample::LinkRelated,
        Sample::BoxScale,
        Sample::Retime,
        Sample::CurveSpeed,
        Sample::InSlope,
        Sample::OutSlope,
        Sample::CurvePresets,
        Sample::AnimationEditors,
        Sample::Constrain,
        Sample::Hierarchy,
        Sample::ChannelLinks,
        Sample::Schematic,
        Sample::BakeKeys,
        Sample::SetupPose,
        Sample::SkeletonTool,
        Sample::SkinMesh,
        Sample::IkTwoBone,
        Sample::IkChain,
        Sample::IkUpVector,
        Sample::IkGoal,
    ];

    /// File stem and icon slug.
    pub fn slug(self) -> &'static str {
        match self {
            Sample::Cube => "prim-cube",
            Sample::SelectedFace => "select-polygon",
            Sample::Sphere => "prim-sphere",
            Sample::Cylinder => "prim-cylinder",
            Sample::EdgeBevel => "edge-bevel",
            Sample::PointLight => "light-point",
            Sample::Cone => "prim-cone",
            Sample::Torus => "prim-torus",
            Sample::Capsule => "prim-capsule",
            Sample::Ellipsoid => "prim-ellipsoid",
            Sample::SelectedVertices => "select-vertex",
            Sample::SelectedEdges => "select-edge",
            Sample::Bevel => "poly-bevel",
            Sample::Extrude => "poly-extrude",
            Sample::Spike => "poly-spike",
            Sample::Curve => "curve",
            Sample::BezierCurve => "curve-bezier",
            Sample::BSpline => "curve-bspline",
            Sample::Pen => "pen",
            Sample::Tube => "prim-tube",
            Sample::Arc => "prim-arc",
            Sample::Bridge => "poly-bridge",
            Sample::Thicken => "poly-thicken",
            Sample::SmoothShift => "poly-smooth-shift",
            Sample::EdgeExtrude => "edge-extrude",
            Sample::EdgeChamfer => "edge-chamfer",
            Sample::VertexBevel => "vertex-bevel",
            Sample::VertexExtrude => "vertex-extrude",
            Sample::LoopSlice => "loop-slice",
            Sample::Inset => "poly-inset",
            Sample::Triple => "poly-triple",
            Sample::MeshItem => "item-mesh",
            Sample::Scene => "scene",
            Sample::Copy => "copy",
            Sample::Clone => "dup-clone",
            Sample::Array => "dup-array",
            Sample::RadialArray => "dup-radial-array",
            Sample::Dicer => "slice-dicer",
            Sample::Julienne => "slice-julienne",
            Sample::Taper => "deform-taper",
            Sample::Bulge => "deform-bulge",
            Sample::Flare => "deform-flare",
            Sample::Twist => "deform-twist",
            Sample::Bend => "deform-bend",
            Sample::Shear => "deform-shear",
            Sample::Text => "text",
            Sample::TwistBevel => "poly-twist-bevel",
            Sample::PolyHaul => "poly-haul",
            Sample::Shift => "poly-shift",
            Sample::ScaleOffset => "poly-scale-offset",
            Sample::ScaleOffsetDetach => "poly-scale-offset-detach",
            Sample::AddLoop => "edge-add-loop",
            Sample::AddPoint => "edge-add-point",
            Sample::EdgeExtend => "edge-extend",
            Sample::EdgeCrease => "edge-crease",
            Sample::VertexTear => "vertex-tear",
            Sample::Merge => "item-merge",
            Sample::Unmerge => "item-unmerge",
            Sample::GroundAlign => "ground-align",
            Sample::Mirror => "dup-mirror",
            Sample::Boolean => "poly-boolean",
            Sample::SketchExtrude => "curve-sketch-extrude",
            Sample::CurveExtrude => "curve-extrude",
            Sample::BezierExtrude => "curve-bezier-extrude",
            Sample::BsplineExtrude => "curve-bspline-extrude",
            Sample::PenExtrude => "curve-pen-extrude",
            Sample::CurveClone => "curve-clone",
            Sample::BezierClone => "curve-bezier-clone",
            Sample::BsplineClone => "curve-bspline-clone",
            Sample::PenClone => "curve-pen-clone",
            Sample::ScatterClone => "curve-scatter-clone",
            Sample::Chamfer => "poly-chamfer",
            Sample::EdgeSlide => "edge-slide",
            Sample::EdgeRelax => "edge-relax",
            Sample::VertexMerge => "vertex-merge",
            Sample::VertexWeld => "vertex-weld",
            Sample::TopologyPen => "topology-pen",
            Sample::VertexSlide => "vertex-slide",
            Sample::Detriangulate => "poly-detriangulate",
            Sample::Symmetrize => "symmetrize",
            Sample::LinkIndependent => "link-independent",
            Sample::LinkCopy => "link-copy",
            Sample::LinkProportional => "link-proportional",
            Sample::LinkRelative => "link-relative",
            Sample::FixBlackDots => "fix-black-dots",
            Sample::EdgeZip => "edge-zip",
            Sample::DiagonalRedirect => "diagonal-redirect",
            Sample::Contour => "contour",
            Sample::MeshPaint => "mesh-paint",
            Sample::MeshPaintInstance => "mesh-paint-instance",
            Sample::ReplicaClone => "replica-clone",
            Sample::CurveReplica => "curve-replica",
            Sample::BezierReplica => "bezier-replica",
            Sample::BsplineReplica => "bspline-replica",
            Sample::PenReplicate => "pen-replicate",
            Sample::ReplicaScatter => "replica-scatter",
            Sample::ReplicaArray => "replica-array",
            Sample::ReplicaRadialArray => "replica-radial-array",
            Sample::ReplicaMirror => "replica-mirror",
            Sample::MeshPaintReplica => "mesh-paint-replica",
            Sample::InstanceClone => "instance-clone",
            Sample::CurveInstance => "curve-instance",
            Sample::BezierInstance => "bezier-instance",
            Sample::BsplineInstance => "bspline-instance",
            Sample::PenInstance => "pen-instance",
            Sample::InstanceScatter => "instance-scatter",
            Sample::InstanceArray => "instance-array",
            Sample::InstanceRadialArray => "instance-radial-array",
            Sample::InstanceMirror => "instance-mirror",
            Sample::VertexNormalTools => "vertex-normal-tools",
            Sample::RecalculateNormals => "recalculate-normals",
            Sample::HardenNormals => "harden-normals",
            Sample::SoftenNormals => "soften-normals",
            Sample::SelectHardEdges => "select-hard-edges",
            Sample::TransferNormals => "transfer-normals",
            Sample::SpinVertexIndices => "spin-vertex-indices",
            Sample::Guides => "guides",
            Sample::TimelineClock => "timeline-clock",
            Sample::FirstFrame => "first-frame",
            Sample::LastFrame => "last-frame",
            Sample::PreviousFrame => "previous-frame",
            Sample::NextFrame => "next-frame",
            Sample::Play => "play",
            Sample::Stop => "stop",
            Sample::PreviousKey => "previous-key",
            Sample::NextKey => "next-key",
            Sample::FirstKey => "first-key",
            Sample::LastKey => "last-key",
            Sample::KeyChannels => "key-channels",
            Sample::KeyItems => "key-items",
            Sample::InsertFrame => "insert-frame",
            Sample::RemoveFrame => "remove-frame",
            Sample::ApplyKeys => "apply-keys",
            Sample::DiscardKeys => "discard-keys",
            Sample::AutoKey => "auto-key",
            Sample::GraphEditor => "graph-editor",
            Sample::KeyKeepingShape => "key-keeping-shape",
            Sample::BreakTangents => "break-tangents",
            Sample::UnifyTangents => "unify-tangents",
            Sample::PreBehaviour => "pre-behaviour",
            Sample::PostBehaviour => "post-behaviour",
            Sample::CycleCurve => "cycle-curve",
            Sample::ShadowCopy => "shadow-copy",
            Sample::ShadowSwap => "shadow-swap",
            Sample::Normalised => "normalised",
            Sample::Stacked => "stacked",
            Sample::IsolateCurve => "isolate-curve",
            Sample::Velocity => "velocity",
            Sample::LinkRelated => "link-related",
            Sample::BoxScale => "box-scale",
            Sample::Retime => "retime",
            Sample::CurveSpeed => "curve-speed",
            Sample::InSlope => "in-slope",
            Sample::OutSlope => "out-slope",
            Sample::CurvePresets => "curve-presets",
            Sample::AnimationEditors => "animation-editors",
            Sample::Constrain => "constrain",
            Sample::Hierarchy => "hierarchy",
            Sample::ChannelLinks => "channel-links",
            Sample::Schematic => "schematic",
            Sample::BakeKeys => "bake-keys",
            Sample::SetupPose => "setup-pose",
            Sample::SkeletonTool => "skeleton-tool",
            Sample::SkinMesh => "skin-mesh",
            Sample::IkTwoBone => "ik-two-bone",
            Sample::IkChain => "ik-chain",
            Sample::IkUpVector => "ik-up-vector",
            Sample::IkGoal => "ik-goal",
        }
    }

    pub fn named(name: &str) -> Option<Sample> {
        Self::ALL.into_iter().find(|s| s.slug() == name)
    }
}

fn tagged(mut mesh: Mesh, tag: &str) -> Mesh {
    for face in &mut mesh.faces {
        face.material = tag.into();
    }
    mesh
}

fn polygon(vertices: Vec<u32>, tag: &str) -> crate::Face {
    crate::Face {
        vertices,
        uv: None,
        material: tag.into(),
        smooth: false,
        part: None,
        material_label: None,
    }
}

/// A 1 m cube with the edge between its top (+Y) and front (+Z) faces cut
/// back `cut` metres: a pentagon profile in the YZ plane run along X. The
/// cut face is tagged New, the rest Clay.
fn bevelled_cube(cut: f64) -> Result<Mesh> {
    let h = 0.5;
    // Profile (z, y), clockwise seen from +X (where +Z points left).
    let profile = [(-h, -h), (h, -h), (h, h - cut), (h - cut, h), (-h, h)];
    let mut mesh = Mesh::default();
    for x in [-h, h] {
        for &(z, y) in &profile {
            mesh.vertices.push(DVec3::new(x, y, z));
        }
    }
    let n = profile.len() as u32;
    // Caps wind counter-clockwise seen from outside: -X takes the profile
    // as it is, +X backwards.
    mesh.faces.push(polygon((0..n).collect(), CLAY));
    mesh.faces.push(polygon((n..2 * n).rev().collect(), CLAY));
    for i in 0..n {
        let j = (i + 1) % n;
        // Side i joins profile points i and j; side 2 is the cut.
        let tag = if i == 2 { "New" } else { CLAY };
        mesh.faces.push(polygon(vec![i, i + n, j + n, j], tag));
    }
    mesh.validate()?;
    Ok(mesh)
}

/// `profile` (radius, height) turned about +Y with `sides` steps. A point
/// with radius 0 is a pole; `closed` joins the last point back to the
/// first (a torus). Polygons face away from the axis for a profile that
/// runs bottom to top on the outside.
pub fn revolve(profile: &[(f64, f64)], sides: u32, closed: bool, tag: &str) -> Result<Mesh> {
    let mut mesh = Mesh::default();
    let mut rings: Vec<Vec<u32>> = Vec::new();
    for &(r, y) in profile {
        let start = mesh.vertices.len() as u32;
        if r.abs() < 1e-9 {
            mesh.vertices.push(DVec3::new(0., y, 0.));
            rings.push(vec![start; sides as usize]);
        } else {
            for k in 0..sides {
                let a = std::f64::consts::TAU * f64::from(k) / f64::from(sides);
                mesh.vertices.push(DVec3::new(r * a.cos(), y, r * a.sin()));
            }
            rings.push((start..start + sides).collect());
        }
    }
    let spans = if closed { rings.len() } else { rings.len() - 1 };
    for i in 0..spans {
        let (lo, hi) = (&rings[i], &rings[(i + 1) % rings.len()]);
        for k in 0..sides as usize {
            let n = (k + 1) % sides as usize;
            let mut v = vec![lo[k], hi[k], hi[n], lo[n]];
            v.dedup();
            if v.len() > 1 && v[0] == v[v.len() - 1] {
                v.pop();
            }
            if v.len() >= 3 {
                mesh.faces.push(polygon(v, tag));
            }
        }
    }
    mesh.validate()?;
    Ok(mesh)
}

/// A round tube of `radius` through `points` (a polyline), closed at both
/// ends, each ring square to the path.
pub fn tube(points: &[DVec3], radius: f64, sides: u32, tag: &str) -> Result<Mesh> {
    tube_with(points, |_| radius, sides, tag)
}

/// [`tube`] whose radius at each point is `radius(t)`, `t` running 0 to 1
/// along the points.
pub fn tube_with(
    points: &[DVec3],
    radius: impl Fn(f64) -> f64,
    sides: u32,
    tag: &str,
) -> Result<Mesh> {
    if points.len() < 2 {
        return Err("A tube needs two points".into());
    }
    let mut mesh = Mesh::default();
    let tangent = |i: usize| {
        let a = points[i.saturating_sub(1)];
        let b = points[(i + 1).min(points.len() - 1)];
        (b - a).normalize_or(DVec3::Y)
    };
    // Parallel transport: each ring turns the least from the last.
    let t0 = tangent(0);
    let mut side = t0.any_orthonormal_vector();
    for (i, &p) in points.iter().enumerate() {
        let t = tangent(i);
        side = (side - t * side.dot(t)).normalize_or(t.any_orthonormal_vector());
        let up = t.cross(side);
        for k in 0..sides {
            let a = std::f64::consts::TAU * f64::from(k) / f64::from(sides);
            let r = radius(i as f64 / (points.len() - 1) as f64);
            mesh.vertices.push(p + (side * a.cos() + up * a.sin()) * r);
        }
    }
    let ring = |i: usize, k: u32| (i as u32) * sides + k % sides;
    for i in 0..points.len() - 1 {
        for k in 0..sides {
            mesh.faces.push(polygon(
                vec![
                    ring(i, k),
                    ring(i, k + 1),
                    ring(i + 1, k + 1),
                    ring(i + 1, k),
                ],
                tag,
            ));
        }
    }
    let last = points.len() - 1;
    mesh.faces
        .push(polygon((0..sides).rev().map(|k| ring(0, k)).collect(), tag));
    mesh.faces
        .push(polygon((0..sides).map(|k| ring(last, k)).collect(), tag));
    mesh.validate()?;
    Ok(mesh)
}

/// A small ball (a curve's point, a selected vertex).
fn ball(center: DVec3, radius: f64, tag: &str) -> Result<Mesh> {
    Ok(tagged(
        crate::radial::globe_geometry(
            center,
            crate::radial::RadialParameters {
                radius: DVec3::splat(radius),
                sides: 24,
                segments: 12,
            },
        )?,
        tag,
    ))
}

/// A smooth curve through `points` (Catmull-Rom), `steps` pieces per span.
pub fn through(points: &[DVec3], steps: usize) -> Vec<DVec3> {
    let n = points.len();
    let at = |i: isize| points[i.clamp(0, n as isize - 1) as usize];
    let mut out = Vec::new();
    for i in 0..n.saturating_sub(1) {
        let (p0, p1, p2, p3) = (
            at(i as isize - 1),
            at(i as isize),
            at(i as isize + 1),
            at(i as isize + 2),
        );
        for s in 0..steps {
            let t = s as f64 / steps as f64;
            let t2 = t * t;
            let t3 = t2 * t;
            out.push(
                0.5 * (2. * p1
                    + (p2 - p0) * t
                    + (2. * p0 - 5. * p1 + 4. * p2 - p3) * t2
                    + (3. * p1 - p0 - 3. * p2 + p3) * t3),
            );
        }
    }
    if let Some(&last) = points.last() {
        out.push(last);
    }
    out
}

fn merge(meshes: Vec<Mesh>) -> Result<Mesh> {
    let mut all = Mesh::default();
    for mesh in meshes {
        crate::primitive::append_geometry(&mut all, mesh)?;
    }
    Ok(all)
}

/// A box from `min` to `max`, its top tagged `top`, sides `sides` and
/// bottom `bottom`.
fn tagged_box(min: DVec3, max: DVec3, top: &str, sides: &str, bottom: &str) -> Result<Mesh> {
    let mut mesh = crate::primitive::cube_geometry((min + max) * 0.5, max - min)?;
    let mid = (min.y + max.y) * 0.5;
    for i in 0..mesh.faces.len() {
        let ys: Vec<f64> = mesh.faces[i]
            .vertices
            .iter()
            .map(|&v| mesh.vertices[v as usize].y)
            .collect();
        mesh.faces[i].material = if ys.iter().all(|&y| y > mid) {
            top
        } else if ys.iter().all(|&y| y < mid) {
            bottom
        } else {
            sides
        }
        .into();
    }
    Ok(mesh)
}

/// The cube's corners the camera sees: all but the one at (−, −, −).
fn visible_corners() -> Vec<DVec3> {
    let mut out = Vec::new();
    for x in [-0.5, 0.5] {
        for y in [-0.5, 0.5] {
            for z in [-0.5, 0.5] {
                if x > 0. || y > 0. || z > 0. {
                    out.push(DVec3::new(x, y, z));
                }
            }
        }
    }
    out
}

/// A box from `min` to `max` whose faces are tagged by `tag_of(normal)`.
fn box_by_normal(min: DVec3, max: DVec3, tag_of: impl Fn(DVec3) -> &'static str) -> Result<Mesh> {
    let mut mesh = crate::primitive::cube_geometry((min + max) * 0.5, max - min)?;
    let centre = (min + max) * 0.5;
    for i in 0..mesh.faces.len() {
        let p: Vec<DVec3> = mesh.faces[i]
            .vertices
            .iter()
            .map(|&v| mesh.vertices[v as usize])
            .collect();
        let c = p.iter().copied().sum::<DVec3>() / p.len() as f64;
        let d = (c - centre) / ((max - min) * 0.5);
        // The face's outward axis: the centroid's largest offset.
        let normal = if d.x.abs() >= d.y.abs() && d.x.abs() >= d.z.abs() {
            DVec3::X * d.x.signum()
        } else if d.y.abs() >= d.z.abs() {
            DVec3::Y * d.y.signum()
        } else {
            DVec3::Z * d.z.signum()
        };
        mesh.faces[i].material = tag_of(normal).into();
    }
    Ok(mesh)
}

/// `mesh` with the corner at `corner` cut back `cut` along each of its
/// edges; returns the new points (in no order) and the mesh.
fn cut_corner(mut mesh: Mesh, corner: DVec3, cut: f64) -> Result<(Mesh, Vec<u32>)> {
    let v = mesh
        .vertices
        .iter()
        .position(|p| p.distance(corner) < 1e-9)
        .ok_or("no such corner")? as u32;
    let mut made: Vec<(u32, u32)> = Vec::new(); // (neighbour, new point)
    let mut point_toward = |mesh: &mut Mesh, n: u32| -> u32 {
        if let Some(&(_, p)) = made.iter().find(|(m, _)| *m == n) {
            return p;
        }
        let a = mesh.vertices[v as usize];
        let b = mesh.vertices[n as usize];
        mesh.vertices.push(a + (b - a).normalize() * cut);
        let p = mesh.vertices.len() as u32 - 1;
        made.push((n, p));
        p
    };
    for f in 0..mesh.faces.len() {
        let verts = mesh.faces[f].vertices.clone();
        let Some(i) = verts.iter().position(|&x| x == v) else {
            continue;
        };
        let k = verts.len();
        let prev = verts[(i + k - 1) % k];
        let next = verts[(i + 1) % k];
        let (a, b) = (point_toward(&mut mesh, prev), point_toward(&mut mesh, next));
        let mut out = verts.clone();
        out.splice(i..=i, [a, b]);
        mesh.faces[f].vertices = out;
    }
    // The old corner point is no longer used; leave it (unused points are
    // allowed) rather than renumber.
    let new: Vec<u32> = made.iter().map(|(_, p)| *p).collect();
    Ok((mesh, new))
}

/// `points` ordered to wind counter-clockwise seen from `outward`.
fn wound(mesh: &Mesh, mut points: Vec<u32>, outward: DVec3) -> Vec<u32> {
    let p: Vec<DVec3> = points.iter().map(|&i| mesh.vertices[i as usize]).collect();
    let c = p.iter().copied().sum::<DVec3>() / p.len() as f64;
    let side = outward.any_orthonormal_vector();
    let up = outward.cross(side);
    let angle = |q: DVec3| (q - c).dot(up).atan2((q - c).dot(side));
    let mut order: Vec<(f64, u32)> = p
        .iter()
        .zip(&points)
        .map(|(&q, &i)| (angle(q), i))
        .collect();
    order.sort_by(|a, b| a.0.total_cmp(&b.0));
    points = order.into_iter().map(|(_, i)| i).collect();
    points
}

/// Uniform cubic B-spline through the control `cage`'s pull, `steps` per
/// span, clamped to start and end at the cage's ends.
fn bspline(cage: &[DVec3], steps: usize) -> Vec<DVec3> {
    let mut pts = vec![cage[0], cage[0]];
    pts.extend_from_slice(cage);
    pts.push(cage[cage.len() - 1]);
    pts.push(cage[cage.len() - 1]);
    let mut out = Vec::new();
    for i in 0..pts.len() - 3 {
        let (p0, p1, p2, p3) = (pts[i], pts[i + 1], pts[i + 2], pts[i + 3]);
        for s in 0..steps {
            let t = s as f64 / steps as f64;
            let (t2, t3) = (t * t, t * t * t);
            out.push(
                (p0 * (1. - t).powi(3)
                    + p1 * (3. * t3 - 6. * t2 + 4.)
                    + p2 * (-3. * t3 + 3. * t2 + 3. * t + 1.)
                    + p3 * t3)
                    / 6.,
            );
        }
    }
    out.push(cage[cage.len() - 1]);
    out
}

/// A square column `2 * half` wide and `2 * half_height` tall about the
/// origin, cut into `segments` along Y so it can be deformed, faces Clay.
fn column(half: f64, half_height: f64, segments: u32) -> Result<Mesh> {
    let r = half * std::f64::consts::SQRT_2;
    let mut profile = vec![(0., -half_height)];
    for k in 0..=segments {
        let y = -half_height + 2. * half_height * f64::from(k) / f64::from(segments);
        profile.push((r, y));
    }
    profile.push((0., half_height));
    let mut mesh = revolve(&profile, 4, false, CLAY)?;
    // Turn the four-sided ring so its sides face ±X and ±Z.
    let c = std::f64::consts::FRAC_1_SQRT_2;
    for v in &mut mesh.vertices {
        *v = DVec3::new(c * (v.x - v.z), v.y, c * (v.x + v.z));
    }
    Ok(mesh)
}

/// An arrow from `from` to `to`: a shaft and a cone head.
fn arrow(from: DVec3, to: DVec3, radius: f64, tag: &str) -> Result<Mesh> {
    let dir = (to - from).normalize();
    let head = radius * 3.2;
    let neck = to - dir * head * 1.4;
    let mut cone = revolve(&[(0., 0.), (head, 0.), (0., head * 1.4)], 16, false, tag)?;
    let turn = glam::DQuat::from_rotation_arc(DVec3::Y, dir);
    for v in &mut cone.vertices {
        *v = neck + turn * *v;
    }
    merge(vec![tube(&[from, neck], radius, 12, tag)?, cone])
}

/// A cube of side `size` at `centre`, its top tagged `top`.
fn cube_at(centre: DVec3, size: f64, top: &'static str) -> Result<Mesh> {
    let h = DVec3::splat(size * 0.5);
    tagged_box(centre - h, centre + h, top, CLAY, CLAY)
}

/// Applies `f` to every point.
fn deformed(mut mesh: Mesh, f: impl Fn(DVec3) -> DVec3) -> Mesh {
    for v in &mut mesh.vertices {
        *v = f(*v);
    }
    mesh
}

/// A 1 m cube with the edge between its top (+Y) and front (+Z) faces
/// rounded to `radius` in `segments` flat rows, tagged New.
fn rounded_cube(radius: f64, segments: u32) -> Result<Mesh> {
    let h = 0.5;
    // Profile (z, y), clockwise seen from +X, as in `bevelled_cube`.
    let mut profile = vec![(-h, -h), (h, -h)];
    for k in 0..=segments {
        let a = std::f64::consts::FRAC_PI_2 * f64::from(k) / f64::from(segments);
        profile.push((h - radius + radius * a.cos(), h - radius + radius * a.sin()));
    }
    profile.push((-h, h));
    let mut mesh = Mesh::default();
    for x in [-h, h] {
        for &(z, y) in &profile {
            mesh.vertices.push(DVec3::new(x, y, z));
        }
    }
    let n = profile.len() as u32;
    mesh.faces.push(polygon((0..n).collect(), CLAY));
    mesh.faces.push(polygon((n..2 * n).rev().collect(), CLAY));
    for i in 0..n {
        let j = (i + 1) % n;
        let rounded = (2..2 + segments).contains(&i);
        mesh.faces.push(polygon(
            vec![i, i + n, j + n, j],
            if rounded { "New" } else { CLAY },
        ));
    }
    mesh.validate()?;
    Ok(mesh)
}

/// The point `t` (0 to 1) of the way along `path` by length.
fn along(path: &[DVec3], t: f64) -> DVec3 {
    let total: f64 = path.windows(2).map(|w| w[0].distance(w[1])).sum();
    let mut left = total * t.clamp(0., 1.);
    for w in path.windows(2) {
        let d = w[0].distance(w[1]);
        if left <= d && d > 0. {
            return w[0].lerp(w[1], left / d);
        }
        left -= d;
    }
    *path.last().unwrap_or(&DVec3::ZERO)
}

/// A curve tool's pieces: the cyan `path`, Clay balls at `knots`, and a
/// selected handle through each of `handles` (centre, half-vector).
fn curve_parts(path: &[DVec3], knots: &[DVec3], handles: &[(DVec3, DVec3)]) -> Result<Vec<Mesh>> {
    let mut parts = vec![tube(path, 0.055, 12, "New")?];
    for &k in knots {
        parts.push(ball(k, 0.08, CLAY)?);
    }
    for &(c, half) in handles {
        parts.push(tube(&[c - half, c + half], 0.03, 8, "Selected")?);
        parts.push(ball(c + half, 0.07, "Selected")?);
    }
    Ok(parts)
}

/// The picture's right and up, in the world.
fn screen_axes() -> (DVec3, DVec3) {
    let forward = -DVec3::ONE.normalize();
    let right = forward.cross(DVec3::Y).normalize();
    (right, right.cross(forward))
}

/// The extrude family's swept path: an S from the lower left, leaving
/// its profile toward the back, to the upper right.
fn sweep_points() -> [DVec3; 4] {
    let (r, u) = screen_axes();
    let back = -DVec3::ONE.normalize();
    let start = r * -0.42 + u * -0.45;
    [
        start,
        r * 0.1 + u * -0.3 + back * 0.25,
        r * -0.1 + u * 0.2,
        r * 0.45 + u * 0.45 + back * 0.2,
    ]
}

/// The clone family's path, across the picture on the ground, zigzagging
/// in depth so it shows as a wave.
fn clone_points() -> [DVec3; 4] {
    [
        DVec3::new(-0.6, -0.3, 0.25),
        DVec3::new(-0.2, -0.3, -0.35),
        DVec3::new(0.2, -0.3, 0.35),
        DVec3::new(0.55, -0.3, -0.2),
    ]
}

/// A square profile plate square to `path`'s start, selected.
fn profile_plate(path: &[DVec3]) -> Result<Mesh> {
    let t = (path[1] - path[0]).normalize();
    let plate = box_by_normal(
        DVec3::new(-0.22, -0.22, -0.025),
        DVec3::new(0.22, 0.22, 0.025),
        |_| "Selected",
    )?;
    let turn = glam::DQuat::from_rotation_arc(DVec3::Z, t);
    Ok(deformed(plate, |v| path[0] + turn * v))
}

/// Cubes along `path` at even steps, the first Clay-topped.
fn cubes_along(path: &[DVec3], count: u32) -> Result<Vec<Mesh>> {
    (0..count)
        .map(|k| {
            let p = along(path, f64::from(k) / f64::from(count - 1));
            cube_at(p + DVec3::Y * 0.14, 0.28, if k == 0 { CLAY } else { "New" })
        })
        .collect()
}

/// A double-headed arrow from `a` to `b`.
fn two_way(a: DVec3, b: DVec3, radius: f64, tag: &str) -> Result<Mesh> {
    let mid = (a + b) * 0.5;
    merge(vec![
        arrow(mid, a, radius, tag)?,
        arrow(mid, b, radius, tag)?,
    ])
}

/// One chain link lying in the picture: a rounded slot `length` long and
/// `width` wide (outside), its long side along `along` (screen right/up
/// weights), tilted `tilt` degrees about that side so a joined link
/// reads as passing through its neighbour.
fn chain_link(
    centre: (f64, f64),
    along: (f64, f64),
    length: f64,
    width: f64,
    tilt: f64,
) -> Result<Mesh> {
    let (right, up) = screen_axes();
    let depth = right.cross(up);
    let dir = (right * along.0 + up * along.1).normalize();
    let side = depth.cross(dir).normalize();
    let tilt = tilt.to_radians();
    let side = side * tilt.cos() + depth * tilt.sin();
    let thick = width * 0.24;
    let (r, half) = (width * 0.5 - thick * 0.5, (length - width) * 0.5);
    let middle = right * centre.0 + up * centre.1;
    let mut path = Vec::new();
    // Round the slot: one end's half circle, then the other's.
    for (end, from) in [
        (half, -std::f64::consts::FRAC_PI_2),
        (-half, std::f64::consts::FRAC_PI_2),
    ] {
        for k in 0..=16 {
            let a = from + std::f64::consts::PI * f64::from(k) / 16.;
            path.push(middle + dir * (end + r * a.cos()) + side * (r * a.sin()));
        }
    }
    path.push(path[0]);
    tube(&path, thick * 0.5, 14, CLAY)
}

/// `mesh` tipped `degrees` toward the camera about the picture's
/// horizontal, so a flat plate shows its face.
fn tipped(mesh: Mesh, degrees: f64) -> Mesh {
    let across = DVec3::new(1., 0., -1.).normalize();
    let tip = glam::DQuat::from_axis_angle(across, degrees.to_radians());
    deformed(mesh, |v| tip * v)
}

/// A thin square Clay plate `2 * half` across, its top at y = -0.05.
fn plate(half: f64) -> Result<Mesh> {
    tagged_box(
        DVec3::new(-half, -0.13, -half),
        DVec3::new(half, -0.05, half),
        CLAY,
        CLAY,
        CLAY,
    )
}

/// How a duplicate tool's Instance or Replica sibling makes its copies.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CopyStyle {
    /// Instance items: each copy drawn as a cyan wireframe of its shape.
    Instance,
    /// A Replicator: each copy drawn as a cyan point cloud of its shape.
    Replica,
}

/// `mesh`'s edges, each once.
fn mesh_edges(mesh: &Mesh) -> Vec<(DVec3, DVec3)> {
    let mut seen = std::collections::BTreeSet::new();
    let mut out = Vec::new();
    for f in &mesh.faces {
        let n = f.vertices.len();
        for k in 0..n {
            let (a, b) = (f.vertices[k], f.vertices[(k + 1) % n]);
            if seen.insert((a.min(b), a.max(b))) {
                out.push((mesh.vertices[a as usize], mesh.vertices[b as usize]));
            }
        }
    }
    out
}

/// `proxy` as a wireframe (Instance) or a point cloud (Replica).
fn copy_in_style(proxy: &Mesh, style: CopyStyle, size: f64) -> Result<Mesh> {
    let edges = mesh_edges(proxy);
    let mut parts = Vec::new();
    match style {
        CopyStyle::Instance => {
            for (a, b) in edges {
                parts.push(tube(&[a, b], (size * 0.07).max(0.018), 6, "New")?);
            }
        }
        CopyStyle::Replica => {
            let mut points: Vec<DVec3> = Vec::new();
            let mut add = |p: DVec3| {
                if !points.iter().any(|q| q.distance(p) < 1e-6) {
                    points.push(p);
                }
            };
            for (a, b) in edges {
                add(a);
                add(b);
                add((a + b) * 0.5);
            }
            for p in points {
                parts.push(ball(p, (size * 0.11).max(0.03), "New")?);
            }
        }
    }
    merge(parts)
}

/// A duplicate tool's picture: fixed parts, an optional source (mesh and
/// centre), and the copies (mesh, centre, half height).
struct Layout {
    base: Vec<Mesh>,
    source: Option<(Mesh, DVec3)>,
    copies: Vec<(Mesh, DVec3, f64)>,
    /// Applied to everything at the end (turns and tips).
    finish: fn(Mesh) -> Mesh,
}

/// The layout of the copies-making tool `family` draws (Clone, the curve
/// clones, Scatter, Array, Radial Array, Mirror, Mesh Paint).
fn dup_layout(family: Sample) -> Result<Layout> {
    let cube = |c: DVec3, size: f64| -> Result<(Mesh, DVec3, f64)> {
        Ok((cube_at(c, size, "New")?, c, size * 0.5))
    };
    let along_path = |base: Vec<Mesh>, path: &[DVec3]| -> Result<Layout> {
        let at = |k: u32| along(path, f64::from(k) / 3.) + DVec3::Y * 0.14;
        Ok(Layout {
            base,
            source: Some((cube_at(at(0), 0.28, CLAY)?, at(0))),
            copies: (1..4).map(|k| cube(at(k), 0.28)).collect::<Result<_>>()?,
            finish: |m| m,
        })
    };
    Ok(match family {
        Sample::Clone => {
            let across = DVec3::new(1., 0., -1.).normalize();
            Layout {
                base: Vec::new(),
                source: Some((cube_at(across * -0.5, 0.38, CLAY)?, across * -0.5)),
                copies: vec![cube(DVec3::ZERO, 0.38)?, cube(across * 0.5, 0.38)?],
                finish: |m| m,
            }
        }
        Sample::CurveClone => {
            let pts = clone_points();
            let path = through(&pts, 16);
            along_path(curve_parts(&path, &pts, &[])?, &path)?
        }
        Sample::BezierClone => {
            let pts = clone_points();
            let path = through(&[pts[0], pts[2], pts[3]], 20);
            let (a, b) = (path[0], path[path.len() - 1]);
            let ta = (path[1] - path[0]).normalize() * 0.18;
            let tb = (path[path.len() - 1] - path[path.len() - 2]).normalize() * 0.18;
            along_path(curve_parts(&path, &[], &[(a, -ta), (b, tb)])?, &path)?
        }
        Sample::BsplineClone => {
            let pts = clone_points();
            let path = bspline(&pts, 16);
            let mut base = curve_parts(&path, &pts, &[])?;
            base.push(tube(&pts, 0.025, 8, CLAY)?);
            along_path(base, &path)?
        }
        Sample::PenClone => {
            let pts = clone_points();
            along_path(curve_parts(&pts, &pts, &[])?, &pts)?
        }
        Sample::ScatterClone => {
            let spots = [
                (-0.3, 0.25),
                (0.05, 0.35),
                (0.3, -0.05),
                (-0.15, -0.2),
                (0.15, -0.38),
            ];
            let at = |(x, z): (f64, f64)| DVec3::new(x, -0.34, z);
            Layout {
                base: vec![tagged_box(
                    DVec3::new(-0.55, -0.5, -0.55),
                    DVec3::new(0.55, -0.44, 0.55),
                    CLAY,
                    CLAY,
                    CLAY,
                )?],
                source: Some((cube_at(at(spots[0]), 0.2, CLAY)?, at(spots[0]))),
                copies: spots[1..]
                    .iter()
                    .map(|&s| cube(at(s), 0.2))
                    .collect::<Result<_>>()?,
                finish: |m| m,
            }
        }
        Sample::Array => {
            let at = |i: i32, j: i32| DVec3::new(f64::from(i) * 0.36, -0.1, f64::from(j) * 0.36);
            let mut copies = Vec::new();
            for i in -1..=1 {
                for j in -1..=1 {
                    if (i, j) != (-1, -1) {
                        copies.push(cube(at(i, j), 0.24)?);
                    }
                }
            }
            Layout {
                base: Vec::new(),
                source: Some((cube_at(at(-1, -1), 0.24, CLAY)?, at(-1, -1))),
                copies,
                finish: |m| m,
            }
        }
        Sample::RadialArray => {
            let at = |k: i32| {
                let a = std::f64::consts::TAU * f64::from(k) / 6.;
                DVec3::new(0.48 * a.cos(), -0.1, 0.48 * a.sin())
            };
            Layout {
                base: Vec::new(),
                source: Some((cube_at(at(0), 0.24, CLAY)?, at(0))),
                copies: (1..6).map(|k| cube(at(k), 0.24)).collect::<Result<_>>()?,
                finish: |m| m,
            }
        }
        Sample::Mirror => {
            let step = |sign: f64, top: &'static str| -> Result<Mesh> {
                let lo = |x0: f64, x1: f64| (x0.min(x1), x0.max(x1));
                let (a0, a1) = lo(sign * 0.08, sign * 0.5);
                let (b0, b1) = lo(sign * 0.08, sign * 0.28);
                merge(vec![
                    tagged_box(
                        DVec3::new(a0, -0.5, -0.35),
                        DVec3::new(a1, -0.1, 0.35),
                        top,
                        CLAY,
                        CLAY,
                    )?,
                    tagged_box(
                        DVec3::new(b0, -0.1, -0.35),
                        DVec3::new(b1, 0.3, 0.35),
                        top,
                        CLAY,
                        CLAY,
                    )?,
                ])
            };
            Layout {
                base: vec![tagged_box(
                    DVec3::new(-0.015, -0.55, -0.5),
                    DVec3::new(0.015, 0.45, 0.5),
                    "New",
                    "New",
                    "New",
                )?],
                source: Some((step(-1., CLAY)?, DVec3::new(-0.29, -0.1, 0.))),
                copies: vec![(step(1., "New")?, DVec3::new(0.29, -0.1, 0.), 0.4)],
                // Turned so the mirror plane is square to the picture.
                finish: |m| {
                    let turn = glam::DQuat::from_rotation_y(std::f64::consts::FRAC_PI_4);
                    deformed(m, |v| turn * v)
                },
            }
        }
        Sample::MeshPaint => {
            let ring: Vec<DVec3> = (0..=40)
                .map(|k| {
                    let a = std::f64::consts::TAU * f64::from(k) / 40.;
                    DVec3::new(0.38 * a.cos(), -0.02, 0.38 * a.sin())
                })
                .collect();
            let spots = [
                (-0.18, 0.12),
                (0.12, 0.2),
                (0.15, -0.15),
                (-0.12, -0.18),
                (0., 0.),
            ];
            Layout {
                base: vec![plate(0.55)?, tube(&ring, 0.03, 8, "Selected")?],
                source: None,
                copies: spots
                    .iter()
                    .map(|&(x, z)| {
                        let c = DVec3::new(x, 0.02, z);
                        // A coarse ball, so its wireframe and points stay few.
                        let coarse = crate::radial::globe_geometry(
                            c,
                            crate::radial::RadialParameters {
                                radius: DVec3::splat(0.1),
                                sides: 6,
                                segments: 4,
                            },
                        )?;
                        Ok((coarse, c, 0.1))
                    })
                    .collect::<Result<_>>()?,
                finish: |m| tipped(m, 25.),
            }
        }
        other => return Err(format!("{other:?} has no duplicate layout")),
    })
}

/// `family`'s picture with its copies made in `style`.
fn styled(family: Sample, style: CopyStyle) -> Result<Mesh> {
    let layout = dup_layout(family)?;
    let mut parts = layout.base;
    if let Some((mesh, _)) = layout.source {
        parts.push(mesh);
    }
    for (mesh, _centre, half) in layout.copies {
        // A copy's size sets its wire and point thickness.
        let size = (half * 2.).min(0.45);
        parts.push(copy_in_style(&mesh, style, size)?);
    }
    Ok((layout.finish)(merge(parts)?))
}

/// A flat symbol's frame: local X right, Y up, Z out of the symbol,
/// turned to face the camera at three quarters so its thickness shows.
fn symbol_frame() -> glam::DQuat {
    let toward = DVec3::ONE.normalize();
    let (right, up) = screen_axes();
    let out = (toward * 0.9 - right * 0.42).normalize();
    let up = (up - out * up.dot(out)).normalize();
    let right = up.cross(out);
    glam::DQuat::from_mat3(&glam::DMat3::from_cols(right, up, out))
}

/// A symbol piece: the 2D polygon `outline` (counter-clockwise, convex)
/// extruded `depth` deep, in the symbol plane.
fn slab(outline: &[(f64, f64)], depth: f64, tag: &str) -> Result<Mesh> {
    let n = outline.len() as u32;
    let mut mesh = Mesh::default();
    for z in [-depth * 0.5, depth * 0.5] {
        for &(x, y) in outline {
            mesh.vertices.push(DVec3::new(x, y, z));
        }
    }
    mesh.faces.push(polygon((0..n).rev().collect(), tag));
    mesh.faces.push(polygon((n..2 * n).collect(), tag));
    for i in 0..n {
        let j = (i + 1) % n;
        mesh.faces.push(polygon(vec![i, j, j + n, i + n], tag));
    }
    mesh.validate()?;
    Ok(mesh)
}

/// A bar from (x0, y0) to (x1, y1) and `width` across, as a slab.
fn bar(x0: f64, y0: f64, x1: f64, y1: f64, width: f64, depth: f64, tag: &str) -> Result<Mesh> {
    let d = DVec3::new(x1 - x0, y1 - y0, 0.).normalize();
    let side = DVec3::new(-d.y, d.x, 0.) * (width * 0.5);
    slab(
        &[
            (x0 - side.x, y0 - side.y),
            (x1 - side.x, y1 - side.y),
            (x1 + side.x, y1 + side.y),
            (x0 + side.x, y0 + side.y),
        ],
        depth,
        tag,
    )
}

/// A triangle pointing right (`dir` 1) or left (-1), centred on (cx, cy).
fn pointer(cx: f64, cy: f64, size: f64, dir: f64, tag: &str) -> Result<Mesh> {
    let h = size * 0.5;
    let pts = if dir > 0. {
        vec![
            (cx - h * 0.85, cy - h),
            (cx + h * 0.85, cy),
            (cx - h * 0.85, cy + h),
        ]
    } else {
        vec![
            (cx + h * 0.85, cy - h),
            (cx + h * 0.85, cy + h),
            (cx - h * 0.85, cy),
        ]
    };
    slab(&pts, 0.14, tag)
}

/// A key: a cyan diamond centred on (cx, cy).
fn key(cx: f64, cy: f64, size: f64) -> Result<Mesh> {
    let h = size * 0.5;
    slab(
        &[(cx, cy - h), (cx + h, cy), (cx, cy + h), (cx - h, cy)],
        0.16,
        "New",
    )
}

/// A frame tile: a thin Clay plate, one frame.
fn frame_tile(cx: f64, cy: f64, w: f64, h: f64) -> Result<Mesh> {
    let mut tile = slab(
        &[
            (cx - w, cy - h),
            (cx + w, cy - h),
            (cx + w, cy + h),
            (cx - w, cy + h),
        ],
        0.06,
        CLAY,
    )?;
    for v in &mut tile.vertices {
        v.z -= 0.1;
    }
    Ok(tile)
}

/// `parts` turned into the symbol frame.
fn symbol(parts: Vec<Mesh>) -> Result<Mesh> {
    let q = symbol_frame();
    Ok(deformed(merge(parts)?, |v| q * v))
}

/// A curve in the symbol plane: `y = f(x)` from `x0` to `x1`, as a tube
/// raised a little off any tile behind it.
fn graph_curve(x0: f64, x1: f64, f: impl Fn(f64) -> f64, radius: f64, tag: &str) -> Result<Mesh> {
    let pts: Vec<DVec3> = (0..=32)
        .map(|k| {
            let x = x0 + (x1 - x0) * f64::from(k) / 32.;
            DVec3::new(x, f(x), 0.04)
        })
        .collect();
    tube(&pts, radius, 10, tag)
}

/// The graph editor's sample curve: an S from low left to high right.
fn s_curve(x: f64) -> f64 {
    0.28 * (x * 4.2).sin() + 0.1 * x
}

/// A key with two selected tangent handles at `left` and `right`
/// (angles, radians, from +X), `len` long.
fn key_handles(cx: f64, cy: f64, left: f64, right: f64, len: f64) -> Result<Vec<Mesh>> {
    let mut parts = vec![key(cx, cy, 0.3)?];
    for a in [left, right] {
        let end = DVec3::new(cx + len * a.cos(), cy + len * a.sin(), 0.06);
        parts.push(tube(
            &[DVec3::new(cx, cy, 0.06), end],
            0.035,
            8,
            "Selected",
        )?);
        parts.push(ball(end, 0.07, "Selected")?);
    }
    Ok(parts)
}

/// A bone from joint `a` to joint `b`: a four-sided spindle, widest a
/// fifth of the way along.
fn bone(a: DVec3, b: DVec3, width: f64, tag: &str) -> Result<Mesh> {
    let len = a.distance(b);
    let mut m = revolve(&[(0., 0.), (width, len * 0.2), (0., len)], 4, false, tag)?;
    let turn = glam::DQuat::from_rotation_arc(DVec3::Y, (b - a).normalize());
    m = deformed(m, |v| a + turn * v);
    Ok(m)
}

/// A joint chain: a ball at each joint, a bone between each pair.
fn chain(joints: &[DVec3], bone_tag: &str, joint_tag: &str) -> Result<Vec<Mesh>> {
    let mut parts = Vec::new();
    for w in joints.windows(2) {
        parts.push(bone(w[0], w[1], 0.09, bone_tag)?);
    }
    for &j in joints {
        parts.push(ball(j, 0.075, joint_tag)?);
    }
    Ok(parts)
}

/// A point in the picture's plane through the origin: `x` across, `y` up.
fn flat(x: f64, y: f64) -> DVec3 {
    DVec3::new(1., 0., -1.).normalize() * x + DVec3::Y * y
}

/// An IK goal: a cyan ring in the picture's plane round a cyan dot.
fn ik_target(centre: DVec3, radius: f64) -> Result<Vec<Mesh>> {
    let ring: Vec<DVec3> = (0..=40)
        .map(|k| {
            let a = std::f64::consts::TAU * f64::from(k) / 40.;
            centre + flat(radius * a.cos(), radius * a.sin())
        })
        .collect();
    Ok(vec![
        tube(&ring, 0.045, 10, "New")?,
        ball(centre, 0.07, "New")?,
    ])
}

/// A thick joint chain for small icons: fat bones, big joints.
fn thick_chain(joints: &[DVec3], tag: &str) -> Result<Vec<Mesh>> {
    let mut parts = Vec::new();
    for w in joints.windows(2) {
        parts.push(bone(w[0], w[1], 0.13, tag)?);
    }
    for &j in joints {
        parts.push(ball(j, 0.1, tag)?);
    }
    Ok(parts)
}

/// The geometry of a sample, inside the icon box.
fn sample_mesh(sample: Sample) -> Result<Mesh> {
    use crate::radial::{RadialParameters, cylinder_geometry, globe_geometry};
    Ok(match sample {
        Sample::Cube => tagged(
            crate::primitive::cube_geometry(DVec3::ZERO, DVec3::ONE)?,
            CLAY,
        ),
        Sample::SelectedFace => {
            let mut mesh = sample_mesh(Sample::Cube)?;
            // The face whose points all sit on top.
            let top = mesh
                .faces
                .iter()
                .position(|f| f.vertices.iter().all(|&v| mesh.vertices[v as usize].y > 0.))
                .ok_or("The cube has no top face")?;
            mesh.faces[top].material = "Selected".into();
            mesh
        }
        Sample::Sphere => tagged(
            globe_geometry(
                DVec3::ZERO,
                RadialParameters {
                    radius: DVec3::splat(0.62),
                    sides: 48,
                    segments: 24,
                },
            )?,
            CLAY,
        ),
        Sample::Cylinder => tagged(
            cylinder_geometry(
                DVec3::ZERO,
                RadialParameters {
                    radius: DVec3::new(0.45, 0.5, 0.45),
                    sides: 48,
                    segments: 1,
                },
            )?,
            CLAY,
        ),
        Sample::EdgeBevel => bevelled_cube(0.4)?,
        Sample::Cone => revolve(&[(0., -0.5), (0.5, -0.5), (0., 0.55)], 48, false, CLAY)?,
        Sample::Torus => {
            let profile: Vec<(f64, f64)> = (0..24)
                .map(|k| {
                    // Round the tube's section from the bottom, outside first.
                    let a =
                        -std::f64::consts::FRAC_PI_2 + std::f64::consts::TAU * f64::from(k) / 24.;
                    (0.47 + 0.2 * a.cos(), 0.2 * a.sin())
                })
                .collect();
            revolve(&profile, 48, true, CLAY)?
        }
        Sample::Capsule => {
            let (r, half) = (0.34, 0.24);
            let mut profile = vec![(0., -half - r)];
            for k in 1..8 {
                let a = std::f64::consts::FRAC_PI_2 * f64::from(k) / 8.;
                profile.push((r * a.sin(), -half - r * a.cos()));
            }
            for k in 0..=8 {
                let a = std::f64::consts::FRAC_PI_2 * f64::from(k) / 8.;
                profile.push((r * a.cos(), half + r * a.sin()));
            }
            profile.push((0., half + r));
            profile.dedup();
            revolve(&profile, 48, false, CLAY)?
        }
        Sample::Ellipsoid => tagged(
            globe_geometry(
                DVec3::ZERO,
                RadialParameters {
                    radius: DVec3::new(0.62, 0.36, 0.62),
                    sides: 48,
                    segments: 24,
                },
            )?,
            CLAY,
        ),
        Sample::SelectedVertices => {
            let mut parts = vec![sample_mesh(Sample::Cube)?];
            for corner in visible_corners() {
                parts.push(ball(corner, 0.065, "Selected")?);
            }
            merge(parts)?
        }
        Sample::SelectedEdges => {
            let mut parts = vec![sample_mesh(Sample::Cube)?];
            let corners = visible_corners();
            for (i, &a) in corners.iter().enumerate() {
                for &b in &corners[i + 1..] {
                    // Cube edges join corners one axis apart.
                    if ((a - b).abs().element_sum() - 1.).abs() < 1e-9 {
                        parts.push(tube(&[a, b], 0.045, 12, "Selected")?);
                    }
                }
            }
            merge(parts)?
        }
        Sample::Bevel => {
            // The edge-bevel cut turned a quarter about Y: from top/front
            // to top/right. Swapping X and Z mirrors, so windings flip.
            let mut mesh = bevelled_cube(0.4)?;
            for v in &mut mesh.vertices {
                *v = DVec3::new(v.z, v.y, v.x);
            }
            for f in &mut mesh.faces {
                f.vertices.reverse();
            }
            mesh
        }
        Sample::Extrude => merge(vec![
            tagged_box(
                DVec3::new(-0.45, -0.5, -0.45),
                DVec3::new(0.45, 0., 0.45),
                CLAY,
                CLAY,
                CLAY,
            )?,
            tagged_box(
                DVec3::new(-0.45, 0., -0.45),
                DVec3::new(0.45, 0.5, 0.45),
                "Selected",
                "New",
                CLAY,
            )?,
        ])?,
        Sample::Spike => {
            let base = tagged_box(
                DVec3::new(-0.45, -0.5, -0.45),
                DVec3::new(0.45, 0.05, 0.45),
                CLAY,
                CLAY,
                CLAY,
            )?;
            let mut spike = revolve(&[(0., 0.05), (0.636, 0.05), (0., 0.8)], 4, false, "New")?;
            // Turn the four-sided cone so its base matches the box's top.
            for v in &mut spike.vertices {
                let (x, z) = (v.x, v.z);
                let c = std::f64::consts::FRAC_1_SQRT_2;
                *v = DVec3::new(c * (x - z), v.y, c * (x + z));
            }
            // Seen straight down the studio's diagonal a pyramid's point
            // faces the camera and its height vanishes (Jason, 2026-11-20:
            // "a more angled view"). The camera is shared, so the subject
            // tips instead: half way from +Y to the picture's up, so
            // the spike stands up in the picture and its sides show.
            let screen_up =
                (DVec3::Y - DVec3::ONE.normalize() * DVec3::ONE.normalize().y).normalize();
            let tip = glam::DQuat::IDENTITY
                .slerp(glam::DQuat::from_rotation_arc(DVec3::Y, screen_up), 0.5);
            let mut mesh = merge(vec![base, spike])?;
            for v in &mut mesh.vertices {
                *v = tip * *v;
            }
            mesh
        }
        Sample::Curve => {
            let points = [
                DVec3::new(-0.55, -0.45, 0.45),
                DVec3::new(-0.15, 0.4, 0.2),
                DVec3::new(0.15, -0.35, -0.15),
                DVec3::new(0.5, 0.45, -0.5),
            ];
            let mut parts = vec![tube(&through(&points, 16), 0.075, 12, "New")?];
            for p in points {
                parts.push(ball(p, 0.12, CLAY)?);
            }
            merge(parts)?
        }
        Sample::BSpline => {
            let cage = [
                DVec3::new(-0.55, -0.45, 0.45),
                DVec3::new(-0.35, 0.45, 0.35),
                DVec3::new(0.3, 0.45, -0.3),
                DVec3::new(0.5, -0.45, -0.5),
            ];
            let mut parts = vec![
                tube(&bspline(&cage, 16), 0.075, 12, "New")?,
                tube(&cage, 0.03, 8, CLAY)?,
            ];
            for p in cage {
                parts.push(ball(p, 0.1, CLAY)?);
            }
            merge(parts)?
        }
        Sample::Pen => {
            let ring: Vec<DVec3> = [
                (-0.55, 0.1),
                (-0.1, -0.55),
                (0.55, -0.35),
                (0.45, 0.4),
                (-0.2, 0.55),
            ]
            .iter()
            .map(|&(x, z)| DVec3::new(x, -0.05, z))
            .collect();
            let mut plate = Mesh {
                vertices: ring.clone(),
                ..Default::default()
            };
            let up = wound(&plate, (0..ring.len() as u32).collect(), DVec3::Y);
            plate.faces.push(polygon(up, CLAY));
            let mut parts = vec![plate];
            let n = ring.len();
            for i in 0..n - 1 {
                parts.push(tube(&[ring[i], ring[i + 1]], 0.035, 10, CLAY)?);
            }
            // The edge closing the polygon is the newest.
            parts.push(tube(&[ring[n - 1], ring[0]], 0.05, 10, "New")?);
            for p in &ring {
                parts.push(ball(*p, 0.09, CLAY)?);
            }
            // Tipped 15° toward the camera so the flat polygon reads
            // (Jason, 2026-11-20).
            let across = DVec3::new(1., 0., -1.).normalize();
            let tip = glam::DQuat::from_axis_angle(across, 15f64.to_radians());
            let mut mesh = merge(parts)?;
            for v in &mut mesh.vertices {
                *v = tip * *v;
            }
            mesh
        }
        Sample::Tube => {
            let points = [
                DVec3::new(-0.45, -0.45, 0.45),
                DVec3::new(-0.3, 0.3, 0.1),
                DVec3::new(0.25, 0.1, -0.2),
                DVec3::new(0.45, 0.45, -0.45),
            ];
            let mut parts = vec![tube(&through(&points, 16), 0.17, 24, CLAY)?];
            for p in points {
                parts.push(ball(p, 0.1, "New")?);
            }
            merge(parts)?
        }
        Sample::Arc => {
            // Stood up square to the camera: lying flat, the arc is seen
            // edge-on from above.
            let across = DVec3::new(1., 0., -1.).normalize();
            let centre = (across + DVec3::Y) * -0.35;
            let arc: Vec<DVec3> = (0..=6)
                .map(|k| {
                    let a = std::f64::consts::FRAC_PI_2 * f64::from(k) / 6.;
                    centre + (across * a.cos() + DVec3::Y * a.sin()) * 0.85
                })
                .collect();
            let mut parts = Vec::new();
            for w in arc.windows(2) {
                parts.push(tube(w, 0.045, 10, "New")?);
            }
            for p in &arc {
                parts.push(ball(*p, 0.08, "New")?);
            }
            parts.push(ball(centre, 0.07, "Ink")?);
            merge(parts)?
        }
        Sample::Bridge => {
            let block = |x0: f64, x1: f64| {
                tagged_box(
                    DVec3::new(x0, -0.5, -0.3),
                    DVec3::new(x1, 0.3, 0.3),
                    CLAY,
                    CLAY,
                    CLAY,
                )
            };
            merge(vec![
                block(-0.5, -0.15)?,
                block(0.15, 0.5)?,
                tagged_box(
                    DVec3::new(-0.15, -0.05, -0.3),
                    DVec3::new(0.15, 0.3, 0.3),
                    "New",
                    "New",
                    "New",
                )?,
            ])?
        }
        Sample::Thicken => tagged_box(
            DVec3::new(-0.5, -0.12, -0.5),
            DVec3::new(0.5, 0.12, 0.5),
            CLAY,
            "New",
            CLAY,
        )?,
        Sample::SmoothShift => merge(vec![
            tagged_box(
                DVec3::new(-0.5, -0.5, -0.5),
                DVec3::new(0.3, 0.3, 0.3),
                CLAY,
                CLAY,
                CLAY,
            )?,
            box_by_normal(
                DVec3::new(-0.5, 0.3, -0.5),
                DVec3::new(0.3, 0.5, 0.5),
                |n| {
                    if n == DVec3::Y { "Selected" } else { "New" }
                },
            )?,
            box_by_normal(
                DVec3::new(-0.5, -0.5, 0.3),
                DVec3::new(0.3, 0.5, 0.5),
                |n| {
                    if n == DVec3::Z { "Selected" } else { "New" }
                },
            )?,
        ])?,
        Sample::EdgeExtrude => {
            // A triangular ridge along the top right edge (+X, +Y, along Z).
            let o = 0.17;
            let mut ridge = Mesh::default();
            for z in [-0.5, 0.5] {
                ridge.vertices.push(DVec3::new(0.5, 0.5, z));
                ridge.vertices.push(DVec3::new(0.5 - 0.25, 0.5, z));
                ridge.vertices.push(DVec3::new(0.5 + o, 0.5 + o, z));
                ridge.vertices.push(DVec3::new(0.5, 0.5 - 0.25, z));
            }
            // Ends (−Z, +Z) and the two outer slopes.
            ridge.faces.push(polygon(vec![0, 1, 2, 3], "New"));
            ridge.faces.push(polygon(vec![7, 6, 5, 4], "New"));
            ridge.faces.push(polygon(vec![1, 5, 6, 2], "New"));
            ridge.faces.push(polygon(vec![2, 6, 7, 3], "New"));
            merge(vec![sample_mesh(Sample::Cube)?, ridge])?
        }
        Sample::EdgeChamfer => {
            // The edge-bevel cut moved from top/front to the vertical front
            // edge: swap X and Y (a mirror, so windings flip).
            let mut mesh = bevelled_cube(0.4)?;
            for v in &mut mesh.vertices {
                *v = DVec3::new(v.y, v.x, v.z);
            }
            for f in &mut mesh.faces {
                f.vertices.reverse();
            }
            mesh
        }
        Sample::VertexBevel => {
            // The corner nearest the camera, so the cut faces it.
            let corner = DVec3::splat(0.5);
            let (mut mesh, new) = cut_corner(sample_mesh(Sample::Cube)?, corner, 0.4)?;
            let tri = wound(&mesh, new, corner.normalize());
            mesh.faces.push(polygon(tri, "New"));
            mesh
        }
        Sample::VertexExtrude => {
            // The upper right corner, side-on, so the point's length shows.
            let corner = DVec3::new(0.5, 0.5, -0.5);
            let (mut mesh, new) = cut_corner(sample_mesh(Sample::Cube)?, corner, 0.35)?;
            let tri = wound(&mesh, new, corner.normalize());
            mesh.vertices.push(corner + corner.normalize() * 0.18);
            let apex = mesh.vertices.len() as u32 - 1;
            for i in 0..3 {
                mesh.faces
                    .push(polygon(vec![tri[i], tri[(i + 1) % 3], apex], "New"));
            }
            mesh
        }
        Sample::Inset => {
            let mut mesh = sample_mesh(Sample::Cube)?;
            let top = mesh
                .faces
                .iter()
                .position(|f| f.vertices.iter().all(|&v| mesh.vertices[v as usize].y > 0.))
                .ok_or("The cube has no top face")?;
            let outer = mesh.faces[top].vertices.clone();
            let start = mesh.vertices.len() as u32;
            for &v in &outer {
                let p = mesh.vertices[v as usize];
                mesh.vertices.push(DVec3::new(p.x * 0.55, p.y, p.z * 0.55));
            }
            let n = outer.len();
            mesh.faces[top].vertices = (start..start + n as u32).collect();
            mesh.faces[top].material = "Selected".into();
            for i in 0..n {
                let j = (i + 1) % n;
                mesh.faces.push(polygon(
                    vec![outer[i], outer[j], start + j as u32, start + i as u32],
                    CLAY,
                ));
            }
            mesh
        }
        Sample::Triple => {
            let mut parts = vec![sample_mesh(Sample::Cube)?];
            let h = 0.505;
            for (a, b) in [
                (DVec3::new(-h, h, -h), DVec3::new(h, h, h)),
                (DVec3::new(h, h, -h), DVec3::new(h, -h, h)),
                (DVec3::new(-h, h, h), DVec3::new(h, -h, h)),
            ] {
                parts.push(tube(&[a, b], 0.035, 8, "New")?);
            }
            merge(parts)?
        }
        Sample::MeshItem => cube_at(DVec3::ZERO, 0.85, CLAY)?,
        Sample::Scene => merge(vec![
            tagged_box(
                DVec3::new(-0.55, -0.5, -0.55),
                DVec3::new(0.55, -0.4, 0.55),
                CLAY,
                CLAY,
                CLAY,
            )?,
            cube_at(DVec3::new(0., -0.1, 0.), 0.6, CLAY)?,
        ])?,
        Sample::Copy => {
            // Side by side across the picture, the copy a little higher.
            let across = DVec3::new(1., 0., -1.).normalize();
            merge(vec![
                cube_at(across * -0.3 - DVec3::Y * 0.12, 0.48, CLAY)?,
                cube_at(across * 0.3 + DVec3::Y * 0.12, 0.48, "New")?,
            ])?
        }
        Sample::Clone => {
            let across = DVec3::new(1., 0., -1.).normalize();
            let mut parts = Vec::new();
            for (k, top) in [(-1., CLAY), (0., "New"), (1., "New")] {
                parts.push(cube_at(across * k * 0.5, 0.38, top)?);
            }
            merge(parts)?
        }
        Sample::Array => {
            let mut parts = Vec::new();
            for i in -1..=1 {
                for j in -1..=1 {
                    let top = if i == -1 && j == -1 { CLAY } else { "New" };
                    parts.push(cube_at(
                        DVec3::new(f64::from(i) * 0.36, -0.1, f64::from(j) * 0.36),
                        0.24,
                        top,
                    )?);
                }
            }
            merge(parts)?
        }
        Sample::RadialArray => {
            let mut parts = Vec::new();
            for k in 0..6 {
                let a = std::f64::consts::TAU * f64::from(k) / 6.;
                let top = if k == 0 { CLAY } else { "New" };
                parts.push(cube_at(
                    DVec3::new(0.48 * a.cos(), -0.1, 0.48 * a.sin()),
                    0.24,
                    top,
                )?);
            }
            merge(parts)?
        }
        Sample::Dicer => {
            let mut parts = Vec::new();
            for x in [-1., 1.] {
                for z in [-1., 1.] {
                    let c = DVec3::new(x * 0.26, 0., z * 0.26);
                    let h = DVec3::new(0.21, 0.45, 0.21);
                    parts.push(tagged_box(c - h, c + h, CLAY, CLAY, CLAY)?);
                }
            }
            merge(parts)?
        }
        Sample::Julienne => {
            let mut parts = Vec::new();
            for x in [-1., 0., 1.] {
                let c = DVec3::new(x * 0.36, 0., 0.);
                let h = DVec3::new(0.14, 0.45, 0.45);
                parts.push(tagged_box(c - h, c + h, CLAY, CLAY, CLAY)?);
            }
            merge(parts)?
        }
        Sample::Taper => deformed(column(0.4, 0.5, 8)?, |p| {
            let t = p.y + 0.5;
            let s = 1. - 0.6 * t;
            DVec3::new(p.x * s, p.y, p.z * s)
        }),
        Sample::Bulge => deformed(column(0.4, 0.5, 12)?, |p| {
            let t = p.y + 0.5;
            let s = 0.55 + 0.45 * (std::f64::consts::PI * t).sin();
            DVec3::new(p.x * s, p.y, p.z * s)
        }),
        Sample::Flare => deformed(column(0.45, 0.5, 10)?, |p| {
            let t = p.y + 0.5;
            let s = 0.45 + 0.55 * t * t;
            DVec3::new(p.x * s, p.y, p.z * s)
        }),
        Sample::Twist => deformed(column(0.3, 0.5, 16)?, |p| {
            let a = (p.y + 0.5) * std::f64::consts::FRAC_PI_2 * 1.4;
            DVec3::new(
                p.x * a.cos() - p.z * a.sin(),
                p.y,
                p.x * a.sin() + p.z * a.cos(),
            )
        }),
        Sample::Bend => {
            // A bar along its length (the column's Y) bent up a quarter turn
            // in the XY plane.
            let length = 1.4;
            let radius = length / std::f64::consts::FRAC_PI_2;
            // Bent in the picture's plane: X turned to point across it.
            let turn = glam::DQuat::from_rotation_y(std::f64::consts::FRAC_PI_4);
            deformed(column(0.17, length * 0.5, 16)?, |p| {
                let a = (p.y + length * 0.5) / radius;
                let (s, c) = a.sin_cos();
                let centre = DVec3::new(radius * s, radius * (1. - c), 0.);
                let normal = DVec3::new(-s, c, 0.);
                // The column's X becomes the offset across the bar.
                turn * (centre + normal * -p.x + DVec3::new(-0.45, -0.45, p.z))
            })
        }
        Sample::Shear => merge(vec![
            deformed(column(0.32, 0.4, 6)?, |p| {
                let t = p.y + 0.4;
                DVec3::new(p.x - 0.12 + t * 0.32, p.y - 0.08, p.z)
            }),
            arrow(
                DVec3::new(-0.05, 0.45, 0.),
                DVec3::new(0.55, 0.45, 0.),
                0.035,
                "Axis X",
            )?,
        ])?,
        Sample::Text => {
            let bar = tagged_box(
                DVec3::new(-0.45, 0.22, -0.13),
                DVec3::new(0.45, 0.5, 0.13),
                CLAY,
                CLAY,
                CLAY,
            )?;
            let stem = tagged_box(
                DVec3::new(-0.14, -0.5, -0.13),
                DVec3::new(0.14, 0.22, 0.13),
                CLAY,
                CLAY,
                CLAY,
            )?;
            // Turned to face the camera.
            let turn = glam::DQuat::from_rotation_y(std::f64::consts::FRAC_PI_4);
            deformed(merge(vec![bar, stem])?, |p| turn * p)
        }
        Sample::TwistBevel => {
            let base = tagged_box(
                DVec3::new(-0.42, -0.5, -0.42),
                DVec3::new(0.42, -0.05, 0.42),
                CLAY,
                CLAY,
                CLAY,
            )?;
            let path: Vec<DVec3> = (0..=24)
                .map(|k| {
                    let t = f64::from(k) / 24.;
                    let a = t * std::f64::consts::PI * 1.3;
                    DVec3::new(0.22 * t * a.cos(), -0.05 + 0.55 * t, 0.22 * t * a.sin())
                })
                .collect();
            let tip = *path.last().ok_or("empty path")?;
            merge(vec![
                base,
                tube_with(&path, |t| 0.26 * (1. - 0.6 * t), 24, "New")?,
                ball(tip, 0.11, "Selected")?,
            ])?
        }
        Sample::PolyHaul => merge(vec![
            tagged_box(
                DVec3::new(-0.45, -0.5, -0.45),
                DVec3::new(0.45, 0.15, 0.45),
                "Selected",
                CLAY,
                CLAY,
            )?,
            arrow(
                DVec3::new(0., 0.15, 0.),
                DVec3::new(0., 0.62, 0.),
                0.05,
                "New",
            )?,
        ])?,
        Sample::Shift => {
            // The cube without its top and front, and those two faces as
            // plates moved out.
            let mut open = sample_mesh(Sample::Cube)?;
            open.faces.retain(|f| {
                let c: DVec3 = f
                    .vertices
                    .iter()
                    .map(|&v| open.vertices[v as usize])
                    .sum::<DVec3>()
                    / f.vertices.len() as f64;
                c.y < 0.4 && c.z < 0.4
            });
            let plate = |min: DVec3, max: DVec3, out: DVec3| {
                box_by_normal(min, max, move |n| if n == out { "Selected" } else { "New" })
            };
            // Shrunk so the moved-out plates stay inside the picture.
            let shifted = merge(vec![
                open,
                plate(
                    DVec3::new(-0.5, 0.78, -0.5),
                    DVec3::new(0.5, 0.84, 0.5),
                    DVec3::Y,
                )?,
                plate(
                    DVec3::new(-0.5, -0.5, 0.78),
                    DVec3::new(0.5, 0.5, 0.84),
                    DVec3::Z,
                )?,
            ])?;
            deformed(shifted, |v| v * 0.72)
        }
        Sample::ScaleOffset => {
            let frustum = revolve(
                &[(0., -0.5), (0.707, -0.5), (0.4, 0.5), (0., 0.5)],
                4,
                false,
                CLAY,
            )?;
            let c = std::f64::consts::FRAC_1_SQRT_2;
            let mut mesh = deformed(frustum, |v| {
                DVec3::new(c * (v.x - v.z), v.y, c * (v.x + v.z))
            });
            for i in 0..mesh.faces.len() {
                if mesh.faces[i]
                    .vertices
                    .iter()
                    .all(|&v| mesh.vertices[v as usize].y > 0.4)
                {
                    mesh.faces[i].material = "Selected".into();
                }
            }
            mesh
        }
        Sample::ScaleOffsetDetach => {
            let mut parts = vec![tagged_box(
                DVec3::new(-0.45, -0.5, -0.45),
                DVec3::new(0.45, 0.1, 0.45),
                CLAY,
                CLAY,
                CLAY,
            )?];
            let h = 0.24;
            parts.push(box_by_normal(
                DVec3::new(-h, 0.36, -h),
                DVec3::new(h, 0.4, h),
                |n| if n == DVec3::Y { "Selected" } else { CLAY },
            )?);
            for (x, z) in [(-h, -h), (h, -h), (h, h), (-h, h)] {
                parts.push(ball(DVec3::new(x, 0.4, z), 0.07, "New")?);
            }
            merge(parts)?
        }
        Sample::AddLoop => {
            let r = 0.505;
            let square = [
                DVec3::new(-r, 0., -r),
                DVec3::new(r, 0., -r),
                DVec3::new(r, 0., r),
                DVec3::new(-r, 0., r),
                DVec3::new(-r, 0., -r),
            ];
            let mut parts = vec![sample_mesh(Sample::Cube)?];
            for w in square.windows(2) {
                parts.push(tube(w, 0.04, 8, "New")?);
            }
            merge(parts)?
        }
        Sample::AddPoint => merge(vec![
            sample_mesh(Sample::Cube)?,
            ball(DVec3::new(0., 0.5, 0.5), 0.11, "New")?,
        ])?,
        Sample::EdgeExtend => {
            let edge = 0.12;
            merge(vec![
                tagged_box(
                    DVec3::new(-0.5, -0.04, -0.5),
                    DVec3::new(edge, 0.04, 0.5),
                    CLAY,
                    CLAY,
                    CLAY,
                )?,
                tagged_box(
                    DVec3::new(edge, -0.04, -0.5),
                    DVec3::new(0.5, 0.04, 0.5),
                    "New",
                    "New",
                    "New",
                )?,
                tube(
                    &[DVec3::new(edge, 0.05, -0.5), DVec3::new(edge, 0.05, 0.5)],
                    0.04,
                    10,
                    "Selected",
                )?,
            ])?
        }
        Sample::EdgeCrease => {
            let h = 0.505;
            let mut parts = vec![
                sample_mesh(Sample::Cube)?,
                tube(
                    &[DVec3::new(h, -h, h), DVec3::new(h, h, h)],
                    0.05,
                    10,
                    "Selected",
                )?,
            ];
            // Short cyan ticks across the edge.
            let out = DVec3::new(1., 0., 1.).normalize();
            let side = DVec3::new(1., 0., -1.).normalize();
            for y in [-0.25, 0., 0.25] {
                let c = DVec3::new(h, y, h) + out * 0.02;
                parts.push(tube(&[c - side * 0.16, c + side * 0.16], 0.03, 8, "New")?);
            }
            merge(parts)?
        }
        Sample::VertexTear => {
            let corner = DVec3::new(0.5, 0.5, -0.5);
            let cut = 0.4;
            let (mut mesh, new) = cut_corner(sample_mesh(Sample::Cube)?, corner, cut)?;
            let tri = wound(&mesh, new.clone(), corner.normalize());
            mesh.faces.push(polygon(tri, CLAY));
            // The torn-off piece: the corner and its three cut points, lifted.
            let lift = DVec3::new(0., 0.16, 0.) + corner.normalize() * 0.05;
            let mut piece = Mesh::default();
            piece.vertices.push(corner + lift);
            for &i in &new {
                piece.vertices.push(mesh.vertices[i as usize] + lift);
            }
            let base = wound(&piece, vec![1, 2, 3], -corner.normalize());
            piece.faces.push(polygon(base.clone(), "Selected"));
            for i in 0..3 {
                let (a, b) = (base[i], base[(i + 1) % 3]);
                piece.faces.push(polygon(vec![b, a, 0], "Selected"));
            }
            merge(vec![mesh, piece])?
        }
        Sample::Merge => {
            let across = DVec3::new(1., 0., -1.).normalize();
            merge(vec![
                cube_at(-across * 0.27, 0.5, CLAY)?,
                cube_at(across * 0.27, 0.5, CLAY)?,
                deformed(
                    tagged_box(
                        DVec3::new(-0.03, -0.24, -0.24),
                        DVec3::new(0.03, 0.24, 0.24),
                        "New",
                        "New",
                        "New",
                    )?,
                    // Square to `across`: X turned onto it.
                    |v| glam::DQuat::from_rotation_y(std::f64::consts::FRAC_PI_4) * v,
                ),
                arrow(
                    -across * 0.75 + DVec3::Y * 0.45,
                    -across * 0.3 + DVec3::Y * 0.45,
                    0.025,
                    "Ink",
                )?,
                arrow(
                    across * 0.75 + DVec3::Y * 0.45,
                    across * 0.3 + DVec3::Y * 0.45,
                    0.025,
                    "Ink",
                )?,
            ])?
        }
        Sample::Unmerge => {
            let across = DVec3::new(1., 0., -1.).normalize();
            merge(vec![
                cube_at(-across * 0.32 - DVec3::Y * 0.1, 0.45, CLAY)?,
                cube_at(across * 0.32 - DVec3::Y * 0.1, 0.45, CLAY)?,
                arrow(
                    DVec3::Y * 0.42 - across * 0.08,
                    DVec3::Y * 0.42 - across * 0.6,
                    0.025,
                    "Ink",
                )?,
                arrow(
                    DVec3::Y * 0.42 + across * 0.08,
                    DVec3::Y * 0.42 + across * 0.6,
                    0.025,
                    "Ink",
                )?,
            ])?
        }
        Sample::GroundAlign => merge(vec![
            tagged_box(
                DVec3::new(-0.55, -0.5, -0.55),
                DVec3::new(0.55, -0.44, 0.55),
                CLAY,
                CLAY,
                CLAY,
            )?,
            tagged_box(
                DVec3::new(-0.25, -0.44, -0.25),
                DVec3::new(0.25, -0.42, 0.25),
                "New",
                "New",
                "New",
            )?,
            box_by_normal(
                DVec3::new(-0.25, 0.05, -0.25),
                DVec3::new(0.25, 0.55, 0.25),
                |_| "Selected",
            )?,
            arrow(
                DVec3::new(0., 0.02, 0.),
                DVec3::new(0., -0.34, 0.),
                0.035,
                "Ink",
            )?,
        ])?,
        Sample::Mirror => {
            // A step block on −X, its mirror on +X with cyan tops, and the
            // mirror plane between.
            let step = |sign: f64, top: &'static str| -> Result<Mesh> {
                let lo = |x0: f64, x1: f64| (x0.min(x1), x0.max(x1));
                let (a0, a1) = lo(sign * 0.08, sign * 0.5);
                let (b0, b1) = lo(sign * 0.08, sign * 0.28);
                merge(vec![
                    tagged_box(
                        DVec3::new(a0, -0.5, -0.35),
                        DVec3::new(a1, -0.1, 0.35),
                        top,
                        CLAY,
                        CLAY,
                    )?,
                    tagged_box(
                        DVec3::new(b0, -0.1, -0.35),
                        DVec3::new(b1, 0.3, 0.35),
                        top,
                        CLAY,
                        CLAY,
                    )?,
                ])
            };
            // Turned so the mirror plane is square to the picture.
            let turn = glam::DQuat::from_rotation_y(std::f64::consts::FRAC_PI_4);
            let both = merge(vec![
                step(-1., CLAY)?,
                step(1., "New")?,
                tagged_box(
                    DVec3::new(-0.015, -0.55, -0.5),
                    DVec3::new(0.015, 0.45, 0.5),
                    "New",
                    "New",
                    "New",
                )?,
            ])?;
            deformed(both, |v| turn * v)
        }
        Sample::Boolean => merge(vec![
            tagged_box(
                DVec3::new(-0.45, -0.5, -0.45),
                DVec3::new(0.45, 0.2, 0.45),
                CLAY,
                CLAY,
                CLAY,
            )?,
            ball(DVec3::new(0.05, 0.22, 0.05), 0.36, "New")?,
        ])?,
        Sample::SketchExtrude => {
            let path = through(&sweep_points(), 16);
            let mut parts = curve_parts(&path, &[], &[])?;
            parts.push(profile_plate(&path)?);
            merge(parts)?
        }
        Sample::CurveExtrude => {
            let pts = sweep_points();
            let path = through(&pts, 16);
            let mut parts = curve_parts(&path, &pts[1..], &[])?;
            parts.push(profile_plate(&path)?);
            merge(parts)?
        }
        Sample::BezierExtrude => {
            let pts = sweep_points();
            let path = through(&[pts[0], pts[2], pts[3]], 20);
            let tangent = |i: usize| (path[i + 1] - path[i]).normalize() * 0.2;
            let last = path.len() - 2;
            let mut parts = curve_parts(&path, &[pts[3]], &[(pts[3], tangent(last))])?;
            parts.push(profile_plate(&path)?);
            merge(parts)?
        }
        Sample::BsplineExtrude => {
            let pts = sweep_points();
            let path = bspline(&pts, 16);
            let mut parts = curve_parts(&path, &pts[1..], &[])?;
            parts.push(tube(&pts, 0.025, 8, CLAY)?);
            parts.push(profile_plate(&path)?);
            merge(parts)?
        }
        Sample::PenExtrude => {
            let pts = sweep_points();
            let mut parts = curve_parts(&pts, &pts[1..], &[])?;
            parts.push(profile_plate(&pts)?);
            merge(parts)?
        }
        Sample::CurveClone => {
            let pts = clone_points();
            let path = through(&pts, 16);
            let mut parts = curve_parts(&path, &pts, &[])?;
            parts.extend(cubes_along(&path, 4)?);
            merge(parts)?
        }
        Sample::BezierClone => {
            let pts = clone_points();
            let path = through(&[pts[0], pts[2], pts[3]], 20);
            let (a, b) = (path[0], path[path.len() - 1]);
            let ta = (path[1] - path[0]).normalize() * 0.18;
            let tb = (path[path.len() - 1] - path[path.len() - 2]).normalize() * 0.18;
            let mut parts = curve_parts(&path, &[], &[(a, -ta), (b, tb)])?;
            parts.extend(cubes_along(&path, 4)?);
            merge(parts)?
        }
        Sample::BsplineClone => {
            let pts = clone_points();
            let path = bspline(&pts, 16);
            let mut parts = curve_parts(&path, &pts, &[])?;
            parts.push(tube(&pts, 0.025, 8, CLAY)?);
            parts.extend(cubes_along(&path, 4)?);
            merge(parts)?
        }
        Sample::PenClone => {
            let pts = clone_points();
            let mut parts = curve_parts(&pts, &pts, &[])?;
            parts.extend(cubes_along(&pts, 4)?);
            merge(parts)?
        }
        Sample::ScatterClone => {
            let mut parts = vec![tagged_box(
                DVec3::new(-0.55, -0.5, -0.55),
                DVec3::new(0.55, -0.44, 0.55),
                CLAY,
                CLAY,
                CLAY,
            )?];
            for (k, (x, z)) in [
                (-0.3, 0.25),
                (0.05, 0.35),
                (0.3, -0.05),
                (-0.15, -0.2),
                (0.15, -0.38),
            ]
            .into_iter()
            .enumerate()
            {
                parts.push(cube_at(
                    DVec3::new(x, -0.34, z),
                    0.2,
                    if k == 0 { CLAY } else { "New" },
                )?);
            }
            merge(parts)?
        }
        Sample::Chamfer => {
            // The rounded edge turned from top/front to top/right, as Bevel.
            let mesh = rounded_cube(0.42, 4)?;
            let mut mesh = deformed(mesh, |v| DVec3::new(v.z, v.y, v.x));
            for f in &mut mesh.faces {
                f.vertices.reverse();
            }
            mesh
        }
        Sample::EdgeSlide => merge(vec![
            sample_mesh(Sample::Cube)?,
            tube(
                &[DVec3::new(-0.5, 0.505, 0.), DVec3::new(0.5, 0.505, 0.)],
                0.045,
                10,
                "Selected",
            )?,
            two_way(
                DVec3::new(0., 0.56, -0.42),
                DVec3::new(0., 0.56, 0.42),
                0.03,
                "Ink",
            )?,
        ])?,
        Sample::EdgeRelax => {
            let jagged: Vec<DVec3> = (0..=6)
                .map(|k| {
                    let t = f64::from(k) / 6.;
                    let wobble = if k % 2 == 0 { 0.18 } else { -0.18 };
                    DVec3::new(-0.55 + 1.1 * t, -0.15 + wobble, 0.45 - 0.9 * t)
                })
                .collect();
            let smooth: Vec<DVec3> = (0..=24)
                .map(|k| {
                    let t = f64::from(k) / 24.;
                    DVec3::new(-0.55 + 1.1 * t, 0.25 + 0.1 * (t * 6.).sin(), 0.45 - 0.9 * t)
                })
                .collect();
            let mut parts = vec![
                tube(&jagged, 0.03, 8, CLAY)?,
                tube(&smooth, 0.055, 12, "New")?,
            ];
            for p in &jagged {
                parts.push(ball(*p, 0.06, CLAY)?);
            }
            merge(parts)?
        }
        Sample::VertexMerge => {
            let target = DVec3::new(0., 0.5, 0.);
            let mut parts = vec![sample_mesh(Sample::Cube)?, ball(target, 0.13, "New")?];
            for p in [
                DVec3::new(-0.4, 0.5, 0.35),
                DVec3::new(0.35, 0.5, 0.35),
                DVec3::new(0.35, 0.5, -0.4),
            ] {
                parts.push(ball(p, 0.12, "Selected")?);
                parts.push(arrow(
                    p + (target - p) * 0.25,
                    p + (target - p) * 0.7,
                    0.035,
                    "Ink",
                )?);
            }
            merge(parts)?
        }
        Sample::TopologyPen => {
            // Three flat quads along X (clay), then the new quad (cyan)
            // pulled out of the end edge and lifted a little, as if still
            // on the drag; its two new points are the yellow dots.
            let t = 0.06;
            let tile = |x0: f64, x1: f64, z0: f64, z1: f64, y: f64, tag: &str| {
                tagged_box(
                    DVec3::new(x0, y - t, z0),
                    DVec3::new(x1, y + t, z1),
                    tag,
                    tag,
                    tag,
                )
            };
            let (y, y_new) = (-0.1, 0.1);
            merge(vec![
                tile(-0.8, -0.32, -0.62, -0.14, y, CLAY)?,
                tile(-0.29, 0.19, -0.62, -0.14, y, CLAY)?,
                tile(-0.8, -0.32, -0.11, 0.37, y, CLAY)?,
                tile(-0.29, 0.19, -0.11, 0.37, y, CLAY)?,
                tile(0.25, 0.73, -0.11, 0.37, y_new, "New")?,
                ball(DVec3::new(0.73, y_new + t, -0.11), 0.13, "Selected")?,
                ball(DVec3::new(0.73, y_new + t, 0.37), 0.13, "Selected")?,
            ])?
        }
        Sample::VertexWeld => {
            let corner = DVec3::splat(0.5);
            let from = DVec3::new(0.5, 0.5, -0.25);
            merge(vec![
                sample_mesh(Sample::Cube)?,
                ball(from, 0.13, "Selected")?,
                ball(corner, 0.1, "New")?,
                arrow(
                    from + DVec3::Z * 0.15,
                    corner - DVec3::Z * 0.13,
                    0.04,
                    "Ink",
                )?,
            ])?
        }
        Sample::VertexSlide => {
            let h = 0.5;
            merge(vec![
                sample_mesh(Sample::Cube)?,
                ball(DVec3::new(0., h, h), 0.13, "Selected")?,
                two_way(
                    DVec3::new(-0.45, h + 0.14, h + 0.14),
                    DVec3::new(0.45, h + 0.14, h + 0.14),
                    0.04,
                    "Ink",
                )?,
            ])?
        }
        Sample::Detriangulate => {
            let mut parts = vec![sample_mesh(Sample::Cube)?];
            let h = 0.505;
            // Each visible face's diagonal as a fading dashed line.
            for (a, b) in [
                (DVec3::new(-h, h, -h), DVec3::new(h, h, h)),
                (DVec3::new(h, h, -h), DVec3::new(h, -h, h)),
                (DVec3::new(-h, h, h), DVec3::new(h, -h, h)),
            ] {
                for k in [0., 2., 4.] {
                    let (t0, t1) = (k / 6. + 0.05, (k + 1.) / 6. + 0.05);
                    parts.push(tube(&[a.lerp(b, t0), a.lerp(b, t1)], 0.025, 8, "Ink")?);
                }
            }
            merge(parts)?
        }
        Sample::LinkIndependent => {
            // Broken: the two links apart along the chain's line.
            merge(vec![
                chain_link((-0.5, -0.3), (1., 0.55), 0.95, 0.56, 0.)?,
                chain_link((0.5, 0.3), (1., 0.55), 0.95, 0.56, 0.)?,
            ])?
        }
        Sample::LinkCopy => merge(vec![
            chain_link((-0.29, -0.29), (1., 1.), 1.15, 0.62, 0.)?,
            chain_link((0.29, 0.29), (1., 1.), 1.15, 0.62, 70.)?,
        ])?,
        Sample::LinkProportional => merge(vec![
            chain_link((-0.27, -0.27), (1., 1.), 1.35, 0.74, 0.)?,
            chain_link((0.16, 0.16), (1., 1.), 0.62, 0.36, 70.)?,
        ])?,
        Sample::LinkRelative => {
            let mut parts = vec![
                chain_link((-0.1, -0.4), (1., 1.), 1.0, 0.56, 0.)?,
                chain_link((0.41, 0.11), (1., 1.), 1.0, 0.56, 70.)?,
            ];
            // The plus, new geometry, in the free top-left corner.
            let (right, up) = screen_axes();
            let c = right * -0.52 + up * 0.46;
            let arm = 0.27;
            parts.push(tube(&[c - right * arm, c + right * arm], 0.09, 12, "New")?);
            parts.push(tube(&[c - up * arm, c + up * arm], 0.09, 12, "New")?);
            merge(parts)?
        }
        Sample::Symmetrize => {
            // A round body cut at X = 0: the kept +X half Clay, the mirrored
            // −X half cyan, and the welded seam ring selected. (Mirror shows
            // two separate copies and a plane; this shows one welded whole.)
            let mut body = crate::radial::globe_geometry(
                DVec3::ZERO,
                crate::radial::RadialParameters {
                    radius: DVec3::new(0.5, 0.58, 0.5),
                    sides: 48,
                    segments: 24,
                },
            )?;
            for i in 0..body.faces.len() {
                let x: f64 = body.faces[i]
                    .vertices
                    .iter()
                    .map(|&v| body.vertices[v as usize].x)
                    .sum();
                body.faces[i].material = if x >= 0. { CLAY } else { "New" }.into();
            }
            let ring: Vec<DVec3> = (0..=48)
                .map(|k| {
                    let a = std::f64::consts::TAU * f64::from(k) / 48.;
                    DVec3::new(0., 0.585 * a.sin(), 0.505 * a.cos())
                })
                .collect();
            // Turned most of the way to put the cut square to the picture,
            // so both halves show and the seam reads as a narrow ring.
            let turn = glam::DQuat::from_rotation_y(35f64.to_radians());
            let all = merge(vec![body, tube(&ring, 0.035, 10, "Selected")?])?;
            deformed(all, |v| turn * v)
        }
        Sample::FixBlackDots => {
            let ring: Vec<DVec3> = (0..=32)
                .map(|k| {
                    let a = std::f64::consts::TAU * f64::from(k) / 32.;
                    DVec3::new(0.3 * a.cos(), -0.02, 0.3 * a.sin())
                })
                .collect();
            tipped(
                merge(vec![
                    plate(0.55)?,
                    ball(DVec3::new(0., -0.08, 0.), 0.14, "Ink")?,
                    tube(&ring, 0.045, 10, "New")?,
                ])?,
                25.,
            )
        }
        Sample::EdgeZip => {
            let line = |z: f64, tag: &str, r: f64| {
                tube(
                    &[DVec3::new(-0.5, -0.03, z), DVec3::new(0.5, -0.03, z)],
                    r,
                    10,
                    tag,
                )
            };
            let mut parts = vec![
                plate(0.55)?,
                line(0., "Selected", 0.045)?,
                line(0.38, CLAY, 0.03)?,
                line(-0.38, CLAY, 0.03)?,
            ];
            for x in [-0.25, 0.25] {
                for side in [1., -1.] {
                    parts.push(arrow(
                        DVec3::new(x, 0.05, 0.36 * side),
                        DVec3::new(x, 0.05, 0.08 * side),
                        0.045,
                        "New",
                    )?);
                }
            }
            tipped(merge(parts)?, 25.)
        }
        Sample::DiagonalRedirect => {
            let mut parts = vec![plate(0.55)?];
            // Two diagonal cells of a 2x2 grid selected, meeting at the centre.
            for (x0, z0) in [(-0.5, -0.5), (0., 0.)] {
                parts.push(box_by_normal(
                    DVec3::new(x0 + 0.02, -0.05, z0 + 0.02),
                    DVec3::new(x0 + 0.48, -0.03, z0 + 0.48),
                    |n| if n == DVec3::Y { "Selected" } else { CLAY },
                )?);
            }
            for t in [-0.5, 0., 0.5] {
                parts.push(tube(
                    &[DVec3::new(t, -0.02, -0.5), DVec3::new(t, -0.02, 0.5)],
                    0.02,
                    8,
                    CLAY,
                )?);
                parts.push(tube(
                    &[DVec3::new(-0.5, -0.02, t), DVec3::new(0.5, -0.02, t)],
                    0.02,
                    8,
                    CLAY,
                )?);
            }
            parts.push(tube(
                &[DVec3::new(-0.5, 0., 0.5), DVec3::new(0.5, 0., -0.5)],
                0.045,
                10,
                "New",
            )?);
            tipped(merge(parts)?, 25.)
        }
        Sample::Contour => {
            let ring: Vec<DVec3> = (0..=48)
                .map(|k| {
                    let a = std::f64::consts::TAU * f64::from(k) / 48.;
                    let p = DVec3::new(0.53 * a.cos(), 0., 0.53 * a.sin());
                    glam::DQuat::from_axis_angle(DVec3::new(1., 0., -1.).normalize(), -0.45) * p
                })
                .collect();
            merge(vec![
                ball(DVec3::ZERO, 0.5, CLAY)?,
                tube(&ring, 0.045, 10, "New")?,
            ])?
        }
        Sample::MeshPaint => {
            let mut parts = vec![plate(0.55)?];
            for (x, z) in [
                (-0.18, 0.12),
                (0.12, 0.2),
                (0.15, -0.15),
                (-0.12, -0.18),
                (0., 0.),
            ] {
                parts.push(ball(DVec3::new(x, 0.02, z), 0.1, "New")?);
            }
            let ring: Vec<DVec3> = (0..=40)
                .map(|k| {
                    let a = std::f64::consts::TAU * f64::from(k) / 40.;
                    DVec3::new(0.38 * a.cos(), -0.02, 0.38 * a.sin())
                })
                .collect();
            parts.push(tube(&ring, 0.03, 8, "Selected")?);
            tipped(merge(parts)?, 25.)
        }
        Sample::MeshPaintInstance => {
            let source = DVec3::new(-0.15, 0.45, -0.15);
            let mut parts = vec![plate(0.55)?, ball(source, 0.13, "Selected")?];
            for (x, z) in [(-0.25, 0.25), (0.2, 0.2), (0.25, -0.25)] {
                let p = DVec3::new(x, 0.02, z);
                parts.push(ball(p, 0.1, "New")?);
                // A dashed link from the source: instances, not copies.
                for k in [0., 2., 4.] {
                    let (t0, t1) = (0.15 + k * 0.12, 0.15 + (k + 1.) * 0.12);
                    parts.push(tube(
                        &[source.lerp(p, t0), source.lerp(p, t1)],
                        0.018,
                        6,
                        "Ink",
                    )?);
                }
            }
            tipped(merge(parts)?, 25.)
        }
        Sample::ReplicaClone => styled(Sample::Clone, CopyStyle::Replica)?,
        Sample::CurveReplica => styled(Sample::CurveClone, CopyStyle::Replica)?,
        Sample::BezierReplica => styled(Sample::BezierClone, CopyStyle::Replica)?,
        Sample::BsplineReplica => styled(Sample::BsplineClone, CopyStyle::Replica)?,
        Sample::PenReplicate => styled(Sample::PenClone, CopyStyle::Replica)?,
        Sample::ReplicaScatter => styled(Sample::ScatterClone, CopyStyle::Replica)?,
        Sample::ReplicaArray => styled(Sample::Array, CopyStyle::Replica)?,
        Sample::ReplicaRadialArray => styled(Sample::RadialArray, CopyStyle::Replica)?,
        Sample::ReplicaMirror => styled(Sample::Mirror, CopyStyle::Replica)?,
        Sample::MeshPaintReplica => styled(Sample::MeshPaint, CopyStyle::Replica)?,
        Sample::InstanceClone => styled(Sample::Clone, CopyStyle::Instance)?,
        Sample::CurveInstance => styled(Sample::CurveClone, CopyStyle::Instance)?,
        Sample::BezierInstance => styled(Sample::BezierClone, CopyStyle::Instance)?,
        Sample::BsplineInstance => styled(Sample::BsplineClone, CopyStyle::Instance)?,
        Sample::PenInstance => styled(Sample::PenClone, CopyStyle::Instance)?,
        Sample::InstanceScatter => styled(Sample::ScatterClone, CopyStyle::Instance)?,
        Sample::InstanceArray => styled(Sample::Array, CopyStyle::Instance)?,
        Sample::InstanceRadialArray => styled(Sample::RadialArray, CopyStyle::Instance)?,
        Sample::InstanceMirror => styled(Sample::Mirror, CopyStyle::Instance)?,
        Sample::VertexNormalTools => {
            let mut parts = vec![ball(DVec3::ZERO, 0.42, CLAY)?];
            for d in [
                DVec3::new(0., 1., 0.),
                DVec3::new(1., 0.4, 0.3),
                DVec3::new(0.3, 0.4, 1.),
                DVec3::new(0.8, -0.5, 0.8),
                DVec3::new(-0.6, 0.6, 0.6),
            ] {
                let d = d.normalize();
                parts.push(arrow(d * 0.42, d * 0.85, 0.045, "New")?);
            }
            merge(parts)?
        }
        Sample::RecalculateNormals => {
            let mut parts = vec![cube_at(DVec3::new(0., -0.12, 0.), 0.62, CLAY)?];
            let c = DVec3::new(0., -0.12, 0.);
            for d in [DVec3::Y, DVec3::X, DVec3::Z] {
                parts.push(arrow(c + d * 0.31, c + d * 0.85, 0.05, "New")?);
            }
            merge(parts)?
        }
        Sample::HardenNormals => {
            let h = 0.32;
            let c = DVec3::new(-0.1, -0.1, -0.1);
            let mut parts = vec![
                cube_at(c, 0.64, CLAY)?,
                tube(
                    &[c + DVec3::new(h, -h, h), c + DVec3::new(h, h, h)],
                    0.04,
                    10,
                    "Selected",
                )?,
            ];
            // At the edge's middle, each face's own normal: split, not shared.
            let mid = c + DVec3::new(h, 0., h);
            parts.push(arrow(
                mid + DVec3::X * 0.02,
                mid + DVec3::X * 0.6,
                0.045,
                "New",
            )?);
            parts.push(arrow(
                mid + DVec3::Z * 0.02,
                mid + DVec3::Z * 0.6,
                0.045,
                "New",
            )?);
            merge(parts)?
        }
        Sample::SoftenNormals => {
            let mesh = rounded_cube(0.42, 5)?;
            let mut mesh = deformed(mesh, |v| v * 0.7 - DVec3::splat(0.08));
            for f in &mut mesh.faces {
                if f.material == "New" {
                    f.material = CLAY.into();
                }
            }
            // One shared normal across the rounded top-front edge.
            let d = DVec3::new(0., 1., 1.).normalize();
            let from = DVec3::new(0., 0.35, 0.35) * 0.7 - DVec3::splat(0.08) + d * 0.08;
            merge(vec![mesh, arrow(from, from + d * 0.62, 0.05, "New")?])?
        }
        Sample::SelectHardEdges => {
            let h = 0.505;
            let mut parts = vec![sample_mesh(Sample::Cube)?];
            for (a, b) in [
                (DVec3::new(h, -h, h), DVec3::new(h, h, h)),
                (DVec3::new(-h, h, h), DVec3::new(h, h, h)),
                (DVec3::new(h, h, -h), DVec3::new(h, h, h)),
            ] {
                parts.push(tube(&[a, b], 0.05, 10, "Selected")?);
            }
            merge(parts)?
        }
        Sample::TransferNormals => {
            let src = DVec3::new(-0.3, 0.25, -0.3);
            let mut parts = vec![ball(src, 0.28, CLAY)?, plate(0.5)?];
            for (x, z) in [(-0.15, 0.3), (0.2, 0.2), (0.3, -0.15)] {
                let p = DVec3::new(x, -0.05, z);
                let d = (p - src).normalize();
                parts.push(arrow(
                    p,
                    p + (d + DVec3::Y).normalize() * 0.45,
                    0.04,
                    "New",
                )?);
            }
            parts.push(arrow(
                src + DVec3::new(0.25, -0.05, 0.25),
                DVec3::new(0.1, 0.12, 0.1),
                0.025,
                "Ink",
            )?);
            merge(parts)?
        }
        Sample::SpinVertexIndices => {
            let mut parts = vec![box_by_normal(
                DVec3::new(-0.45, -0.12, -0.45),
                DVec3::new(0.45, -0.04, 0.45),
                |n| if n == DVec3::Y { "Selected" } else { CLAY },
            )?];
            for (x, z) in [(-0.45, -0.45), (0.45, -0.45), (0.45, 0.45), (-0.45, 0.45)] {
                parts.push(ball(DVec3::new(x, -0.04, z), 0.07, "Ink")?);
            }
            let arc: Vec<DVec3> = (0..=24)
                .map(|k| {
                    let a = std::f64::consts::TAU * 0.75 * f64::from(k) / 24.;
                    DVec3::new(0.26 * a.cos(), 0.05, 0.26 * a.sin())
                })
                .collect();
            let n = arc.len();
            parts.push(tube(&arc[..n - 2], 0.04, 10, "New")?);
            parts.push(arrow(
                arc[n - 3],
                arc[n - 1] + (arc[n - 1] - arc[n - 3]),
                0.04,
                "New",
            )?);
            tipped(merge(parts)?, 25.)
        }
        Sample::Guides => {
            let mut parts = vec![plate(0.55)?];
            for (a, b) in [
                (DVec3::new(-0.6, 0., 0.15), DVec3::new(0.6, 0., 0.15)),
                (DVec3::new(-0.2, 0., -0.6), DVec3::new(-0.2, 0., 0.6)),
            ] {
                parts.push(tube(&[a, b], 0.03, 8, "New")?);
            }
            parts.push(ball(DVec3::new(0.25, 0., 0.15), 0.08, "Selected")?);
            tipped(merge(parts)?, 25.)
        }
        Sample::TimelineClock => {
            let mut face = revolve(
                &[(0., -0.06), (0.55, -0.06), (0.55, 0.06), (0., 0.06)],
                48,
                false,
                CLAY,
            )?;
            // Turn the disc's axis (Y) out of the symbol plane (Z).
            let lay = glam::DQuat::from_rotation_x(std::f64::consts::FRAC_PI_2);
            face = deformed(face, |v| lay * v);
            let mut hands = vec![
                bar(0., 0., 0., 0.38, 0.07, 0.04, "Ink")?,
                bar(0., 0., 0.27, -0.12, 0.07, 0.04, "Ink")?,
            ];
            for h in &mut hands {
                for v in &mut h.vertices {
                    v.z += 0.08;
                }
            }
            let mut parts = vec![face];
            parts.extend(hands);
            symbol(parts)?
        }
        Sample::FirstFrame => symbol(vec![
            bar(-0.4, -0.38, -0.4, 0.38, 0.14, 0.14, CLAY)?,
            pointer(0.08, 0., 0.7, -1., CLAY)?,
        ])?,
        Sample::LastFrame => symbol(vec![
            bar(0.4, -0.38, 0.4, 0.38, 0.14, 0.14, CLAY)?,
            pointer(-0.08, 0., 0.7, 1., CLAY)?,
        ])?,
        Sample::PreviousFrame => symbol(vec![
            frame_tile(0.33, 0., 0.16, 0.32)?,
            pointer(-0.2, 0., 0.55, -1., CLAY)?,
        ])?,
        Sample::NextFrame => symbol(vec![
            frame_tile(-0.33, 0., 0.16, 0.32)?,
            pointer(0.2, 0., 0.55, 1., CLAY)?,
        ])?,
        Sample::Play => symbol(vec![pointer(0.05, 0., 0.9, 1., CLAY)?])?,
        Sample::Stop => symbol(vec![slab(
            &[(-0.36, -0.36), (0.36, -0.36), (0.36, 0.36), (-0.36, 0.36)],
            0.14,
            CLAY,
        )?])?,
        Sample::PreviousKey => symbol(vec![
            pointer(-0.25, 0., 0.5, -1., CLAY)?,
            key(0.25, 0., 0.55)?,
        ])?,
        Sample::NextKey => symbol(vec![
            key(-0.25, 0., 0.55)?,
            pointer(0.25, 0., 0.5, 1., CLAY)?,
        ])?,
        Sample::FirstKey => symbol(vec![
            bar(-0.4, -0.36, -0.4, 0.36, 0.14, 0.14, CLAY)?,
            key(0.12, 0., 0.62)?,
        ])?,
        Sample::LastKey => symbol(vec![
            key(-0.12, 0., 0.62)?,
            bar(0.4, -0.36, 0.4, 0.36, 0.14, 0.14, CLAY)?,
        ])?,
        Sample::KeyChannels => {
            let mut parts = Vec::new();
            for y in [-0.3, 0., 0.3] {
                parts.push(bar(-0.5, y, 0.5, y, 0.07, 0.06, CLAY)?);
            }
            parts.push(key(0.1, 0., 0.5)?);
            symbol(parts)?
        }
        Sample::KeyItems => merge(vec![
            cube_at(DVec3::new(0., -0.22, 0.), 0.56, CLAY)?,
            deformed(key(0., 0., 0.5)?, |v| {
                symbol_frame() * v + DVec3::new(0., 0.36, 0.)
            }),
        ])?,
        Sample::InsertFrame => symbol(vec![
            frame_tile(0., 0., 0.38, 0.45)?,
            bar(-0.28, 0., 0.28, 0., 0.13, 0.14, "New")?,
            bar(0., -0.28, 0., 0.28, 0.13, 0.14, "New")?,
        ])?,
        Sample::RemoveFrame => symbol(vec![
            frame_tile(0., 0., 0.38, 0.45)?,
            bar(-0.28, 0., 0.28, 0., 0.13, 0.14, "Ink")?,
        ])?,
        Sample::ApplyKeys => symbol(vec![
            key(-0.2, 0.12, 0.5)?,
            bar(0.0, -0.2, 0.15, -0.38, 0.12, 0.14, "New")?,
            bar(0.13, -0.37, 0.48, 0.1, 0.12, 0.14, "New")?,
        ])?,
        Sample::DiscardKeys => symbol(vec![
            key(-0.2, 0.12, 0.5)?,
            bar(0.06, -0.42, 0.44, -0.04, 0.12, 0.14, "Ink")?,
            bar(0.06, -0.04, 0.44, -0.42, 0.12, 0.14, "Ink")?,
        ])?,
        Sample::AutoKey => {
            let arc: Vec<DVec3> = (0..=30)
                .map(|k| {
                    let a = 0.4 + std::f64::consts::TAU * 0.78 * f64::from(k) / 30.;
                    DVec3::new(0.45 * a.cos(), 0.45 * a.sin(), 0.)
                })
                .collect();
            let n = arc.len();
            symbol(vec![
                key(0., 0., 0.42)?,
                tube(&arc[..n - 2], 0.045, 10, CLAY)?,
                arrow(
                    arc[n - 3],
                    arc[n - 1] + (arc[n - 1] - arc[n - 3]),
                    0.045,
                    CLAY,
                )?,
            ])?
        }
        Sample::GraphEditor => symbol(vec![
            frame_tile(0., 0., 0.55, 0.45)?,
            graph_curve(-0.45, 0.45, s_curve, 0.04, "Ink")?,
            key(-0.45, s_curve(-0.45), 0.2)?,
            key(0., s_curve(0.), 0.2)?,
            key(0.45, s_curve(0.45), 0.2)?,
        ])?,
        Sample::KeyKeepingShape => symbol(vec![
            graph_curve(-0.55, 0.55, s_curve, 0.045, "Ink")?,
            key(-0.55, s_curve(-0.55), 0.2)?,
            key(0.55, s_curve(0.55), 0.2)?,
            deformed(key(0.18, s_curve(0.18), 0.32)?, |v| v + DVec3::Z * 0.06),
        ])?,
        Sample::BreakTangents => {
            let mut parts = vec![graph_curve(
                -0.55,
                0.55,
                |x| -0.3 * x * x * 2.,
                0.035,
                "Ink",
            )?];
            parts.extend(key_handles(0., 0., 2.4, 0.35, 0.5)?);
            symbol(parts)?
        }
        Sample::UnifyTangents => {
            let mut parts = vec![graph_curve(-0.55, 0.55, |x| 0.35 * x, 0.035, "Ink")?];
            parts.extend(key_handles(0., 0., std::f64::consts::PI + 0.3, 0.3, 0.5)?);
            symbol(parts)?
        }
        Sample::PreBehaviour | Sample::PostBehaviour => {
            // The curve between its keys, dark; its repeat outside them,
            // grey; a cyan arrow pointing the way the repeat runs.
            let post = sample == Sample::PostBehaviour;
            let side = if post { 1. } else { -1. };
            let wave = |x: f64| 0.22 * (x * 7.).sin() - 0.1;
            let (main, rep) = if post {
                ((-0.55, 0.), (0., 0.55))
            } else {
                ((0., 0.55), (-0.55, 0.))
            };
            symbol(vec![
                graph_curve(main.0, main.1, wave, 0.045, "Ink")?,
                graph_curve(rep.0, rep.1, wave, 0.03, CLAY)?,
                bar(0., -0.42, 0., 0.42, 0.04, 0.05, CLAY)?,
                arrow(
                    DVec3::new(side * 0.08, 0.38, 0.08),
                    DVec3::new(side * 0.55, 0.38, 0.08),
                    0.04,
                    "New",
                )?,
            ])?
        }
        Sample::CycleCurve => {
            let arc: Vec<DVec3> = (0..=24)
                .map(|k| {
                    let a = 0.6 + std::f64::consts::TAU * 0.75 * f64::from(k) / 24.;
                    DVec3::new(0.3 * a.cos(), 0.3 * a.sin() + 0.12, 0.08)
                })
                .collect();
            let n = arc.len();
            symbol(vec![
                graph_curve(-0.55, 0.55, |x| 0.18 * (x * 8.).sin() - 0.3, 0.04, "Ink")?,
                tube(&arc[..n - 2], 0.04, 10, "New")?,
                arrow(
                    arc[n - 3],
                    arc[n - 1] + (arc[n - 1] - arc[n - 3]),
                    0.04,
                    "New",
                )?,
            ])?
        }
        Sample::ShadowCopy => symbol(vec![
            graph_curve(-0.5, 0.5, |x| s_curve(x) - 0.18, 0.05, CLAY)?,
            graph_curve(-0.5, 0.5, |x| s_curve(x) + 0.12, 0.04, "Ink")?,
        ])?,
        Sample::ShadowSwap => symbol(vec![
            graph_curve(-0.55, 0.3, |x| s_curve(x) - 0.2, 0.05, CLAY)?,
            graph_curve(-0.55, 0.3, |x| s_curve(x) + 0.12, 0.04, "Ink")?,
            two_way(
                DVec3::new(0.45, -0.35, 0.08),
                DVec3::new(0.45, 0.4, 0.08),
                0.035,
                "New",
            )?,
        ])?,
        Sample::Normalised => symbol(vec![
            bar(-0.55, 0.35, 0.55, 0.35, 0.05, 0.05, "New")?,
            bar(-0.55, -0.35, 0.55, -0.35, 0.05, 0.05, "New")?,
            graph_curve(-0.5, 0.5, |x| 0.3 * (x * 6.).sin(), 0.04, "Ink")?,
        ])?,
        Sample::Stacked => {
            let mut parts = Vec::new();
            for y in [-0.36, 0., 0.36] {
                parts.push(graph_curve(
                    -0.5,
                    0.5,
                    move |x| y + 0.1 * (x * 6. + y * 5.).sin(),
                    0.04,
                    "Ink",
                )?);
            }
            symbol(parts)?
        }
        Sample::IsolateCurve => symbol(vec![
            graph_curve(-0.5, 0.5, |x| 0.3 * (x * 4.).sin() + 0.15, 0.025, CLAY)?,
            graph_curve(-0.5, 0.5, |x| -0.25 * (x * 3.).cos(), 0.025, CLAY)?,
            graph_curve(-0.5, 0.5, s_curve, 0.055, "New")?,
        ])?,
        Sample::Velocity => {
            let x = -0.05;
            let slope = (s_curve(x + 0.01) - s_curve(x - 0.01)) / 0.02;
            let d = DVec3::new(1., slope, 0.).normalize();
            let p = DVec3::new(x, s_curve(x), 0.1);
            symbol(vec![
                graph_curve(-0.55, 0.55, s_curve, 0.04, "Ink")?,
                arrow(p, p + d * 0.55, 0.045, "New")?,
            ])?
        }
        Sample::LinkRelated => {
            let mut parts = vec![key(-0.32, -0.12, 0.34)?, key(0.32, 0.12, 0.34)?];
            for k in [0., 2., 4.] {
                let a = DVec3::new(-0.32, -0.12, 0.08);
                let b = DVec3::new(0.32, 0.12, 0.08);
                let (t0, t1) = (0.22 + k * 0.1, 0.22 + (k + 1.) * 0.1);
                parts.push(tube(&[a.lerp(b, t0), a.lerp(b, t1)], 0.035, 8, "Ink")?);
            }
            symbol(parts)?
        }
        Sample::BoxScale => {
            let (w, h) = (0.42, 0.3);
            let mut parts = vec![
                graph_curve(-0.3, 0.3, |x| 0.18 * (x * 6.).sin(), 0.035, "Ink")?,
                key(-0.3, 0.18 * (-1.8f64).sin(), 0.18)?,
                key(0.3, 0.18 * (1.8f64).sin(), 0.18)?,
            ];
            for (a, b) in [
                ((-w, -h), (w, -h)),
                ((w, -h), (w, h)),
                ((w, h), (-w, h)),
                ((-w, h), (-w, -h)),
            ] {
                parts.push(bar(a.0, a.1, b.0, b.1, 0.035, 0.04, "Selected")?);
            }
            for (x, y) in [(-w, -h), (w, -h), (w, h), (-w, h)] {
                parts.push(slab(
                    &[
                        (x - 0.07, y - 0.07),
                        (x + 0.07, y - 0.07),
                        (x + 0.07, y + 0.07),
                        (x - 0.07, y + 0.07),
                    ],
                    0.1,
                    "Selected",
                )?);
            }
            symbol(parts)?
        }
        Sample::Retime => symbol(vec![
            graph_curve(-0.5, 0.5, |x| 0.2 * (x * 4.).sin() + 0.15, 0.035, "Ink")?,
            key(-0.5, 0.2 * (-2f64).sin() + 0.15, 0.2)?,
            key(0.5, 0.2 * (2f64).sin() + 0.15, 0.2)?,
            two_way(
                DVec3::new(-0.52, -0.3, 0.06),
                DVec3::new(0.52, -0.3, 0.06),
                0.04,
                "New",
            )?,
        ])?,
        Sample::CurveSpeed => {
            let arc: Vec<DVec3> = (0..=24)
                .map(|k| {
                    let a = std::f64::consts::PI * f64::from(k) / 24.;
                    DVec3::new(0.5 * a.cos(), 0.5 * a.sin() - 0.2, 0.)
                })
                .collect();
            let tip = DVec3::new(0.28, 0.2, 0.06);
            let hub = DVec3::new(0., -0.2, 0.06);
            symbol(vec![
                tube(&arc, 0.05, 10, CLAY)?,
                tube(&[hub, tip], 0.04, 8, "Ink")?,
                ball(tip, 0.07, "New")?,
                ball(hub, 0.08, "Ink")?,
            ])?
        }
        Sample::InSlope | Sample::OutSlope => {
            let out = sample == Sample::OutSlope;
            let wave = |x: f64| 0.12 * x - 0.25 * x * x;
            let (sel, other) = if out {
                (0.35, std::f64::consts::PI + 0.35)
            } else {
                (std::f64::consts::PI + 0.35, 0.35)
            };
            let mut parts = vec![
                graph_curve(-0.55, 0.55, wave, 0.035, "Ink")?,
                key(0., 0., 0.3)?,
            ];
            let len = 0.5;
            let s_end = DVec3::new(len * sel.cos(), len * sel.sin(), 0.06);
            parts.push(tube(
                &[DVec3::new(0., 0., 0.06), s_end],
                0.04,
                8,
                "Selected",
            )?);
            parts.push(ball(s_end, 0.08, "Selected")?);
            let o_end = DVec3::new(len * other.cos(), len * other.sin(), 0.06);
            parts.push(tube(&[DVec3::new(0., 0., 0.06), o_end], 0.025, 8, CLAY)?);
            parts.push(ball(o_end, 0.055, CLAY)?);
            symbol(parts)?
        }
        Sample::CurvePresets => {
            // A list of three presets, each a tile with its curve shape:
            // ease in, linear, ease out (the last one picked, cyan).
            let mut parts = Vec::new();
            let shapes: [fn(f64) -> f64; 3] = [|t| t * t, |t| t, |t| 1. - (1. - t) * (1. - t)];
            for (k, f) in shapes.into_iter().enumerate() {
                let cy = 0.36 - 0.36 * k as f64;
                parts.push(frame_tile(0., cy, 0.5, 0.15)?);
                parts.push(graph_curve(
                    -0.4,
                    0.4,
                    move |x| cy - 0.1 + 0.2 * f((x + 0.4) / 0.8),
                    0.035,
                    if k == 2 { "New" } else { "Ink" },
                )?);
            }
            symbol(parts)?
        }
        Sample::AnimationEditors => {
            let mut parts = vec![
                frame_tile(0., 0., 0.55, 0.45)?,
                graph_curve(-0.45, 0.45, |x| 0.12 * (x * 5.).sin() + 0.18, 0.035, "Ink")?,
                bar(-0.5, -0.08, 0.5, -0.08, 0.03, 0.04, CLAY)?,
            ];
            for x in [-0.32, 0.02, 0.3] {
                parts.push(key(x, -0.27, 0.18)?);
            }
            symbol(parts)?
        }
        Sample::Constrain => {
            let across = DVec3::new(1., 0., -1.).normalize();
            let target = across * 0.35 - DVec3::Y * 0.1;
            let from = -across * 0.4 + DVec3::Y * 0.25;
            let dir = (target - from).normalize();
            let mut cone = revolve(&[(0., -0.2), (0.17, -0.2), (0., 0.25)], 32, false, "New")?;
            let turn = glam::DQuat::from_rotation_arc(DVec3::Y, dir);
            cone = deformed(cone, |v| from + turn * v);
            let mut parts = vec![cube_at(target, 0.42, CLAY)?, cone];
            for k in [0., 2., 4.] {
                let (t0, t1) = (0.35 + k * 0.08, 0.35 + (k + 1.) * 0.08);
                parts.push(tube(
                    &[from.lerp(target, t0), from.lerp(target, t1)],
                    0.025,
                    8,
                    "Ink",
                )?);
            }
            merge(parts)?
        }
        Sample::Hierarchy => {
            let across = DVec3::new(1., 0., -1.).normalize();
            let top = DVec3::Y * 0.3;
            let kids = [
                across * -0.38 - DVec3::Y * 0.3,
                across * 0.38 - DVec3::Y * 0.3,
            ];
            let mut parts = vec![cube_at(top, 0.32, "Selected")?];
            for k in kids {
                parts.push(cube_at(k, 0.28, CLAY)?);
                parts.push(tube(
                    &[top - DVec3::Y * 0.16, k + DVec3::Y * 0.14],
                    0.03,
                    8,
                    "Ink",
                )?);
            }
            merge(parts)?
        }
        Sample::ChannelLinks => {
            let (a, b) = ((-0.15, 0.25), (0.2, -0.25));
            symbol(vec![
                bar(-0.5, 0.25, 0.5, 0.25, 0.07, 0.06, CLAY)?,
                bar(-0.5, -0.25, 0.5, -0.25, 0.07, 0.06, CLAY)?,
                ball(DVec3::new(a.0, a.1, 0.), 0.1, "Selected")?,
                ball(DVec3::new(b.0, b.1, 0.), 0.1, "Selected")?,
                tube(
                    &through(
                        &[
                            DVec3::new(a.0, a.1, 0.06),
                            DVec3::new(0.1, 0.05, 0.12),
                            DVec3::new(b.0, b.1, 0.06),
                        ],
                        10,
                    ),
                    0.035,
                    8,
                    "New",
                )?,
            ])?
        }
        Sample::Schematic => {
            let nodes = [(-0.32, 0.25), (-0.32, -0.25), (0.32, 0.)];
            let mut parts = Vec::new();
            for &(x, y) in &nodes {
                parts.push(frame_tile(x, y, 0.2, 0.13)?);
            }
            for &(x, y) in &nodes[..2] {
                parts.push(tube(
                    &through(
                        &[
                            DVec3::new(x + 0.2, y, -0.05),
                            DVec3::new(0.0, y * 0.6, -0.05),
                            DVec3::new(0.12, 0., -0.05),
                        ],
                        10,
                    ),
                    0.035,
                    8,
                    "New",
                )?);
            }
            symbol(parts)?
        }
        Sample::BakeKeys => {
            let f = |x: f64| 0.3 * (x * 4.).sin();
            let mut parts = vec![graph_curve(-0.55, 0.55, f, 0.03, "Ink")?];
            for k in 0..8 {
                let x = -0.5 + f64::from(k) / 7.;
                parts.push(key(x, f(x), 0.15)?);
            }
            symbol(parts)?
        }
        Sample::SetupPose => {
            // A T-pose seen square on: spine, arms straight out, legs down.
            let across = DVec3::new(1., 0., -1.).normalize();
            let at = |x: f64, y: f64| across * x * 1.3 + DVec3::Y * (y * 1.25 + 0.02);
            let mut parts = vec![box_by_normal(
                DVec3::new(-0.22, -0.6, -0.22),
                DVec3::new(0.22, -0.54, 0.22),
                |n| if n == DVec3::Y { "Selected" } else { CLAY },
            )?];
            parts.extend(chain(
                &[at(0., -0.05), at(0., 0.25), at(0., 0.45)],
                CLAY,
                CLAY,
            )?);
            parts.extend(chain(
                &[at(-0.48, 0.25), at(-0.22, 0.25), at(0., 0.25)],
                CLAY,
                CLAY,
            )?);
            parts.extend(chain(
                &[at(0., 0.25), at(0.22, 0.25), at(0.48, 0.25)],
                CLAY,
                CLAY,
            )?);
            parts.extend(chain(
                &[
                    at(-0.12, -0.5),
                    at(-0.12, -0.05),
                    at(0.12, -0.05),
                    at(0.12, -0.5),
                ],
                CLAY,
                CLAY,
            )?);
            merge(parts)?
        }
        Sample::SkeletonTool => {
            let across = DVec3::new(1., 0., -1.).normalize();
            let at = |x: f64, y: f64| across * x + DVec3::Y * y;
            let joints = [at(-0.45, -0.45), at(-0.15, -0.05), at(0.05, 0.3)];
            let mut parts = chain(&joints, CLAY, CLAY)?;
            let new_joint = at(0.45, 0.4);
            parts.push(bone(joints[2], new_joint, 0.09, "New")?);
            parts.push(ball(new_joint, 0.09, "New")?);
            merge(parts)?
        }
        Sample::SkinMesh => {
            let across = DVec3::new(1., 0., -1.).normalize();
            let at = |x: f64, y: f64| across * x + DVec3::Y * y;
            let joints = [at(-0.45, -0.35), at(0.05, -0.2), at(0.4, 0.35)];
            let path = through(&joints, 12);
            let toward = DVec3::ONE.normalize();
            // The mesh sits behind; the bones it is bound to just in front.
            let behind = |v: DVec3| v - toward * 0.12;
            let mut parts = vec![deformed(tube_with(&path, |_| 0.16, 24, CLAY)?, behind)];
            let front: Vec<DVec3> = joints.iter().map(|&j| j + toward * 0.3).collect();
            parts.extend(chain(&front, "New", "New")?);
            merge(parts)?
        }
        Sample::IkTwoBone => {
            let goal = flat(0.4, -0.3);
            let mut parts = thick_chain(&[flat(-0.45, -0.3), flat(-0.05, 0.38), goal], CLAY)?;
            parts.extend(ik_target(goal, 0.2)?);
            merge(parts)?
        }
        Sample::IkChain => {
            let goal = flat(0.45, 0.35);
            let mut parts = thick_chain(
                &[
                    flat(-0.5, -0.4),
                    flat(-0.25, 0.05),
                    flat(0.05, 0.2),
                    flat(0.28, 0.1),
                    goal,
                ],
                CLAY,
            )?;
            parts.extend(ik_target(goal, 0.18)?);
            merge(parts)?
        }
        Sample::IkUpVector => {
            let knee = flat(-0.1, 0.2);
            let pole = flat(-0.1, 0.55) + DVec3::ONE.normalize() * 0.15;
            let mut parts = thick_chain(&[flat(-0.5, -0.4), knee, flat(0.35, -0.4)], CLAY)?;
            parts.push(ball(pole, 0.12, "Selected")?);
            for k in [0., 2.] {
                let (t0, t1) = (0.25 + k * 0.17, 0.25 + (k + 1.) * 0.17);
                parts.push(tube(
                    &[knee.lerp(pole, t0), knee.lerp(pole, t1)],
                    0.035,
                    8,
                    "Ink",
                )?);
            }
            merge(parts)?
        }
        Sample::IkGoal => {
            let goal = flat(0.12, 0.08);
            let mut parts = ik_target(goal, 0.36)?;
            parts.push(bone(flat(-0.5, -0.45), goal, 0.13, CLAY)?);
            parts.push(ball(flat(-0.5, -0.45), 0.1, CLAY)?);
            merge(parts)?
        }
        Sample::LoopSlice => {
            let mut parts = vec![sample_mesh(Sample::Cube)?];
            let r = 0.505;
            for x in [-0.25, 0., 0.25] {
                let square = [
                    DVec3::new(x, -r, -r),
                    DVec3::new(x, r, -r),
                    DVec3::new(x, r, r),
                    DVec3::new(x, -r, r),
                    DVec3::new(x, -r, -r),
                ];
                for w in square.windows(2) {
                    parts.push(tube(w, 0.03, 8, "New")?);
                }
            }
            merge(parts)?
        }
        Sample::BezierCurve => {
            let (a, b, c) = (
                DVec3::new(-0.55, -0.5, 0.45),
                DVec3::new(0., 0.3, 0.),
                DVec3::new(0.5, -0.45, -0.5),
            );
            let mut parts = vec![tube(&through(&[a, b, c], 24), 0.075, 12, "New")?];
            for p in [a, c] {
                parts.push(ball(p, 0.12, CLAY)?);
            }
            // The middle point's tangent handle, selected.
            let along = (c - a).normalize() * 0.42;
            let (h0, h1) = (b - along, b + along);
            parts.push(tube(&[h0, h1], 0.045, 10, "Selected")?);
            for p in [h0, h1] {
                parts.push(ball(p, 0.1, "Selected")?);
            }
            parts.push(ball(b, 0.13, CLAY)?);
            merge(parts)?
        }
        Sample::PointLight => {
            let mut bulb = tagged(
                globe_geometry(
                    DVec3::new(0., 0.12, 0.),
                    RadialParameters {
                        radius: DVec3::splat(0.38),
                        sides: 48,
                        segments: 24,
                    },
                )?,
                "Glow",
            );
            let base = tagged(
                cylinder_geometry(
                    DVec3::new(0., -0.36, 0.),
                    RadialParameters {
                        radius: DVec3::new(0.17, 0.14, 0.17),
                        sides: 32,
                        segments: 1,
                    },
                )?,
                "Ink",
            );
            crate::primitive::append_geometry(&mut bulb, base)?;
            bulb
        }
    })
}

/// How far from the picture's centre a point may land, metres: two 32 px
/// pixels short of the edge, for the outline.
pub fn picture_reach() -> f64 {
    HALF_SIZE * (1. - 2. / 16.)
}

/// `mesh` shrunk about the origin, if need be, so it stays inside
/// [`picture_reach`] seen through the studio camera.
fn fit_to_picture(mesh: Mesh) -> Mesh {
    let forward = -DVec3::ONE.normalize();
    let right = forward.cross(DVec3::Y).normalize();
    let up = right.cross(forward);
    let reach = mesh
        .vertices
        .iter()
        .map(|v| v.dot(right).abs().max(v.dot(up).abs()))
        .fold(0., f64::max);
    let limit = picture_reach() * 0.99;
    if reach <= limit {
        return mesh;
    }
    deformed(mesh, |v| v * (limit / reach))
}

/// The studio with a sample's geometry added.
pub fn sample_scene(sample: Sample) -> Result<Scene> {
    let mut scene = studio_scene()?;
    let mesh = fit_to_picture(sample_mesh(sample)?);
    scene.new_mesh_named(Some(sample.slug().into()))?;
    let item = scene.items.last_mut().ok_or("no mesh item")?;
    item.mesh = mesh.into();
    Ok(scene)
}

/// Whether `item` is one of the studio's own kinds (camera, light, sky),
/// which an icon scene's copy of the studio carries but never renders.
fn studio_kind(item: &crate::Item) -> bool {
    item.camera.is_some() || item.light.is_some() || item.environment.is_some()
}

/// The scene to render for `icon`: a copy of `studio` with the icon's own
/// items (everything but cameras, lights and environments) added, under
/// new ids. The studio's camera, lights, sky and materials are used as
/// they are; the icon's copies are ignored. Polygons pick materials by
/// tag, so the icon's tags must name studio materials (others render as
/// Clay).
pub fn render_scene(studio: &Scene, icon: &Scene) -> Result<Scene> {
    let mut scene = studio.clone();
    // The studio itself renders nothing but what the icon adds.
    scene.items.retain(studio_kind);
    let copied: Vec<&crate::Item> = icon.items.iter().filter(|i| !studio_kind(i)).collect();
    if !copied.iter().any(|i| !i.mesh.faces.is_empty()) {
        return Err("The icon scene has no polygons to render".into());
    }
    let first = scene.next_id;
    let new_id = |old: u64| {
        copied
            .iter()
            .position(|i| i.id == old)
            .map(|k| first + k as u64)
    };
    for item in &copied {
        let mut item = (*item).clone();
        item.id = new_id(item.id).ok_or("lost an item id")?;
        // A parent that isn't copied (a camera, say) is dropped; such a
        // parent would be odd in an icon scene.
        item.parent = item.parent.and_then(new_id);
        item.instance_of = item.instance_of.and_then(new_id);
        scene.items.push(item);
    }
    scene.next_id = first + copied.len() as u64;
    scene.selection = Default::default();
    Ok(scene)
}

/// The tags in `icon` that name no studio material.
pub fn unknown_tags(icon: &Scene) -> Vec<String> {
    let mut unknown: Vec<String> = icon
        .items
        .iter()
        .flat_map(|i| i.mesh.faces.iter().map(|f| f.material.clone()))
        .filter(|t| !t.is_empty() && !MATERIALS.iter().any(|m| m.tag == t))
        .collect();
    unknown.sort();
    unknown.dedup();
    unknown
}
