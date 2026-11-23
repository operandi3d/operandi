//! The preview scenes the browser renders thumbnails from (PB-113, PB-114).
//!
//! - [`cutaway_ball`]: Operandi's own procedural model (Jason, 2026-11-11):
//!   a hollow sphere with a small round hole on its upper right (PB-114,
//!   changed by Jason 2026-11-11: the old big hole faced the camera and
//!   the thumbnail stared into the cavern). Seen obliquely, the opening
//!   shows the cut edge and a sliver of the inside wall while the glossy
//!   outside fills most of the picture. The grid runs around the hole's axis, so the
//!   hole's edge is a clean circle and the texture wraps the ball once.
//! - [`material_scene`], [`environment_scene`], [`item_scene`]: a preset
//!   placed on a shadow-catcher floor (only its contact shadow shows; the
//!   app puts a soft grey gradient behind), with a camera [`Framing`]. Material
//!   presets go onto the ball through the ordinary preset load, so the
//!   thumbnail shows exactly what a load gives.
//!
//! The renderer, lights and picture size belong to the app.
use super::{Preset, PresetKind};
use crate::shader_tree::{GroupMask, NewLayer, NodeKind, TagMask, TagType};
use crate::{Editor, Item, Mesh, Result, Scene, Transform};
use glam::{DVec2, DVec3};
use std::f64::consts::{PI, TAU};

/// The ball's radius in metres.
pub const RADIUS: f64 = 0.5;
/// Wall thickness as a fraction of the radius.
pub const WALL: f64 = 0.1;
/// Half the hole's angle seen from the ball's centre.
pub const HOLE_HALF_ANGLE: f64 = 0.33;
/// Texture repeats around the hole's axis and from the hole to the pole.
const UV_AROUND: f64 = 3.;
const UV_DOWN: f64 = 1.5;
/// Polygon tags of the preview scenes.
pub const BALL_TAG: &str = "Preset Ball";
pub const FLOOR_TAG: &str = "Preset Floor";

/// The hole's direction: up and to the right, turned only partly towards
/// the camera (+Z), so the opening is seen at a slant.
pub fn hole_axis() -> DVec3 {
    DVec3::new(0.62, 0.62, 0.48).normalize()
}

/// How much of the picture's height the ball fills.
pub const BALL_FILL: f64 = 0.88;

/// Where the camera looks from and at.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Framing {
    pub eye: DVec3,
    pub target: DVec3,
    /// Tangent of half the vertical field of view.
    pub half_height_tan: f64,
}

/// A camera a little above the ball looking slightly down on it (PB-114),
/// close enough that the ball fills [`BALL_FILL`] of a square picture.
pub fn ball_framing() -> Framing {
    let target = DVec3::new(0., RADIUS * 0.97, 0.);
    let half_height_tan = (20f64.to_radians() / 2.).tan();
    // The sphere's silhouette subtends asin(R / d) from the eye.
    let half_angle = (BALL_FILL * half_height_tan).atan();
    let distance = RADIUS / half_angle.sin();
    let down = 10f64.to_radians();
    Framing {
        eye: target + DVec3::new(0., down.sin(), down.cos()) * distance,
        target,
        half_height_tan,
    }
}

/// A camera that frames the box `min..max` from the front and a little
/// above, for item presets.
pub fn bounds_framing(min: DVec3, max: DVec3) -> Framing {
    let center = (min + max) / 2.;
    let radius = ((max - min).length() / 2.).max(0.01);
    let half_height_tan = (30f64.to_radians() / 2.).tan();
    let distance = radius / (0.85 * half_height_tan.atan().sin());
    let dir = DVec3::new(0.45, 0.4, 1.).normalize();
    Framing {
        eye: center + dir * distance,
        target: center,
        half_height_tan,
    }
}

