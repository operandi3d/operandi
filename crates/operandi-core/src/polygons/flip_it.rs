//! Flip It and Flip It Origin (tool presets spec, section 6b): mirror the
//! polygons Flip works on to the other side of an axis, then flip their
//! facing so the mirrored surface still points out. The mirror plane is
//! square to the chosen world axis and passes through the row's action
//! centre: the selection's bounding-box centre (Flip It) or the world
//! origin (Flip It Origin). Symmetry is ignored, since flipping across the
//! symmetry plane is the point. One undo step.
//!
//! Which polygons: as Flip (flip.rs), the selected ones in Polygon mode,
//! every polygon when none is selected or in Item mode; Vertex and Edge
//! mode refuse. Points shared with polygons that aren't flipped move with
//! the flipped ones, as a Scale of −1 on those points would.
use crate::tool_preset::{CenterName, Layer};
use crate::*;
use glam::DVec3;
use std::collections::BTreeSet;

impl Editor {
    pub(crate) fn flip_it(&mut self, preset: &str, axis: usize) -> Result<()> {
        let row = crate::tool_preset::row(preset)
            .filter(|r| r.actor.operation.as_deref() == Some("flip_it"))
            .ok_or_else(|| format!("{preset} is not a Flip It preset"))?;
        if axis > 2 {
            return Err("Flip It's axis is X, Y or Z".into());
        }
        if self.session.has_active_task() || self.selection_start.is_some() {
            return Err(format!(
                "Drop or Cancel the active task before {}",
                row.label
            ));
        }
        let through_origin = row.layers.iter().any(|l| {
            matches!(
                l,
                Layer::ActionCenter {
                    center: CenterName::Origin
                }
            )
        });
        let mut scene = self.scene.clone();
        scene.validate()?;
        if matches!(scene.selection.mode, Mode::Vertex | Mode::Edge) {
            return Err(format!(
                "{} works on polygons; switch to Polygon mode",
                row.label
            ));
        }
        let item = scene
            .active()
            .filter(|i| scene.shows(i))
            .ok_or_else(|| format!("Select a visible mesh for {}", row.label))?;
        let world = scene.world_matrix(item.id)?;
        let local = world.inverse();
        let mesh = &item.mesh;
        let all = scene.selection.mode != Mode::Polygon || scene.selection.faces.is_empty();
        let points: BTreeSet<u32> = (0..mesh.faces.len())
            .filter(|&f| !mesh.is_polyline(f))
            .filter(|&f| all || scene.selection.faces.contains(&(f as u32)))
            .flat_map(|f| mesh.faces[f].vertices.iter().copied())
            .collect();
        if points.is_empty() {
            return Err(format!("{} needs some polygons", row.label));
        }
        let at = |v: u32| world.transform_point3(mesh.vertices[v as usize]);
        let plane = if through_origin {
            0.
        } else {
            let (lo, hi) = points.iter().fold(
                (DVec3::splat(f64::INFINITY), DVec3::splat(f64::NEG_INFINITY)),
                |(lo, hi), &v| (lo.min(at(v)), hi.max(at(v))),
            );
            (lo[axis] + hi[axis]) / 2.
        };
        let mut out = mesh.clone();
        for &v in &points {
            let mut p = at(v);
            p[axis] = 2. * plane - p[axis];
            out.vertices[v as usize] = local.transform_point3(p);
        }
        scene.active_mut().ok_or("Missing Flip It target")?.mesh = out;
        let candidate = crate::flip::flip_winding(&scene)?;
        let before = std::mem::replace(&mut self.scene, candidate);
        self.record(row.label.clone(), before);
        Ok(())
    }
}
