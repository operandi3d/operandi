//! Candidate loading is isolated from Editor; only commit_import publishes once.
use super::*;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
    mpsc,
};
pub(super) struct ImportJob {
    cancel: Arc<AtomicBool>,
    stage: Arc<Mutex<String>>,
    result: mpsc::Receiver<operandi_core::lxo::Result<operandi_core::lxo::Candidate>>,
    before: Editor,
    path: Option<PathBuf>,
    saved: Scene,
}
pub(super) struct PreparedImport {
    candidate: operandi_core::lxo::Candidate,
    before: Editor,
    path: Option<PathBuf>,
    saved: Scene,
}
/// An imported scene opens in Item mode with a mesh selected (Jason,
/// 2026-11-05; provisional, IMP-Q01): moving it then moves the item. In the
/// default Polygon mode with nothing selected, a Move moved every point of
/// the mesh instead, which looks the same but edits the whole mesh.
pub(super) fn item_mode_after_import(scene: &mut operandi_core::Scene) {
    let is_mesh = |i: &operandi_core::Item| i.is_mesh() && !i.mesh.faces.is_empty();
    if !scene
        .selection
        .item
        .is_some_and(|id| scene.items.iter().any(|i| i.id == id && is_mesh(i)))
    {
        scene.selection.item = scene.items.iter().find(|i| is_mesh(i)).map(|i| i.id);
    }
    scene.selection.mode = operandi_core::Mode::Item;
    scene.selection.vertices.clear();
    scene.selection.edges.clear();
    scene.selection.faces.clear();
}

impl Operandi {
    pub(super) fn request_open_path(&mut self, path: PathBuf) {
        if self.has_modal() {
            return;
        }
        self.queued_open = Some(path);
        self.request_file(FileAction::Open);
    }
    pub(super) fn begin_lxo(&mut self, path: PathBuf) {
        if self.import_job.is_some() {
            return;
        }
        let cancel = Arc::new(AtomicBool::new(false));
        let stage = Arc::new(Mutex::new("Reading".into()));
        let (send, result) = mpsc::channel();
        let cancelled = cancel.clone();
        let progress = stage.clone();
        let seam = WorkerSeam::take();
        std::thread::spawn(move || {
            let mut callback = |phase: &str| {
                if let Ok(mut s) = progress.lock()
                    && *s != phase
                {
                    *s = phase.to_owned();
                }
                seam.at_phase(phase);
                !cancelled.load(Ordering::Acquire)
            };
            let mut control = operandi_core::lxo::Control::new(&mut callback);
            seam.limit(&mut control.limits);
            let result = operandi_core::lxo::load(&path, &mut control);
            seam.before_send();
            let _ = send.send(result);
        });
        self.import_job = Some(ImportJob {
            cancel,
            stage,
            result,
            before: self.editor.clone(),
            path: self.path.clone(),
            saved: self.saved.clone(),
        });
    }
    pub(super) fn poll_import(&mut self) {
        let result = self
            .import_job
            .as_ref()
            .and_then(|job| match job.result.try_recv() {
                Ok(r) => Some(r),
                Err(mpsc::TryRecvError::Disconnected) => Some(Err(operandi_core::lxo::Error::new(
                    "worker_failure",
                    0,
                    "Import worker stopped; current document preserved",
                ))),
                _ => None,
            });
        if let Some(result) = result {
            let job = self.import_job.take().unwrap();
            if job.cancel.load(Ordering::Acquire) {
                self.status = "LXO import cancelled; current document preserved".into();
                return;
            }
            match result {
                Ok(candidate) => {
                    let ready = PreparedImport {
                        candidate,
                        before: job.before,
                        path: job.path,
                        saved: job.saved,
                    };
                    if ready.candidate.report.rejected_groups.is_empty() {
                        self.commit_import(ready);
                    } else {
                        self.import_ready = Some(ready);
                    }
                }
                Err(e) => {
                    self.status = e.to_string();
                    self.import_message = Some(self.status.clone());
                }
            }
        }
    }
    pub(super) fn commit_import(&mut self, mut ready: PreparedImport) {
        if self.editor != ready.before || self.path != ready.path || self.saved != ready.saved {
            self.status="Import result is stale: current work changed. Open the source again through unsaved-work protection.".into();
            self.import_message = Some(self.status.clone());
            return;
        }
        // LXO cameras and lights are not imported yet (CL-3): give the
        // scene the new-scene ones so Camera view is available.
        if let Err(e) = ready.candidate.scene.ensure_camera_and_light() {
            self.status = format!("LXO import: {e}");
            self.import_message = Some(self.status.clone());
            return;
        }
        // RI-05: a scene made from a file starts with new-scene render
        // settings.
        ready.candidate.scene.use_new_scene_render_settings();
        item_mode_after_import(&mut ready.candidate.scene);
        self.import_message = Some(ready.candidate.report.summary());
        self.make_room_for_document();
        self.editor = Editor {
            bevel_states: self.editor.bevel_states.clone(),
            bevel_state: self.editor.bevel_state.clone(),
            bevel_profile: self.editor.bevel_profile,
            scene: ready.candidate.scene,
            ..Default::default()
        };
        // This import becomes a new, unsaved document. Provenance can also
        // appear in a saved native template, so it does not imply edits.
        self.saved = Scene::default();
        self.path = None;
        self.clear_fields();
        self.name.clear();
        self.status = "LXO imported as an unsaved native document · review import results".into();
        self.frame(false);
    }
    /// Returns true while an import modal owns input.
    pub(super) fn import_dialogs(&mut self, ctx: &Context) -> bool {
        self.poll_import();
        if let Some(job) = &self.import_job {
            crate::dialog_place::modal(ctx, Id::new("LXO progress")).show(ctx, |ui| {
                ui.set_max_width(500.);
                ui.heading("Import Modo Scene");
                ui.spinner();
                ui.label(
                    job.stage
                        .lock()
                        .map(|s| s.clone())
                        .unwrap_or_else(|_| "Working".into()),
                );
                let cancel = ui.button("Cancel");
                cancel.request_focus();
                if cancel.clicked() || ui.input(|i| i.key_pressed(Key::Escape)) {
                    job.cancel.store(true, Ordering::Release);
                }
                ui.label("Current scene remains active until validation completes.");
            });
            ctx.request_repaint_after(std::time::Duration::from_millis(30));
            return true;
        }
        if let Some(ready) = self.import_ready.take() {
            let mut keep = true;
            let mut commit = false;
            crate::dialog_place::modal(ctx, Id::new("LXO partial choice")).show(ctx,|ui|{
                ui.set_max_width(620.);ui.heading("Strict import refused unsupported groups");
                ScrollArea::vertical().max_height(ctx.content_rect().height()*0.65).show(ui,|ui|{ui.label(ready.candidate.report.summary());});
                ui.label("Only the independently complete groups listed above can be imported. The omitted groups and their geometry/attributes will be absent.");
                ui.horizontal(|ui| {
                    if ui.button("Import complete groups").clicked() {commit = true; keep = false;}
                    let cancel = ui.button("Cancel");
                    if !self.prompt_focused {cancel.request_focus(); self.prompt_focused = true;}
                    if cancel.clicked() || ui.input(|i| i.key_pressed(Key::Escape)) {keep = false;}
                });
            });
            if commit {
                self.commit_import(ready);
            } else if keep {
                self.import_ready = Some(ready);
            } else {
                self.status = "Partial import cancelled; current document preserved".into();
            }
            return true;
        }
        if let Some(message) = self.import_message.take() {
            let mut keep = true;
            crate::dialog_place::modal(ctx, Id::new("LXO results")).show(ctx, |ui| {
                ui.set_max_width(620.);
                ui.heading("Import Result");
                ScrollArea::vertical()
                    .max_height(ctx.content_rect().height() * 0.7)
                    .show(ui, |ui| {
                        ui.label(&message);
                    });
                let close = ui.button("Close");
                close.request_focus();
                if close.clicked() || ui.input(|i| i.key_pressed(Key::Escape)) {
                    keep = false;
                }
            });
            if keep {
                self.import_message = Some(message);
            }
            return true;
        }
        false
    }
}