/// The hollow ball with its hole, resting on the ground (y = 0), every
/// polygon tagged `tag`, with UVs in the primitives' "Texture" map.
/// `sides` around the hole's axis, `rings` from the hole to the far pole.
pub fn cutaway_ball(sides: u32, rings: u32, tag: &str) -> Result<Mesh> {
    let sides = sides.max(8) as usize;
    let rings = rings.max(4) as usize;
    let axis = hole_axis();
    // An orthonormal frame around the axis.
    let helper = if axis.y.abs() < 0.9 {
        DVec3::Y
    } else {
        DVec3::X
    };
    let a = axis.cross(helper).normalize();
    let b = axis.cross(a);
    let center = DVec3::new(0., RADIUS, 0.);
    let direction =
        |theta: f64, phi: f64| axis * theta.cos() + (a * phi.cos() + b * phi.sin()) * theta.sin();
    let theta_at =
        |ring: usize| HOLE_HALF_ANGLE + (PI - HOLE_HALF_ANGLE) * ring as f64 / rings as f64;
    let phi_at = |side: usize| TAU * side as f64 / sides as f64;
    let mut mesh = Mesh::default();
    let mut uvs: Vec<Vec<Option<DVec2>>> = Vec::new();
    // Rings 0..rings-1 hold `sides` points each; the far pole is one.
    let shell = |radius: f64, mesh: &mut Mesh| -> (usize, usize) {
        let start = mesh.vertices.len();
        for ring in 0..rings {
            for side in 0..sides {
                mesh.vertices
                    .push(center + direction(theta_at(ring), phi_at(side)) * radius);
            }
        }
        mesh.vertices.push(center + direction(PI, 0.) * radius);
        (start, start + rings * sides)
    };
    let (outer, outer_pole) = shell(RADIUS, &mut mesh);
    let (inner, inner_pole) = shell(RADIUS * (1. - WALL), &mut mesh);
    let at = |start: usize, ring: usize, side: usize| (start + ring * sides + side % sides) as u32;
    // About real-world size: a texture set covers roughly a metre or two,
    // so it repeats three times around and one and a half times from the
    // hole to the far pole [chosen].
    let uv = |ring: f64, side: usize| {
        Some(DVec2::new(
            UV_AROUND * side as f64 / sides as f64,
            UV_DOWN * (1. - ring / rings as f64),
        ))
    };
    let mut faces: Vec<(Vec<u32>, Vec<Option<DVec2>>, DVec3)> = Vec::new();
    for (start, pole, outward) in [(outer, outer_pole, 1.), (inner, inner_pole, -1.)] {
        for ring in 0..rings {
            for side in 0..sides {
                let mid_theta = theta_at(ring) + 0.5 * (PI - HOLE_HALF_ANGLE) / rings as f64;
                let want = direction(mid_theta, phi_at(side) + PI / sides as f64) * outward;
                if ring + 1 < rings {
                    faces.push((
                        vec![
                            at(start, ring, side),
                            at(start, ring, side + 1),
                            at(start, ring + 1, side + 1),
                            at(start, ring + 1, side),
                        ],
                        vec![
                            uv(ring as f64, side),
                            uv(ring as f64, side + 1),
                            uv(ring as f64 + 1., side + 1),
                            uv(ring as f64 + 1., side),
                        ],
                        want,
                    ));
                } else {
                    faces.push((
                        vec![
                            at(start, ring, side),
                            at(start, ring, side + 1),
                            pole as u32,
                        ],
                        vec![
                            uv(ring as f64, side),
                            uv(ring as f64, side + 1),
                            Some(DVec2::new(
                                UV_AROUND * (side as f64 + 0.5) / sides as f64,
                                0.,
                            )),
                        ],
                        want,
                    ));
                }
            }
        }
    }
    // The cut edge: outer ring 0 to inner ring 0, facing out of the hole.
    for side in 0..sides {
        let phi = phi_at(side) + PI / sides as f64;
        let tangent =
            axis * -HOLE_HALF_ANGLE.sin() + (a * phi.cos() + b * phi.sin()) * HOLE_HALF_ANGLE.cos();
        faces.push((
            vec![
                at(outer, 0, side),
                at(outer, 0, side + 1),
                at(inner, 0, side + 1),
                at(inner, 0, side),
            ],
            vec![
                uv(0., side),
                uv(0., side + 1),
                uv(-0.3, side + 1),
                uv(-0.3, side),
            ],
            -tangent,
        ));
    }
    for (mut vertices, mut corner_uvs, want) in faces {
        // Wind each polygon so its normal faces `want`.
        if newell(&mesh.vertices, &vertices).dot(want) < 0. {
            vertices.reverse();
            corner_uvs.reverse();
        }
        mesh.faces.push(crate::mesh::Face {
            vertices,
            uv: None,
            material: tag.into(),
            smooth: true,
            part: None,
            material_label: None,
        });
        uvs.push(corner_uvs);
    }
    mesh.uv_maps
        .insert(crate::primitive_uv::PRIMITIVE_UV_MAP.into(), uvs);
    mesh.validate()?;
    Ok(mesh)
}

