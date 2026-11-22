//! Remesh... and Automatic Retopology... (remesh.rs in core): two dialogs
//! on the shared form (UIC-80..99). OK runs one command, one undo step
//! (UIC-96): `sculpt.remesh` for Voxel, `operandi.remesh` for Blocks,
//! Smooth and Sharp, `mesh.retopo` for Automatic Retopology.
//!
//! The work can take seconds on big meshes, so it runs on a worker thread
//! behind a progress window with Cancel. The finished mesh is handed to the
//! very same command (`RemeshPrepared`), which checks that the scene, item
//! and options still match, so the dialog and a typed command always give
//! the same result. The dialogs start at the commands' last-used values,
//! which typed commands share (UIC-95).
use super::*;
use operandi_core::remesh::{RemeshPrepared, remesh_mesh};
use operandi_core::{CountMode, OctreeMode, OctreeOptions, RemeshOp, RetopoOptions, VoxelOptions};
use std::sync::mpsc;
use std::sync::{Arc, Mutex, atomic::AtomicBool};

/// Which dialog is open.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum RemeshKind {
    Remesh,
    Retopo,
}

/// The open dialog.
pub(super) struct RemeshDialog {
    pub(super) kind: RemeshKind,
    pub(super) form: form::Dialog,
}

/// A remesh running on the worker thread.
pub(super) struct RemeshJob {
    pub(super) op: RemeshOp,
    item: u64,
    revision: u64,
    cancel: Arc<AtomicBool>,
    progress: Arc<Mutex<f32>>,
    result: mpsc::Receiver<std::result::Result<operandi_core::Mesh, String>>,
    started: std::time::Instant,
}

/// The Remesh dialog's Mode choice, in order.
const MODES: [&str; 4] = ["Voxel", "Blocks", "Smooth", "Sharp"];

fn remesh_rows(v: &VoxelOptions, o: &OctreeOptions, mode: usize) -> Vec<form::Row> {
    use form::{Row, Unit, When};
    let octree = || When::ChoiceIn("mode", vec![1, 2, 3]);
    vec![
        Row::choice("mode", "Mode", &MODES, mode).tip(
            "Voxel: an even skin on a voxel grid; overlaps join, holes close (sculpt.remesh) · Blocks: cubes on an octree · Smooth: a smooth surface on an octree · Sharp: keeps sharp edges and corners (operandi.remesh)",
        ),
        Row::number("density", "Density", Unit::Factor, v.density)
            .range(0., 1.)
            .when(When::ChoiceIn("mode", vec![0]))
            .tip("Higher is finer: from 16 voxels along the longest side at 0 to 640 at 100 %"),
        Row::number("voxel_size", "Voxel Size", Unit::Distance, v.voxel_size)
            .range(0., f64::INFINITY)
            .when(When::ChoiceIn("mode", vec![0]))
            .tip("Sets the voxel size directly; 0 uses Density"),
        Row::check("keep_maps", "Keep Vertex Maps", v.keep_maps)
            .when(When::ChoiceIn("mode", vec![0]))
            .tip("Weight and colour maps (sculpt masks among them) carry over from the nearest old point"),
        Row::number("depth", "Octree Depth", Unit::Count, o.depth as f64)
            .range(2., 12.)
            .when(octree())
            .tip("Each step doubles the cells along the longest side (2 to the power of the depth)"),
        Row::number("scale", "Scale", Unit::Factor, o.scale)
            .range(0.01, 0.99)
            .when(octree())
            .tip("How much of the octree the mesh fills; lower is coarser"),
        Row::number("sharpness", "Sharpness", Unit::Scalar, o.sharpness)
            .range(0., 1000.)
            .when(When::ChoiceIn("mode", vec![3]))
            .tip("Higher keeps more edges sharp"),
        Row::check("remove_disconnected", "Remove Disconnected", o.remove_disconnected)
            .when(octree())
            .tip("Drop small loose pieces"),
        Row::number("threshold", "Threshold", Unit::Factor, o.threshold)
            .range(0., 1.)
            .when(When::All(vec![octree(), When::Checked("remove_disconnected")]))
            .tip("Pieces smaller than this share of the biggest are dropped"),
        Row::check("smooth", "Smooth Shading", v.vert_normals)
            .tip("Smooth shade the new polygons (vertNormals)"),
    ]
    .into_iter()
    .map(form::Row::fresh)
    .collect()
}

