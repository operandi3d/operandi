//! AppKit adapter. Compiled out on Windows/Linux, which retain in-window menus.
//! Native objects and their unretained target/delegate live on the main thread.
#[cfg(target_os = "macos")]
mod mac {
    use crate::app_menu::{Action, Entry, Item, Menu};
    use objc2::rc::Retained;
    use objc2::runtime::ProtocolObject;
    use objc2::{ClassType, DeclaredClass, declare_class, msg_send_id, mutability, sel};
    use objc2_app_kit::{
        NSApplication, NSControlStateValueOff, NSControlStateValueOn, NSMenu, NSMenuDelegate,
        NSMenuItem,
    };
    use objc2_foundation::{MainThreadMarker, NSObject, NSObjectProtocol, NSString};
    use std::{
        cell::RefCell,
        collections::HashMap,
        sync::{
            Mutex, OnceLock,
            atomic::{AtomicUsize, Ordering},
        },
    };
    static CONTEXT: OnceLock<eframe::egui::Context> = OnceLock::new();
    static ACTIONS: Mutex<Vec<usize>> = Mutex::new(Vec::new());
    static TRACKING: AtomicUsize = AtomicUsize::new(0);
    fn repaint() {
        if let Some(ctx) = CONTEXT.get() {
            ctx.request_repaint();
        }
    }
    declare_class!(
        struct MenuTarget;
        // SAFETY: NSObject has no extra subclass invariants. This target is
        // allocated, retained and called exclusively on AppKit's main thread.
        unsafe impl ClassType for MenuTarget {
            type Super = NSObject;
            type Mutability = mutability::MainThreadOnly;
            const NAME: &'static str = "OperandiRustMenuTarget";
        }
        impl DeclaredClass for MenuTarget {
            type Ivars = ();
        }
        unsafe impl NSObjectProtocol for MenuTarget {}
        unsafe impl MenuTarget {
            #[method(operandiMenuAction:)]
            fn action(&self, sender: &NSMenuItem) {
                // SAFETY: our NSMenuItems invoke this selector with themselves;
                // tag is a checked, nonnegative index, never a Rust pointer.
                if let Ok(id) = usize::try_from(unsafe { sender.tag() })
                    && let Ok(mut actions) = ACTIONS.lock()
                    && actions.len() < 64
                {
                    actions.push(id);
                }
                repaint();
            }
        }
        unsafe impl NSMenuDelegate for MenuTarget {
            #[method(menuWillOpen:)]
            unsafe fn menu_will_open(&self, _menu: &NSMenu) {
                TRACKING.fetch_add(1, Ordering::AcqRel);
                repaint();
            }
            #[method(menuDidClose:)]
            unsafe fn menu_did_close(&self, _menu: &NSMenu) {
                let _ = TRACKING.try_update(Ordering::AcqRel, Ordering::Acquire, |n| {
                    Some(n.saturating_sub(1))
                });
                repaint();
            }
        }
    );
    struct State {
        // NSMenuItem targets / NSMenu delegates are unretained: keep the target
        // alive at least as long as NSApplication's retained menu tree.
        _target: Retained<MenuTarget>,
        leaves: Vec<(Retained<NSMenuItem>, Action)>,
        model: Vec<Menu>,
        /// The menu bar, to rebuild the model's menus in when their shape
        /// changes (menus as forms: live lists, edited forms).
        main: Retained<NSMenu>,
        /// Leaves of the app menu (About, Preferences…), which a rebuild keeps.
        app_leaves: usize,
        /// What each app-menu command entry (by tag) was last set to:
        /// enabled, tooltip, key equivalent.
        app_items: HashMap<isize, (bool, Option<String>, String)>,
    }
    thread_local! {static STATE:RefCell<Option<State>>=const{RefCell::new(None)};}
    pub fn installed() -> bool {
        STATE.with(|state| state.borrow().is_some())
    }
    pub fn tracking() -> bool {
        TRACKING.load(Ordering::Acquire) > 0
    }
    fn apply(native: &NSMenuItem, item: &Item) {
        // SAFETY: main-thread retained NSMenuItem, bounded owned NSString and
        // documented BOOL/NSInteger AppKit setters; no borrowed data escapes.
        unsafe {
            native.setTitle(&NSString::from_str(&item.title));
            native.setEnabled(item.enabled);
            // A greyed entry says why on hover (unavailable.rs).
            native.setToolTip(item.why.as_deref().map(NSString::from_str).as_deref());
            native.setState(if item.checked {
                NSControlStateValueOn
            } else {
                NSControlStateValueOff
            });
            native.setKeyEquivalent(&NSString::from_str(&item.command_key));
        }
    }
    fn build(
        mtm: MainThreadMarker,
        model: &Menu,
        target: &MenuTarget,
        leaves: &mut Vec<(Retained<NSMenuItem>, Action)>,
    ) -> Retained<NSMenu> {
        // SAFETY: AppKit APIs below are confined to its main thread. Each menu
        // retains its items/submenus. STATE retains the shared target/delegate.
        unsafe {
            let menu = NSMenu::initWithTitle(mtm.alloc(), &NSString::from_str(&model.title));
            menu.setAutoenablesItems(false);
            menu.setDelegate(Some(ProtocolObject::from_ref(target)));
            for entry in &model.entries {
                match entry {
                    Entry::Separator => menu.addItem(&NSMenuItem::separatorItem(mtm)),
                    Entry::Submenu(sub) => {
                        let item = NSMenuItem::initWithTitle_action_keyEquivalent(
                            mtm.alloc(),
                            &NSString::from_str(&sub.title),
                            None,
                            &NSString::from_str(""),
                        );
                        item.setSubmenu(Some(&build(mtm, sub, target, leaves)));
                        menu.addItem(&item);
                    }
                    Entry::Item(model) => {
                        let item = NSMenuItem::initWithTitle_action_keyEquivalent(
                            mtm.alloc(),
                            &NSString::from_str(&model.title),
                            Some(sel!(operandiMenuAction:)),
                            &NSString::from_str(&model.command_key),
                        );
                        item.setTarget(Some(target));
                        item.setTag(leaves.len() as isize);
                        apply(&item, model);
                        menu.addItem(&item);
                        leaves.push((item, model.action));
                    }
                }
            }
            menu
        }
    }
    /// Adds the model's menus to the bar after the app menu.
    fn add_menus(
        mtm: MainThreadMarker,
        main: &NSMenu,
        model: &[Menu],
        target: &MenuTarget,
        leaves: &mut Vec<(Retained<NSMenuItem>, Action)>,
    ) {
        // SAFETY: main-thread AppKit calls on the retained menu bar; each
        // item retains its submenu.
        unsafe {
            for menu in model {
                let item = NSMenuItem::initWithTitle_action_keyEquivalent(
                    mtm.alloc(),
                    &NSString::from_str(&menu.title),
                    None,
                    &NSString::from_str(""),
                );
                item.setSubmenu(Some(&build(mtm, menu, target, leaves)));
                main.addItem(&item);
            }
        }
    }
    pub fn install(ctx: eframe::egui::Context, model: Vec<Menu>) {
        let Some(mtm) = MainThreadMarker::new() else {
            return;
        };
        if installed() {
            return;
        }
        let _ = CONTEXT.set(ctx);
        // SAFETY: NSObject init on a newly allocated main-thread target; the
        // returned +1 object is retained by STATE before this function returns.
        let target: Retained<MenuTarget> =
            unsafe { msg_send_id![super(mtm.alloc::<MenuTarget>().set_ivars(())), init] };
        let app = NSApplication::sharedApplication(mtm);
        let mut leaves = Vec::new();
        // SAFETY: only the running application's owned menu tree is modified.
        // Preserve winit's original application submenu (About/Services/Hide/
        // Quit), including the existing graceful-termination adapter.
        unsafe {
            let main = NSMenu::new(mtm);
            let application_item = app.mainMenu().and_then(|old| {
                let item = old.itemAtIndex(0)?;
                old.removeItemAtIndex(0);
                Some(item)
            });
            if let Some(item) = application_item {
                if let Some(menu) = item.submenu() {
                    let mut about_at = None;
                    for entry in menu.itemArray().iter() {
                        let title = match entry.action() {
                            Some(action) if action == sel!(orderFrontStandardAboutPanel:) => {
                                // Our own About window (about.rs) replaces
                                // AppKit's standard panel. Its leaf sits
                                // before the model's, which `sync` allows.
                                entry.setAction(Some(sel!(operandiMenuAction:)));
                                entry.setTarget(Some(&target));
                                entry.setTag(leaves.len() as isize);
                                leaves.push((entry.retain(), Action::About));
                                about_at = Some(menu.indexOfItem(entry));
                                Some("About Operandi")
                            }
                            Some(action) if action == sel!(hide:) => Some("Hide Operandi"),
                            Some(action) if action == sel!(terminate:) => Some("Quit Operandi"),
                            _ => None,
                        };
                        if let Some(title) = title {
                            entry.setTitle(&NSString::from_str(title));
                        }
                    }
                    // #67: Preferences… in the Mac's usual place, About,
                    // a separator, then Preferences (winit's own separator
                    // follows, before Services). It runs the same line as
                    // Edit > Preferences…, which stays. `sync` keeps its
                    // greying and key equivalent current.
                    let at = about_at.map_or(0, |at| at + 1);
                    menu.insertItem_atIndex(&NSMenuItem::separatorItem(mtm), at);
                    let preferences = NSMenuItem::initWithTitle_action_keyEquivalent(
                        mtm.alloc(),
                        &NSString::from_str("Preferences…"),
                        Some(sel!(operandiMenuAction:)),
                        &NSString::from_str(""),
                    );
                    preferences.setTarget(Some(&target));
                    preferences.setTag(leaves.len() as isize);
                    menu.insertItem_atIndex(&preferences, at + 1);
                    leaves.push((
                        preferences,
                        crate::app_menu::line_action(crate::app_menu::PREFERENCES_LINE),
                    ));
                }
                main.addItem(&item);
            } else {
                let fallback = Menu {
                    id: String::new(),
                    title: "Operandi".into(),
                    entries: vec![Entry::Item(Item {
                        action: Action::Close,
                        title: "Quit Operandi".into(),
                        enabled: true,
                        why: None,
                        checked: false,
                        command_key: "q".into(),
                    })],
                };
                let item = NSMenuItem::new(mtm);
                item.setSubmenu(Some(&build(mtm, &fallback, &target, &mut leaves)));
                main.addItem(&item);
            }
            let app_leaves = leaves.len();
            add_menus(mtm, &main, &model, &target, &mut leaves);
            app.setMainMenu(Some(&main));
            STATE.with(|state| {
                *state.borrow_mut() = Some(State {
                    _target: target,
                    leaves,
                    model,
                    main,
                    app_leaves,
                    app_items: HashMap::new(),
                })
            });
        }
    }
    /// Brings the app menu's command entries (Preferences…) up to date:
    /// greyed when the same line is greyed in the model's menus, with
    /// `preferences_key` as its key equivalent.
    fn sync_app_leaves(state: &mut State, model: &[Menu], preferences_key: &str) {
        for (native, action) in &state.leaves[..state.app_leaves] {
            if !matches!(action, Action::Line(_)) {
                continue;
            }
            let item = model
                .iter()
                .flat_map(Menu::leaves)
                .find(|i| i.action == *action);
            let enabled = item.is_some_and(|i| i.enabled);
            let why = item.and_then(|i| i.why.clone());
            let wanted = (enabled, why, preferences_key.to_owned());
            // SAFETY: main-thread retained NSMenuItem and owned NSStrings,
            // as in `apply`; the tag is a plain NSInteger.
            let tag = unsafe { native.tag() };
            if state.app_items.get(&tag) == Some(&wanted) {
                continue;
            }
            // SAFETY: as above.
            unsafe {
                native.setEnabled(wanted.0);
                native.setToolTip(wanted.1.as_deref().map(NSString::from_str).as_deref());
                native.setKeyEquivalent(&NSString::from_str(&wanted.2));
            }
            state.app_items.insert(tag, wanted);
        }
    }
    pub fn sync(model: Vec<Menu>, preferences_key: &str) {
        if tracking() {
            return;
        }
        let Some(mtm) = MainThreadMarker::new() else {
            return;
        };
        STATE.with(|state| {
            let mut state = state.borrow_mut();
            let Some(state) = state.as_mut() else {
                return;
            };
            sync_app_leaves(state, &model, preferences_key);
            if state.model == model {
                return;
            }
            // Same shape (menus, submenu titles, separators, each entry's
            // action): only titles, ticks, greying and keys change, so the
            // native items are updated where they are. IDs stay stable.
            if crate::app_menu::same_shape(&state.model, &model) {
                let items: Vec<_> = model.iter().flat_map(Menu::leaves).collect();
                let offset = state.app_leaves;
                for ((native, _), item) in state.leaves[offset..].iter().zip(items) {
                    apply(native, item);
                }
                state.model = model;
                return;
            }
            // The shape changed (a live list grew, a form was edited):
            // rebuild the model's menus, never while a menu is open.
            // SAFETY: main-thread AppKit calls on the retained menu bar;
            // the app menu (index 0) and its leaves stay.
            unsafe {
                while state.main.numberOfItems() > 1 {
                    state.main.removeItemAtIndex(state.main.numberOfItems() - 1);
                }
            }
            state.leaves.truncate(state.app_leaves);
            add_menus(mtm, &state.main, &model, &state._target, &mut state.leaves);
            state.model = model;
        });
    }
    pub fn take_actions() -> Vec<Action> {
        let ids = ACTIONS
            .lock()
            .map(|mut actions| std::mem::take(&mut *actions))
            .unwrap_or_default();
        STATE.with(|state| {
            let state = state.borrow();
            let Some(state) = state.as_ref() else {
                return Vec::new();
            };
            ids.into_iter()
                .filter_map(|id| state.leaves.get(id).map(|(_, action)| *action))
                .collect()
        })
    }
}
#[cfg(target_os = "macos")]
pub use mac::{install, installed, sync, take_actions, tracking};
#[cfg(not(target_os = "macos"))]
pub fn installed() -> bool {
    false
}
#[cfg(not(target_os = "macos"))]
pub fn tracking() -> bool {
    false
}
#[cfg(not(target_os = "macos"))]
pub fn install(_: eframe::egui::Context, _: Vec<crate::app_menu::Menu>) {}
#[cfg(not(target_os = "macos"))]
pub fn sync(_: Vec<crate::app_menu::Menu>, _: &str) {}
#[cfg(not(target_os = "macos"))]
pub fn take_actions() -> Vec<crate::app_menu::Action> {
    vec![]
}
