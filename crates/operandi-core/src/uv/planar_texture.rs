//! Placing a Planar texture (Planar spec 2026-11-09; choices logged as
//! PD-Q01.. in docs/SPECIFICATION_QUESTIONS.md).
//!
//! There is no dedicated drag: a texture is placed with the ordinary
//! Move/Rotate/Scale/Transform tools on its Texture Locator, and selecting
//! the texture layer makes its locator the tools' target (PD-Q02, in the
//! app). This module holds what that needs in the core:
//!
//! - which locator a texture layer places (`Scene::layer_locator`);
//! - the surface item whose space a World-Coordinates-off locator is drawn
//!   in, so the plane draws where the render puts the texture
//!   (`Scene::locator_host`, PD-Q07);
//! - the image's U and V directions per Projection Axis (`plane_axes`), for
//!   the plane's up tick;
//! - typed or scripted transform edits of the live tool's locator joining
//!   that tool as one tweak (`route_item_transform`, UIC-160/161, PD-Q12).
use crate::scene::{Scene, Transform};
use crate::shader_tree::{NodeId, NodeKind};
use glam::DVec3;

/// The locator-space component and sign that carry the image's U and V for
/// Projection Axis `axis`, matching `texture_locator::planar`: X maps
/// u = −Lz, v = Ly; Y maps u = Lx, v = −Lz; Z maps u = Lx, v = Ly.
pub fn plane_axes(axis: u8) -> [(usize, f64); 2] {
    match axis {
        0 => [(2, -1.), (1, 1.)],
        1 => [(0, 1.), (2, -1.)],
        _ => [(0, 1.), (1, 1.)],
    }
}

impl Scene {
    /// The Texture Locator texture layer `layer` places, if it has one
    /// (any projection: selecting the layer targets it, PD-Q02).
    pub fn layer_locator(&self, layer: NodeId) -> Option<u64> {
        let tree = self.shading_tree();
        let NodeKind::Texture(t) = &tree.get(layer)?.kind else {
            return None;
        };
        let id = t.locator?;
        self.texture_locator(id).map(|_| id)
    }

    /// The surface item whose space a World-Coordinates-off locator is
    /// drawn in (PD-Q07): the first visible mesh, in item order, with a
    /// polygon that a layer using `locator` applies to. `None` when none
    /// does (the locator is then drawn in world space).
    pub fn locator_host(&self, locator: u64) -> Option<u64> {
        let tree = self.shading_tree();
        let layers: Vec<NodeId> = tree
            .nodes
            .iter()
            .filter(|n| matches!(&n.kind, NodeKind::Texture(t) if t.locator == Some(locator)))
            .map(|n| n.id)
            .collect();
        if layers.is_empty() {
            return None;
        }
        for item in self.items.iter().filter(|i| i.is_mesh() && self.shows(i)) {
            let ancestors = self.item_ancestors(item.id);
            let mut sets: Vec<Vec<&str>> = vec![Vec::new(); item.mesh.faces.len()];
            for (name, faces) in &item.mesh.attributes.polygon_sets {
                for &f in faces {
                    if let Some(list) = sets.get_mut(f as usize) {
                        list.push(name.as_str());
                    }
                }
            }
            let mut checked: std::collections::HashSet<(&str, Option<&str>, &[&str])> =
                Default::default();
            for (f, face) in item.mesh.faces.iter().enumerate() {
                if !checked.insert((
                    face.material.as_str(),
                    face.part.as_deref(),
                    sets[f].as_slice(),
                )) {
                    continue;
                }
                let ctx = crate::shader_eval::Context {
                    item: item.id,
                    ancestors: &ancestors,
                    material: &face.material,
                    part: face.part.as_deref(),
                    sets: &sets[f],
                    locators: &[],
                };
                if layers.iter().any(|&l| tree.layer_applies(l, &ctx)) {
                    return Some(item.id);
                }
            }
        }
        None
    }

    /// Locator `locator`'s matrix in the space the render places it: its
    /// world matrix, with `host`'s world matrix in front for World
    /// Coordinates off (the render's `inverse(locator) · inverse(item)`).
    pub fn locator_render_matrix(&self, locator: u64, host: Option<u64>) -> Option<glam::DMat4> {
        let settings = self.texture_locator(locator)?;
        let world = self.world_matrix(locator).ok()?;
        let m = match host {
            Some(h) if !settings.world_coordinates => self.world_matrix(h).ok()? * world,
            _ => world,
        };
        m.is_finite().then_some(m)
    }
}

/// `Command::ItemTransform` of a Texture Locator that a live Move, Rotate,
/// Scale or Transform holds alone (Item mode) becomes a tweak of that tool
/// (`ItemChannelEdit::Transform`), so `transform.channel` typed, scripted
/// or from the MCP bridge is one step inside the tool, as a drag of the
/// tool's handle is (UIC-160/161, PD-Q12). Anything else passes through
/// unchanged (idle: its own step).
pub(crate) fn route_item_transform(e: &crate::Editor, cmd: crate::Command) -> crate::Command {
    use crate::{Command, Mode};
    match cmd {
        Command::ItemTransform { item, transform }
            if e.session
                .live
                .as_ref()
                .is_some_and(|l| l.tool.transforms_items())
                && e.scene.selection.mode == Mode::Item
                && e.scene.selected_items() == [item]
                && e.scene.texture_locator(item).is_some() =>
        {
            Command::ToolItemChannel {
                item,
                edit: crate::camera_light::ItemChannelEdit::Transform(transform),
            }
        }
        other => other,
    }
}