fn retopo_rows(o: &RetopoOptions) -> Vec<form::Row> {
    use form::{Row, Unit, When};
    let mode = match o.count_mode {
        CountMode::Number => 0,
        CountMode::Percent => 1,
    };
    vec![
        Row::choice(
            "count_mode",
            "Poly Count Mode",
            &["Absolute", "Relative"],
            mode,
        )
        .tip(
            "Absolute: about Polygon Target quads · Relative: a share of the mesh's polygon count",
        ),
        Row::number(
            "target_count",
            "Polygon Target",
            Unit::Count,
            o.target_count as f64,
        )
        .range(1., 1_000_000.)
        .when(When::ChoiceIn("count_mode", vec![0]))
        .tip("About how many quads to make"),
        Row::number(
            "target_percent",
            "Relative Polygon Target Count",
            Unit::Percent,
            o.target_percent * 100.,
        )
        .range(0.01, 1000.)
        .when(When::ChoiceIn("count_mode", vec![1]))
        .tip("About this share of the mesh's polygon count"),
        Row::number(
            "crease_angle",
            "Crease Angle",
            Unit::Angle,
            o.crease_angle.to_degrees(),
        )
        .range(0., 180.)
        .tip("New edges follow creases sharper than this; 0° is off"),
        Row::check("boundary_align", "Boundary Align", o.boundary_align)
            .tip("New edges follow open borders"),
        Row::check("vert_normals", "Vertex Normals", o.vert_normals)
            .tip("Smooth shade the new polygons"),
        Row::check("new_mesh", "New Mesh", o.new_mesh)
            .tip("Put the result in a new mesh item next to this one, which is left as it was"),
        Row::number("seed", "Seed", Unit::Count, o.seed as f64)
            .range(0., i32::MAX as f64)
            .tip("Random start; the same seed gives the same quads"),
    ]
    .into_iter()
    .map(form::Row::fresh)
    .collect()
}

impl RemeshDialog {
    /// The command the dialog's values ask for.
    fn op(&self) -> RemeshOp {
        let v = &self.form.values;
        match self.kind {
            RemeshKind::Remesh => match v.choice("mode") {
                0 => RemeshOp::Voxel(VoxelOptions {
                    density: v.number("density"),
                    voxel_size: v.number("voxel_size"),
                    keep_maps: v.bool("keep_maps"),
                    vert_normals: v.bool("smooth"),
                }),
                m => RemeshOp::Octree(OctreeOptions {
                    mode: OctreeMode::ALL[(m - 1).min(2)],
                    depth: v.number("depth").round().clamp(0., 64.) as u32,
                    scale: v.number("scale"),
                    sharpness: v.number("sharpness"),
                    remove_disconnected: v.bool("remove_disconnected"),
                    threshold: v.number("threshold"),
                    vert_normals: v.bool("smooth"),
                }),
            },
            RemeshKind::Retopo => RemeshOp::Retopo(RetopoOptions {
                count_mode: if v.choice("count_mode") == 1 {
                    CountMode::Percent
                } else {
                    CountMode::Number
                },
                target_count: v.number("target_count").round().clamp(0., u32::MAX as f64) as u32,
                target_percent: v.number("target_percent") / 100.,
                crease_angle: v.number("crease_angle").to_radians(),
                boundary_align: v.bool("boundary_align"),
                vert_normals: v.bool("vert_normals"),
                new_mesh: v.bool("new_mesh"),
                seed: v.number("seed").round().clamp(0., i32::MAX as f64) as i32,
            }),
        }
    }
}

