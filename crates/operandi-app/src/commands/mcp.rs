//! Window > MCP Bridge: answer MCP requests against the scene on screen.
//!
//! The socket and its threads live in `operandi_mcp::server`; they only pass
//! JSON lines. Every request runs here, on the UI thread between frames,
//! against the app's own `Editor` through the same `Bridge` the headless
//! sidecar uses, so history, refusals and the viewport stay in step. Clients
//! connect with `operandi-mcp --attach` (docs/MCP_BRIDGE.md).
//!
//! On by default in development/QA builds, off by default in packaged release
//! builds (an always-open control port is not something to ship switched
//! on). `OPERANDI_MCP=1` or `=0` overrides the default at launch.
//!
//! The server listens on a Unix socket, so on Windows the bridge can't be
//! switched on yet and nothing reaches the request handling below. It is
//! kept compiling there (not cfg'd out) so a Windows transport only has to
//! feed `mcp_queue`.
#![cfg_attr(not(unix), allow(dead_code))]
use super::*;
use serde_json::{Value, json};
use std::time::{Duration, Instant};

/// Sends a deferred answer back to the client that asked.
type Respond = Box<dyn FnOnce(Option<Value>) + Send>;

/// How long a viewport capture waits for its frame before giving up (the
/// window may be hidden, so no frame is drawn).
const CAPTURE_TIMEOUT: Duration = Duration::from_secs(10);
/// A render call's wait when it names none, seconds.
const RENDER_WAIT: f64 = 120.;

#[derive(Default)]
pub(super) struct McpState {
    #[cfg(unix)]
    server: Option<operandi_mcp::server::Server>,
    /// Why the last attempt to switch on failed, shown in the status bar.
    pub error: Option<String>,
    /// Calls answered on a later frame (captures, renders).
    waiting: Vec<Waiting>,
    /// A capture asked for this frame: send the screenshot command.
    screenshot: Option<u64>,
    next_tag: u64,
    /// A `pointer_stroke` being fed in, one frame at a time (mcp_pointer.rs).
    pub(super) stroke: Option<crate::mcp_pointer::StrokeRun>,
}

/// A call whose answer comes on a later frame.
struct Waiting {
    message: Value,
    respond: Respond,
    kind: Wait,
}

pub(super) enum Wait {
    Capture(Box<Capture>),
    /// Answered when `RenderState::finished` passes `finished`, or at
    /// `deadline` with "still rendering".
    Render {
        finished: u64,
        deadline: Instant,
        /// Longest side of the picture returned with the answer; `None`
        /// returns no picture (`include_image: false`).
        image: Option<u32>,
    },
    /// Answered once a `pointer_stroke`'s last frame has been handled.
    Stroke,
}

/// A viewport capture in flight.
pub(super) struct Capture {
    tag: u64,
    /// The view's camera before the capture changed it, put back once the
    /// frame is drawn.
    restore: Option<Camera>,
    /// The frame showing the requested view has been drawn.
    drawn: bool,
    /// The view's area in points, as drawn.
    rect: Rect,
    path: PathBuf,
    include_image: bool,
    max_size: u32,
    view: String,
    started: Instant,
}

/// The answer to one request: now, or on a later frame.
pub(super) enum McpAnswer {
    Now(Option<Value>),
    Later(Value, Wait),
}

/// Whether the bridge starts switched on.
pub(super) fn on_at_launch() -> bool {
    match std::env::var("OPERANDI_MCP").ok().as_deref() {
        Some("1" | "on" | "true") => true,
        Some("0" | "off" | "false") => false,
        _ => !packaged_release(),
    }
}

/// Packaged builds (scripts set `OPERANDI_MILESTONE1_PACKAGE=1` at compile
/// time) are the ones that leave the machine; dev and QA builds are not.
pub(super) fn packaged_release() -> bool {
    option_env!("OPERANDI_MILESTONE1_PACKAGE") == Some("1")
}

impl McpState {
    pub fn is_on(&self) -> bool {
        #[cfg(unix)]
        return self.server.is_some();
        #[cfg(not(unix))]
        false
    }
    pub fn available() -> bool {
        cfg!(unix)
    }
}

impl Operandi {
    /// Switch the bridge on or off and say so in the status bar.
    pub(super) fn set_mcp(&mut self, ctx: &Context, on: bool) {
        #[cfg(unix)]
        {
            if on == self.mcp.is_on() {
                return;
            }
            if !on {
                self.mcp.server = None;
                self.mcp.error = None;
                self.status = "MCP bridge off".into();
                return;
            }
            let wake = ctx.clone();
            let path = operandi_mcp::server::default_socket_path();
            match operandi_mcp::server::Server::start(&path, move || wake.request_repaint()) {
                Ok(server) => {
                    self.status = format!("MCP bridge on · {}", server.path().display());
                    self.mcp.server = Some(server);
                    self.mcp.error = None;
                }
                Err(e) => {
                    self.status = format!("MCP bridge could not start: {e}");
                    self.mcp.error = Some(e);
                }
            }
        }
        #[cfg(not(unix))]
        {
            let _ = (ctx, on);
            self.status = "MCP bridge needs macOS or Linux".into();
        }
    }

    /// Answer every request that arrived since the last frame, and the
    /// waiting ones whose frame or render has come.
    pub(super) fn serve_mcp(&mut self, ctx: &Context) {
        let shots: Vec<(u64, std::sync::Arc<egui::ColorImage>)> = ctx.input(|i| {
            i.raw
                .events
                .iter()
                .filter_map(|e| match e {
                    Event::Screenshot {
                        user_data, image, ..
                    } => user_data
                        .data
                        .as_ref()
                        .and_then(|d| d.downcast_ref::<McpShot>())
                        .map(|t| (t.0, image.clone())),
                    _ => None,
                })
                .collect()
        });
        for (tag, image) in shots {
            self.mcp_screenshot(tag, &image, ctx.pixels_per_point());
        }
        self.mcp_check_waiting();
        #[cfg(unix)]
        {
            let requests = match &self.mcp.server {
                Some(server) => server.take_requests(),
                None => Vec::new(),
            };
            for request in requests {
                let line = request_line(&request);
                self.mcp_queue(&line, Box::new(move |v| request.respond(v)));
            }
        }
        if let Some(tag) = self.mcp.screenshot.take() {
            ctx.send_viewport_cmd_to(
                egui::ViewportId::ROOT,
                ViewportCommand::Screenshot(egui::UserData::new(McpShot(tag))),
            );
        }
        if self.mcp_stroke_running() {
            // One stroke point per frame: keep the frames coming.
            ctx.request_repaint();
        } else if !self.mcp.waiting.is_empty() {
            ctx.request_repaint_after(Duration::from_millis(100));
        }
    }

    /// Answer `line` now through `respond`, or keep it for a later frame.
    pub(super) fn mcp_queue(&mut self, line: &str, respond: Respond) {
        match self.mcp_begin(line) {
            McpAnswer::Now(response) => respond(response),
            McpAnswer::Later(message, kind) => self.mcp.waiting.push(Waiting {
                message,
                respond,
                kind,
            }),
        }
    }

    /// After the frame's views are drawn: remember where the view was for
    /// the captures asked for this frame. The camera they changed stays
    /// until the picture arrives, whichever frame eframe captures.
    pub(super) fn mcp_after_compose(&mut self) {
        let rect = self
            .view_rects
            .get(&self.active_view)
            .copied()
            .unwrap_or(self.viewport_rect);
        for w in &mut self.mcp.waiting {
            if let Wait::Capture(c) = &mut w.kind
                && !c.drawn
            {
                c.drawn = true;
                c.rect = rect;
            }
        }
    }

    /// The screenshot for capture `tag` arrived: crop it to the view, save
    /// it, answer.
    pub(super) fn mcp_screenshot(&mut self, tag: u64, image: &egui::ColorImage, ppp: f32) {
        let Some(index) = self
            .mcp
            .waiting
            .iter()
            .position(|w| matches!(&w.kind, Wait::Capture(c) if c.tag == tag && c.drawn))
        else {
            return;
        };
        let mut w = self.mcp.waiting.remove(index);
        let Wait::Capture(c) = &mut w.kind else {
            unreachable!("found as a capture")
        };
        if let Some(camera) = c.restore.take() {
            self.camera = camera.clone();
            self.cameras.insert(self.active_view, camera);
        }
        let answer = capture_answer(&w.message, c, image, ppp);
        if let Ok(path) = &answer.1 {
            self.status = format!("Viewport captured (MCP) · {}", path.display());
        }
        (w.respond)(Some(answer.0));
    }

    /// Answer renders that finished, and anything that waited too long.
    fn mcp_check_waiting(&mut self) {
        use operandi_mcp::protocol;
        let now = Instant::now();
        let mut i = 0;
        while i < self.mcp.waiting.len() {
            let answer = match &self.mcp.waiting[i].kind {
                Wait::Render {
                    finished, image, ..
                } if self.render.finished > *finished => {
                    let image = *image;
                    let w = self.mcp.waiting.remove(i);
                    (w.respond)(Some(render_answer(&w.message, &self.render, image)));
                    continue;
                }
                Wait::Render { deadline, .. } if now >= *deadline => Some(Ok(json!({
                    "status": "rendering",
                    "path": self.render.rendering_to,
                    "progress": self.render_progress(),
                    "note": "still rendering; render_status reports it when done",
                }))),
                Wait::Stroke if !self.mcp_stroke_running() => Some(Ok(self.mcp_stroke_answer())),
                Wait::Capture(c) if now.duration_since(c.started) > CAPTURE_TIMEOUT => {
                    Some(Err("No frame was drawn for the capture; is the Operandi window hidden or minimised?".to_string()))
                }
                _ => None,
            };
            match answer {
                Some(result) => {
                    let w = self.mcp.waiting.remove(i);
                    if let Wait::Capture(c) = w.kind
                        && let Some(camera) = c.restore
                    {
                        self.camera = camera.clone();
                        self.cameras.insert(self.active_view, camera);
                    }
                    (w.respond)(Some(protocol::answer(&w.message, result)));
                }
                None => i += 1,
            }
        }
    }

