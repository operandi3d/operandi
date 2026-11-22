use super::*;
use glam::{DVec2, DVec3};

/// Test meshes: `other`'s points and polygons added to `mesh`.
trait Append {
    fn append(&mut self, other: &Mesh) -> Result<()>;
}
impl Append for Mesh {
    fn append(&mut self, other: &Mesh) -> Result<()> {
        let base = self.vertices.len() as u32;
        self.vertices.extend(&other.vertices);
        let mut faces = self.faces.to_vec();
        for f in other.faces.iter() {
            let mut f = f.clone();
            for v in &mut f.vertices {
                *v += base;
            }
            faces.push(f);
        }
        self.faces = faces.into();
        self.validate()
    }
}

/// Each quad gets its own cell of a 4×4 grid, with a cell-wide margin.
fn grid_uvs(mesh: &mut Mesh) {
    let uvs = (0..mesh.faces.len())
        .map(|f| {
            let (cx, cy) = ((f % 4) as f64 * 0.25, (f / 4) as f64 * 0.25);
            let square = [(0.02, 0.02), (0.23, 0.02), (0.23, 0.23), (0.02, 0.23)];
            (0..mesh.faces[f].vertices.len())
                .map(|c| {
                    let (u, v) = square[c % 4];
                    Some(DVec2::new(cx + u, cy + v))
                })
                .collect()
        })
        .collect();
    mesh.uv_maps.insert(crate::PRIMITIVE_UV_MAP.into(), uvs);
}

fn scene_with(mesh: Mesh) -> (Scene, u64) {
    let mut scene = Scene::default();
    scene.new_mesh().unwrap();
    let item = scene.items.last_mut().unwrap();
    item.mesh = mesh.into();
    let id = item.id;
    scene.select_item(Some(id));
    (scene, id)
}

/// The pixel at the middle of face `f`'s cell, and one 1 % in from its
/// first corner (next to the face's edges).
fn cell_pixels(f: usize, size: u32) -> (usize, usize) {
    let s = size as f64;
    let at = |u: f64, v: f64| ((1.0 - v) * s) as usize * size as usize + (u * s) as usize;
    let (cx, cy) = ((f % 4) as f64 * 0.25, (f / 4) as f64 * 0.25);
    (at(cx + 0.125, cy + 0.125), at(cx + 0.025, cy + 0.025))
}

#[test]
fn curvature_is_grey_on_flat_faces_and_light_on_convex_edges() {
    let mut mesh = Mesh::box_mesh(DVec3::ZERO, DVec3::ONE, 1).unwrap();
    grid_uvs(&mut mesh);
    let (scene, id) = scene_with(mesh);
    let mut o = BakeOptions::new(BakeMap::Curvature);
    o.size = 256;
    o.radius = 0.004;
    let baked = bake(&scene, id, &o).unwrap();
    assert_eq!(baked.pixels.len(), 256 * 256);
    for f in 0..6 {
        let (middle, edge) = cell_pixels(f, 256);
        assert!(
            (baked.pixels[middle] - 0.5).abs() < 0.05,
            "face {f} middle {}",
            baked.pixels[middle]
        );
        assert!(
            baked.pixels[edge] > 0.6,
            "face {f} edge {}",
            baked.pixels[edge]
        );
    }
    assert!(baked.pixels.iter().all(|v| (0.0..=1.0).contains(v)));
}

#[test]
fn curvature_is_dark_in_a_concave_corner() {
    // An L of two boxes: the inner corner where they meet is concave.
    let mut mesh = Mesh::box_mesh(DVec3::new(0.0, 0.0, 0.0), DVec3::new(2.0, 0.5, 2.0), 1).unwrap();
    let wall = Mesh::box_mesh(DVec3::new(-0.75, 1.0, 0.0), DVec3::new(0.5, 1.5, 2.0), 1).unwrap();
    mesh.append(&wall).unwrap();
    grid_uvs(&mut mesh);
    let (scene, id) = scene_with(mesh.clone());
    let mut o = BakeOptions::new(BakeMap::Curvature);
    o.size = 512;
    o.radius = 0.01;
    let baked = bake(&scene, id, &o).unwrap();
    // The first box's top face meets the wall along its x = -0.5 side.
    let top = (0..6)
        .find(|&f| mesh.normal(f).normalize().y > 0.9)
        .expect("a top face");
    let lowest = baked
        .pixels
        .iter()
        .zip(rasterise(
            &BakeMesh::new(&mesh, &scene.smoothing, None).unwrap(),
            512,
            512,
        ))
        .filter(|(_, s)| s.is_some_and(|s| s.tri / 2 == top as u32))
        .map(|(v, _)| *v)
        .fold(1.0f32, f32::min);
    assert!(lowest < 0.4, "{lowest}");
}

#[test]
fn thickness_is_darker_where_the_mesh_is_thin() {
    let mut mesh = Mesh::box_mesh(DVec3::ZERO, DVec3::new(2.0, 0.05, 2.0), 1).unwrap();
    mesh.append(&Mesh::box_mesh(DVec3::new(4.0, 0.0, 0.0), DVec3::splat(2.0), 1).unwrap())
        .unwrap();
    grid_uvs(&mut mesh);
    let (scene, id) = scene_with(mesh.clone());
    let mut o = BakeOptions::new(BakeMap::Thickness);
    o.size = 128;
    o.rays = 32;
    o.distance = 0.5;
    let baked = bake(&scene, id, &o).unwrap();
    let up = |first: usize| {
        (first..first + 6)
            .find(|&f| mesh.normal(f).normalize().y > 0.9)
            .unwrap()
    };
    let thin = baked.pixels[cell_pixels(up(0), 128).0];
    let thick = baked.pixels[cell_pixels(up(6), 128).0];
    assert!(thin < 0.2 && thick > 0.8, "thin {thin} thick {thick}");
}

