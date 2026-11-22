//! Scenes for Operandi's own matcap renders ("Our own renders" in the
//! preset-browser matcap spec, 2026-11-12).
//!
//! A matcap is a picture of a lit sphere seen straight on, so the scene is
//! just that: a plain smooth [`sphere`] centred on the origin and a
//! [`MatcapFraming`] for an orthographic camera on +Z whose picture the
//! sphere's silhouette exactly fills. The renderer, the studio HDRI and any
//! post-processing (the toon bands) belong to the app.
//!
//! [`LOOKS`] is the fixed batch set. Each look is either a material preset
//! from the library (loaded onto the sphere through the ordinary preset
//! load, like the browser thumbnails) or simple material values set here.
use super::{Preset, PresetKind};
use crate::shader_tree::{GroupMask, MaterialChannels, NewLayer, NodeKind, TagMask, TagType};
use crate::{Editor, Item, Mesh, Result, Scene, Transform};
use glam::{DVec2, DVec3};
use std::f64::consts::{PI, TAU};

/// The sphere's radius in metres.
pub const RADIUS: f64 = 0.5;
/// Polygon tag of the matcap sphere.
pub const SPHERE_TAG: &str = "Matcap Sphere";
/// Sides around and rings from pole to pole: enough that the silhouette
/// stays round at 1024 px and the smooth normals show no facets.
pub const SIDES: u32 = 192;
pub const RINGS: u32 = 96;

/// The orthographic camera: on +Z looking at the origin, +Y up.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MatcapFraming {
    pub eye: DVec3,
    pub target: DVec3,
    /// Half the picture's height (and width: matcaps are square) in metres.
    pub half_height: f64,
}

/// The framing for a square picture the sphere exactly fills: the circle
/// touches all four edges, the corners stay empty.
pub fn framing() -> MatcapFraming {
    MatcapFraming {
        eye: DVec3::new(0., 0., RADIUS * 4.),
        target: DVec3::ZERO,
        half_height: RADIUS,
    }
}

/// How much of a square `size`-pixel picture pixel `(x, y)` (top-left
/// origin) lies inside the sphere's disc, 0..1, from a 4×4 grid of
/// samples. The renderer's own coverage is used for the picture; this is
/// what it should match, and what the tests check the framing against.
pub fn disc_coverage(x: u32, y: u32, size: u32) -> f64 {
    let size = f64::from(size.max(1));
    let mut inside = 0;
    for sy in 0..4 {
        for sx in 0..4 {
            let px = (f64::from(x) + (f64::from(sx) + 0.5) / 4.) / size * 2. - 1.;
            let py = (f64::from(y) + (f64::from(sy) + 0.5) / 4.) / size * 2. - 1.;
            if px * px + py * py <= 1. {
                inside += 1;
            }
        }
    }
    f64::from(inside) / 16.
}