impl Operandi {
    /// Remesh... (the Edit tab and Sculpt rows, the menu, bare
    /// `sculpt.remesh` / `operandi.remesh`). `voxel` picks the Mode the
    /// dialog opens on.
    pub(super) fn open_remesh(&mut self, kind: RemeshKind, voxel: Option<bool>) {
        if self.remesh.is_some() || self.remesh_job.is_some() {
            return;
        }
        if self.has_modal() {
            self.status = format!("Close the open dialog before {}", kind_title(kind));
            return;
        }
        let rows = match kind {
            RemeshKind::Remesh => {
                let o = &self.editor.octree_remesh_preset;
                let last = self.remesh_last_voxel;
                let mode = match voxel.unwrap_or(last) {
                    true => 0,
                    false => {
                        1 + OctreeMode::ALL
                            .iter()
                            .position(|m| *m == o.mode)
                            .unwrap_or(1)
                    }
                };
                remesh_rows(&self.editor.voxel_remesh_preset, o, mode)
            }
            RemeshKind::Retopo => retopo_rows(&self.editor.retopo_preset),
        };
        self.remesh = Some(RemeshDialog {
            kind,
            form: self.form_dialog(kind_title(kind), rows),
        });
    }

    /// The dialog and the progress window, once per frame.
    pub(super) fn remesh_dialogs(&mut self, ctx: &Context) {
        self.poll_remesh_job();
        if let Some(job) = &self.remesh_job {
            let progress = job.progress.lock().map(|p| *p).unwrap_or(0.);
            let elapsed = job.started.elapsed().as_secs();
            let label = job.op.label();
            let cancel = job.cancel.clone();
            self.modal_owned_input = true;
            self.editor_owned_input = true;
            crate::dialog_place::modal(ctx, Id::new("Remesh progress")).show(ctx, |ui| {
                ui.set_max_width(360.);
                ui.label(RichText::new(label).strong());
                ui.add(egui::ProgressBar::new(progress).show_percentage());
                ui.label(format!(
                    "{elapsed} s · the scene is unchanged until it finishes"
                ));
                let button = ui.button("Cancel");
                if button.clicked() || ui.input(|i| i.key_pressed(Key::Escape)) {
                    cancel.store(true, std::sync::atomic::Ordering::Release);
                }
            });
            ctx.request_repaint_after(std::time::Duration::from_millis(50));
            return;
        }
        let Some(mut dialog) = self.remesh.take() else {
            return;
        };
        let title = kind_title(dialog.kind);
        if self.has_modal() {
            self.status = format!("{title} closed; nothing changed");
            return;
        }
        match self.show_form(ctx, &mut dialog.form) {
            form::Outcome::Cancel => self.status = format!("{title} canceled; nothing changed"),
            form::Outcome::Ok => {
                let op = dialog.op();
                match op.validate() {
                    Ok(()) => {
                        self.remember_remesh(&op);
                        self.start_remesh(op);
                    }
                    Err(e) => {
                        self.status = e.clone();
                        dialog.form.error = Some(e);
                        self.remesh = Some(dialog);
                    }
                }
            }
            _ => self.remesh = Some(dialog),
        }
    }

    /// The options become the command's last-used values (UIC-95).
    fn remember_remesh(&mut self, op: &RemeshOp) {
        match op {
            RemeshOp::Voxel(o) => {
                self.editor.voxel_remesh_preset = o.clone();
                self.remesh_last_voxel = true;
            }
            RemeshOp::Octree(o) => {
                self.editor.octree_remesh_preset = o.clone();
                self.remesh_last_voxel = false;
            }
            RemeshOp::Retopo(o) => self.editor.retopo_preset = o.clone(),
        }
    }