fn newell(points: &[DVec3], face: &[u32]) -> DVec3 {
    let mut n = DVec3::ZERO;
    for (i, &v) in face.iter().enumerate() {
        let p = points[v as usize];
        let q = points[face[(i + 1) % face.len()] as usize];
        n += DVec3::new(
            (p.y - q.y) * (p.z + q.z),
            (p.z - q.z) * (p.x + q.x),
            (p.x - q.x) * (p.y + q.y),
        );
    }
    n
}

/// A flat square floor at y = 0, tagged [`FLOOR_TAG`].
pub fn floor(size: f64) -> Mesh {
    let h = size / 2.;
    Mesh {
        vertices: vec![
            DVec3::new(-h, 0., h),
            DVec3::new(h, 0., h),
            DVec3::new(h, 0., -h),
            DVec3::new(-h, 0., -h),
        ],
        faces: vec![crate::mesh::Face {
            vertices: vec![0, 1, 2, 3],
            uv: None,
            material: FLOOR_TAG.into(),
            smooth: false,
            part: None,
            material_label: None,
        }]
        .into(),
        ..Mesh::default()
    }
}

pub(crate) fn mesh_item(id: u64, name: &str, mesh: Mesh) -> Item {
    Item {
        id,
        name: name.into(),
        visible: true,
        transform: Transform::default(),
        mesh: mesh.into(),
        parent: None,
        source_id: None,
        source_display_name: None,
        subdivision: None,
        backdrop: None,
        camera: None,
        light: None,
        texture: None,
        locator: None,
        environment: None,
        common: Default::default(),
        elements: Default::default(),
        hides_children: false,
        mesh_stack: None,
        kit: None,
        instance_of: None,
        replicator: None,
    }
}

/// A material group for `tag` holding one material set up by `set`.
fn tag_group(
    scene: &mut Scene,
    tag: &str,
    set: impl FnOnce(&mut crate::shader_tree::MaterialChannels),
) -> Result<u64> {
    let group = scene.shader_tree.add(NewLayer::Group, None)?;
    let mut node = scene.shader_tree.get(group).ok_or("no group")?.clone();
    node.name = tag.into();
    node.kind = NodeKind::Group(GroupMask {
        tag_type: TagType::Material,
        tag: TagMask::Name(tag.into()),
        ..GroupMask::default()
    });
    scene.shader_tree.replace(node)?;
    let material = scene.shader_tree.add(NewLayer::Material, Some(group))?;
    let mut node = scene
        .shader_tree
        .get(material)
        .ok_or("no material")?
        .clone();
    // A new layer added "on" a group lands inside it; make sure.
    node.parent = Some(group);
    if let NodeKind::Material(m) = &mut node.kind {
        set(m);
    }
    scene.shader_tree.replace(node)?;
    Ok(group)
}

