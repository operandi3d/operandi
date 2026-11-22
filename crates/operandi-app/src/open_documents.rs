//! Scene files the system asks us to open: a double-click in Finder, a
//! drop on the Dock or app icon, "Open With", or a path on the command line.
//!
//! macOS sends these as an "open documents" Apple Event, both when the event
//! starts the app and while it runs. winit doesn't pass them on, so we take
//! the event ourselves: `install_before_launch` (called from `main`, before
//! eframe starts the event loop) registers for NSApplication's
//! will-finish-launching notification and installs our Apple Event handler
//! there, after AppKit has installed its own defaults and before the launch
//! event is delivered. That is the moment Apple's documentation names for
//! installing Apple Event handlers.
//!
//! The handler only queues paths. The app drains the queue each frame and
//! opens one file at a time through the same path as File > Open and a drop
//! on the window (`Operandi::request_open_path`), waiting while a dialog is
//! open so no request is lost.
//!
//! Windows and Linux pass the file as a command-line argument instead
//! (`from_args`); see docs/FILE_ASSOCIATION.md for what registering the type
//! there would take.

use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

static PENDING: Mutex<Vec<PathBuf>> = Mutex::new(Vec::new());
static CONTEXT: OnceLock<eframe::egui::Context> = OnceLock::new();

/// Queues files to open and wakes the app if it is idle.
fn push(paths: impl IntoIterator<Item = PathBuf>) {
    if let Ok(mut pending) = PENDING.lock() {
        pending.extend(paths);
    }
    if let Some(ctx) = CONTEXT.get() {
        ctx.request_repaint();
    }
}

/// Files the system asked us to open since the last call.
pub fn take() -> Vec<PathBuf> {
    PENDING
        .lock()
        .map(|mut pending| std::mem::take(&mut *pending))
        .unwrap_or_default()
}

/// Lets queued opens wake an idle app. Called once the egui context exists.
pub fn connect(ctx: eframe::egui::Context) {
    let _ = CONTEXT.set(ctx);
    // A safety net: if the launch notification was somehow missed, the
    // handler still takes over for opens while the app runs.
    #[cfg(target_os = "macos")]
    mac::install_handler();
}

/// Called from `main` before the event loop starts. Queues files named on
/// the command line, and on macOS gets ready for Finder's open events.
pub fn install_before_launch() {
    push(from_args(std::env::args_os().skip(1)));
    #[cfg(target_os = "macos")]
    mac::observe_launch();
}

/// The command-line arguments that name files to open. Options (anything
/// starting with `-`, such as the `-psn_…` older macOS versions add) and
/// paths that aren't existing files are skipped.
pub fn from_args(args: impl IntoIterator<Item = std::ffi::OsString>) -> Vec<PathBuf> {
    args.into_iter()
        .filter(|arg| !arg.to_string_lossy().starts_with('-'))
        .map(PathBuf::from)
        .filter(|path| path.is_file())
        .collect()
}

/// Files waiting to open, one at a time.
#[derive(Default)]
pub struct Queue {
    waiting: VecDeque<PathBuf>,
}

impl Queue {
    pub fn extend(&mut self, paths: Vec<PathBuf>) {
        self.waiting.extend(paths);
    }
    /// The next file to open, or nothing while the app is busy (a dialog,
    /// an import or an earlier open still on screen). One per frame, so each
    /// open finishes before the next starts.
    pub fn next(&mut self, busy: bool) -> Option<PathBuf> {
        if busy {
            return None;
        }
        self.waiting.pop_front()
    }
}

#[cfg(target_os = "macos")]
mod mac {
    use objc2::rc::Retained;
    use objc2::{ClassType, DeclaredClass, declare_class, msg_send, msg_send_id, mutability, sel};
    use objc2_app_kit::NSApplicationWillFinishLaunchingNotification;
    use objc2_foundation::{
        MainThreadMarker, NSAppleEventDescriptor, NSAppleEventManager, NSNotification,
        NSNotificationCenter, NSObject, NSObjectProtocol,
    };
    use std::cell::OnceCell;
    use std::path::PathBuf;

    /// Four-character codes from the macOS SDK's AppleEvents.h.
    const CORE_EVENT_CLASS: u32 = u32::from_be_bytes(*b"aevt");
    const OPEN_DOCUMENTS: u32 = u32::from_be_bytes(*b"odoc");
    const DIRECT_OBJECT: u32 = u32::from_be_bytes(*b"----");

    declare_class!(
        struct OpenHandler;
        // SAFETY: NSObject has no extra subclass invariants. The handler is
        // created, registered and called only on AppKit's main thread.
        unsafe impl ClassType for OpenHandler {
            type Super = NSObject;
            type Mutability = mutability::MainThreadOnly;
            const NAME: &'static str = "OperandiOpenDocumentsHandler";
        }
        impl DeclaredClass for OpenHandler {
            type Ivars = ();
        }
        unsafe impl NSObjectProtocol for OpenHandler {}
        unsafe impl OpenHandler {
            #[method(applicationWillFinishLaunching:)]
            fn will_finish_launching(&self, _note: &NSNotification) {
                install_handler();
            }
            #[method(handleOpenDocuments:withReplyEvent:)]
            fn open_documents(&self, event: &NSAppleEventDescriptor, _reply: &NSAppleEventDescriptor) {
                super::push(paths(event));
            }
        }
    );