/// A closed UV sphere of [`RADIUS`] about the origin, poles on ±Y, every
/// polygon tagged `tag` and smooth, with UVs in the primitives' "Texture"
/// map (once around, once pole to pole; the seam faces −Z, away from the
/// camera).
pub fn sphere(sides: u32, rings: u32, tag: &str) -> Result<Mesh> {
    let sides = sides.max(8) as usize;
    let rings = rings.max(4) as usize;
    // Longitude 0 at −Z so the UV seam is behind the sphere.
    let point = |ring: usize, side: usize| {
        let theta = PI * ring as f64 / rings as f64;
        let phi = TAU * side as f64 / sides as f64;
        DVec3::new(
            -theta.sin() * phi.sin(),
            theta.cos(),
            -theta.sin() * phi.cos(),
        ) * RADIUS
    };
    let mut mesh = Mesh::default();
    // Point 0 is the top pole, then rings 1..rings-1, then the bottom pole.
    mesh.vertices.push(point(0, 0));
    for ring in 1..rings {
        for side in 0..sides {
            mesh.vertices.push(point(ring, side));
        }
    }
    mesh.vertices.push(point(rings, 0));
    let bottom = (mesh.vertices.len() - 1) as u32;
    let at = |ring: usize, side: usize| (1 + (ring - 1) * sides + side % sides) as u32;
    let uv = |ring: f64, side: f64| Some(DVec2::new(side / sides as f64, 1. - ring / rings as f64));
    let mut uvs = Vec::new();
    let mut push = |mesh: &mut Mesh, vertices: Vec<u32>, corner: Vec<Option<DVec2>>| {
        mesh.faces.push(crate::mesh::Face {
            vertices,
            uv: None,
            material: tag.into(),
            smooth: true,
            part: None,
            material_label: None,
        });
        uvs.push(corner);
    };
    // Wound counter-clockwise seen from outside, so normals face out.
    for side in 0..sides {
        let s = side as f64;
        push(
            &mut mesh,
            vec![0, at(1, side), at(1, side + 1)],
            vec![uv(0., s + 0.5), uv(1., s), uv(1., s + 1.)],
        );
    }
    for ring in 1..rings - 1 {
        let r = ring as f64;
        for side in 0..sides {
            let s = side as f64;
            push(
                &mut mesh,
                vec![
                    at(ring, side),
                    at(ring + 1, side),
                    at(ring + 1, side + 1),
                    at(ring, side + 1),
                ],
                vec![uv(r, s), uv(r + 1., s), uv(r + 1., s + 1.), uv(r, s + 1.)],
            );
        }
    }
    let last = rings - 1;
    for side in 0..sides {
        let s = side as f64;
        push(
            &mut mesh,
            vec![at(last, side), bottom, at(last, side + 1)],
            vec![
                uv(last as f64, s),
                uv(rings as f64, s + 0.5),
                uv(last as f64, s + 1.),
            ],
        );
    }
    mesh.uv_maps
        .insert(crate::primitive_uv::PRIMITIVE_UV_MAP.into(), uvs);
    mesh.validate()?;
    Ok(mesh)
}

/// Simple material values for a look defined in code. Colours are linear.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CodeLook {
    pub base_color: [f64; 3],
    pub metallic: f64,
    pub roughness: f64,
    pub coat: f64,
    pub coat_roughness: f64,
    /// Subsurface weight; 0 is none.
    pub subsurface: f64,
    pub subsurface_color: [f64; 3],
    /// Mean free path in metres (the sphere is 1 m across).
    pub subsurface_distance: f64,
}

impl CodeLook {
    const fn plain(base_color: [f64; 3], metallic: f64, roughness: f64) -> Self {
        Self {
            base_color,
            metallic,
            roughness,
            coat: 0.,
            coat_roughness: 0.,
            subsurface: 0.,
            subsurface_color: [0.8; 3],
            subsurface_distance: 0.005,
        }
    }

    /// Writes these values onto a material layer.
    pub fn apply(&self, m: &mut MaterialChannels) {
        m.diffuse_color = self.base_color;
        m.metallic = self.metallic;
        m.roughness = self.roughness;
        m.clearcoat_amount = self.coat;
        m.clearcoat_roughness = self.coat_roughness;
        m.subsurface_amount = self.subsurface;
        m.subsurface_color = self.subsurface_color;
        // With subsurface on, Cycles' base colour leans to the OpenPBR
        // subsurface albedo (shader_eval, OPM-Q01): it carries the body
        // colour.
        m.openpbr.subsurface_albedo = self.subsurface_color;
        m.subsurface_distance = self.subsurface_distance;
        m.smoothing = 1.;
        m.smoothing_angle = 89.;
    }
}

/// Post-processing a look gets after the render.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LookPost {
    None,
    /// Lightness quantised into this many flat bands (the renderer has no
    /// stepped ramp, so toon looks are a clay render banded afterwards).
    Toon(u8),
}

/// One matcap of the fixed batch set.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Look {
    /// Display name and file stem (`<name>.png`).
    pub name: &'static str,
    pub material: CodeLook,
    pub post: LookPost,
}

const CLAY_WARM: CodeLook = CodeLook::plain([0.60, 0.42, 0.32], 0., 0.65);
const CLAY_COOL: CodeLook = CodeLook::plain([0.40, 0.45, 0.52], 0., 0.65);
/// A neutral light grey clay for the toon bands to quantise.
const TOON_BASE: CodeLook = CodeLook::plain([0.62, 0.60, 0.58], 0., 0.8);