    /// Starts `op` on the active mesh on a worker thread.
    pub(super) fn start_remesh(&mut self, op: RemeshOp) {
        if self.remesh_job.is_some() {
            self.status = "A remesh is already running".into();
            return;
        }
        let Some(item) = self.editor.scene.active() else {
            self.status = format!("Select a mesh before {}", op.label());
            return;
        };
        if item.mesh.faces.is_empty() {
            self.status = format!("The mesh has no polygons to {}", op.label().to_lowercase());
            return;
        }
        let id = item.id;
        let mesh: operandi_core::Mesh = (*item.mesh).clone();
        let scale = self.editor.remesh_world_scale(id);
        let cancel = Arc::new(AtomicBool::new(false));
        let progress = Arc::new(Mutex::new(0f32));
        let (send, result) = mpsc::channel();
        let (worker_op, worker_cancel, worker_progress) =
            (op.clone(), cancel.clone(), progress.clone());
        std::thread::spawn(move || {
            let mut report = |p: f32| {
                if let Ok(mut v) = worker_progress.lock() {
                    *v = p;
                }
            };
            let out = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                remesh_mesh(&mesh, &worker_op, scale, &worker_cancel, &mut report)
            }))
            .unwrap_or_else(|_| Err("Remesh stopped on an internal error; nothing changed".into()));
            let _ = send.send(out);
        });
        self.remesh_job = Some(RemeshJob {
            op,
            item: id,
            revision: self.editor.revision,
            cancel,
            progress,
            result,
            started: std::time::Instant::now(),
        });
    }

    /// Publishes a finished job through its command.
    fn poll_remesh_job(&mut self) {
        let Some(job) = &self.remesh_job else {
            return;
        };
        let outcome = match job.result.try_recv() {
            Ok(r) => r,
            Err(mpsc::TryRecvError::Empty) => return,
            Err(mpsc::TryRecvError::Disconnected) => {
                Err("Remesh stopped on an internal error; nothing changed".into())
            }
        };
        let job = self.remesh_job.take().expect("checked above");
        let label = job.op.label();
        if job.cancel.load(std::sync::atomic::Ordering::Acquire) {
            self.status = format!("{label} canceled; nothing changed");
            return;
        }
        match outcome {
            Ok(mesh) => {
                let polygons = mesh.faces.len();
                self.editor.remesh_prepared = Some(Box::new(RemeshPrepared {
                    revision: job.revision,
                    item: job.item,
                    op: job.op.clone(),
                    mesh,
                }));
                if self.run_ok(Command::Remesh(job.op)) {
                    self.status = format!("{label}: {polygons} polygons");
                }
                self.editor.remesh_prepared = None;
            }
            Err(e) => self.status = e,
        }
    }

    /// `mesh.retopo`, `sculpt.remesh` or `operandi.remesh` typed: bare
    /// opens the dialog; with arguments the command runs (on the worker,
    /// like OK). Returns false for other lines.
    pub(super) fn remesh_command(&mut self, text: &str) -> bool {
        let words = match operandi_core::command_line::tokens(text) {
            Ok(w) => w,
            Err(_) => return false,
        };
        let Some(first) = words.first().map(|w| w.to_ascii_lowercase()) else {
            return false;
        };
        let kind = match first.as_str() {
            "mesh.retopo" => RemeshKind::Retopo,
            "sculpt.remesh" | "operandi.remesh" => RemeshKind::Remesh,
            _ => return false,
        };
        if words.len() == 1 {
            self.open_remesh(kind, Some(first == "sculpt.remesh"));
            return true;
        }
        match operandi_core::command_line::command_for_line(&mut self.editor, text) {
            Some(Ok(Some(Command::Remesh(op)))) => {
                self.remember_remesh(&op);
                self.start_remesh(op);
            }
            Some(Ok(_)) | None => self.status = format!("{first}: nothing to run"),
            Some(Err(e)) => self.status = e,
        }
        true
    }
}