    thread_local! {
        // The notification centre and the event manager don't retain their
        // targets, so the handler lives for the whole process.
        static HANDLER: OnceCell<Retained<OpenHandler>> = const { OnceCell::new() };
    }

    fn handler() -> Option<Retained<OpenHandler>> {
        let mtm = MainThreadMarker::new()?;
        Some(HANDLER.with(|cell| {
            cell.get_or_init(|| {
                // SAFETY: plain NSObject init on the main thread.
                unsafe { msg_send_id![mtm.alloc::<OpenHandler>(), init] }
            })
            .clone()
        }))
    }

    pub fn observe_launch() {
        let Some(handler) = handler() else { return };
        // SAFETY: main thread; the observer stays alive (HANDLER) and its
        // selector takes one NSNotification, as the centre calls it.
        unsafe {
            NSNotificationCenter::defaultCenter().addObserver_selector_name_object(
                &handler,
                sel!(applicationWillFinishLaunching:),
                Some(NSApplicationWillFinishLaunchingNotification),
                None,
            );
        }
    }

    pub fn install_handler() {
        let Some(handler) = handler() else { return };
        // SAFETY: main thread; -[NSAppleEventManager
        // setEventHandler:andSelector:forEventClass:andEventID:] takes an
        // object, a selector of the documented (event, reply) form and two
        // FourCharCodes. Installing again replaces our own earlier entry.
        unsafe {
            let manager = NSAppleEventManager::sharedAppleEventManager();
            let _: () = msg_send![
                &manager,
                setEventHandler: &*handler,
                andSelector: sel!(handleOpenDocuments:withReplyEvent:),
                forEventClass: CORE_EVENT_CLASS,
                andEventID: OPEN_DOCUMENTS
            ];
        }
    }

    /// The file paths in an open-documents event: a list of file
    /// references, or (rarely) a single one.
    fn paths(event: &NSAppleEventDescriptor) -> Vec<PathBuf> {
        // SAFETY: documented NSAppleEventDescriptor accessors on a live
        // event; list indices are 1-based and bounded by numberOfItems.
        unsafe {
            let direct: Option<Retained<NSAppleEventDescriptor>> =
                msg_send_id![event, paramDescriptorForKeyword: DIRECT_OBJECT];
            let Some(direct) = direct else {
                return Vec::new();
            };
            let count = direct.numberOfItems();
            let items: Vec<Retained<NSAppleEventDescriptor>> = if count > 0 {
                (1..=count)
                    .filter_map(|i| direct.descriptorAtIndex(i))
                    .collect()
            } else {
                vec![direct]
            };
            items
                .iter()
                .filter_map(|item| item.fileURLValue())
                .filter_map(|url| url.path())
                .map(|path| PathBuf::from(path.to_string()))
                .collect()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn queue_waits_while_busy_then_opens_in_order() {
        let mut queue = Queue::default();
        queue.extend(vec!["a.opscene".into(), "b.opscene".into()]);
        assert_eq!(queue.next(true), None, "a dialog is open: keep waiting");
        assert_eq!(queue.next(false), Some(PathBuf::from("a.opscene")));
        assert_eq!(queue.next(true), None);
        assert_eq!(queue.next(false), Some(PathBuf::from("b.opscene")));
        assert_eq!(queue.next(false), None);
    }

    #[test]
    fn command_line_keeps_existing_files_only() {
        let dir = std::env::temp_dir().join(format!("operandi-open-args-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let scene = dir.join("cube.opscene");
        std::fs::write(&scene, b"x").unwrap();
        let args = [
            "-psn_0_12345".into(),
            scene.clone().into_os_string(),
            dir.join("missing.opscene").into_os_string(),
            dir.clone().into_os_string(),
        ];
        assert_eq!(from_args(args), vec![scene]);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn pushed_paths_are_taken_once() {
        // Shared static: take whatever earlier code queued first.
        let _ = take();
        push([PathBuf::from("x.opscene")]);
        assert_eq!(take(), vec![PathBuf::from("x.opscene")]);
        assert!(take().is_empty());
    }
}

#[cfg(test)]
mod bundle_tests {
    /// The bundle declares the type the app opens, and the bundle script
    /// puts the declaration and its icon into every macOS app.
    #[test]
    fn macos_bundle_declares_opscene() {
        let plist = include_str!("../../../assets/macos/opscene-document-type.plist");
        for needle in [
            "<string>opscene</string>",
            "<string>org.operandi3d.opscene</string>",
            "<string>OperandiScene.icns</string>",
            "<string>Owner</string>",
            "UTExportedTypeDeclarations",
        ] {
            assert!(plist.contains(needle), "document type plist lacks {needle}");
        }
        let script = include_str!("../../../scripts/macos-app-bundle.sh");
        assert!(script.contains("Merge assets/macos/opscene-document-type.plist"));
        assert!(script.contains("OperandiScene.icns"));
        assert!(script.contains("assets/branding/operandi-document-icon.png"));
    }
}