/// Test-only synchronization and limit seam for the import worker. Outside
/// tests it is an empty type whose hooks do nothing, so the worker runs the
/// exact production path; tests supply lowered limits and deterministic pause
/// points from `transaction_tests`.
#[cfg(not(test))]
struct WorkerSeam;
#[cfg(not(test))]
impl WorkerSeam {
    fn take() -> Self {
        Self
    }
    fn limit(&self, _: &mut operandi_core::lxo::Limits) {}
    fn at_phase(&self, _: &str) {}
    fn before_send(&self) {}
}
#[cfg(test)]
use transaction_tests::WorkerSeam;
#[cfg(test)]
#[path = "import_transaction_tests.rs"]
mod transaction_tests;

#[cfg(test)]
mod tests {
    use super::*;
    fn old_work() -> Operandi {
        let mut app = Operandi::empty();
        app.run(Command::NewMesh);
        app.activate(Tool::Box);
        app.run(Command::Apply);
        app.run(Command::Drop);
        app.activate(Tool::Move);
        app.run(Command::Preview(Parameters {
            delta: DVec3::new(1., 2., 3.),
            ..Default::default()
        }));
        app.path = Some(PathBuf::from("/private/tmp/old-work.opscene"));
        app
    }
    #[test]
    fn lxo_import_stays_dirty_until_saved_as_a_native_scene() {
        let mut app = Operandi::empty();
        app.initialize_new_document();
        let root = app.environment.paths().data_root().to_owned();
        let source = operandi_core::ImportSource {
            path: root.join("Original.lxo").display().to_string(),
            producer: "Test fixture".into(),
            device: None,
            inode: None,
        };
        app.commit_import(PreparedImport {
            candidate: operandi_core::lxo::Candidate {
                scene: Scene {
                    import_source: Some(source.clone()),
                    ..Default::default()
                },
                report: Default::default(),
            },
            before: app.editor.clone(),
            path: app.path.clone(),
            saved: app.saved.clone(),
        });
        assert_eq!(app.editor.scene.import_source.as_ref(), Some(&source));
        assert!(app.dirty() && app.dirty_exactly());
        assert!(app.save(root.join("Imported.opscene")));
        assert!(!app.dirty() && !app.dirty_exactly());
    }

    #[test]
    fn lxo_stale_candidate_cannot_replace_newer_work() {
        let mut app = old_work();
        let ready = PreparedImport {
            candidate: operandi_core::lxo::Candidate {
                scene: Scene::default(),
                report: Default::default(),
            },
            before: app.editor.clone(),
            path: app.path.clone(),
            saved: app.saved.clone(),
        };
        app.run(Command::Cancel);
        let current = app.editor.clone();
        app.commit_import(ready);
        assert!(app.editor == current);
        assert!(app.status.contains("stale"));
    }
}