/// The locator's base Transform that, with the live tool's `params`
/// applied as the item tools apply them (editor.rs, single item), gives
/// `target`. Lets a typed edit set the scene's Transform while the tool's
/// own offset keeps working on top of it.
pub fn base_for_tool(
    target: &Transform,
    tool: crate::Tool,
    p: &crate::Parameters,
) -> crate::Result<Transform> {
    use crate::Tool;
    let mut base = target.clone();
    let unscale = |s: DVec3, f: DVec3| {
        if f.abs().min_element() <= 1e-12 {
            Err("Scale can't be zero")
        } else {
            Ok(s / f)
        }
    };
    match tool {
        Tool::Transform => {
            base.translation -= p.delta;
            base.rotation -= p.rotation_delta.map(f64::to_radians);
            base.scale = unscale(base.scale, p.scale_delta)?;
        }
        Tool::Move => base.translation -= p.delta,
        Tool::Rotate => base.rotation -= p.delta.map(f64::to_radians),
        Tool::Scale => base.scale = unscale(base.scale, p.delta)?,
        _ => return Err("Use Move, Rotate, Scale or Transform".into()),
    }
    Ok(base)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn near(a: DVec3, b: DVec3) -> bool {
        (a - b).length() < 1e-9
    }

    #[test]
    fn plane_axes_match_the_texture_maths() {
        // The +U and +V unit points land at u = 1 and v = 1 for an image.
        for axis in 0..3u8 {
            let [(cu, su), (cv, sv)] = plane_axes(axis);
            let mut u = DVec3::ZERO;
            u[cu] = 0.5 * su;
            let mut v = DVec3::ZERO;
            v[cv] = 0.5 * sv;
            let pu = crate::texture_locator::planar(axis, u, 0.5);
            let pv = crate::texture_locator::planar(axis, v, 0.5);
            assert!((pu[0] - 1.).abs() < 1e-12 && (pu[1] - 0.5).abs() < 1e-12);
            assert!((pv[1] - 1.).abs() < 1e-12 && (pv[0] - 0.5).abs() < 1e-12);
        }
    }

    /// Planar spec: Size clamps at 0.001 m, sign kept, through the panel's
    /// SetLocator as well.
    #[test]
    fn size_clamps_at_a_millimetre() {
        use crate::texture_locator::{MIN_SIZE, clamp_size};
        assert_eq!(
            clamp_size(DVec3::new(0., -1e-6, 2.)),
            DVec3::new(MIN_SIZE, -MIN_SIZE, 2.)
        );
        let mut e = crate::Editor::default();
        e.dispatch(crate::Command::ShaderTree(
            crate::shader_tree::ShaderTreeEdit::Add {
                layer: crate::shader_tree::NewLayer::ImageMap,
                selected: None,
            },
        ))
        .unwrap();
        let item = e
            .scene
            .items
            .iter()
            .find(|i| i.texture_locator().is_some())
            .unwrap();
        let (id, settings) = (item.id, item.texture_locator().unwrap().clone());
        e.dispatch(crate::Command::ShaderTree(
            crate::shader_tree::ShaderTreeEdit::SetLocator {
                locator: id,
                settings: Box::new(settings),
                transform: Transform {
                    scale: DVec3::new(1e-6, 1., 1.),
                    ..Default::default()
                },
                continuing: false,
            },
        ))
        .unwrap();
        let item = e.scene.item(id).unwrap();
        assert_eq!(item.transform.scale.x, MIN_SIZE);
    }

    #[test]
    fn base_for_tool_inverts_each_tool() {
        use crate::{Parameters, Tool};
        let target = Transform {
            translation: DVec3::new(1., 2., 3.),
            rotation: DVec3::new(0.1, 0.2, 0.3),
            scale: DVec3::new(2., 4., 8.),
            zxy: false,
            order: None,
        };
        let p = Parameters {
            delta: DVec3::new(1., 1., 2.),
            rotation_delta: DVec3::new(90., 0., 0.),
            scale_delta: DVec3::new(2., 2., 2.),
            ..Default::default()
        };
        let b = base_for_tool(&target, Tool::Move, &p).unwrap();
        assert!(near(b.translation + p.delta, target.translation));
        let b = base_for_tool(&target, Tool::Scale, &p).unwrap();
        assert!(near(b.scale * p.delta, target.scale));
        let b = base_for_tool(&target, Tool::Rotate, &p).unwrap();
        assert!(near(
            b.rotation + p.delta.map(f64::to_radians),
            target.rotation
        ));
        let b = base_for_tool(&target, Tool::Transform, &p).unwrap();
        assert!(near(b.scale * p.scale_delta, target.scale));
        assert!(near(b.translation + p.delta, target.translation));
        assert!(base_for_tool(&target, Tool::Bevel, &p).is_err());
    }
}
