//! Placing a Planar texture in the viewport (Planar spec 2026-11-09; core:
//! planar_texture.rs; choices PD-Q01.. in docs/SPECIFICATION_QUESTIONS.md).
//!
//! There is no dedicated drag: the ordinary Move/Rotate/Scale/Transform
//! tools move, turn and size the Texture Locator, and selecting a Planar
//! texture layer makes its locator their target (PD-Q02). The plane (an
//! outline, an up tick and the projection-axis line) is drawn by
//! `camera_light_ui::draw_locators`, through the surface item for World
//! Coordinates off (PD-Q07). `transform.channel` (core command_exec)
//! types the same edits (UIC-160..163, PD-Q12).
use super::*;

/// What the host memo is keyed on: the locator and the editor's edit
/// counters (an edit that could change which surface uses it).
pub(crate) type HostKey = (u64, u64, usize, usize);

impl Operandi {
    /// The locator of the one texture layer selected in the Shading panel
    /// (PD-Q01: with several texture layers selected, none).
    pub(super) fn shading_layer_locator(&self) -> Option<u64> {
        let scene = &self.editor.scene;
        let tree = scene.shading_tree();
        let mut textures = self.shading.selected.iter().filter(|&&id| {
            matches!(
                tree.get(id).map(|n| &n.kind),
                Some(operandi_core::shader_tree::NodeKind::Texture(_))
            )
        });
        let layer = *textures.next()?;
        if textures.next().is_some() {
            return None;
        }
        scene.layer_locator(layer)
    }

    /// `locator`'s host item for World Coordinates off (PD-Q07), memoised
    /// until the next edit.
    fn planar_host(&self, locator: u64) -> Option<u64> {
        let key = (
            locator,
            self.editor.revision,
            self.editor.history.undo.len(),
            self.editor.scene.items.len(),
        );
        if let Some((k, host)) = *self.planar_host.borrow()
            && k == key
        {
            return host;
        }
        let host = self.editor.scene.locator_host(locator);
        *self.planar_host.borrow_mut() = Some((key, host));
        host
    }

    /// The locator's matrix for drawing its gizmo: through its host for
    /// World Coordinates off (PD-Q07, was TL-Q20's limitation).
    pub(super) fn locator_render_matrix(&self, locator: u64) -> Option<glam::DMat4> {
        let host = self
            .editor
            .scene
            .texture_locator(locator)
            .filter(|l| !l.world_coordinates)
            .and_then(|_| self.planar_host(locator));
        self.editor.scene.locator_render_matrix(locator, host)
    }