#[test]
fn padding_spreads_islands_and_fill_leaves_no_background() {
    let mut mesh = Mesh::box_mesh(DVec3::ZERO, DVec3::ONE, 1).unwrap();
    grid_uvs(&mut mesh);
    let (scene, id) = scene_with(mesh);
    let mut o = BakeOptions::new(BakeMap::Thickness);
    o.size = 64;
    o.rays = 8;
    o.padding = 0;
    o.fill = false;
    let bare = bake(&scene, id, &o).unwrap();
    // The bottom row of the image is outside every cell.
    assert!(
        bare.pixels[64 * 63..].iter().all(|v| *v == 0.0)
            || bare.pixels[64 * 63..]
                .iter()
                .all(|v| v.is_nan() || *v == 0.0)
    );
    o.padding = 2;
    let padded = bake(&scene, id, &o).unwrap();
    // One pixel left of the first cell's left edge copies the edge.
    let row = 64 - 8;
    let next_to = [row - 1, row, row + 1].map(|r| padded.pixels[r * 64 + 1]);
    assert!(next_to.contains(&padded.pixels[row * 64]));
    assert_eq!(bare.pixels[row * 64], 0.0);
    o.fill = true;
    let filled = bake(&scene, id, &o).unwrap();
    assert!(
        filled
            .pixels
            .iter()
            .all(|v| v.is_finite() && (0.0..=1.0).contains(v))
    );
}

#[test]
fn bakes_refuse_meshes_without_uvs_and_bad_settings() {
    let (scene, id) = scene_with(Mesh::box_mesh(DVec3::ZERO, DVec3::ONE, 1).unwrap());
    let mut no_uv = scene.clone();
    no_uv.items.last_mut().unwrap().mesh = {
        let mut m = (*scene.items.last().unwrap().mesh).clone();
        m.uv_maps.clear();
        for f in m.faces.iter_mut() {
            f.uv = None;
        }
        m.into()
    };
    let o = BakeOptions::new(BakeMap::Curvature);
    assert!(bake(&no_uv, id, &o).unwrap_err().contains("no UVs"));
    let mut mesh = Mesh::box_mesh(DVec3::ZERO, DVec3::ONE, 1).unwrap();
    grid_uvs(&mut mesh);
    let (scene, id) = scene_with(mesh);
    let mut small = o.clone();
    small.size = 4;
    assert!(bake(&scene, id, &small).is_err());
    let mut named = o.clone();
    named.uv_map = Some("Missing".into());
    assert!(bake(&scene, id, &named).unwrap_err().contains("Missing"));
    assert!(bake(&scene, 999_999, &o).is_err());
}

fn scratch(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("operandi-bake-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn commands_write_a_png_and_pad_an_existing_texture() {
    let mut mesh = Mesh::box_mesh(DVec3::ZERO, DVec3::ONE, 1).unwrap();
    grid_uvs(&mut mesh);
    let (scene, _) = scene_with(mesh);
    let dir = scratch("cmd");
    let out = dir.join("curv.png");
    let mut a = Map::new();
    a.insert("filename".into(), Value::String(out.display().to_string()));
    a.insert("size".into(), Value::from(64));
    let base = BakeOptions::new(BakeMap::Curvature);
    let o = bake_args(&a, &base).unwrap();
    assert_eq!(o.size, 64);
    let note = run_bake(&scene, &a, &o).unwrap();
    assert!(note.contains("Curvature 64×64"), "{note}");
    let decoder = png::Decoder::new(std::io::BufReader::new(std::fs::File::open(&out).unwrap()));
    let info = decoder.read_info().unwrap().info().clone();
    assert_eq!((info.width, info.bit_depth), (64, png::BitDepth::Sixteen));

    // An RGBA texture: red inside every island, transparent outside.
    let samples = rasterise(
        &BakeMesh::new(&scene.items[0].mesh, &scene.smoothing, None).unwrap(),
        32,
        32,
    );
    let mut rgba = vec![0u8; 32 * 32 * 4];
    for (i, s) in samples.iter().enumerate() {
        if s.is_some() {
            rgba[i * 4..i * 4 + 4].copy_from_slice(&[255, 0, 0, 255]);
        }
    }
    let tex = dir.join("tex.png");
    write_png(
        &tex,
        32,
        32,
        png::ColorType::Rgba,
        png::BitDepth::Eight,
        &rgba,
    )
    .unwrap();
    let mut a = Map::new();
    a.insert("filename".into(), Value::String(tex.display().to_string()));
    a.insert("padding".into(), Value::from(2));
    let note = run_pad(&scene, &a).unwrap();
    assert!(note.starts_with("Padded "), "{note}");
    let mut reader = png::Decoder::new(std::io::BufReader::new(std::fs::File::open(&tex).unwrap()))
        .read_info()
        .unwrap();
    let mut buf = vec![0; reader.output_buffer_size().unwrap()];
    reader.next_frame(&mut buf).unwrap();
    // A pixel next to an island that was empty is red now.
    let grown = (0..32 * 32)
        .find(|&i| samples[i].is_none() && buf[i * 4] == 255)
        .is_some();
    assert!(grown);
    assert!(run_pad(&scene, &Map::new()).is_err());
}