    fn render_progress(&self) -> Value {
        match self.render.control() {
            Some(c) => {
                let p = c.progress();
                json!({"samples_done": p.samples_done, "samples_total": p.samples_total, "paused": p.paused})
            }
            None => Value::Null,
        }
    }

    /// One JSON-RPC line in, its answer out (`None` for a notification).
    /// Calls answered on a later frame (capture, render) are refused here;
    /// the app answers them through `serve_mcp`. The QA harness uses it
    /// too (OPERANDI_QA_SELECT).
    #[cfg(any(test, feature = "qa"))]
    pub(super) fn mcp_answer(&mut self, line: &str) -> Option<serde_json::Value> {
        match self.mcp_begin(line) {
            McpAnswer::Now(response) => response,
            McpAnswer::Later(message, _) => Some(operandi_mcp::protocol::refusal(
                &message,
                "answered on a later frame",
            )),
        }
    }

    /// One JSON-RPC line in: its answer, or what it waits for.
    pub(super) fn mcp_begin(&mut self, line: &str) -> McpAnswer {
        use McpAnswer::Now;
        use operandi_mcp::protocol;
        let Ok(message) = serde_json::from_str::<serde_json::Value>(line) else {
            let mut scratch = operandi_mcp::Bridge::default();
            return Now(protocol::handle_line(&mut scratch, line));
        };
        if let Some(tool) = protocol::called_tool(&message) {
            let editing = !protocol::NON_EDITING_TOOLS.contains(&tool);
            if editing && let Some(refusal) = self.mcp_refusal(tool) {
                return Now(Some(protocol::refusal(&message, &refusal)));
            }
            let args = message
                .get("params")
                .and_then(|p| p.get("arguments"))
                .cloned()
                .unwrap_or(serde_json::Value::Null);
            // New and Open go through the app's own document paths, which
            // keep app-wide settings (Bevel recipes, Cone preset), the saved
            // copy and the window title right.
            match tool {
                "python_run" if !crate::python_switch::enabled() => {
                    return Now(Some(protocol::refusal(&message, crate::python_switch::OFF)));
                }
                "scene_new" => {
                    self.initialize_new_document();
                    if args.get("empty_mesh").and_then(serde_json::Value::as_bool) == Some(false) {
                        self.editor.scene = Scene::default();
                        self.saved = self.editor.scene.clone();
                        self.name.clear();
                    }
                    self.status = "New scene (MCP)".into();
                    return Now(Some(self.mcp_scene_reply(&message)));
                }
                "scene_load" => {
                    let Some(path) = args.get("path").and_then(serde_json::Value::as_str) else {
                        return Now(Some(protocol::refusal(&message, "scene_load needs a path")));
                    };
                    let path = PathBuf::from(path);
                    if !path
                        .extension()
                        .is_some_and(|e| e.eq_ignore_ascii_case("opscene"))
                    {
                        return Now(Some(protocol::refusal(
                            &message,
                            "scene_load opens .opscene files; use File > Open for imports",
                        )));
                    }
                    // The file is already open: read it again (it may have
                    // changed on disk) instead of keeping the copy in
                    // memory. Unsaved changes were refused above.
                    if self.path.as_deref().is_some_and(|p| same_file(p, &path)) {
                        return Now(Some(self.mcp_reread(&message, "Scene reopened (MCP)")));
                    }
                    if let Some((i, dirty)) = self.parked_tab_with(|p| same_file(p, &path)) {
                        if dirty {
                            return Now(Some(protocol::refusal(
                                &message,
                                "That file is open in another scene tab with unsaved changes; save or revert it in the app first",
                            )));
                        }
                        if !self.switch_scene(i) {
                            return Now(Some(protocol::refusal(
                                &message,
                                "That file is open in another scene tab, and Operandi couldn't switch to it now; try again",
                            )));
                        }
                        return Now(Some(self.mcp_reread(&message, "Scene reopened (MCP)")));
                    }
                    // BRQ-60: a mesh with polygons Mesh Cleanup fixes opens
                    // as-is through the bridge (File > Open still refuses it).
                    match persistence::load(&path) {
                        Ok(_) => self.open(path),
                        Err(_)
                            if let Ok(scene) = persistence::load_keeping_broken_polygons(&path) =>
                        {
                            self.open_loaded(path, scene);
                        }
                        Err(e) => {
                            return Now(Some(protocol::refusal(
                                &message,
                                &format!("Open {}: {e}", path.display()),
                            )));
                        }
                    }
                    return Now(Some(self.mcp_scene_reply(&message)));
                }
                "scene_save" if args.get("copy").and_then(Value::as_bool) != Some(true) => {
                    return Now(Some(self.mcp_save(&message, &args)));
                }
                "scene_revert" => {
                    return Now(Some(self.mcp_reread(&message, "Scene reverted (MCP)")));
                }
                "view_set" => {
                    let result = self.mcp_view_set(&args);
                    return Now(Some(protocol::answer(&message, result)));
                }
                "online_library" => {
                    let result = self.mcp_online_library(&args);
                    return Now(Some(protocol::answer(&message, result)));
                }
                "app_command" => {
                    let result = self.mcp_app_command(&args);
                    return Now(Some(protocol::answer(&message, result)));
                }
                "pointer_stroke" => {
                    return match self.mcp_pointer_stroke(&args) {
                        Ok(()) => McpAnswer::Later(message, Wait::Stroke),
                        Err(e) => Now(Some(protocol::refusal(&message, &e))),
                    };
                }
                "panel_show" => {
                    let result = self.mcp_panel_show(&args);
                    return Now(Some(protocol::answer(&message, result)));
                }
                "viewport_capture" => {
                    return match self.mcp_capture(&args) {
                        Ok(capture) => McpAnswer::Later(message, Wait::Capture(Box::new(capture))),
                        Err(e) => Now(Some(protocol::refusal(&message, &e))),
                    };
                }
                "render" => {
                    return match self.mcp_render(&args) {
                        Ok(Some(wait)) => McpAnswer::Later(message, wait),
                        Ok(None) => Now(Some(protocol::answer(
                            &message,
                            Ok(json!({"status": "rendering", "path": self.render.rendering_to})),
                        ))),
                        Err(e) => Now(Some(protocol::refusal(&message, &e))),
                    };
                }
                "render_status" => {
                    if let Err(e) =
                        check_keys("render_status", &args, &["include_image", "max_size"])
                    {
                        return Now(Some(protocol::refusal(&message, &e)));
                    }
                    // A status poll carries no picture unless asked for.
                    let image = match image_options(&args, false) {
                        Ok(image) => image,
                        Err(e) => return Now(Some(protocol::refusal(&message, &e))),
                    };
                    let mut status = json!({
                        "rendering": self.render.busy(),
                        "path": self.render.rendering_to.as_ref().filter(|_| self.render.busy()),
                        "progress": self.render_progress(),
                        "last": match &self.render.last {
                            None => Value::Null,
                            Some(_) => match render_outcome(&self.render) {
                                Ok(v) => v,
                                Err(e) => json!({"error": e}),
                            },
                        },
                    });
                    if let Some(max) = image
                        && let Some(Ok(done)) = &self.render.last
                    {
                        match shrunk_png_of_file(&done.path, max) {
                            Ok(png) => {
                                return Now(Some(protocol::answer_with_image(
                                    &message, status, &png,
                                )));
                            }
                            Err(e) => status["image_error"] = json!(e),
                        }
                    }
                    return Now(Some(protocol::answer(&message, Ok(status))));
                }
                _ => {}
            }
        }
        // UIC-224: an editing call leaves a record (source bridge) in the
        // History tab and on the undo step it makes. `command_run` records
        // its line; other tools their name and JSON arguments.
        let record = protocol::called_tool(&message)
            .filter(|t| !protocol::NON_EDITING_TOOLS.contains(t))
            .map(|tool| self.bridge_record(tool, &message["params"]["arguments"]));
        let serial = self.editor.history.history_serial;
        if let Some(r) = &record {
            self.editor.history.pending_records.0.push(r.clone());
        }
        let mut bridge = operandi_mcp::Bridge {
            editor: std::mem::take(&mut self.editor),
            attached: true,
            ..Default::default()
        };
        let response = protocol::handle(&mut bridge, &message);
        self.editor = bridge.editor;
        if let Some(r) = record {
            self.bridge_recorded(r, serial, response.as_ref());
        }
        // scene_info: the document fields are the app's own document.
        let response = match (protocol::called_tool(&message), response) {
            (Some("scene_info"), Some(mut r)) => {
                let doc = json!({
                    "path": self.path,
                    "name": self.path.as_ref().and_then(|p| p.file_name()).map(|n| n.to_string_lossy()),
                    "modified": self.dirty_exactly(),
                });
                if r["result"]["isError"] == json!(false) {
                    r["result"]["structuredContent"]["document"] = doc;
                    // An open dialog blocks edits; say which one.
                    r["result"]["structuredContent"]["dialog"] = json!(self.open_dialog());
                    let text = r["result"]["structuredContent"].to_string();
                    r["result"]["content"][0]["text"] = json!(text);
                }
                Some(r)
            }
            (_, response) => response,
        };
        let response = Now(response);
        if protocol::called_tool(&message)
            .is_some_and(|t| !protocol::NON_EDITING_TOOLS.contains(&t))
        {
            // Numeric field drafts describe the scene before the call.
            self.clear_fields();
            if let Some(i) = self.editor.scene.active() {
                self.name = i.name.clone();
            }
        }
        response
    }