    /// PD-Q02 (Planar spec: selecting a texture layer makes its locator
    /// the item tools' target, as if the locator were selected): selecting
    /// one texture layer in the Shading panel selects its locator with a
    /// plain `select.item`, as an Items-list click does, tool or not. A
    /// live Move/Rotate/Scale/Transform moves there too; one started later
    /// starts on it. Only on a change, so picking the mesh afterwards
    /// sticks. Items mode only (spec): in a component mode the tools act on
    /// the mesh and the item selection is left alone (the plane still
    /// draws).
    pub(super) fn sync_layer_locator(&mut self) {
        let now = self.shading_layer_locator();
        if now == self.layer_seen {
            return;
        }
        self.layer_seen = now;
        if let Some(locator) = now
            && self.editor.scene.selection.mode == Mode::Item
            && !self.pointer_held
            && self.editor.scene.selected_items() != [locator]
            && self
                .editor
                .scene
                .items
                .iter()
                .any(|i| i.id == locator && i.visible && !i.is_locked())
        {
            self.run(Command::SelectItem(Some(locator)));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use operandi_core::shader_tree::{NewLayer, Projection, ShaderTreeEdit};
    use operandi_core::texture_locator::TextureLocator;

    /// A new document with a cube and an Image Map layer on a Planar
    /// locator (axis Z, Size 1); returns (layer, locator, cube).
    fn planar_scene(a: &mut Operandi) -> (operandi_core::shader_tree::NodeId, u64, u64) {
        a.initialize_new_document();
        a.run(Command::NewMesh);
        let mesh = a.editor.scene.selection.item.unwrap();
        let i = a.editor.scene.item_index(mesh).unwrap();
        a.editor.scene.items[i].mesh = operandi_core::cube_geometry(DVec3::ZERO, DVec3::ONE)
            .unwrap()
            .into();
        let layer = a.editor.scene.shader_tree.next_id;
        a.run(Command::ShaderTree(ShaderTreeEdit::Add {
            layer: NewLayer::ImageMap,
            selected: None,
        }));
        let operandi_core::shader_tree::NodeKind::Texture(t) =
            &a.editor.scene.shader_tree.get(layer).unwrap().kind
        else {
            panic!("a texture layer")
        };
        let locator = t.locator.unwrap();
        let tr = transform(a, locator);
        let mut settings = TextureLocator::new(Projection::Planar);
        settings.axis = 2;
        a.run(Command::ShaderTree(ShaderTreeEdit::SetLocator {
            locator,
            settings: Box::new(settings),
            transform: tr,
            continuing: false,
        }));
        (layer, locator, mesh)
    }
    fn transform(a: &Operandi, id: u64) -> operandi_core::scene::Transform {
        a.editor.scene.item(id).unwrap().transform.clone()
    }

    /// PD-Q02: selecting the layer (a change) selects its locator, tool or
    /// not: a live item tool moves there, and one started afterwards (W)
    /// starts on it; picking the mesh afterwards sticks; several texture
    /// layers do nothing.
    #[test]
    fn shading_layer_targets_its_locator() {
        let mut a = Operandi::empty();
        let (layer, locator, mesh) = planar_scene(&mut a);
        a.run(Command::Mode(Mode::Item));
        a.run(Command::SelectItem(Some(mesh)));
        // No tool: select the layer, then press W → Move is on the locator.
        a.shading.selected = vec![layer];
        a.sync_layer_locator();
        assert_eq!(a.editor.scene.selection.item, Some(locator));
        a.activate(Tool::Move);
        assert_eq!(
            a.editor.session.live.as_ref().map(|l| l.tool),
            Some(Tool::Move)
        );
        assert_eq!(a.editor.scene.selected_items(), vec![locator]);
        // Unchanged layer: picking the mesh again isn't undone, and the
        // tool follows the mesh.
        a.run(Command::SelectItem(Some(mesh)));
        a.sync_layer_locator();
        assert_eq!(a.editor.scene.selection.item, Some(mesh));
        // With the tool live, selecting the layer again moves it back.
        a.shading.selected.clear();
        a.sync_layer_locator();
        a.shading.selected = vec![layer];
        a.sync_layer_locator();
        assert_eq!(a.editor.scene.selection.item, Some(locator));
        assert_eq!(
            a.editor.session.live.as_ref().map(|l| l.tool),
            Some(Tool::Move)
        );
        // Two texture layers: no single target.
        a.run(Command::Drop);
        let second = a.editor.scene.shader_tree.next_id;
        a.run(Command::ShaderTree(ShaderTreeEdit::Add {
            layer: NewLayer::Constant,
            selected: None,
        }));
        a.shading.selected = vec![layer, second];
        assert_eq!(a.shading_layer_locator(), None);
    }

    /// PD-Q02, spec: in a component mode selecting the layer leaves the
    /// item selection alone, so the tools act on the mesh; the plane still
    /// draws (its layer is selected).
    #[test]
    fn component_mode_leaves_the_mesh_selected() {
        let mut a = Operandi::empty();
        let (layer, locator, mesh) = planar_scene(&mut a);
        a.run(Command::SelectItem(Some(mesh)));
        a.run(Command::Mode(Mode::Polygon));
        a.shading.selected = vec![layer];
        a.sync_layer_locator();
        assert_eq!(a.editor.scene.selection.item, Some(mesh));
        assert_eq!(a.editor.scene.selection.mode, Mode::Polygon);
        a.activate(Tool::Move);
        assert_eq!(a.editor.scene.selected_items(), vec![mesh]);
        assert!(
            a.locator_render_matrix(locator).is_some(),
            "the plane draws"
        );
    }

    /// PD-Q07: World Coordinates off draws the plane through the surface
    /// item's matrix, as the render places it; on, in world space.
    #[test]
    fn world_coordinates_off_follows_the_surface() {
        let mut a = Operandi::empty();
        let (_, locator, mesh_id) = planar_scene(&mut a);
        let mesh = a.editor.scene.item_index(mesh_id).unwrap();
        a.editor.scene.items[mesh].transform.translation = DVec3::new(3., 0., 0.);
        assert_eq!(a.editor.scene.locator_host(locator), Some(mesh_id));
        let at = |a: &Operandi| {
            a.locator_render_matrix(locator)
                .unwrap()
                .transform_point3(DVec3::ZERO)
        };
        assert!((at(&a) - DVec3::new(3., 0., 0.)).length() < 1e-9);
        let i = a.editor.scene.item_index(locator).unwrap();
        if let Some(operandi_core::texture_locator::TextureItem::Locator(l)) =
            &mut a.editor.scene.items[i].texture
        {
            l.world_coordinates = true;
        }
        a.editor.revision += 1;
        assert!(at(&a).length() < 1e-9);
    }

    /// UIC-160/161: typing a Scale-tool drag's result is the same edit and
    /// the same undo step (one tweak of the live tool); UIC-163: the query
    /// form reports it; PD-Q02's move is a plain `select.item`; Size
    /// clamps at 1 mm both ways (Planar spec).
    #[test]
    fn typed_transform_channel_matches_a_scale_drag() {
        let mut dragged = Operandi::empty();
        let (layer, locator, mesh) = planar_scene(&mut dragged);
        let mut typed = Operandi::empty();
        assert_eq!(
            planar_scene(&mut typed),
            (layer, locator, mesh),
            "the same scene"
        );
        // A Scale-tool drag on the locator: what its X handle publishes.
        dragged.run(Command::SelectItem(Some(locator)));
        dragged.run(Command::Activate(Tool::Scale));
        let before = dragged.editor.session.tweaks.undo_steps();
        let mut p = dragged.editor.session.live.as_ref().unwrap().params.clone();
        p.delta = DVec3::new(2., 1., 1.);
        dragged.run(Command::Preview(p.clone()));
        dragged.editor.end_tweak();
        assert_eq!(dragged.editor.session.tweaks.undo_steps(), before + 1);
        let want = transform(&dragged, locator);
        assert_eq!(want.scale, DVec3::new(2., 1., 1.));
        // Typed: the Shading layer's locator selected by command, then the
        // drag's result as transform.channel.
        typed.shading.selected = vec![layer];
        typed.execute(&format!("select.item {locator}"));
        assert_eq!(typed.editor.scene.selection.item, Some(locator));
        typed.run(Command::Activate(Tool::Scale));
        let before = typed.editor.session.tweaks.undo_steps();
        typed.execute("transform.channel name:scl.X value:2");
        assert_eq!(transform(&typed, locator), want, "{}", typed.status);
        assert_eq!(
            typed.editor.session.tweaks.undo_steps(),
            before + 1,
            "one tweak"
        );
        assert_eq!(
            typed.editor.session.live.as_ref().map(|l| l.tool),
            Some(Tool::Scale),
            "the tool stays"
        );
        typed.execute("transform.channel name:scl.X value:?");
        assert!(typed.status.contains("scl.X = 2"), "{}", typed.status);
        // Size clamps at 1 mm, typed or dragged.
        typed.execute("transform.channel name:scl.Y value:0");
        assert_eq!(transform(&typed, locator).scale.y, 0.001);
        p.delta = DVec3::new(1e-6, 1., 1.);
        dragged.run(Command::Preview(p));
        assert_eq!(transform(&dragged, locator).scale.x, 0.001);
        typed.run(Command::Undo);
        typed.run(Command::Undo);
        assert_eq!(transform(&typed, locator).scale, DVec3::ONE);
        // Idle, it is an ordinary Item Transform step.
        typed.run(Command::Drop);
        typed.execute(&format!("transform.channel name:pos.Y value:2 item:{mesh}"));
        assert_eq!(transform(&typed, mesh).translation.y, 2.);
        typed.execute("transform.channel name:size.X value:1");
        assert!(typed.status.contains("unknown channel"), "{}", typed.status);
        // The old select name still works.
        typed.execute(&format!("item.select {mesh}"));
        assert_eq!(typed.editor.scene.selection.item, Some(mesh));
        // item.transform (registry-2's Operandi-only command) takes the same
        // route as transform.channel.
        typed.execute(&format!("item.transform item:{mesh} translation:5,5,5"));
        assert_eq!(transform(&typed, mesh).translation, DVec3::splat(5.));
    }
}
