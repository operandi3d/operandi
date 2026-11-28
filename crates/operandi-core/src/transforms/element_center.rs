//! C-ELEM and A-ELEM (AC-RULES v6): the Element Action Center's frame for
//! a picked polygon, edge or point, and what a viewport press off the
//! handle does to each Action Center.
use crate::{ActionContext, ActionFrame, AxisOrientation, CenterMode, PickedFrame, PickedPoint};
use crate::{Result, Scene, TweakElement};
use glam::{DMat3, DVec3};

/// What a press off the tool's handle does to the Action Center (the rule
/// sheet's "Viewport press off the handle" column). The press then drags
/// the tool either way.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CenterPress {
    /// C-ELEM / A-ELEM in either part: the element under the pointer is
    /// picked, on every press. A press on empty space changes nothing.
    Pick,
    /// C-AUTO (Automatic, None) and C-SCREEN: every press puts the centre
    /// where it lands on the work plane; Screen's is never snapped and
    /// takes the view's axes with it.
    Place { snapped: bool },
    /// Every other centre: a press never moves it.
    Stay,
}

impl ActionContext {
    /// Element as it is before anything is picked: the world origin with
    /// World axes.
    pub const ELEMENT_UNPICKED: Self = Self {
        center: CenterMode::Element(PickedPoint::ORIGIN),
        axis: AxisOrientation::Element(PickedFrame::WORLD),
    };

    /// What a press off the handle does under this Action Center.
    pub fn press(self) -> CenterPress {
        if matches!(self.center, CenterMode::Element(_))
            || matches!(self.axis, AxisOrientation::Element(_))
        {
            return CenterPress::Pick;
        }
        match self.center {
            CenterMode::Automatic | CenterMode::None => CenterPress::Place { snapped: true },
            CenterMode::Screen(_) => CenterPress::Place { snapped: false },
            _ => CenterPress::Stay,
        }
    }

    /// This Action Center after `pick` (a picked element's frame): the
    /// Element parts take it, the other part is kept (the Center and Axis
    /// lists' mixing rule).
    pub fn picked(self, pick: ActionFrame) -> Self {
        Self {
            center: match self.center {
                CenterMode::Element(_) => CenterMode::Element(PickedPoint::new(pick.center)),
                other => other,
            },
            axis: match self.axis {
                AxisOrientation::Element(_) => {
                    AxisOrientation::Element(PickedFrame::from_basis(pick.basis))
                }
                other => other,
            },
        }
    }
}

/// A-ELEM's axes from an element's `normal` and, for an edge, its own
/// direction `along` (zero otherwise): Z along the normal; X the edge's
/// direction made square to Z, otherwise world X made square to Z (world Z
/// when Z runs along X); Y = Z × X. No normal (a wire) gives World.
pub fn element_axes(normal: DVec3, along: DVec3) -> DMat3 {
    let Some(z) = normal.try_normalize().filter(|z| z.is_finite()) else {
        return DMat3::IDENTITY;
    };
    // `v` laid square to Z, unless it runs along Z.
    let across = |v: DVec3| {
        let v = v.normalize_or_zero();
        let flat = v - z * v.dot(z);
        (flat.is_finite() && flat.length() > 1e-6).then(|| flat.normalize())
    };
    match across(along)
        .or_else(|| across(DVec3::X))
        .or_else(|| across(DVec3::Z))
    {
        Some(x) => DMat3::from_cols(x, z.cross(x), z),
        None => DMat3::IDENTITY,
    }
}