/// The floor and ball with plain grey materials; returns the scene and the
/// ball's material group.
fn studio(
    ball_set: impl FnOnce(&mut crate::shader_tree::MaterialChannels),
) -> Result<(Scene, u64)> {
    let mut scene = Scene::default();
    scene.items.push(mesh_item(1, "Floor", floor(40.)));
    scene
        .items
        .push(mesh_item(2, "Ball", cutaway_ball(48, 24, BALL_TAG)?));
    scene.next_id = 3;
    let floor = tag_group(&mut scene, FLOOR_TAG, |m| {
        m.diffuse_color = [0.5, 0.5, 0.5];
        m.roughness = 0.8;
    })?;
    // The floor only catches the ball's shadow: rendered over a clear
    // background, it leaves a soft contact shadow and nothing else.
    let shader = scene.shader_tree.add(NewLayer::Shader, Some(floor))?;
    let mut node = scene.shader_tree.get(shader).ok_or("no shader")?.clone();
    node.parent = Some(floor);
    if let NodeKind::Shader(s) = &mut node.kind {
        s.alpha_type = crate::shader_tree::AlphaType::ShadowCatcher;
    }
    scene.shader_tree.replace(node)?;
    let ball = tag_group(&mut scene, BALL_TAG, ball_set)?;
    Ok((scene, ball))
}

/// A material preset on the cutaway ball (PB-114).
pub fn material_scene(preset: &Preset) -> Result<(Scene, Framing)> {
    if preset.header.kind != PresetKind::Material {
        return Err("Not a material preset".into());
    }
    let (scene, ball) = studio(|_| {})?;
    let mut editor = Editor {
        scene,
        ..Editor::default()
    };
    editor.load_preset(preset.clone(), Some(ball))?;
    Ok((editor.scene, ball_framing()))
}

/// An environment preset: a mirror-ish ball under its lights (PB-113).
pub fn environment_scene(preset: &Preset) -> Result<(Scene, Framing)> {
    if preset.header.kind != PresetKind::Environment {
        return Err("Not an environment preset".into());
    }
    let (scene, _) = studio(|m| {
        m.diffuse_color = [0.8, 0.8, 0.8];
        m.metallic = 1.;
        m.roughness = 0.08;
    })?;
    let mut editor = Editor {
        scene,
        ..Editor::default()
    };
    editor.load_preset(preset.clone(), None)?;
    Ok((editor.scene, ball_framing()))
}

/// An item or assembly preset framed from the front (PB-113).
pub fn item_scene(preset: &Preset) -> Result<(Scene, Framing)> {
    if !matches!(preset.header.kind, PresetKind::Item | PresetKind::Assembly) {
        return Err("Not an item preset".into());
    }
    let mut editor = Editor::default();
    editor.load_preset(preset.clone(), None)?;
    let scene = editor.scene;
    let mut min = DVec3::splat(f64::INFINITY);
    let mut max = DVec3::splat(f64::NEG_INFINITY);
    for item in scene.items.iter().filter(|i| !i.mesh.vertices.is_empty()) {
        let m = scene.world_matrix(item.id)?;
        for v in &item.mesh.vertices {
            let p = m.transform_point3(*v);
            min = min.min(p);
            max = max.max(p);
        }
    }
    if !min.is_finite() {
        return Err("This preset has no geometry to show".into());
    }
    Ok((scene, bounds_framing(min, max)))
}

