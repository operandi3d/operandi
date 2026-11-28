//! Unit Cone, Torus and Capsule (Ctrl-click / Shift-click on their Create
//! panel cells), like the unit Cube, Sphere, Cylinder and Ellipsoid: a
//! primitive about 1 m across at the mesh's origin, made at once as one
//! undo step. Ctrl-click adds it to the active mesh; Shift-click makes a
//! new mesh for it. Each is its tool's reset shape (unit-primitive-2026-11-05):
//! Cone ±0.5 × ±0.75, Torus 1.5 m across, Capsule 1.5 m tall.
use crate::*;

/// The unit shapes beyond Cube/Sphere/Cylinder/Ellipsoid.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnitShape {
    Cone,
    Torus,
    Capsule,
    /// Plane Item (plane_item.rs, MT5).
    Plane,
    /// Teapot Item (teapot.rs, MT6-05).
    Teapot,
}
impl UnitShape {
    pub fn label(self) -> &'static str {
        match self {
            Self::Cone => "Cone",
            Self::Torus => "Torus",
            Self::Capsule => "Capsule",
            Self::Plane => "Plane",
            Self::Teapot => "Teapot",
        }
    }
}

impl Editor {
    /// The unit shape's geometry: the tool's reset shape, whatever its
    /// last-used settings (unit-primitive-2026-11-05 rules 9 and 11).
    pub fn unit_shape_geometry(&self, shape: UnitShape) -> Result<Mesh> {
        match shape {
            UnitShape::Cone => cone_geometry(&ConeParameters::default()),
            UnitShape::Torus => ShapeParameters::Torus(TorusParameters::default()).geometry(),
            UnitShape::Capsule => ShapeParameters::Capsule(CapsuleParameters::default()).geometry(),
            UnitShape::Plane => Ok(crate::plane_item::plane_item_geometry()),
            UnitShape::Teapot => crate::teapot::teapot_geometry(&Default::default()),
        }
    }

    pub(crate) fn insert_shape_unit(
        &mut self,
        shape: UnitShape,
        destination: UnitDestination,
    ) -> Result<()> {
        let name = shape.label();
        if self.session.has_active_task() || self.selection_start.is_some() {
            return Err(format!(
                "Drop or Cancel the active tool and finish selection before Unit {name}"
            ));
        }
        self.scene.validate()?;
        let mut scene = self.scene.clone();
        if destination == UnitDestination::NewMesh {
            let next = scene.next_id.max(self.next_item_id);
            if next >= u64::MAX - 1 || scene.items.len() >= crate::MAX_SCENE_ITEMS {
                return Err("Scene item/identity budget exhausted".into());
            }
            scene.next_id = next;
            scene.new_mesh()?;
            scene.active_mut().ok_or("Missing new mesh")?.name = name.into();
        }
        let item = scene.active().ok_or_else(|| {
            format!("Select a mesh for Unit {name}; Shift-click makes a new mesh")
        })?;
        if item.backdrop.is_some() {
            return Err(BACKDROP_NO_GEOMETRY.into());
        }
        if !item.is_mesh() {
            return Err(LOCATOR_NO_GEOMETRY.into());
        }
        if !scene.shows(item) {
            return Err(format!(
                "Unit {name} needs one visible mesh; Shift-click makes a new mesh"
            ));
        }
        let item_id = item.id;
        let geometry = crate::user_defaults::primitive_symmetry(self.active_symmetry())
            .doubled(self.unit_shape_geometry(shape)?)?;
        // At the world origin, mapped into a moved mesh's local space
        // (provisional, PRIM-MOVED-Q01).
        let geometry = scene.mesh_world_to_local(item_id, geometry)?;
        geometry.validate()?;
        append_geometry(
            &mut scene.active_mut().ok_or("Select a mesh")?.mesh,
            geometry,
        )?;
        scene.validate()?;
        let before = std::mem::replace(&mut self.scene, scene);
        self.next_item_id = self.next_item_id.max(self.scene.next_id);
        self.record(format!("Unit {name}"), before);
        Ok(())
    }
}