    /// Why an editing call can't run now, if it can't. The person at the
    /// keyboard wins: an open dialog or a tool they are using is theirs.
    pub(super) fn mcp_refusal(&self, tool: &str) -> Option<String> {
        if let Some(dialog) = self.open_dialog() {
            return Some(format!(
                "Operandi has a dialog open ({dialog}); close it in the app first"
            ));
        }
        if self.navigation.is_some() || self.editor.selection_start.is_some() {
            return Some("Operandi is in the middle of a mouse drag; try again".into());
        }
        // app_command and pointer_stroke act as the person does: they turn
        // tools on and drag them, so an active tool doesn't stop them.
        let acts_as_person = matches!(tool, "app_command" | "pointer_stroke");
        if !acts_as_person
            && (self.editor.session.has_active_task()
                || self.edge_click.is_some()
                || self.vertex_weld)
        {
            return Some("A tool is active in Operandi; drop or cancel it in the app first".into());
        }
        if matches!(tool, "scene_new" | "scene_load") && self.dirty_exactly() {
            return Some(
                "Operandi has unsaved changes; scene_save or scene_revert first (or save or revert in the app)"
                    .into(),
            );
        }
        None
    }

    fn mcp_scene_reply(&self, message: &serde_json::Value) -> serde_json::Value {
        let bridge = operandi_mcp::Bridge {
            editor: self.editor.clone(),
            ..Default::default()
        };
        operandi_mcp::protocol::answer(message, Ok(bridge.summary()))
    }

    /// `scene_save`: Save As, so the document is clean and named.
    fn mcp_save(&mut self, message: &Value, args: &Value) -> Value {
        use operandi_mcp::protocol;
        let Some(path) = args.get("path").and_then(Value::as_str) else {
            return protocol::refusal(message, "scene_save needs a path");
        };
        let path = PathBuf::from(path);
        if !path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("opscene"))
        {
            return protocol::refusal(message, "scene_save writes .opscene files");
        }
        if self.save(path.clone()) {
            protocol::answer(message, Ok(json!({"saved": path, "document": path})))
        } else {
            protocol::refusal(message, &self.status)
        }
    }

    /// `scene_revert`, and `scene_load` of the open file: read the
    /// document's file again, dropping unsaved changes and history.
    fn mcp_reread(&mut self, message: &Value, status: &str) -> Value {
        use operandi_mcp::protocol;
        let Some(path) = self.path.clone() else {
            return protocol::refusal(
                message,
                "The scene was never saved, so there is nothing to revert to",
            );
        };
        match persistence::load_for_bridge(&path) {
            Ok(scene) => {
                self.editor = Editor {
                    bevel_states: self.editor.bevel_states.clone(),
                    bevel_state: self.editor.bevel_state.clone(),
                    bevel_profile: self.editor.bevel_profile,
                    scene: scene.clone(),
                    ..Default::default()
                };
                self.saved = scene;
                self.clear_fields();
                self.name = self
                    .editor
                    .scene
                    .active()
                    .map(|i| i.name.clone())
                    .unwrap_or_default();
                self.status = status.into();
                self.mcp_scene_reply(message)
            }
            Err(e) => protocol::refusal(message, &format!("Revert {}: {e}", path.display())),
        }
    }

    /// Point the active view at `view` (a name or "camera") or through
    /// camera `camera`. Returns the camera to put back.
    fn mcp_point_view(&mut self, args: &Value) -> std::result::Result<String, String> {
        let camera = match args.get("camera").filter(|v| !v.is_null()) {
            Some(v) => Some(v.as_u64().ok_or("camera is an item id")?),
            None => None,
        };
        let view = args
            .get("view")
            .and_then(Value::as_str)
            .unwrap_or(if camera.is_some() {
                "camera"
            } else {
                "current"
            })
            .to_ascii_lowercase();
        let named = match view.as_str() {
            "current" if camera.is_none() => return Ok(self.view_label()),
            "camera" | "current" => None,
            "perspective" => Some(View::Perspective),
            "orthographic" => Some(View::Orthographic),
            "top" => Some(View::Top),
            "bottom" => Some(View::Bottom),
            "front" => Some(View::Front),
            "back" => Some(View::Back),
            "right" => Some(View::Right),
            "left" => Some(View::Left),
            other => {
                return Err(format!(
                    "Unknown view {other}; use current, {}",
                    operandi_mcp::bridge::VIEW_NAMES.join(", ")
                ));
            }
        };
        match (named, camera) {
            (Some(_), Some(_)) => Err("Give a view or a camera, not both".into()),
            (Some(v), None) => {
                self.leave_camera_view();
                self.camera.view = v;
                Ok(self.view_label())
            }
            (None, id) => {
                let id = id
                    .or_else(|| {
                        let scene = &self.editor.scene;
                        scene.render_camera()
                    })
                    .ok_or("The scene has no camera to look through")?;
                self.look_through(id)?;
                Ok(self.view_label())
            }
        }
    }

    fn view_label(&self) -> String {
        if let Some(id) = self.through_light() {
            return format!("light {id}");
        }
        if self.camera.through {
            let id = self
                .camera
                .through_item
                .or_else(|| crate::camera_light_ui::view_camera(&self.editor.scene));
            return match id {
                Some(id) => format!("camera {id}"),
                None => "camera".into(),
            };
        }
        format!("{:?}", self.camera.view).to_lowercase()
    }

    /// `view_set`: what a view shows (its view or camera, and its shading
    /// style), kept. `viewport` makes that view the active one first.
    fn mcp_view_set(&mut self, args: &Value) -> std::result::Result<Value, String> {
        check_keys("view_set", args, &["view", "camera", "shading", "viewport"])?;
        let views = self.views.viewports();
        // Check every argument before changing anything, so a bad one
        // leaves the views as they were.
        let target = match args.get("viewport").filter(|v| !v.is_null()) {
            None => None,
            Some(v) => {
                let index = v.as_u64().ok_or("viewport is an index, 0 or more")? as usize;
                Some(*views.get(index).ok_or_else(|| {
                    format!(
                        "No viewport {index}; there are {} (0 to {})",
                        views.len(),
                        views.len().saturating_sub(1)
                    )
                })?)
            }
        };
        let style = match args.get("shading").filter(|v| !v.is_null()) {
            None => None,
            Some(v) => Some(shading_style(v.as_str().ok_or("shading is a style name")?)?),
        };
        let moves_view = ["view", "camera"]
            .iter()
            .any(|k| args.get(*k).is_some_and(|v| !v.is_null()));
        if moves_view && (self.navigation.is_some() || self.pointer_held) {
            return Err("Operandi is in the middle of a mouse drag; try again".into());
        }
        if let Some(view) = target {
            self.activate_view(view);
        }
        let view = self.mcp_point_view(args)?;
        if let Some(style) = style {
            // The Viewport Properties row's own command (UIC-165), so the
            // panel, the Display menu and this call agree.
            let line = format!("view3d.shadingStyle {{{}}}", style.label());
            if !self.view3d_canonical(&line) || self.display.settings.active.style != style {
                return Err(format!("Shading Style: {}", self.status));
            }
        }
        let shading = shading_name(self.display.settings.active.style);
        self.status = format!("View: {view}, {shading} (MCP)");
        Ok(json!({
            "view": view,
            "shading": shading,
            "viewport": views.iter().position(|&v| v == self.active_view),
            "viewports": views.len(),
        }))
    }

    /// `panel_show`: bring a side panel to the front of its group, adding
    /// it back where it was closed (as the "+" menu does).
    fn mcp_panel_show(&mut self, args: &Value) -> std::result::Result<Value, String> {
        use panel_registry::PanelKind;
        check_keys("panel_show", args, &["panel"])?;
        let name = args
            .get("panel")
            .and_then(Value::as_str)
            .ok_or("panel_show needs a panel")?;
        let kind = panel_kind(name)?;
        let front =
            |g: &mut panel_registry::PanelGroup| match g.tabs.iter().position(|&k| k == kind) {
                Some(i) => {
                    g.show_page(i);
                    true
                }
                None => false,
            };
        use operandi_layout::Pane;
        let pane = if front(&mut self.upper_frame) {
            Pane::Items
        } else if front(&mut self.lower_frame) {
            Pane::Properties
        } else if (self.layouts.mixed_dock && self.upper_frame.tabs.is_empty())
            || (kind == PanelKind::Properties
                && (!self.layouts.mixed_dock || !self.lower_frame.tabs.is_empty()))
        {
            self.lower_frame.add(kind);
            Pane::Properties
        } else {
            self.upper_frame.add(kind);
            Pane::Items
        };
        self.views.reveal_docked_tab(pane);
        let shown = panel_registry::panel_type(kind).name;
        self.status = format!("Panel: {shown} (MCP)");
        let group = |g: &panel_registry::PanelGroup| {
            json!({
                "tabs": g.tabs.iter().map(|&k| panel_name(k)).collect::<Vec<_>>(),
                "front": g.front().map(panel_name),
            })
        };
        Ok(json!({
            "panel": panel_name(kind),
            "upper": group(&self.upper_frame),
            "lower": group(&self.lower_frame),
        }))
    }

    /// `viewport_capture`: point the view, ask for this frame's picture.
    fn mcp_capture(&mut self, args: &Value) -> std::result::Result<Capture, String> {
        check_keys(
            "viewport_capture",
            args,
            &["view", "camera", "path", "include_image", "max_size"],
        )?;
        if !self.corner.areas.contains_key(&self.active_view) && !self.corner.areas.is_empty() {
            return Err("The active view is in a floating window; captures work on views in the main window".into());
        }
        let changes_view = args
            .get("view")
            .and_then(Value::as_str)
            .is_some_and(|v| v != "current")
            || args.get("camera").is_some_and(|v| !v.is_null());
        if changes_view && (self.navigation.is_some() || self.pointer_held) {
            return Err("Operandi is in the middle of a mouse drag; try again".into());
        }
        let path = match args.get("path").and_then(Value::as_str) {
            Some(p) => PathBuf::from(p),
            None => crate::render_ui::numbered_path(None, "Viewport", "png")?,
        };
        if !path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("png"))
        {
            return Err("viewport_capture saves .png files".into());
        }
        let image = image_options(args, true)?;
        let (include_image, max_size) = (image.is_some(), image.unwrap_or(DEFAULT_IMAGE_SIZE));
        let before = self.camera.clone();
        let view = self.mcp_point_view(args)?;
        self.mcp.next_tag += 1;
        let tag = self.mcp.next_tag;
        self.mcp.screenshot = Some(tag);
        Ok(Capture {
            tag,
            restore: changes_view.then_some(before),
            drawn: false,
            rect: self.viewport_rect,
            path,
            include_image,
            max_size,
            view,
            started: Instant::now(),
        })
    }

    /// `render`: start a Cycles render with the call's settings.
    fn mcp_render(&mut self, args: &Value) -> std::result::Result<Option<Wait>, String> {
        check_keys(
            "render",
            args,
            &[
                "path",
                "width",
                "height",
                "samples",
                "camera",
                "wait",
                "timeout",
                "include_image",
                "max_size",
            ],
        )?;
        let image = image_options(args, true)?;
        if self.render.busy() {
            return Err("A render is already running; render_status reports it".into());
        }
        let whole = |key: &str, max: u64| -> std::result::Result<Option<u32>, String> {
            match args.get(key).filter(|v| !v.is_null()) {
                None => Ok(None),
                Some(v) => v
                    .as_u64()
                    .filter(|n| (1..=max).contains(n))
                    .map(|n| Some(n as u32))
                    .ok_or_else(|| format!("{key} is a whole number from 1 to {max}")),
            }
        };
        let size = match (whole("width", 16384)?, whole("height", 16384)?) {
            (Some(w), Some(h)) => Some([w, h]),
            (None, None) => None,
            _ => return Err("Give both width and height, or neither".into()),
        };
        let path = args.get("path").and_then(Value::as_str).map(PathBuf::from);
        if let Some(p) = &path
            && !p.extension().is_some_and(|e| {
                ["png", "jpg", "jpeg"]
                    .iter()
                    .any(|x| e.eq_ignore_ascii_case(x))
            })
        {
            return Err("render writes .png or .jpg files".into());
        }
        let camera = match args.get("camera").filter(|v| !v.is_null()) {
            Some(v) => Some(v.as_u64().ok_or("camera is an item id")?),
            None => None,
        };
        let wait = args.get("wait").and_then(Value::as_bool).unwrap_or(true);
        let timeout = match args.get("timeout").filter(|v| !v.is_null()) {
            Some(v) => v
                .as_f64()
                .filter(|t| (0.0..=3600.0).contains(t))
                .ok_or("timeout is 0 to 3600 seconds")?,
            None => RENDER_WAIT,
        };
        self.render.overrides = Some(crate::render_ui::RenderOverrides {
            size,
            samples: whole("samples", 1 << 20)?,
            path,
            camera,
        });
        let before = self.render.finished;
        self.begin_render(self.render.pixels_per_point, false);
        self.render.overrides = None;
        if !self.render.busy() {
            return Err(self.status.clone());
        }
        Ok(wait.then(|| Wait::Render {
            finished: before,
            deadline: Instant::now() + Duration::from_secs_f64(timeout),
            image,
        }))
    }

    /// Window title suffix while the bridge is on.
    pub(super) fn mcp_title(&self) -> &'static str {
        if self.mcp.is_on() { " · MCP" } else { "" }
    }
}