/// The preview scene for any preset that has one.
pub fn preview_scene(preset: &Preset) -> Result<(Scene, Framing)> {
    match preset.header.kind {
        PresetKind::Material => material_scene(preset),
        PresetKind::Environment => environment_scene(preset),
        PresetKind::Item | PresetKind::Assembly => item_scene(preset),
        PresetKind::RenderSettings => Err("Render settings have no picture".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_ball_is_closed_hollow_and_faces_outward() {
        let mesh = cutaway_ball(24, 12, BALL_TAG).unwrap();
        // Outer + inner shells (quads and pole triangles) and the rim.
        assert_eq!(mesh.faces.len(), 24 * 12 * 2 + 24);
        let center = DVec3::new(0., RADIUS, 0.);
        let (mut outward, mut inward) = (0, 0);
        for f in &mesh.faces {
            let mid = f
                .vertices
                .iter()
                .map(|&v| mesh.vertices[v as usize])
                .sum::<DVec3>()
                / f.vertices.len() as f64;
            let n = newell(&mesh.vertices, &f.vertices);
            let r = (mid - center).length();
            if r > RADIUS * 0.98 {
                assert!(n.dot(mid - center) > 0., "outer shell faces out");
                outward += 1;
            } else if r < RADIUS * (1. - WALL) * 1.02 {
                assert!(n.dot(mid - center) < 0., "inner wall faces the hollow");
                inward += 1;
            }
        }
        assert_eq!((outward, inward), (24 * 12, 24 * 12));
        // Rests on the floor; nothing pokes through the hole's side.
        let lowest = mesh
            .vertices
            .iter()
            .map(|v| v.y)
            .fold(f64::INFINITY, f64::min);
        assert!(lowest > -1e-9 && lowest < 0.02);
        // Every edge is shared by exactly two polygons (no gaps at the rim).
        let mut edges = std::collections::HashMap::new();
        for f in &mesh.faces {
            for i in 0..f.vertices.len() {
                let (a, b) = (f.vertices[i], f.vertices[(i + 1) % f.vertices.len()]);
                *edges.entry((a.min(b), a.max(b))).or_insert(0) += 1;
            }
        }
        assert!(edges.values().all(|&n| n == 2));
        assert_eq!(mesh.uv_maps["Texture"].len(), mesh.faces.len());
    }

    /// PB-114 (Jason, 2026-11-11): the hole is visible but mostly turned
    /// away from the camera, up and to the right on screen, and the ball
    /// fills most of the picture.
    #[test]
    fn the_hole_is_seen_at_a_slant_upper_right() {
        let f = ball_framing();
        let center = DVec3::new(0., RADIUS, 0.);
        let to_camera = (f.eye - center).normalize();
        let facing = hole_axis().dot(to_camera);
        assert!((0.2..0.6).contains(&facing), "{facing}");
        let forward = (f.target - f.eye).normalize();
        let right = forward.cross(DVec3::Y).normalize();
        let up = right.cross(forward);
        assert!(hole_axis().dot(right) > 0.3, "to the right on screen");
        assert!(hole_axis().dot(up) > 0.3, "upwards on screen");
        assert!(f.eye.y > f.target.y, "looks a little down");
        // The silhouette's height over the picture's.
        let fill = (RADIUS / (f.eye - center).length()).asin().tan() / f.half_height_tan;
        assert!((0.85..0.9).contains(&fill), "{fill}");
        // A small opening: well under a tenth of the sphere.
        const { assert!(HOLE_HALF_ANGLE < 0.4) };
    }

    #[test]
    fn the_floor_only_catches_shadows() {
        let (scene, _) = studio(|_| {}).unwrap();
        assert!(scene.shader_tree.nodes.iter().any(|n| matches!(
            &n.kind,
            NodeKind::Shader(s) if s.alpha_type == crate::shader_tree::AlphaType::ShadowCatcher
        )));
    }

    #[test]
    fn a_material_preset_lands_on_the_ball() {
        let mut source = Editor::default();
        let group = source.scene.shader_tree.add(NewLayer::Group, None).unwrap();
        source
            .scene
            .shader_tree
            .add(NewLayer::Material, Some(group))
            .unwrap();
        let preset = super::super::PresetSource::Material(group)
            .build(&source.scene)
            .unwrap()
            .preset;
        let (scene, _) = material_scene(&preset).unwrap();
        let tree = &scene.shader_tree;
        let ball = tree
            .nodes
            .iter()
            .find(|n| matches!(&n.kind, NodeKind::Group(g) if g.tag == TagMask::Name(BALL_TAG.into())))
            .unwrap();
        assert!(
            tree.children(Some(ball.id))
                .any(|n| matches!(n.kind, NodeKind::Material(_)))
        );
        assert!(scene.items.iter().any(|i| i.name == "Ball"));
        assert!(
            preview_scene(&Preset {
                header: super::super::PresetHeader {
                    kind: PresetKind::RenderSettings,
                    thumbnail: None,
                    source: None,
                },
                scene: Scene::default()
            })
            .is_err()
        );
    }
}
