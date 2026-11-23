//! The evaluated scene (AR-01, KIT-2; master plan phase 0 door 4): the
//! one step that turns the document scene into what gets drawn, rendered,
//! exported and picked.
//!
//! Today that is MeshOps results (meshops.rs): an item with a stack shows
//! its result instead of its Base Mesh. Later the same step applies
//! deformers, rigs and sims at the given time (AR-05 order: base mesh →
//! MeshOps → morphs → skinning → deformers → sim cache). Everyone who
//! needs the shown scene asks here, through [`crate::Editor::evaluated`],
//! so those later layers land in one place.
//!
//! What each reader gets differs only by [`Purpose`]:
//! - **Display**: the viewport. In the background, so a slow stack never
//!   blocks a frame; the item in Edit Base Mesh shows its Base Mesh
//!   (MO-27).
//! - **Final**: render, export, bridges, baking. Evaluated now on the
//!   caller's thread, never stale (SAFE-14).
//! - **Pick**: viewport picking and snapping. The document scene for now:
//!   tools edit the Base Mesh, and picking on a result would need a map
//!   back to Base Mesh elements (AR-04, provisional). Kept here so the day
//!   that map exists only this arm changes.
use crate::Scene;
use std::borrow::Cow;

/// Who is asking, which decides how up to date and which mesh.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Purpose {
    /// What the viewport draws: `base_edit` (Edit Base Mesh) shows its
    /// Base Mesh, and results come from background evaluation.
    Display { base_edit: Option<u64> },
    /// What renders, exports and bridges read: every result current.
    Final,
    /// What viewport picking and snapping read.
    Pick,
}

/// The scene as `purpose` sees it at `time` (exact ticks; nothing reads
/// it yet). Borrowed when nothing differs from the document scene.
pub fn evaluate<'a>(
    scene: &'a Scene,
    meshops: &crate::meshops::Runtime,
    eval: &crate::eval_state::EvalState,
    time: crate::time_tick::Tick,
    purpose: Purpose,
) -> Cow<'a, Scene> {
    // Time is an input from day one (AR-02); deformers and keys read it.
    let _ = time;
    // Rig pieces first (RG-01; eval graph steps 4, 6): constrained and
    // parented items at their evaluated worlds, for every purpose (a
    // constrained item is picked where it shows). MeshOps links then read
    // the rigged worlds.
    let rigged = if scene.animation.rig.pieces.is_empty() {
        None
    } else {
        crate::rig_eval::apply(scene, &eval.rig(scene))
    };
    let base = rigged.as_ref().unwrap_or(scene);
    let shown = match purpose {
        Purpose::Display { base_edit } => meshops.display_scene(base, base_edit),
        Purpose::Final => meshops.result_scene(base),
        Purpose::Pick => None,
    };
    let shown = shown.or(rigged);
    let shown = match purpose {
        Purpose::Pick => shown,
        _ => with_patches(shown.as_ref().unwrap_or(scene)).or(shown),
    };
    // Replicators (G8 RP-1): each copy shows as a Mesh Instance of its
    // prototype, Display Density's share in the viewport and Density's in
    // renders and exports. Picking keeps the document's items.
    let shown = match purpose {
        Purpose::Pick => shown,
        Purpose::Display { .. } => {
            crate::replicator::expand(shown.as_ref().unwrap_or(scene), true).or(shown)
        }
        Purpose::Final => {
            crate::replicator::expand(shown.as_ref().unwrap_or(scene), false).or(shown)
        }
    };
    match shown {
        Some(s) => Cow::Owned(s),
        None => Cow::Borrowed(scene),
    }
}

/// Live patches (G7 PC-7) shown as their tessellation; `None` when no mesh
/// has one. Picking keeps the patch polygon (provisional, as for MeshOps).
fn with_patches(scene: &Scene) -> Option<Scene> {
    let cells = |i: &crate::Item| {
        i.subdivision
            .as_ref()
            .map_or(crate::patch_live::DEFAULT_CELLS, |s| s.patch_subdivisions)
    };
    let changed: Vec<(usize, crate::MeshSlot)> = scene
        .items
        .iter()
        .enumerate()
        // Packed meshes are big imports nobody is editing; reading them
        // would open them.
        .filter(|(_, i)| i.is_mesh() && !i.mesh.is_packed())
        .filter_map(|(n, i)| crate::patch_live::shown(i.mesh.shared(), cells(i)).map(|m| (n, m)))
        .collect();
    if changed.is_empty() {
        return None;
    }
    let mut out = scene.clone();
    for (n, mesh) in changed {
        out.items[n].mesh = mesh;
    }
    Some(out)
}

impl crate::Editor {
    /// The scene as `purpose` sees it at the scene's current time (AR-01).
    /// The viewport, Cycles, export, the bridge and picking all read the
    /// shown scene through this.
    pub fn evaluated(&self, purpose: Purpose) -> Cow<'_, Scene> {
        evaluate(
            &self.scene,
            &self.meshops,
            &self.eval,
            self.scene.time.current,
            purpose,
        )
    }
    /// What the viewport draws: `None` when it is the document scene.
    pub fn evaluated_display(&self) -> Option<Scene> {
        match self.evaluated(Purpose::Display {
            base_edit: self.base_edit,
        }) {
            Cow::Owned(s) => Some(s),
            Cow::Borrowed(_) => None,
        }
    }
    /// The preview of an open numeric tool that stands in for the whole
    /// scene until it is applied (Ellipsoid, Torus/Capsule/Arc/Tube...,
    /// Cone). Tools that edit the scene live (Cube, Sphere, Cylinder and
    /// the modelling tools) show their preview in `scene` itself.
    pub fn tool_preview_display(&self) -> Option<Scene> {
        let ok = |r: crate::Result<Option<Scene>>| r.ok().flatten();
        ok(self.ellipsoid_preview_scene())
            .or_else(|| ok(self.shape_preview_scene()))
            .or_else(|| ok(self.cone_preview_scene()))
    }
    /// Everything the viewport shows when it isn't the document scene: an
    /// open tool's preview, else the MeshOps result. Drawing and view
    /// framing (Frame All / Frame Selection) both read this, so framing
    /// always takes in what the active tool is previewing (Jason,
    /// 2026-11-21).
    pub fn shown_display(&self) -> Option<Scene> {
        self.tool_preview_display()
            .or_else(|| self.evaluated_display())
    }
}