#[cfg(unix)]
fn request_line(request: &operandi_mcp::server::Request) -> String {
    request.line.clone()
}

/// Whether two paths name the same file (spelled alike, or the same file
/// once links and `..` are resolved).
fn same_file(a: &std::path::Path, b: &std::path::Path) -> bool {
    a == b
        || std::fs::canonicalize(a)
            .ok()
            .zip(std::fs::canonicalize(b).ok())
            .is_some_and(|(a, b)| a == b)
}

/// `view_set shading`'s name for a style (`operandi_mcp::bridge::SHADING_STYLES`).
fn shading_name(style: view_display::Style) -> &'static str {
    use view_display::Style;
    match style {
        Style::Wireframe => "wireframe",
        Style::Solid => "solid",
        Style::VertexMap => "vertex_map",
        Style::Shaded => "shaded",
        Style::Texture => "texture",
        Style::Default => "default",
        Style::Advanced => "advanced",
        Style::Gooch => "gooch",
        Style::Cel => "cel",
        Style::Reflection => "matcap",
    }
}

/// A style by its bridge name, its Display menu name or its short name,
/// in any case.
fn shading_style(name: &str) -> std::result::Result<view_display::Style, String> {
    let squash = |s: &str| {
        s.chars()
            .filter(|c| c.is_alphanumeric())
            .flat_map(char::to_lowercase)
            .collect::<String>()
    };
    let wanted = squash(name);
    view_display::Style::ALL
        .into_iter()
        .find(|&s| {
            [shading_name(s), s.label(), s.short_label()]
                .into_iter()
                .chain(s.old_label())
                .any(|n| squash(n) == wanted)
        })
        .ok_or_else(|| {
            format!(
                "Unknown shading {name}; use {}",
                operandi_mcp::bridge::SHADING_STYLES.join(", ")
            )
        })
}

/// `panel_show`'s name for a panel kind (`operandi_mcp::bridge::PANEL_NAMES`).
fn panel_name(kind: panel_registry::PanelKind) -> &'static str {
    use panel_registry::PanelKind;
    match kind {
        PanelKind::Items => "items",
        PanelKind::ShaderTree => "shader_tree",
        PanelKind::Groups => "groups",
        PanelKind::Properties => "properties",
        PanelKind::MeshStack => "mesh_stack",
        PanelKind::PresetBrowser => "preset_browser",
        PanelKind::UvMaps => "uv_maps",
        PanelKind::CommandHistory => "command_history",
        PanelKind::VertexNormalTools => "vertex_normal_tools",
        PanelKind::Statistics => "statistics",
        PanelKind::NodeView => "node_view",
        // A layout's tab of a kind this build lacks (LAY-49).
        PanelKind::Missing(_) => "missing",
    }
}

/// A panel kind by its bridge name or its tab name, in any case.
fn panel_kind(name: &str) -> std::result::Result<panel_registry::PanelKind, String> {
    let squash = |s: &str| {
        s.chars()
            .filter(|c| c.is_alphanumeric())
            .flat_map(char::to_lowercase)
            .collect::<String>()
    };
    let wanted = squash(name);
    // "mesh_ops", the panel's bridge name before 2026-11-23.
    if wanted == "meshops" {
        return Ok(panel_registry::PanelKind::MeshStack);
    }
    panel_registry::REGISTRY
        .iter()
        .map(|t| t.kind)
        .find(|&k| {
            squash(panel_name(k)) == wanted || squash(panel_registry::panel_type(k).name) == wanted
        })
        .ok_or_else(|| {
            format!(
                "Unknown panel {name}; use {}",
                operandi_mcp::bridge::PANEL_NAMES.join(", ")
            )
        })
}

/// The screenshot request's tag, carried in egui's `UserData`.
struct McpShot(u64);

fn check_keys(tool: &str, args: &Value, known: &[&str]) -> std::result::Result<(), String> {
    match args {
        Value::Null => Ok(()),
        Value::Object(m) => operandi_mcp::commands::check_keys(tool, m, known),
        _ => Err("Arguments must be an object".into()),
    }
}

/// The last render as an MCP result.
fn render_outcome(render: &crate::render_ui::RenderState) -> std::result::Result<Value, String> {
    match &render.last {
        Some(Ok(done)) => Ok(json!({
            "status": "done",
            "path": done.path,
            "width": done.width,
            "height": done.height,
            "samples": done.samples,
            "seconds": done.seconds,
            "device": done.device,
            "denoised": done.denoised,
        })),
        Some(Err(e)) => Err(e.clone()),
        None => Err("No render has finished".into()),
    }
}

/// Longest side of a returned picture unless `max_size` says otherwise.
const DEFAULT_IMAGE_SIZE: u32 = 1024;

/// `include_image` (default `include`) and `max_size`: the longest side of
/// the picture to return, or `None` for no picture.
fn image_options(args: &Value, include: bool) -> std::result::Result<Option<u32>, String> {
    let include = match args.get("include_image") {
        None | Some(Value::Null) => include,
        Some(v) => v.as_bool().ok_or("include_image is true or false")?,
    };
    let max_size = match args.get("max_size") {
        None | Some(Value::Null) => DEFAULT_IMAGE_SIZE,
        Some(v) => v
            .as_u64()
            .filter(|n| (16..=8192).contains(n))
            .ok_or("max_size is 16 to 8192 pixels")? as u32,
    };
    Ok(include.then_some(max_size))
}

/// `picture` as PNG bytes, scaled down so its longest side is at most
/// `max_size` (never scaled up).
fn shrunk_png(picture: image::RgbaImage, max_size: u32) -> std::result::Result<Vec<u8>, String> {
    let (w, h) = picture.dimensions();
    let longest = w.max(h);
    let small = if longest > max_size {
        let scale = f64::from(max_size) / f64::from(longest);
        image::imageops::resize(
            &picture,
            ((f64::from(w) * scale).round() as u32).max(1),
            ((f64::from(h) * scale).round() as u32).max(1),
            image::imageops::FilterType::Triangle,
        )
    } else {
        picture
    };
    let mut png = Vec::new();
    small
        .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
        .map_err(|e| e.to_string())?;
    Ok(png)
}

/// A saved picture file (a render's PNG or JPEG), shrunk for an answer.
fn shrunk_png_of_file(
    path: &std::path::Path,
    max_size: u32,
) -> std::result::Result<Vec<u8>, String> {
    let picture = crate::image_decode::open(path)
        .map_err(|e| format!("{}: {e}", path.display()))?
        .to_rgba8();
    shrunk_png(picture, max_size)
}

/// The answer to a finished render: its outcome, with the picture when
/// `image` asks for one. A picture that can't be read is reported in
/// `image_error`; the render itself still succeeded.
fn render_answer(
    message: &Value,
    render: &crate::render_ui::RenderState,
    image: Option<u32>,
) -> Value {
    use operandi_mcp::protocol;
    let mut value = match render_outcome(render) {
        Ok(v) => v,
        Err(e) => return protocol::refusal(message, &e),
    };
    if let (Some(max), Some(Ok(done))) = (image, &render.last) {
        match shrunk_png_of_file(&done.path, max) {
            Ok(png) => return protocol::answer_with_image(message, value, &png),
            Err(e) => value["image_error"] = json!(e),
        }
    }
    protocol::answer(message, Ok(value))
}