fn kind_title(kind: RemeshKind) -> &'static str {
    match kind {
        RemeshKind::Remesh => "Remesh",
        RemeshKind::Retopo => "Automatic Retopology",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app() -> Operandi {
        let mut a = Operandi::empty();
        a.run(Command::NewMesh);
        a.editor.scene.active_mut().unwrap().mesh =
            operandi_core::Mesh::box_mesh(DVec3::ZERO, DVec3::splat(2.), 0)
                .unwrap()
                .into();
        a
    }

    /// Waits for the worker and publishes its result.
    fn finish(a: &mut Operandi) {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(60);
        while a.remesh_job.is_some() && std::time::Instant::now() < deadline {
            std::thread::sleep(std::time::Duration::from_millis(5));
            a.poll_remesh_job();
        }
        assert!(a.remesh_job.is_none(), "remesh never finished");
    }

    #[test]
    fn bare_commands_open_the_dialogs() {
        let mut a = app();
        a.execute("sculpt.remesh");
        let d = a.remesh.take().expect("sculpt.remesh opens Remesh");
        assert_eq!(
            (d.kind, d.form.values.choice("mode")),
            (RemeshKind::Remesh, 0)
        );
        a.execute("operandi.remesh");
        let d = a.remesh.take().expect("operandi.remesh opens Remesh");
        assert_eq!(
            d.form.values.choice("mode"),
            2,
            "Smooth, the default octree mode"
        );
        a.execute("mesh.retopo");
        assert_eq!(a.remesh.take().map(|d| d.kind), Some(RemeshKind::Retopo));
        a.execute("Automatic Retopology");
        assert!(a.remesh.take().is_some());
        a.execute("Remesh");
        assert!(a.remesh.take().is_some());
    }

    #[test]
    fn typed_command_and_dialog_give_the_same_mesh_in_one_step() {
        // Typed, through the worker.
        let mut a = app();
        let steps = a.editor.undo.len();
        a.execute("sculpt.remesh density:0.3");
        finish(&mut a);
        assert_eq!(a.editor.undo.len(), steps + 1, "{}", a.status);
        assert_eq!(a.editor.undo.last().unwrap().label, "Voxel Remesh");
        let typed = (*a.editor.scene.active().unwrap().mesh).clone();
        assert!(typed.faces.len() > 100);
        // Left-out arguments keep the last values.
        assert_eq!(a.editor.voxel_remesh_preset.density, 0.3);

        // The dialog's OK at the same values.
        let mut b = app();
        b.editor.voxel_remesh_preset.density = 0.3;
        b.open_remesh(RemeshKind::Remesh, Some(true));
        let op = b.remesh.take().unwrap().op();
        assert_eq!(op, RemeshOp::Voxel(b.editor.voxel_remesh_preset.clone()));
        b.start_remesh(op);
        finish(&mut b);
        assert_eq!(*b.editor.scene.active().unwrap().mesh, typed);

        // Headless (scripts, MCP) runs the same command directly.
        let mut c = app();
        operandi_core::command_line::run_line(&mut c.editor, "sculpt.remesh density:0.3")
            .unwrap()
            .unwrap();
        assert_eq!(*c.editor.scene.active().unwrap().mesh, typed);

        // Undo puts the cube back.
        a.run(Command::Undo);
        assert_eq!(a.editor.scene.active().unwrap().mesh.faces.len(), 6);
    }

    #[test]
    fn retopo_and_octree_run_from_the_command_line() {
        let mut a = app();
        a.execute("mesh.retopo polyTargetNum:300");
        finish(&mut a);
        assert!(
            a.status.starts_with("Automatic Retopology:"),
            "{}",
            a.status
        );
        assert!(
            a.editor
                .scene
                .active()
                .unwrap()
                .mesh
                .faces
                .iter()
                .all(|f| f.vertices.len() == 4)
        );
        a.execute("mesh.retopo newMesh:true polyTargetNum:200");
        finish(&mut a);
        assert!(
            a.editor.scene.active().unwrap().name.ends_with("Retopo"),
            "{}",
            a.status
        );
        a.execute("operandi.remesh mode:blocks depth:4");
        finish(&mut a);
        assert!(a.status.starts_with("Remesh Blocks:"), "{}", a.status);
        a.execute("operandi.remesh depth:40");
        assert!(a.status.contains("Octree Depth"), "{}", a.status);
        assert!(a.remesh_job.is_none());
    }

    #[test]
    fn cancel_leaves_the_scene_alone() {
        let mut a = app();
        let before = a.editor.scene.clone();
        a.start_remesh(RemeshOp::Retopo(RetopoOptions {
            target_count: 20_000,
            ..Default::default()
        }));
        a.remesh_job
            .as_ref()
            .unwrap()
            .cancel
            .store(true, std::sync::atomic::Ordering::Release);
        finish(&mut a);
        assert!(a.status.contains("canceled"), "{}", a.status);
        assert_eq!(a.editor.scene, before);
    }

    #[test]
    fn a_stale_result_is_worked_out_again() {
        let mut a = app();
        a.start_remesh(RemeshOp::Voxel(VoxelOptions {
            density: 0.2,
            ..Default::default()
        }));
        // The scene changes while the worker runs: the command does the
        // work again on the current mesh rather than publish the old one.
        a.run(Command::NewMesh);
        a.editor.scene.active_mut().unwrap().mesh =
            operandi_core::Mesh::box_mesh(DVec3::splat(5.), DVec3::ONE, 0)
                .unwrap()
                .into();
        finish(&mut a);
        let m = &a.editor.scene.active().unwrap().mesh;
        assert!(
            m.vertices
                .iter()
                .all(|p| (*p - DVec3::splat(5.)).abs().max_element() < 0.7),
            "{}",
            a.status
        );
    }
}