/// The batch set, in contact-sheet order. Gold and copper colours are the
/// usual measured F0 reflectances (linear), chrome a bright neutral mirror.
pub const LOOKS: &[Look] = &[
    Look {
        name: "Clay Warm",
        material: CLAY_WARM,
        post: LookPost::None,
    },
    Look {
        name: "Clay Cool",
        material: CLAY_COOL,
        post: LookPost::None,
    },
    Look {
        name: "Chrome",
        material: CodeLook::plain([0.90, 0.90, 0.92], 1., 0.04),
        post: LookPost::None,
    },
    Look {
        name: "Gold",
        material: CodeLook::plain([1.00, 0.71, 0.29], 1., 0.18),
        post: LookPost::None,
    },
    Look {
        name: "Copper",
        material: CodeLook::plain([0.95, 0.64, 0.54], 1., 0.22),
        post: LookPost::None,
    },
    Look {
        name: "Car Paint Red",
        material: CodeLook {
            coat: 1.,
            coat_roughness: 0.02,
            ..CodeLook::plain([0.50, 0.02, 0.02], 0.3, 0.35)
        },
        post: LookPost::None,
    },
    Look {
        name: "Wax",
        material: CodeLook {
            subsurface: 1.,
            subsurface_color: [0.85, 0.55, 0.28],
            subsurface_distance: 0.04,
            ..CodeLook::plain([0.80, 0.60, 0.36], 0., 0.35)
        },
        post: LookPost::None,
    },
    Look {
        name: "Jade",
        material: CodeLook {
            subsurface: 1.,
            subsurface_color: [0.06, 0.42, 0.16],
            subsurface_distance: 0.03,
            ..CodeLook::plain([0.04, 0.28, 0.10], 0., 0.12)
        },
        post: LookPost::None,
    },
    Look {
        name: "Toon 2-step",
        material: TOON_BASE,
        post: LookPost::Toon(2),
    },
    Look {
        name: "Toon 3-step",
        material: TOON_BASE,
        post: LookPost::Toon(3),
    },
];

/// The look called `name` (case and spaces/hyphens ignored, so
/// `car-paint-red` finds "Car Paint Red").
pub fn look_named(name: &str) -> Option<&'static Look> {
    let key = |s: &str| {
        s.chars()
            .filter(|c| c.is_alphanumeric())
            .flat_map(char::to_lowercase)
            .collect::<String>()
    };
    let want = key(name);
    LOOKS.iter().find(|l| key(l.name) == want)
}

fn sphere_item(mesh: Mesh) -> Item {
    Item {
        id: 1,
        name: "Matcap Sphere".into(),
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
        mesh_ops: None,
        kit: None,
        instance_of: None,
        replicator: None,
    }
}