/// Crop the window picture to the view, save it, and build the answer
/// (with the saved path, or the error).
fn capture_answer(
    message: &Value,
    c: &Capture,
    image: &egui::ColorImage,
    ppp: f32,
) -> (Value, std::result::Result<PathBuf, String>) {
    use operandi_mcp::protocol;
    let result = (|| -> std::result::Result<(Value, Option<Vec<u8>>), String> {
        let [iw, ih] = image.size;
        let x0 = ((c.rect.min.x * ppp).round().max(0.) as usize).min(iw);
        let y0 = ((c.rect.min.y * ppp).round().max(0.) as usize).min(ih);
        let x1 = ((c.rect.max.x * ppp).round().max(0.) as usize).min(iw);
        let y1 = ((c.rect.max.y * ppp).round().max(0.) as usize).min(ih);
        let (w, h) = (x1.saturating_sub(x0), y1.saturating_sub(y0));
        if w < 2 || h < 2 {
            return Err("The view has no visible area to capture".into());
        }
        let mut rgba = Vec::with_capacity(w * h * 4);
        for y in y0..y1 {
            for p in &image.pixels[y * iw + x0..y * iw + x1] {
                // Opaque: the window behind the view isn't part of it.
                rgba.extend_from_slice(&[p.r(), p.g(), p.b(), 255]);
            }
        }
        if let Some(parent) = c.path.parent().filter(|p| !p.as_os_str().is_empty()) {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let picture = image::RgbaImage::from_raw(w as u32, h as u32, rgba)
            .ok_or("The capture has the wrong size")?;
        picture
            .save_with_format(&c.path, image::ImageFormat::Png)
            .map_err(|e| format!("{}: {e}", c.path.display()))?;
        let shown = if c.include_image {
            Some(shrunk_png(picture, c.max_size)?)
        } else {
            None
        };
        Ok((
            json!({"path": c.path, "width": w, "height": h, "view": c.view}),
            shown,
        ))
    })();
    match result {
        Ok((value, Some(png))) => (
            protocol::answer_with_image(message, value, &png),
            Ok(c.path.clone()),
        ),
        Ok((value, None)) => (protocol::answer(message, Ok(value)), Ok(c.path.clone())),
        Err(e) => (protocol::refusal(message, &e), Err(e)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};

    fn call(a: &mut Operandi, name: &str, arguments: Value) -> std::result::Result<Value, String> {
        let line = json!({"jsonrpc": "2.0", "id": 7, "method": "tools/call",
            "params": {"name": name, "arguments": arguments}})
        .to_string();
        let r = a.mcp_answer(&line).expect("a request gets an answer");
        assert_eq!(r["id"], 7);
        let result = &r["result"];
        if result["isError"] == json!(true) {
            Err(result["content"][0]["text"].as_str().unwrap().to_string())
        } else {
            Ok(result["structuredContent"].clone())
        }
    }

    fn app() -> Operandi {
        let mut a = Operandi::empty();
        a.initialize_new_document();
        a
    }

    /// PF-12 in the running app (Click to Start on): tool_run edge.extend
    /// at its Reset values adds the zero-width row, even after other runs.
    #[test]
    fn edge_extend_at_reset_through_the_app() {
        let mut a = app();
        a.editor.auto_activate_flags = true; // the running app honours Click to Start
        a.editor.scene.active_mut().unwrap().mesh = {
            let mut m = operandi_core::Mesh {
                vertices: (0..9)
                    .map(|k| glam::DVec3::new((k % 3) as f64, 0., (k / 3) as f64))
                    .collect(),
                ..Default::default()
            };
            m.faces = [[0, 3, 4, 1], [1, 4, 5, 2], [3, 6, 7, 4], [4, 7, 8, 5]]
                .into_iter()
                .map(|v| operandi_core::Face {
                    vertices: v.to_vec(),
                    uv: None,
                    material: "Default".into(),
                    smooth: false,
                    part: None,
                    material_label: None,
                })
                .collect::<Vec<_>>()
                .into();
            m.into()
        };
        a.editor.synchronize_identities().unwrap();
        let grid = a.editor.scene.clone();
        // Another run first, leaving its values behind.
        call(&mut a, "select", json!({"mode": "edge", "edges": [[0, 1]]})).unwrap();
        call(
            &mut a,
            "tool_run",
            json!({"tool": "edge.extend", "params": {"delta": [0, 0, -0.5]}}),
        )
        .unwrap();
        a.editor.scene = grid;
        a.editor.synchronize_identities().unwrap();
        call(&mut a, "select", json!({"mode": "edge", "edges": [[0, 1]]})).unwrap();
        call(&mut a, "tool_run", json!({"tool": "edge.extend"})).unwrap();
        let m = &a.editor.scene.active().unwrap().mesh;
        assert_eq!(m.vertices.len(), 11, "{}", a.status);
        assert_eq!(m.vertices[9], glam::DVec3::new(1., 0., 0.));
        assert_eq!(m.vertices[10], glam::DVec3::ZERO);
        assert_eq!(a.editor.scene.selection.edges, [(9, 10)]);
        let mut points = a.editor.scene.selection.vertices.clone();
        points.sort();
        assert_eq!(points, [9, 10]);
    }

    #[test]
    fn calls_edit_the_scene_on_screen_with_app_history() {
        let mut a = app();
        call(&mut a, "tool_run", json!({"tool": "prim.cube"})).unwrap();
        // The app's own editor changed, with one undo step and a dirty title.
        assert_eq!(a.editor.scene.active().unwrap().mesh.faces.len(), 6);
        assert_eq!(a.editor.history.undo.len(), 1);
        assert!(a.dirty());
        // The person's Undo takes the agent's step back, and the agent sees it.
        a.run(Command::Undo);
        let info = call(&mut a, "scene_info", json!({})).unwrap();
        assert_eq!(info["undo"].as_array().map(Vec::len), Some(0));
        assert!(a.editor.scene.active().unwrap().mesh.faces.is_empty());
    }

    #[test]
    fn the_person_at_the_keyboard_wins() {
        let mut a = app();
        call(&mut a, "tool_run", json!({"tool": "prim.cube"})).unwrap();
        a.activate(Tool::Move);
        assert!(a.editor.session.has_active_task(), "{}", a.status);
        let refused = call(&mut a, "tool_run", json!({"tool": "prim.sphere"})).unwrap_err();
        assert!(refused.contains("tool is active"), "{refused}");
        // Reading is still fine.
        assert!(call(&mut a, "scene_info", json!({})).is_ok());
        a.run(Command::Drop);
        // A new scene opens in Item mode; component selection needs Polygon.
        a.run(Command::Mode(Mode::Polygon));
        let after = call(&mut a, "selection_op", json!({"op": "all"}));
        assert!(after.is_ok(), "{after:?}");
        // New would throw away unsaved work.
        let refused = call(&mut a, "scene_new", json!({})).unwrap_err();
        assert!(refused.contains("unsaved"), "{refused}");
        assert_eq!(a.editor.scene.active().unwrap().mesh.faces.len(), 6);
    }

    #[test]
    fn new_and_load_use_the_app_document_paths() {
        let mut a = app();
        call(&mut a, "tool_run", json!({"tool": "prim.cube"})).unwrap();
        let path = a.environment.paths().data_root().join("mcp-cube.opscene");
        call(&mut a, "scene_save", json!({"path": path})).unwrap();
        a.saved = a.editor.scene.clone(); // as if the person saved
        call(&mut a, "scene_new", json!({})).unwrap();
        assert!(a.path.is_none() && !a.dirty());
        assert!(a.editor.scene.active().unwrap().mesh.faces.is_empty());
        let info = call(&mut a, "scene_load", json!({"path": path})).unwrap();
        assert_eq!(info["active"]["faces"], 6, "{info}");
        assert_eq!(a.path.as_deref(), Some(path.as_path()));
        assert!(!a.dirty());
        assert!(call(&mut a, "scene_load", json!({"path": "x.lxo"})).is_err());
    }

    /// Hero-render gap 3: scene_load of the file already open read nothing
    /// and still reported success. It now reads the file again, so an edit
    /// made to it on disk shows; unsaved changes are still refused.
    #[test]
    fn scene_load_of_the_open_file_reads_it_again() {
        let mut a = app();
        let dir = temp("reload");
        let path = dir.join("hero.opscene");
        call(&mut a, "primitive_add", json!({"shape": "cube"})).unwrap();
        call(&mut a, "scene_save", json!({"path": path})).unwrap();
        // The file changes on disk (an edited .opscene, another session).
        let mut other = operandi_mcp::Bridge::new();
        other.call("scene_load", &json!({"path": path})).unwrap();
        other
            .call("primitive_add", &json!({"shape": "sphere"}))
            .unwrap();
        other.call("scene_save", &json!({"path": path})).unwrap();
        let disk = other.editor.scene.active().unwrap().mesh.faces.len();
        assert_ne!(disk, 6);
        let info = call(&mut a, "scene_load", json!({"path": path})).unwrap();
        assert_eq!(info["active"]["faces"], disk, "{info}");
        assert_eq!(a.path.as_deref(), Some(path.as_path()));
        assert!(!a.dirty());
        assert!(a.editor.history.undo.is_empty());
        assert!(!a.has_modal());
        // With unsaved changes it refuses and keeps them.
        call(&mut a, "primitive_add", json!({"shape": "cube"})).unwrap();
        let e = call(&mut a, "scene_load", json!({"path": path})).unwrap_err();
        assert!(e.contains("unsaved"), "{e}");
        assert!(a.dirty());
    }

    /// Hero-render gap 7: a refusal names the dialog that blocks edits, and
    /// scene_info reports it.
    #[test]
    fn refusals_name_the_open_dialog() {
        let mut a = app();
        assert_eq!(
            call(&mut a, "scene_info", json!({})).unwrap()["dialog"],
            Value::Null
        );
        a.show_preferences = true;
        let e = call(&mut a, "primitive_add", json!({"shape": "cube"})).unwrap_err();
        assert!(e.contains("Preferences window"), "{e}");
        assert_eq!(
            call(&mut a, "scene_info", json!({})).unwrap()["dialog"],
            "Preferences window"
        );
        // Material and shader edits are refused the same way.
        let e = call(&mut a, "material_set", json!({"roughness": 0.2})).unwrap_err();
        assert!(e.contains("dialog open"), "{e}");
        a.show_preferences = false;
        call(&mut a, "material_set", json!({"roughness": 0.2})).unwrap();
        assert!(a.dirty());
    }

    #[test]
    fn protocol_messages_still_work() {
        let mut a = app();
        let init = a
            .mcp_answer(r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#)
            .unwrap();
        assert_eq!(init["result"]["serverInfo"]["name"], "operandi-mcp");
        assert!(
            a.mcp_answer(r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#)
                .is_none()
        );
        assert_eq!(a.mcp_answer("{oops").unwrap()["error"]["code"], -32700);
    }

    /// Queue `name` with `arguments`; the answer lands in the returned cell
    /// whenever the app gives it.
    fn queue(
        a: &mut Operandi,
        name: &str,
        arguments: Value,
    ) -> std::sync::Arc<std::sync::Mutex<Option<Value>>> {
        let cell = std::sync::Arc::new(std::sync::Mutex::new(None));
        let out = cell.clone();
        let line = json!({"jsonrpc": "2.0", "id": 9, "method": "tools/call",
            "params": {"name": name, "arguments": arguments}})
        .to_string();
        a.mcp_queue(&line, Box::new(move |v| *out.lock().unwrap() = v));
        cell
    }

    fn result(cell: &std::sync::Arc<std::sync::Mutex<Option<Value>>>) -> Option<Value> {
        cell.lock().unwrap().as_ref().map(|r| r["result"].clone())
    }

    fn temp(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("operandi-mcp-app-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// 4: scene_save is Save As: the document is named and clean, so
    /// scene_new works; copy keeps the old behaviour. scene_revert drops
    /// unsaved changes.
    #[test]
    fn scene_save_cleans_the_document_and_revert_reopens_it() {
        let mut a = app();
        let dir = temp("save");
        call(&mut a, "primitive_add", json!({"shape": "cube"})).unwrap();
        assert!(a.dirty());
        let copy = dir.join("copy.opscene");
        call(&mut a, "scene_save", json!({"path": copy, "copy": true})).unwrap();
        assert!(
            a.dirty() && a.path.is_none(),
            "a copy leaves the document alone"
        );
        let path = dir.join("still life.opscene");
        call(&mut a, "scene_save", json!({"path": path})).unwrap();
        assert!(!a.dirty());
        assert_eq!(a.path.as_deref(), Some(path.as_path()));
        // Changes, then revert: back to the six faces on disk.
        call(&mut a, "primitive_add", json!({"shape": "sphere"})).unwrap();
        assert!(a.dirty());
        let info = call(&mut a, "scene_revert", json!({})).unwrap();
        assert_eq!(info["active"]["faces"], 6, "{info}");
        assert!(!a.dirty());
        call(&mut a, "scene_new", json!({})).unwrap();
        assert!(a.path.is_none());
        let e = call(&mut a, "scene_revert", json!({})).unwrap_err();
        assert!(e.contains("never saved"), "{e}");
    }

    /// a: view_set looks through a camera, or shows a named view.
    #[test]
    fn view_set_looks_through_a_camera() {
        let mut a = app();
        let cam = call(
            &mut a,
            "command_run",
            json!({"command": "item.add", "args": {"type": "camera"}}),
        )
        .unwrap()["selection"]["item"]
            .as_u64()
            .unwrap();
        let mesh = a
            .editor
            .scene
            .items
            .iter()
            .find(|i| i.camera.is_none() && i.light.is_none())
            .unwrap()
            .id;
        call(&mut a, "select", json!({"mode": "item", "item": mesh})).unwrap();
        let r = call(&mut a, "view_set", json!({"camera": cam})).unwrap();
        assert_eq!(r["view"], format!("camera {cam}"));
        assert!(a.camera.through);
        assert_eq!(a.camera.through_item, Some(cam));
        a.sync_camera_view(Rect::from_min_size(Pos2::ZERO, vec2(800., 600.)));
        assert!(
            a.camera.through,
            "the chosen camera stays, though another item is active"
        );
        call(&mut a, "view_set", json!({"view": "top"})).unwrap();
        assert!(!a.camera.through && a.camera.through_item.is_none());
        assert_eq!(a.camera.view, View::Top);
        assert!(call(&mut a, "view_set", json!({"camera": mesh})).is_err());
        assert!(call(&mut a, "view_set", json!({"view": "sideways"})).is_err());
    }

    /// a: a capture draws the asked-for view for one frame, crops the
    /// window picture to the view, saves it, returns it, and puts the view
    /// back.
    #[test]
    fn viewport_capture_crops_saves_and_restores_the_view() {
        let mut a = crate::tests::box_app();
        a.camera.view = View::Perspective;
        let ctx = egui::Context::default();
        crate::tests::draw(&mut a, &ctx, vec2(1000., 700.), vec![]);
        let dir = temp("capture");
        let path = dir.join("front.png");
        let cell = queue(
            &mut a,
            "viewport_capture",
            json!({"view": "front", "path": path, "max_size": 64}),
        );
        assert!(result(&cell).is_none(), "answered after the frame");
        let tag = a.mcp.screenshot.take().expect("a screenshot was asked for");
        assert_eq!(
            a.camera.view,
            View::Front,
            "this frame draws the front view"
        );
        crate::tests::draw(&mut a, &ctx, vec2(1000., 700.), vec![]);
        a.mcp_after_compose();
        assert_eq!(a.camera.view, View::Front, "kept until the picture comes");
        let rect = a.viewport_rect;
        assert!(rect.width() > 100., "{rect:?}");
        // The window picture: the view's area blue, the rest red.
        let (w, h) = (1000usize, 700usize);
        let mut image = egui::ColorImage::filled([w, h], Color32::RED);
        for y in 0..h {
            for x in 0..w {
                if rect.contains(pos2(x as f32 + 0.5, y as f32 + 0.5)) {
                    image.pixels[y * w + x] = Color32::BLUE;
                }
            }
        }
        a.mcp_screenshot(tag, &image, 1.0);
        assert_eq!(a.camera.view, View::Perspective, "the view is put back");
        let r = result(&cell).expect("answered");
        assert_eq!(r["isError"], false, "{r}");
        assert_eq!(r["content"][1]["type"], "image");
        assert_eq!(r["structuredContent"]["view"], "front");
        let saved = image::open(&path).unwrap().to_rgba8();
        assert_eq!(saved.width() as f32, rect.width().round());
        assert!(
            saved.pixels().all(|p| p.0 == [0, 0, 255, 255]),
            "only the view"
        );
        let shown = r["content"][1]["data"].as_str().unwrap();
        assert!(!shown.is_empty());
        // Refusals: bad view, not png.
        let bad = queue(&mut a, "viewport_capture", json!({"view": "sideways"}));
        assert_eq!(result(&bad).unwrap()["isError"], true);
        let bad = queue(
            &mut a,
            "viewport_capture",
            json!({"path": dir.join("x.jpg")}),
        );
        assert_eq!(result(&bad).unwrap()["isError"], true);
    }

    fn fake_render(
        _meshes: &[operandi_cycles::RenderMesh],
        s: &operandi_cycles::Settings,
        _base: Option<Vec<f32>>,
        _control: &operandi_cycles::RenderControl,
    ) -> operandi_core::Result<operandi_cycles::Image> {
        Ok(operandi_cycles::Image {
            width: s.width,
            height: s.height,
            rgba: vec![0.5; s.width as usize * s.height as usize * 4],
            device: "Test".into(),
            used_gpu: false,
            denoised: false,
            samples: s.samples,
            light_groups: Vec::new(),
            passes: Vec::new(),
            values: Vec::new(),
            cryptomatte_manifests: Vec::new(),
        })
    }

    /// b: render to a chosen file at a chosen size and sample count; the
    /// answer waits for the render and reports the file.
    #[test]
    fn render_writes_the_file_and_answers_when_done() {
        let mut a = crate::tests::box_app();
        // No camera in this scene: renders use the Perspective view.
        a.camera.view = View::Perspective;
        let dir = temp("render");
        a.render.set_renderer(fake_render, dir.clone());
        let path = dir.join("still life.png");
        let cell = queue(
            &mut a,
            "render",
            json!({"path": path, "width": 64, "height": 48, "samples": 7, "timeout": 30}),
        );
        let ctx = egui::Context::default();
        for _ in 0..400 {
            if result(&cell).is_some() {
                break;
            }
            std::thread::sleep(Duration::from_millis(5));
            a.poll_render(&ctx);
            a.mcp_check_waiting();
        }
        let r = result(&cell).expect("the render answered");
        assert_eq!(r["isError"], false, "{r}");
        let done = &r["structuredContent"];
        assert_eq!(done["status"], "done");
        assert_eq!(done["width"], 64);
        assert_eq!(done["height"], 48);
        assert_eq!(done["samples"], 7);
        assert_eq!(done["path"], json!(path));
        let picture = image::open(&path).unwrap();
        assert_eq!((picture.width(), picture.height()), (64, 48));
        // The answer carries the picture, so an agent can look at it.
        assert_eq!(r["content"][1]["type"], "image", "{r}");
        assert_eq!(r["content"][1]["mimeType"], "image/png");
        let status = call(&mut a, "render_status", json!({})).unwrap();
        assert_eq!(status["rendering"], false);
        assert_eq!(status["last"]["path"], json!(path));
        // render_status: no picture unless asked; then the last render's,
        // scaled to max_size.
        let line = |args: Value| {
            json!({"jsonrpc": "2.0", "id": 3, "method": "tools/call",
                   "params": {"name": "render_status", "arguments": args}})
            .to_string()
        };
        let plain = a.mcp_answer(&line(json!({}))).unwrap();
        assert_eq!(plain["result"]["content"].as_array().unwrap().len(), 1);
        let shown = a
            .mcp_answer(&line(json!({"include_image": true, "max_size": 32})))
            .unwrap();
        let data = shown["result"]["content"][1]["data"].as_str().unwrap();
        let png = decode_base64(data);
        let small = image::load_from_memory(&png).unwrap();
        assert_eq!((small.width(), small.height()), (32, 24));
        assert!(a.mcp_answer(&line(json!({"max_size": 4}))).unwrap()["result"]["isError"] == true);
        // No wait: answered at once while it renders.
        let cell = queue(
            &mut a,
            "render",
            json!({"wait": false, "width": 8, "height": 8}),
        );
        assert_eq!(
            result(&cell).unwrap()["structuredContent"]["status"],
            "rendering"
        );
        assert!(
            call(&mut a, "render", json!({"width": 8})).is_err(),
            "busy, or half a size"
        );
        for _ in 0..400 {
            if !a.render.busy() {
                break;
            }
            std::thread::sleep(Duration::from_millis(5));
            a.poll_render(&ctx);
        }
        let e = call(&mut a, "render", json!({"width": 8})).unwrap_err();
        assert!(e.contains("both"), "{e}");
        let e = call(&mut a, "render", json!({"path": dir.join("x.exr")})).unwrap_err();
        assert!(e.contains(".png"), "{e}");
        // include_image false: the file only, no picture in the answer.
        let cell = queue(
            &mut a,
            "render",
            json!({"width": 16, "height": 16, "include_image": false, "timeout": 30}),
        );
        for _ in 0..400 {
            if result(&cell).is_some() {
                break;
            }
            std::thread::sleep(Duration::from_millis(5));
            a.poll_render(&ctx);
            a.mcp_check_waiting();
        }
        let r = result(&cell).expect("the render answered");
        assert_eq!(r["structuredContent"]["status"], "done", "{r}");
        assert_eq!(r["content"].as_array().unwrap().len(), 1);
    }

    /// Standard base64 back to bytes (test helper).
    fn decode_base64(text: &str) -> Vec<u8> {
        const TABLE: &[u8; 64] =
            b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let mut bits = 0u32;
        let mut count = 0;
        let mut out = Vec::new();
        for c in text.bytes().filter(|&c| c != b'=') {
            let v = TABLE.iter().position(|&t| t == c).unwrap() as u32;
            bits = (bits << 6) | v;
            count += 6;
            if count >= 8 {
                count -= 8;
                out.push((bits >> count) as u8);
                bits &= (1 << count) - 1;
            }
        }
        out
    }

    /// b: a render that outlasts its wait answers "still rendering".
    #[test]
    fn a_long_render_answers_when_the_wait_runs_out() {
        let mut a = crate::tests::box_app();
        // No camera in this scene: renders use the Perspective view.
        a.camera.view = View::Perspective;
        let dir = temp("render-wait");
        fn slow(
            m: &[operandi_cycles::RenderMesh],
            s: &operandi_cycles::Settings,
            b: Option<Vec<f32>>,
            c: &operandi_cycles::RenderControl,
        ) -> operandi_core::Result<operandi_cycles::Image> {
            while !c.cancelled() {
                std::thread::sleep(Duration::from_millis(5));
            }
            fake_render(m, s, b, c)
        }
        a.render.set_renderer(slow, dir);
        let cell = queue(
            &mut a,
            "render",
            json!({"timeout": 0, "width": 8, "height": 8}),
        );
        a.mcp_check_waiting();
        let r = result(&cell).expect("answered when the wait ran out");
        assert_eq!(r["structuredContent"]["status"], "rendering", "{r}");
        assert!(call(&mut a, "render_status", json!({})).unwrap()["rendering"] == true);
        a.render.control().unwrap().cancel();
    }

    /// BRQ-60: the app's bridge opens a mesh with polygons Mesh Cleanup
    /// fixes as-is; File > Open's loader still refuses it.
    #[test]
    fn scene_load_opens_broken_polygons_as_given() {
        let mut a = app();
        let path = temp("broken").join("broken.opscene");
        let face = |v: &[u32]| json!({"vertices": v, "uv": null, "material": "", "smooth": false});
        let file = json!({"format": "Operandi", "version": 1, "scene": {"items": [{
            "id": 1, "name": "Broken", "visible": true,
            "transform": {"translation": [0, 0, 0], "rotation": [0, 0, 0], "scale": [1, 1, 1]},
            "mesh": {"vertices": [[0, 0, 0], [1, 0, 0], [1, 1, 0], [3, 0, 0]],
                     "faces": [face(&[0, 1, 2]), face(&[3]), face(&[0, 1, 1, 2])]}
        }], "next_id": 2,
            "selection": {"item": 1, "mode": "Polygon", "vertices": [], "edges": [], "faces": []},
            "workplane": {"fixed": true, "plane": 0, "center": [0, 0, 0]}}});
        std::fs::write(&path, file.to_string()).unwrap();
        assert!(persistence::load(&path).is_err());
        call(&mut a, "scene_load", json!({"path": path})).unwrap();
        let m = call(&mut a, "mesh_get", json!({"edges": false})).unwrap();
        assert_eq!(m["faces"][1]["vertices"], json!([3]));
        assert_eq!(m["faces"][2]["vertices"], json!([0, 1, 1, 2]));
        assert_eq!(a.path.as_deref(), Some(path.as_path()));
    }

    /// A full undo after a multi-item Mesh Stack case leaves the app's
    /// document clean (replay quirk).
    #[test]
    fn undoing_a_mesh_stack_case_leaves_the_document_clean() {
        let mut a = app();
        call(&mut a, "primitive_add", json!({"shape": "cube"})).unwrap();
        let path = temp("dirty-ops").join("cube.opscene");
        call(&mut a, "scene_save", json!({"path": path})).unwrap();
        call(&mut a, "scene_load", json!({"path": path})).unwrap();
        let saved = a.editor.scene.clone();
        let first = a.editor.scene.items[0].id;
        let run = |a: &mut Operandi, c: &str, args: Value| {
            call(a, "command_run", json!({"command": c, "args": args})).unwrap()
        };
        run(&mut a, "mesh.new", json!({}));
        call(
            &mut a,
            "primitive_add",
            json!({"shape": "cube", "params": {"position": [3, 0, 0]}}),
        )
        .unwrap();
        let second = a.editor.scene.selection.item.unwrap();
        run(&mut a, "select.item", json!({"item": first}));
        run(&mut a, "meshop.create", json!({"type": "pmodel.meshmerge"}));
        let op = call(&mut a, "item_get", json!({"item": first})).unwrap()["mesh_stack"][0]["op"]
            .clone();
        call(
            &mut a,
            "meshop_set",
            json!({"op": op, "values": {"sources": [second]}}),
        )
        .unwrap();
        let steps = a.editor.history.undo.len();
        call(&mut a, "undo", json!({"count": steps})).unwrap();
        if a.editor.scene != saved {
            let ours = format!("{:#?}", a.editor.scene);
            let theirs = format!("{:#?}", saved);
            let diff: Vec<_> = ours
                .lines()
                .zip(theirs.lines())
                .filter(|(x, y)| x != y)
                .take(10)
                .collect();
            panic!("scene differs after full undo: {diff:#?}");
        }
        let d = call(&mut a, "scene_info", json!({})).unwrap()["document"].clone();
        assert_eq!(d["modified"], false, "{d}");
    }

    /// Follow-up 1: scene_info's document fields are the app's document.
    #[test]
    fn scene_info_reports_the_app_document() {
        let mut a = app();
        let doc = |a: &mut Operandi| call(a, "scene_info", json!({})).unwrap()["document"].clone();
        assert_eq!(doc(&mut a)["modified"], false);
        call(&mut a, "primitive_add", json!({"shape": "cube"})).unwrap();
        assert_eq!(doc(&mut a)["modified"], true);
        let path = temp("doc").join("vase.opscene");
        call(&mut a, "scene_save", json!({"path": path})).unwrap();
        let d = doc(&mut a);
        assert_eq!(d["modified"], false);
        assert_eq!(d["name"], "vase.opscene");
        assert_eq!(d["path"], json!(path));
        // A mesh that an undo rebuilt as an equal copy is unchanged to the
        // bridge, so scene_load isn't refused (replay quirk after Curve
        // Slice); the window's quick per-frame check may still offer a save.
        let id = a.editor.scene.items[0].id;
        let copy: operandi_core::Mesh = (*a.editor.scene.item(id).unwrap().mesh).clone();
        a.editor.scene.items[0].mesh = copy.into();
        assert!(a.dirty(), "the per-frame check compares allocations");
        assert_eq!(doc(&mut a)["modified"], false);
        call(&mut a, "scene_load", json!({"path": path})).unwrap();
    }

    fn probe_render(
        _meshes: &[operandi_cycles::RenderMesh],
        s: &operandi_cycles::Settings,
        _base: Option<Vec<f32>>,
        _control: &operandi_cycles::RenderControl,
    ) -> operandi_core::Result<operandi_cycles::Image> {
        // Records the denoise switch in the samples it reports.
        let mut image = fake_render(_meshes, s, _base, _control)?;
        image.samples = s.samples * 10 + u32::from(s.denoise);
        image.rgba.fill(0.25);
        Ok(image)
    }

    /// Follow-up 3: renders use the scene's samples, denoise and exposure.
    #[test]
    fn renders_use_the_scene_render_settings() {
        let mut a = crate::tests::box_app();
        a.camera.view = View::Perspective;
        let dir = temp("render-settings");
        a.render.set_renderer(probe_render, dir.clone());
        call(
            &mut a,
            "render_settings_set",
            json!({"samples": 5, "denoise": false}),
        )
        .unwrap();
        let path = dir.join("e.png");
        let cell = queue(
            &mut a,
            "render",
            json!({"path": path, "width": 4, "height": 4}),
        );
        let ctx = egui::Context::default();
        for _ in 0..400 {
            if result(&cell).is_some() {
                break;
            }
            std::thread::sleep(Duration::from_millis(5));
            a.poll_render(&ctx);
            a.mcp_check_waiting();
        }
        let r = result(&cell).expect("answered");
        assert_eq!(
            r["structuredContent"]["samples"], 50,
            "5 samples, denoise off: {r}"
        );
    }

    const QUAD_OBJ: &str = "o Card\nv 0 0 0\nv 1 0 0\nv 1 1 0\nv 0 1 0\nf 1 2 3 4\n";

    /// Hero gap 1: scene_import adds an OBJ to the open scene as one undo
    /// step; every problem is an error with no dialog and no change.
    #[test]
    fn scene_import_adds_to_the_open_scene_without_dialogs() {
        let mut a = app();
        call(&mut a, "primitive_add", json!({"shape": "cube"})).unwrap();
        let dir = temp("import");
        let obj = dir.join("card.obj");
        std::fs::write(&obj, QUAD_OBJ).unwrap();
        let items_before = a.editor.scene.items.len();
        let undo_before = a.editor.history.undo.len();
        let r = call(&mut a, "scene_import", json!({"path": obj})).unwrap();
        let added = r["imported"].as_array().unwrap();
        assert_eq!(added.len(), 1, "{r}");
        assert_eq!(a.editor.scene.items.len(), items_before + 1);
        assert_eq!(a.editor.history.undo.len(), undo_before + 1);
        assert_eq!(
            a.editor.history.undo.last().unwrap().label,
            "Import card.obj"
        );
        let id = added[0]["id"].as_u64().unwrap();
        let card = a.editor.scene.item(id).unwrap();
        assert_eq!(card.mesh.faces.len(), 1);
        assert_eq!(a.editor.scene.selection.item, Some(id));
        // The cube is still there: the document was kept, not replaced.
        assert!(a.editor.scene.items.iter().any(|i| i.mesh.faces.len() == 6));
        assert!(!a.has_modal() && a.import_message.is_none() && a.file_dialog.is_none());
        // One Undo takes it all back.
        a.run(Command::Undo);
        assert_eq!(a.editor.scene.items.len(), items_before);
        // Refusals: nothing changes and no dialog opens.
        let bad = dir.join("broken.obj");
        std::fs::write(&bad, "f 1 2 9\n").unwrap();
        let scene = a.editor.scene.clone();
        for (path, says) in [
            (dir.join("missing.obj"), "No file"),
            (bad, ""),
            (dir.join("old.lxo"), "File > Open"),
            (dir.join("x.opscene"), "scene_load"),
            (dir.join("notes.txt"), "reads obj"),
        ] {
            let e = call(&mut a, "scene_import", json!({"path": path})).unwrap_err();
            assert!(e.contains(says), "{path:?}: {e}");
            assert!(a.editor.scene == scene, "{path:?} changed the scene");
            assert!(!a.has_modal() && a.import_message.is_none() && a.file_dialog.is_none());
        }
        assert!(call(&mut a, "scene_import", json!({"path": obj, "x": 1})).is_err());
        // The person at the keyboard wins: an open dialog refuses it.
        a.show_preferences = true;
        let e = call(&mut a, "scene_import", json!({"path": obj})).unwrap_err();
        assert!(e.contains("dialog open"), "{e}");
        assert!(a.editor.scene == scene);
    }

    /// Hero gap 2: view_set shading sets the view's Shading Style through
    /// view3d.shadingStyle; viewport picks a view by index.
    #[test]
    fn view_set_sets_the_shading_style() {
        // The bridge's names are the app's styles, in the menu's order.
        let names: Vec<&str> = view_display::Style::ALL
            .into_iter()
            .map(shading_name)
            .collect();
        assert_eq!(names, operandi_mcp::bridge::SHADING_STYLES);
        let mut a = app();
        let r = call(&mut a, "view_set", json!({"shading": "wireframe"})).unwrap();
        assert_eq!(r["shading"], "wireframe", "{r}");
        assert_eq!(
            a.display.settings.active.style,
            view_display::Style::Wireframe
        );
        call(
            &mut a,
            "view_set",
            json!({"shading": "Gooch Tone Shading", "view": "top"}),
        )
        .unwrap();
        assert_eq!(a.display.settings.active.style, view_display::Style::Gooch);
        assert_eq!(a.camera.view, View::Top);
        // Not a scene edit.
        assert!(a.editor.history.undo.is_empty() && !a.dirty());
        // A bad argument changes nothing.
        let e = call(
            &mut a,
            "view_set",
            json!({"shading": "toon", "view": "front"}),
        )
        .unwrap_err();
        assert!(e.contains("Unknown shading"), "{e}");
        assert_eq!(a.camera.view, View::Top);
        assert!(call(&mut a, "view_set", json!({"viewport": 99})).is_err());
        // A second view: viewport picks it and makes it active; the first
        // view keeps its own style.
        let first = a.active_view;
        a.split_view();
        let views = a.views.viewports();
        assert_eq!(views.len(), 2);
        let index = views.iter().position(|&v| v == first).unwrap();
        let other = 1 - index;
        let r = call(
            &mut a,
            "view_set",
            json!({"viewport": other, "shading": "solid"}),
        )
        .unwrap();
        assert_eq!(r["viewport"], other, "{r}");
        assert_eq!(r["viewports"], 2);
        assert_eq!(a.active_view, views[other]);
        assert_eq!(a.display.settings.active.style, view_display::Style::Solid);
        call(&mut a, "view_set", json!({"viewport": index})).unwrap();
        assert_eq!(a.display.settings.active.style, view_display::Style::Gooch);
    }

    /// Hero gap 3: panel_show brings a panel's tab to the front, or adds
    /// it back where it was closed.
    #[test]
    fn panel_show_brings_a_panel_to_the_front() {
        let registered: Vec<&str> = panel_registry::REGISTRY
            .iter()
            .map(|t| panel_name(t.kind))
            .collect();
        assert_eq!(registered, operandi_mcp::bridge::PANEL_NAMES);
        use panel_registry::PanelKind;
        let mut a = app();
        let r = call(&mut a, "panel_show", json!({"panel": "shader_tree"})).unwrap();
        assert_eq!(a.upper_frame.front(), Some(PanelKind::ShaderTree));
        assert_eq!(r["upper"]["front"], "shader_tree", "{r}");
        call(&mut a, "panel_show", json!({"panel": "Items"})).unwrap();
        assert_eq!(a.upper_frame.front(), Some(PanelKind::Items));
        // A closed panel comes back to its group, in front.
        a.upper_frame.remove(PanelKind::Groups);
        call(&mut a, "panel_show", json!({"panel": "groups"})).unwrap();
        assert_eq!(a.upper_frame.front(), Some(PanelKind::Groups));
        a.lower_frame.remove(PanelKind::Properties);
        call(&mut a, "panel_show", json!({"panel": "properties"})).unwrap();
        assert_eq!(a.lower_frame.front(), Some(PanelKind::Properties));
        call(&mut a, "panel_show", json!({"panel": "mesh ops"})).unwrap();
        assert_eq!(a.upper_frame.front(), Some(PanelKind::MeshStack));
        // The bridge name before 2026-11-23 still opens the Mesh Stack.
        a.upper_frame.show_page(0);
        call(&mut a, "panel_show", json!({"panel": "mesh_ops"})).unwrap();
        assert_eq!(a.upper_frame.front(), Some(PanelKind::MeshStack));
        a.upper_frame.show_page(0);
        call(&mut a, "panel_show", json!({"panel": "mesh_stack"})).unwrap();
        assert_eq!(a.upper_frame.front(), Some(PanelKind::MeshStack));
        a.upper_frame.show_page(0);
        call(&mut a, "panel_show", json!({"panel": "Mesh Stack"})).unwrap();
        assert_eq!(a.upper_frame.front(), Some(PanelKind::MeshStack));
        assert!(call(&mut a, "panel_show", json!({"panel": "no such panel"})).is_err());
        // Layout only: runs with a dialog open, opens none, edits nothing.
        a.show_preferences = true;
        call(&mut a, "panel_show", json!({"panel": "shader_tree"})).unwrap();
        a.show_preferences = false;
        assert!(a.editor.history.undo.is_empty() && !a.dirty() && !a.has_modal());
    }

    #[test]
    fn release_builds_start_with_the_bridge_off() {
        // This test binary is a dev build; packaged builds flip the default.
        assert!(!packaged_release());
        if std::env::var_os("OPERANDI_MCP").is_none() {
            assert!(on_at_launch());
        }
    }

    /// command_run scene.saveAs exports from the scene on screen; the
    /// document keeps its file and saved state, and USD reaches the
    /// exporter the app registers.
    #[test]
    fn command_run_save_as_exports_without_touching_the_document() {
        let mut a = app();
        call(&mut a, "tool_run", json!({"tool": "prim.cube"})).unwrap();
        let dir = a.environment.paths().data_root().to_path_buf();
        let doc = dir.join("mcp-export-doc.opscene");
        call(&mut a, "scene_save", json!({"path": doc})).unwrap();
        assert!(!a.dirty());
        let obj = dir.join("mcp-export.obj");
        let _ = std::fs::remove_file(&obj);
        call(
            &mut a,
            "command_run",
            json!({"command": "scene.saveAs", "args": {"filename": obj}}),
        )
        .unwrap();
        assert!(obj.exists());
        assert_eq!(a.path.as_deref(), Some(doc.as_path()));
        assert!(!a.dirty());
        // USD goes to the registered exporter (it may say this build lacks
        // OpenUSD), never the headless "needs the running app" refusal.
        let usd = dir.join("mcp-export.usda");
        let outcome = call(
            &mut a,
            "command_run",
            json!({"command": "scene.saveAs", "args": {"filename": usd}}),
        );
        if let Err(e) = outcome {
            assert!(!e.contains("--attach"), "{e}");
        }
        assert_eq!(a.path.as_deref(), Some(doc.as_path()));
        assert!(!a.dirty());
    }
}
