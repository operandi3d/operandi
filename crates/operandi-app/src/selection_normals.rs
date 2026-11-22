//! Show Selection Normals (Viewport Properties > Visibility, spec v2.1
//! section 8): a short dashed line out of each selected polygon's centre,
//! along the way it faces. Polygon mode only, every active mesh, in this
//! view only. The length is half the polygon's size (provisional: the spec
//! gives none).
use super::*;

/// One selected polygon's normal line, in world space: its centre and tip.
pub(crate) fn normal_lines(scene: &operandi_core::Scene) -> Vec<[DVec3; 2]> {
    let selection = &scene.selection;
    if selection.mode != Mode::Polygon {
        return Vec::new();
    }
    let mut lines = Vec::new();
    for id in scene.shown_active_meshes() {
        let Some(item) = scene.item(id) else {
            continue;
        };
        let Ok(world) = scene.world_matrix(id) else {
            continue;
        };
        let mesh = &item.mesh;
        for &f in selection.faces_of(id) {
            let Some(face) = mesh.faces.get(f as usize) else {
                continue;
            };
            if face.vertices.len() < 3 {
                continue;
            }
            let points: Vec<DVec3> = face
                .vertices
                .iter()
                .map(|&v| world.transform_point3(mesh.vertices[v as usize]))
                .collect();
            let centre = points.iter().sum::<DVec3>() / points.len() as f64;
            // The world normal from the world corners, so mirrored and
            // non-uniformly scaled items still point the way they face.
            let mut normal = DVec3::ZERO;
            for i in 0..points.len() {
                normal += (points[i] - points[0]).cross(points[(i + 1) % points.len()] - points[0]);
            }
            let normal = normal.normalize_or_zero();
            let size = points.iter().map(|p| p.distance(centre)).fold(0., f64::max);
            if normal != DVec3::ZERO && size > 0. {
                lines.push([centre, centre + normal * size * 0.5]);
            }
        }
    }
    lines
}

impl Operandi {
    /// Draw the selected polygons' normals when this view shows them.
    pub(super) fn draw_selection_normals(&self, painter: &egui::Painter, r: Rect) {
        let d = &self.display.settings.drawing;
        if !(d.selections && d.selection_normals) {
            return;
        }
        let stroke = Stroke::new(1.0f32, theme::viewport::selection());
        for [a, b] in normal_lines(&self.editor.scene) {
            if let (Some((a, _)), Some((b, _))) =
                (self.camera.project(a, r), self.camera.project(b, r))
            {
                painter.extend(egui::Shape::dashed_line(&[a, b], stroke, 3.0, 3.0));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_line_per_selected_polygon_pointing_out() {
        let mut a = crate::tests::box_app();
        let id = a.editor.scene.items[0].id;
        a.editor.dispatch(Command::Mode(Mode::Polygon)).unwrap();
        assert!(normal_lines(&a.editor.scene).is_empty(), "nothing selected");
        let mesh = a.editor.scene.items[0].mesh.clone();
        a.editor.scene.selection.faces = vec![0, 2];
        let lines = normal_lines(&a.editor.scene);
        assert_eq!(lines.len(), 2);
        for (line, f) in lines.iter().zip([0usize, 2]) {
            let along = (line[1] - line[0]).normalize();
            let world = a.editor.scene.world_matrix(id).unwrap();
            let expected = world.transform_vector3(mesh.normal(f)).normalize();
            assert!(along.dot(expected) > 0.999, "{along} vs {expected}");
        }
        // Other modes draw none.
        a.editor.scene.selection.mode = Mode::Vertex;
        assert!(normal_lines(&a.editor.scene).is_empty());
    }
}