/// The Element frame of `element` of mesh item `item`, in world space.
///
/// Centre (C-ELEM): a polygon's is the average of its points, an edge's
/// its midpoint, a point's the point. Axes (A-ELEM, `element_axes`): the
/// normal is the polygon's own, or for an edge or a point the sum of the
/// normals of the polygons that use it; an edge's X runs along it, from
/// its lower-numbered point to the other.
pub fn element_frame(scene: &Scene, item: u64, element: TweakElement) -> Result<ActionFrame> {
    let it = scene.item(item).ok_or("That item no longer exists")?;
    let mesh = &it.mesh;
    let world = scene.world_matrix(item)?;
    let points = element.points(mesh)?;
    let center = points
        .iter()
        .map(|&v| world.transform_point3(mesh.vertices[v as usize]))
        .sum::<DVec3>()
        / points.len().max(1) as f64;
    if !center.is_finite() {
        return Err("That element has no finite centre".into());
    }
    let surface = |f: usize| !mesh.is_polyline(f);
    let mut using: Vec<usize> = match element {
        TweakElement::Polygon(f) => vec![f as usize],
        TweakElement::Edge(a, b) => mesh
            .edge_faces()
            .get(&crate::topology::edge_key(a, b))
            .cloned()
            .unwrap_or_default(),
        TweakElement::Vertex(v) => mesh
            .point_faces()
            .of(v)
            .iter()
            .map(|&f| f as usize)
            .collect(),
    };
    using.sort_unstable();
    using.dedup();
    // Normals go to the world by the inverse transpose, so they stay
    // square to the surface on a stretched item.
    let linear = DMat3::from_mat4(world);
    let to_world = if linear.determinant().abs() > 1e-300 {
        linear.inverse().transpose()
    } else {
        linear
    };
    let normal: DVec3 = using
        .into_iter()
        .filter(|&f| f < mesh.faces.len() && surface(f))
        .map(|f| (to_world * mesh.normal(f)).normalize_or_zero())
        .sum();
    let along = match element {
        TweakElement::Edge(a, b) => {
            let (a, b) = crate::topology::edge_key(a, b);
            linear * (mesh.vertices[b as usize] - mesh.vertices[a as usize])
        }
        _ => DVec3::ZERO,
    };
    Ok(ActionFrame {
        center,
        basis: element_axes(normal, along),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: DVec3, b: DVec3) {
        assert!((a - b).length() < 1e-9, "{a:?} != {b:?}");
    }

    #[test]
    fn z_is_the_normal_and_x_is_world_x_laid_square() {
        let n = DVec3::new(0., 1., -1.).normalize();
        let b = element_axes(n * 3., DVec3::ZERO);
        close(b.z_axis, n);
        close(b.x_axis, DVec3::X);
        close(b.y_axis, n.cross(DVec3::X));
        // Facing along world X: world Z stands in.
        let b = element_axes(DVec3::X, DVec3::ZERO);
        close(b.z_axis, DVec3::X);
        close(b.x_axis, DVec3::Z);
        close(b.y_axis, DVec3::X.cross(DVec3::Z));
    }

    #[test]
    fn an_edge_gives_x_its_own_direction() {
        let along = DVec3::new(1., 0., 1.);
        let b = element_axes(DVec3::Y, along);
        close(b.x_axis, along.normalize());
        close(b.z_axis, DVec3::Y);
        // An edge that runs along its normal has no direction across it.
        close(element_axes(DVec3::Y, DVec3::Y).x_axis, DVec3::X);
    }

    #[test]
    fn a_wire_is_world() {
        assert_eq!(element_axes(DVec3::ZERO, DVec3::X), DMat3::IDENTITY);
        assert_eq!(
            ActionContext::ELEMENT_UNPICKED.axis.basis().unwrap(),
            DMat3::IDENTITY
        );
    }

    #[test]
    fn the_press_rule_follows_the_centre_and_either_element_part() {
        let with = |center, axis| ActionContext { center, axis }.press();
        let world = AxisOrientation::World;
        assert_eq!(ActionContext::ELEMENT_UNPICKED.press(), CenterPress::Pick);
        assert_eq!(
            with(
                CenterMode::Origin,
                AxisOrientation::Element(PickedFrame::WORLD)
            ),
            CenterPress::Pick
        );
        assert_eq!(
            with(CenterMode::Element(PickedPoint::ORIGIN), world),
            CenterPress::Pick
        );
        for center in [CenterMode::None, CenterMode::Automatic] {
            assert_eq!(with(center, world), CenterPress::Place { snapped: true });
        }
        assert_eq!(
            with(CenterMode::Screen(PickedPoint::ORIGIN), world),
            CenterPress::Place { snapped: false }
        );
        for center in [
            CenterMode::Selection,
            CenterMode::SelectionBorder,
            CenterMode::SelectionAutoAxis,
            CenterMode::Origin,
            CenterMode::Parent,
            CenterMode::Local,
            CenterMode::Pivot,
        ] {
            assert_eq!(with(center, world), CenterPress::Stay, "{center:?}");
        }
    }

    #[test]
    fn a_pick_fills_only_the_element_parts() {
        let pick = ActionFrame {
            center: DVec3::new(1., 2., 3.),
            basis: element_axes(DVec3::X, DVec3::ZERO),
        };
        let both = ActionContext::ELEMENT_UNPICKED.picked(pick);
        assert_eq!(
            both.center,
            CenterMode::Element(PickedPoint::new(pick.center))
        );
        assert_eq!(both.axis.basis().unwrap(), pick.basis);
        let centre_only = ActionContext {
            center: CenterMode::Element(PickedPoint::ORIGIN),
            axis: AxisOrientation::World,
        }
        .picked(pick);
        assert_eq!(centre_only.axis, AxisOrientation::World);
        let axis_only = ActionContext {
            center: CenterMode::Origin,
            axis: AxisOrientation::Element(PickedFrame::WORLD),
        }
        .picked(pick);
        assert_eq!(axis_only.center, CenterMode::Origin);
        assert_eq!(axis_only.axis.basis().unwrap(), pick.basis);
    }
}
