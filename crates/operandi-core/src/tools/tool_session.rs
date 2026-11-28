//! The running tool session an `Editor` owns (untangle E3): the open
//! tool and whether it is armed, its tweak history and the tool Redo can
//! open again, the other task slots (numeric sessions, the Pen draft,
//! Solid Sketch), and the bookkeeping of the command and gesture running
//! now. Callers say `editor.session.…`.
//!
//! Which of these follow a scene tab and which stay with the app is
//! sorted field by field in document_state.rs.
use crate::Tool;

#[derive(Clone, Default, PartialEq)]
pub struct ToolSession {
    /// The open tool: its base scene, parameters and preview (editor.rs).
    pub live: Option<crate::Live>,
    /// CS-01: the live tool is armed: picked, values editable, nothing
    /// drawn and the mesh as it was, until the first click or Apply.
    pub armed: Option<Tool>,
    /// N25: Edge Extend was started by a click, so its rows exist even at
    /// zero offset (click_start.rs).
    pub extend_from_click: bool,
    /// An open tool preset row (Soft Move, Twist, Flex, Flare ...): its
    /// live tool is the row's tool, and the pipe slots it took go back on
    /// drop (tool_preset.rs).
    pub preset: Option<crate::PresetSession>,
    /// The active tool's own tweak history (`mid_tool_undo.rs`).
    pub tweaks: crate::ToolTweaks,
    /// Set by the app while a drag or scrub is held, so its changes are one
    /// tweak; `end_tweak` clears it.
    pub tweak_held: bool,
    /// The tool Undo just closed, which Redo opens again.
    pub tool_reopen: Option<crate::ToolReopen>,
    /// The transform drag committed last, for Shift-continue (HM-02) and
    /// middle-drag repeat (HM-04). Not document state.
    pub last_transform_step: Option<Box<crate::transform_repeat::TransformStep>>,
    /// Mirror partners added during the open selection gesture (lasso,
    /// rectangle, paint), moved after the swept elements when it ends
    /// (spec v3.1 section 11: swept elements first, then partners).
    pub gesture_partners: Vec<(u64, crate::Component)>,
    /// How deep inside `dispatch` the editor is: arming and the armed
    /// rules act only on top-level commands (click_start.rs).
    #[doc(hidden)]
    pub dispatch_depth: u32,
    // The other task slots; at most one task is open (active_task.rs).
    pub bridge: Option<crate::NumericBridge>,
    pub spike: Option<crate::NumericSpike>,
    /// MP-T11 Polygon Reduction session; `None` when the tool is off.
    pub reduction: Option<crate::NumericReduction>,
    pub ellipsoid: Option<crate::NumericEllipsoid>,
    /// Torus, Capsule, Arc or Tube numeric session (NPR), and each one's
    /// last settings.
    pub shape: Option<crate::NumericShape>,
    pub cone: Option<crate::NumericCone>,
    pub pen_draft: Option<crate::PenDraft>,
    /// The open Solid Sketch session (solid_sketch_editor.rs).
    pub solid_sketch: Option<crate::SketchSession>,
}