/// The sphere with a material group for [`SPHERE_TAG`] holding one
/// material set up by `set`; returns the scene and the group.
fn sphere_scene(set: impl FnOnce(&mut MaterialChannels)) -> Result<(Scene, u64)> {
    let mut scene = Scene::default();
    scene
        .items
        .push(sphere_item(sphere(SIDES, RINGS, SPHERE_TAG)?));
    scene.next_id = 2;
    let group = scene.shader_tree.add(NewLayer::Group, None)?;
    let mut node = scene.shader_tree.get(group).ok_or("no group")?.clone();
    node.name = SPHERE_TAG.into();
    node.kind = NodeKind::Group(GroupMask {
        tag_type: TagType::Material,
        tag: TagMask::Name(SPHERE_TAG.into()),
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
    if let NodeKind::Material(m) = &mut node.kind {
        set(m);
    }
    scene.shader_tree.replace(node)?;
    Ok((scene, group))
}

/// The sphere wearing a code-defined look.
pub fn code_scene(look: &CodeLook) -> Result<Scene> {
    Ok(sphere_scene(|m| look.apply(m))?.0)
}

/// The sphere wearing a material preset, through the ordinary preset load.
pub fn preset_scene(preset: &Preset) -> Result<Scene> {
    if preset.header.kind != PresetKind::Material {
        return Err("Not a material preset".into());
    }
    let (scene, group) = sphere_scene(|_| {})?;
    let mut editor = Editor {
        scene,
        ..Editor::default()
    };
    editor.load_preset(preset.clone(), Some(group))?;
    Ok(editor.scene)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_sphere_is_closed_round_and_faces_outward() {
        let mesh = sphere(24, 12, SPHERE_TAG).unwrap();
        assert_eq!(mesh.faces.len(), 24 * 12);
        assert_eq!(mesh.vertices.len(), 24 * 11 + 2);
        for v in &mesh.vertices {
            assert!((v.length() - RADIUS).abs() < 1e-12);
        }
        for f in &mesh.faces {
            let pts: Vec<DVec3> = f
                .vertices
                .iter()
                .map(|&v| mesh.vertices[v as usize])
                .collect();
            let mid = pts.iter().sum::<DVec3>() / pts.len() as f64;
            let mut n = DVec3::ZERO;
            for i in 0..pts.len() {
                n += pts[i].cross(pts[(i + 1) % pts.len()]);
            }
            assert!(n.dot(mid) > 0., "a polygon faces inward");
            assert!(f.smooth);
            assert_eq!(f.material, SPHERE_TAG);
        }
        // Closed: every edge is shared by exactly two polygons.
        let mut edges = std::collections::HashMap::new();
        for f in &mesh.faces {
            for i in 0..f.vertices.len() {
                let (a, b) = (f.vertices[i], f.vertices[(i + 1) % f.vertices.len()]);
                *edges.entry((a.min(b), a.max(b))).or_insert(0) += 1;
            }
        }
        assert!(edges.values().all(|&n| n == 2));
    }

    #[test]
    fn the_framing_fits_the_sphere_to_the_square_exactly() {
        let f = framing();
        // Looking straight down −Z at the centre, orthographic half height
        // equal to the radius: the silhouette touches every edge.
        assert_eq!(f.target, DVec3::ZERO);
        assert!(f.eye.x == 0. && f.eye.y == 0. && f.eye.z > RADIUS);
        assert_eq!(f.half_height, RADIUS);
        // Centre pixels are fully inside, corners fully outside, and the
        // edge midpoints are on the rim.
        let size = 512;
        assert_eq!(disc_coverage(256, 256, size), 1.);
        assert_eq!(disc_coverage(0, 0, size), 0.);
        assert_eq!(disc_coverage(size - 1, size - 1, size), 0.);
        let rim = disc_coverage(0, 256, size);
        assert!(
            rim > 0.4 && rim <= 1.,
            "left edge midpoint is on the rim: {rim}"
        );
        // The disc covers π/4 of the square.
        let total: f64 = (0..64)
            .flat_map(|y| (0..64).map(move |x| disc_coverage(x, y, 64)))
            .sum();
        assert!((total / (64. * 64.) - PI / 4.).abs() < 0.01);
    }

    #[test]
    fn the_batch_set_is_the_ten_looks_and_names_resolve() {
        let names: Vec<_> = LOOKS.iter().map(|l| l.name).collect();
        assert_eq!(
            names,
            [
                "Clay Warm",
                "Clay Cool",
                "Chrome",
                "Gold",
                "Copper",
                "Car Paint Red",
                "Wax",
                "Jade",
                "Toon 2-step",
                "Toon 3-step"
            ]
        );
        assert_eq!(look_named("car-paint-red").unwrap().name, "Car Paint Red");
        assert_eq!(look_named("toon 3 step").unwrap().post, LookPost::Toon(3));
        assert!(look_named("plutonium").is_none());
    }

    #[test]
    fn code_looks_land_on_the_sphere_material() {
        let look = LOOKS.iter().find(|l| l.name == "Gold").unwrap();
        let code = look.material;
        let scene = code_scene(&code).unwrap();
        assert_eq!(scene.items.len(), 1);
        let m = scene
            .shader_tree
            .nodes
            .iter()
            .find_map(|n| match &n.kind {
                NodeKind::Material(m) => Some(m),
                _ => None,
            })
            .unwrap();
        assert_eq!(m.metallic, 1.);
        assert_eq!(m.diffuse_color, code.base_color);
    }
}
