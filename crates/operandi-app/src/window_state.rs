//! The main window's size, place and maximised/fullscreen state, saved on
//! exit and restored on launch (main-window.json in the data folder).
//!
//! The convention keeps floating windows' places across restarts (UIC-73);
//! the main window follows the same rule. [OP] Where the saved place is no
//! longer on any screen (a monitor was unplugged, a resolution changed) the
//! window opens at the system's default place instead.
use eframe::egui::{self, Pos2, Rect, Vec2, ViewportInfo};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Inner size of a first launch, and of a reset.
pub const DEFAULT_SIZE: [f32; 2] = [1440., 900.];
/// The smallest the main window may be.
pub const MIN_SIZE: [f32; 2] = [640., 480.];
/// How much of the window's top edge (its title bar) must be on a screen for
/// the saved place to count as visible: a strip this tall...
const TITLE_STRIP: f32 = 28.;
/// ...overlapping a screen's usable area by at least this much.
const MIN_VISIBLE: Vec2 = Vec2::new(80., 8.);
/// Sizes past this are corrupt files, not windows.
const MAX_EXTENT: f32 = 100_000.;

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct MainWindow {
    pub version: u8,
    /// Inner size in points while neither maximised nor fullscreen, so
    /// leaving those states returns to it.
    pub size: [f32; 2],
    /// Outer top-left in points (same frame), once known.
    pub position: Option<[f32; 2]>,
    pub maximized: bool,
    pub fullscreen: bool,
    /// Physical pixels per point when saved. Windows lists screens in
    /// physical pixels; this converts them to points.
    pub pixels_per_point: f32,
}

impl Default for MainWindow {
    fn default() -> Self {
        Self {
            version: 1,
            size: DEFAULT_SIZE,
            position: None,
            maximized: false,
            fullscreen: false,
            pixels_per_point: 1.,
        }
    }
}

impl MainWindow {
    fn valid(&self) -> bool {
        let size_ok = self
            .size
            .iter()
            .zip(MIN_SIZE)
            .all(|(v, min)| v.is_finite() && *v >= min && *v <= MAX_EXTENT);
        let position_ok = self
            .position
            .is_none_or(|p| p.iter().all(|v| v.is_finite() && v.abs() <= MAX_EXTENT));
        self.version == 1
            && size_ok
            && position_ok
            && self.pixels_per_point.is_finite()
            && (0.25..=8.).contains(&self.pixels_per_point)
    }

    /// The saved state, or the default when the file is missing or damaged.
    pub fn load(path: &Path) -> Self {
        std::fs::metadata(path)
            .ok()
            .filter(|m| m.len() <= 4096)
            .and_then(|_| std::fs::read(path).ok())
            .and_then(|bytes| serde_json::from_slice::<Self>(&bytes).ok())
            .filter(Self::valid)
            .unwrap_or_default()
    }

    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        let bytes = serde_json::to_vec_pretty(self).map_err(std::io::Error::other)?;
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let temp = path.with_extension(format!("json.tmp-{}", std::process::id()));
        crate::resource_write::write_bytes(path, &temp, &bytes)
    }

    /// The state to open with, given the screens' usable areas in points
    /// (`None` when this platform can't list them; the saved place is then
    /// trusted). A place whose title bar is on no screen is dropped, so the
    /// system places the window, and the size shrinks to fit the screen it
    /// will open on.
    pub fn placement(&self, screens: Option<&[Rect]>) -> Self {
        let mut out = *self;
        let Some(screens) = screens.filter(|s| !s.is_empty()) else {
            return out;
        };
        let size = Vec2::from(self.size);
        let home = self.position.and_then(|p| {
            let strip = Rect::from_min_size(Pos2::from(p), Vec2::new(size.x, TITLE_STRIP));
            screens.iter().copied().find(|screen| {
                let overlap = strip.intersect(*screen);
                overlap.width() >= MIN_VISIBLE.x && overlap.height() >= MIN_VISIBLE.y
            })
        });
        let fit = match home {
            Some(screen) => screen.size(),
            None => {
                out.position = None;
                // Without a place the system uses the main screen (first).
                screens[0].size()
            }
        };
        out.size = [
            self.size[0].min(fit.x).max(MIN_SIZE[0]),
            self.size[1].min(fit.y).max(MIN_SIZE[1]),
        ];
        out
    }

    /// The main window's builder settings for this state.
    pub fn apply(&self, builder: egui::ViewportBuilder) -> egui::ViewportBuilder {
        let mut builder = builder
            .with_inner_size(self.size)
            .with_min_inner_size(MIN_SIZE)
            .with_maximized(self.maximized)
            .with_fullscreen(self.fullscreen);
        if let Some(p) = self.position {
            builder = builder.with_position(p);
        }
        builder
    }
}

/// Follows the main window while the app runs; its state is saved on exit.
#[derive(Debug, Default)]
pub struct Tracker {
    pub state: MainWindow,
    /// Set once the window has reported itself. Nothing is saved before
    /// that (tests and headless runs never write the file).
    pub seen: bool,
}

impl Tracker {
    pub fn new(state: MainWindow) -> Self {
        Self { state, seen: false }
    }

    /// Note the root viewport's latest report. Size and place are only taken
    /// from the normal (not maximised, fullscreen or minimised) window.
    pub fn observe(&mut self, info: &ViewportInfo) {
        if info.inner_rect.is_none() && info.outer_rect.is_none() {
            return;
        }
        self.seen = true;
        if info.minimized == Some(true) {
            return;
        }
        if let Some(fullscreen) = info.fullscreen {
            self.state.fullscreen = fullscreen;
        }
        if let Some(maximized) = info.maximized {
            self.state.maximized = maximized;
        }
        if self.state.fullscreen || self.state.maximized {
            return;
        }
        if let Some(r) = info.inner_rect {
            let size = [r.width(), r.height()];
            if size
                .iter()
                .zip(MIN_SIZE)
                .all(|(v, min)| v.is_finite() && *v >= min)
            {
                self.state.size = size;
            }
        }
        if let Some(r) = info.outer_rect
            && r.min.x.is_finite()
            && r.min.y.is_finite()
        {
            self.state.position = Some([r.min.x, r.min.y]);
        }
        if let Some(ppp) = info
            .native_pixels_per_point
            .filter(|v| v.is_finite() && *v > 0.)
        {
            self.state.pixels_per_point = ppp;
        }
    }

    /// Write the state when the window has been seen.
    pub fn save(&self, path: &Path) {
        if !self.seen {
            return;
        }
        if let Err(e) = self.state.save(path) {
            crate::log_ring::log_line!("Couldn't save the window place to {}: {e}", path.display());
        }
    }
}

/// A floating window pulled fully onto one screen (VST-31): the screen
/// holding `pointer` (else the one holding most of the window, else the
/// first). `position` is the window's top-left and `size` its inner size;
/// the title bar above the content counts as part of the window. The size
/// shrinks to fit the screen but never below `min`, and the window moves
/// (never grows) to sit inside the screen. With no screens listed the
/// window is left as asked.
pub fn place_on_screen(
    position: [f32; 2],
    size: [f32; 2],
    min: [f32; 2],
    screens: &[Rect],
    pointer: Option<Pos2>,
) -> ([f32; 2], [f32; 2]) {
    let size = [size[0].max(min[0]), size[1].max(min[1])];
    let window = Rect::from_min_size(
        Pos2::from(position),
        Vec2::new(size[0], size[1] + TITLE_STRIP),
    );
    let screen = pointer
        .and_then(|p| screens.iter().find(|s| s.contains(p)))
        .or_else(|| {
            screens
                .iter()
                .filter(|s| s.intersects(window))
                .max_by(|a, b| {
                    let area = |s: &Rect| s.intersect(window).area();
                    area(a).total_cmp(&area(b))
                })
        })
        .or(screens.first());
    let Some(screen) = screen else {
        return (position, size);
    };
    let size = [
        size[0].min(screen.width()).max(min[0]),
        size[1].min(screen.height() - TITLE_STRIP).max(min[1]),
    ];
    let outer = Vec2::new(size[0], size[1] + TITLE_STRIP);
    let x = position[0].min(screen.right() - outer.x).max(screen.left());
    let y = position[1].min(screen.bottom() - outer.y).max(screen.top());
    ([x, y], size)
}

/// Where a saved floating window reopens (VST-37): its saved place while
/// its title bar is on a connected screen; otherwise on the main (first)
/// screen near the main window's top-left (`main_window`), or the system's
/// place when the main window's isn't known. With no screens listed the
/// saved place is trusted.
pub fn reopen_place(
    saved: Option<[f32; 2]>,
    size: [f32; 2],
    screens: Option<&[Rect]>,
    main_window: Option<[f32; 2]>,
) -> Option<[f32; 2]> {
    /// How far in from the main window's corner a rescued window opens.
    const NEAR: f32 = 60.;
    let saved = saved?;
    let Some(screens) = screens.filter(|s| !s.is_empty()) else {
        return Some(saved);
    };
    let strip = Rect::from_min_size(Pos2::from(saved), Vec2::new(size[0], TITLE_STRIP));
    let visible = screens.iter().any(|screen| {
        let overlap = strip.intersect(*screen);
        overlap.width() >= MIN_VISIBLE.x && overlap.height() >= MIN_VISIBLE.y
    });
    if visible {
        return Some(saved);
    }
    let near = main_window.map(|[x, y]| [x + NEAR, y + NEAR])?;
    Some(place_on_screen(near, size, [0., 0.], &screens[..1], None).0)
}

/// The screens' usable areas (menu bar, Dock and taskbar left out) in the
/// points the window position uses, or `None` where they can't be listed.
pub fn screens(pixels_per_point: f32) -> Option<Vec<Rect>> {
    let screens = platform::screens(pixels_per_point)?;
    (!screens.is_empty()).then_some(screens)
}

#[cfg(target_os = "macos")]
mod platform {
    use eframe::egui::{Pos2, Rect, Vec2};
    use objc2_app_kit::NSScreen;
    use objc2_foundation::MainThreadMarker;

    pub fn screens(_pixels_per_point: f32) -> Option<Vec<Rect>> {
        let mtm = MainThreadMarker::new()?;
        let screens = NSScreen::screens(mtm);
        // AppKit counts y up from the bottom of the first (menu bar) screen;
        // window positions count down from its top.
        let main_height = screens.iter().next()?.frame().size.height;
        Some(
            screens
                .iter()
                .map(|screen| {
                    let f = screen.visibleFrame();
                    let top = main_height - (f.origin.y + f.size.height);
                    Rect::from_min_size(
                        Pos2::new(f.origin.x as f32, top as f32),
                        Vec2::new(f.size.width as f32, f.size.height as f32),
                    )
                })
                .collect(),
        )
    }
}

#[cfg(target_os = "windows")]
mod platform {
    use eframe::egui::{Pos2, Rect};
    use windows_sys::Win32::{
        Foundation::{BOOL, LPARAM, RECT},
        Graphics::Gdi::{EnumDisplayMonitors, GetMonitorInfoW, HDC, HMONITOR, MONITORINFO},
    };

    unsafe extern "system" fn each(monitor: HMONITOR, _: HDC, _: *mut RECT, data: LPARAM) -> BOOL {
        // SAFETY: `data` is the `&mut Vec<RECT>` passed to EnumDisplayMonitors
        // below, which calls this synchronously while that borrow is live.
        let out = unsafe { &mut *(data as *mut Vec<RECT>) };
        let mut info = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            rcMonitor: RECT {
                left: 0,
                top: 0,
                right: 0,
                bottom: 0,
            },
            rcWork: RECT {
                left: 0,
                top: 0,
                right: 0,
                bottom: 0,
            },
            dwFlags: 0,
        };
        // SAFETY: `monitor` comes from the enumeration and `info` is a valid
        // MONITORINFO with cbSize set, as GetMonitorInfoW requires.
        if unsafe { GetMonitorInfoW(monitor, &mut info) } != 0 {
            out.push(info.rcWork);
        }
        1
    }

    pub fn screens(pixels_per_point: f32) -> Option<Vec<Rect>> {
        let mut found: Vec<RECT> = Vec::new();
        // SAFETY: a null clip rect and HDC enumerate every monitor; `each`
        // only uses `found` through the pointer during this call.
        let ok = unsafe {
            EnumDisplayMonitors(
                0,
                std::ptr::null(),
                Some(each),
                &mut found as *mut Vec<RECT> as LPARAM,
            )
        };
        if ok == 0 {
            return None;
        }
        // Monitors are listed in physical pixels; positions are points.
        // Mixed-DPI setups convert with the saved window's scale, which is
        // close enough to tell on-screen from off-screen.
        let s = pixels_per_point.max(0.25);
        Some(
            found
                .iter()
                .map(|r| {
                    Rect::from_min_max(
                        Pos2::new(r.left as f32 / s, r.top as f32 / s),
                        Pos2::new(r.right as f32 / s, r.bottom as f32 / s),
                    )
                })
                .collect(),
        )
    }
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
mod platform {
    /// Linux: Wayland ignores window positions, and X11 window managers keep
    /// new windows on screen themselves, so the saved place is passed on.
    pub fn screens(_pixels_per_point: f32) -> Option<Vec<eframe::egui::Rect>> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn screen(x: f32, y: f32, w: f32, h: f32) -> Rect {
        Rect::from_min_size(Pos2::new(x, y), Vec2::new(w, h))
    }

    fn saved(position: [f32; 2], size: [f32; 2]) -> MainWindow {
        MainWindow {
            position: Some(position),
            size,
            ..Default::default()
        }
    }

    #[test]
    fn first_launch_uses_the_default_size_and_system_place() {
        let dir =
            std::env::temp_dir().join(format!("operandi-window-first-{}", std::process::id()));
        let state = MainWindow::load(&dir.join("main-window.json"));
        assert_eq!(state, MainWindow::default());
        assert_eq!(state.size, DEFAULT_SIZE);
        assert_eq!(state.position, None);
    }

    #[test]
    fn saved_state_round_trips_and_damaged_files_fall_back() {
        let dir = std::env::temp_dir().join(format!("operandi-window-rt-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("main-window.json");
        let s = MainWindow {
            size: [1200., 800.],
            position: Some([40., 60.]),
            maximized: true,
            fullscreen: false,
            pixels_per_point: 2.,
            ..Default::default()
        };
        s.save(&path).unwrap();
        assert_eq!(MainWindow::load(&path), s);
        for bad in [
            &br#"{"version":1,"size":[10,10]}"#[..],
            br#"{"version":1,"size":[1e30,800]}"#,
            br#"{"version":9}"#,
            b"not json",
        ] {
            std::fs::write(&path, bad).unwrap();
            assert_eq!(MainWindow::load(&path), MainWindow::default());
        }
        // Older files missing a field keep the rest.
        std::fs::write(&path, br#"{"version":1,"size":[1000,700]}"#).unwrap();
        assert_eq!(MainWindow::load(&path).size, [1000., 700.]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn visible_place_is_kept() {
        let screens = [screen(0., 25., 1728., 1092.)];
        let s = saved([100., 80.], [1200., 800.]);
        assert_eq!(s.placement(Some(&screens)), s);
    }

    #[test]
    fn place_on_an_unplugged_monitor_falls_back_to_the_system_default() {
        // Saved on a second monitor to the right, which is gone now.
        let screens = [screen(0., 25., 1728., 1092.)];
        let s = saved([2000., 100.], [1440., 900.]);
        let p = s.placement(Some(&screens));
        assert_eq!(p.position, None);
        assert_eq!(p.size, [1440., 900.]);
    }

    #[test]
    fn title_bar_must_be_on_a_screen() {
        let screens = [screen(0., 25., 1728., 1092.)];
        // Only the bottom of the window shows: the title bar is above the top.
        assert_eq!(
            saved([100., -300.], [800., 600.])
                .placement(Some(&screens))
                .position,
            None
        );
        // A sliver of the title bar at the left edge isn't enough to grab.
        assert_eq!(
            saved([-780., 100.], [800., 600.])
                .placement(Some(&screens))
                .position,
            None
        );
        // Most of the window off the right edge, title bar still reachable.
        let p = saved([1600., 100.], [800., 600.]).placement(Some(&screens));
        assert_eq!(p.position, Some([1600., 100.]));
    }

    #[test]
    fn second_monitor_place_is_kept_while_it_is_connected() {
        // Second screen to the left of and above the main one.
        let screens = [
            screen(0., 25., 1728., 1092.),
            screen(-2560., -400., 2560., 1440.),
        ];
        let s = saved([-2400., -300.], [2400., 1300.]);
        assert_eq!(s.placement(Some(&screens)), s);
    }

    #[test]
    fn size_shrinks_to_fit_a_smaller_screen() {
        // Saved full size on a big monitor; now only a laptop screen.
        let screens = [screen(0., 25., 1440., 875.)];
        let p = saved([3000., 0.], [2500., 1400.]).placement(Some(&screens));
        assert_eq!(p.position, None);
        assert_eq!(p.size, [1440., 875.]);
        // Never below the minimum, even on a tiny screen.
        let tiny = [screen(0., 0., 600., 400.)];
        assert_eq!(
            saved([0., 0.], [1000., 800.]).placement(Some(&tiny)).size,
            MIN_SIZE
        );
    }

    #[test]
    fn unknown_screens_trust_the_saved_place() {
        let s = saved([5000., 5000.], [1200., 800.]);
        assert_eq!(s.placement(None), s);
        assert_eq!(s.placement(Some(&[])), s);
    }

    #[test]
    fn maximised_and_fullscreen_are_kept_with_the_normal_rect() {
        let screens = [screen(0., 25., 1728., 1092.)];
        let s = MainWindow {
            maximized: true,
            fullscreen: true,
            ..saved([100., 80.], [1200., 800.])
        };
        assert_eq!(s.placement(Some(&screens)), s);
    }

    fn info(inner: Rect, outer: Rect) -> ViewportInfo {
        ViewportInfo {
            inner_rect: Some(inner),
            outer_rect: Some(outer),
            maximized: Some(false),
            fullscreen: Some(false),
            minimized: Some(false),
            native_pixels_per_point: Some(2.),
            ..Default::default()
        }
    }

    #[test]
    fn tracker_keeps_the_normal_rect_while_maximised() {
        let mut t = Tracker::new(MainWindow::default());
        assert!(!t.seen);
        t.observe(&ViewportInfo::default());
        assert!(!t.seen, "no report yet");
        t.observe(&info(
            screen(100., 108., 1200., 800.),
            screen(100., 80., 1200., 828.),
        ));
        assert!(t.seen);
        assert_eq!(t.state.size, [1200., 800.]);
        assert_eq!(t.state.position, Some([100., 80.]));
        assert_eq!(t.state.pixels_per_point, 2.);
        // Maximise: the big rect isn't taken as the normal one.
        let mut max = info(screen(0., 53., 1728., 1064.), screen(0., 25., 1728., 1092.));
        max.maximized = Some(true);
        t.observe(&max);
        assert!(t.state.maximized);
        assert_eq!(t.state.size, [1200., 800.]);
        assert_eq!(t.state.position, Some([100., 80.]));
        // Fullscreen too.
        let mut full = info(screen(0., 0., 1728., 1117.), screen(0., 0., 1728., 1117.));
        full.fullscreen = Some(true);
        t.observe(&full);
        assert!(t.state.fullscreen);
        assert_eq!(t.state.size, [1200., 800.]);
        // Minimised: nothing changes.
        let mut min = info(screen(0., 0., 1., 1.), screen(-9000., 0., 1., 1.));
        min.minimized = Some(true);
        t.observe(&min);
        assert!(t.state.fullscreen);
        assert_eq!(t.state.position, Some([100., 80.]));
        // Back to normal at a new place.
        t.observe(&info(
            screen(300., 228., 1000., 700.),
            screen(300., 200., 1000., 728.),
        ));
        assert!(!t.state.maximized && !t.state.fullscreen);
        assert_eq!(t.state.size, [1000., 700.]);
        assert_eq!(t.state.position, Some([300., 200.]));
    }

    #[test]
    fn tracker_saves_only_after_the_window_reported() {
        let dir = std::env::temp_dir().join(format!("operandi-window-tr-{}", std::process::id()));
        let path = dir.join("main-window.json");
        let mut t = Tracker::new(MainWindow::default());
        t.save(&path);
        assert!(!path.exists(), "an unseen window writes nothing");
        t.observe(&info(
            screen(10., 38., 900., 700.),
            screen(10., 10., 900., 728.),
        ));
        t.save(&path);
        let back = MainWindow::load(&path);
        assert_eq!(back.size, [900., 700.]);
        assert_eq!(back.position, Some([10., 10.]));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn builder_carries_the_restored_state() {
        let s = MainWindow {
            maximized: true,
            ..saved([100., 80.], [1200., 800.])
        };
        let b = s.apply(egui::ViewportBuilder::default());
        assert_eq!(b.inner_size, Some(Vec2::new(1200., 800.)));
        assert_eq!(b.min_inner_size, Some(Vec2::from(MIN_SIZE)));
        assert_eq!(b.position, Some(Pos2::new(100., 80.)));
        assert_eq!(b.maximized, Some(true));
        assert_eq!(b.fullscreen, Some(false));
        let none = MainWindow::default().apply(egui::ViewportBuilder::default());
        assert_eq!(none.position, None);
    }

    #[test]
    fn floating_windows_are_pulled_onto_the_pointers_screen() {
        let screens = [
            screen(0., 25., 1440., 875.),
            screen(1440., 0., 1920., 1080.),
        ];
        // Released near the right edge of the laptop screen: moved left so
        // the whole window shows; the size is kept.
        let (at, size) = place_on_screen(
            [1300., 300.],
            [400., 300.],
            [200., 160.],
            &screens,
            Some(Pos2::new(1300., 300.)),
        );
        assert_eq!(size, [400., 300.]);
        assert_eq!(at, [1040., 300.]);
        // Above the menu bar: down to the screen's top.
        let (at, _) = place_on_screen([100., 0.], [400., 300.], [200., 160.], &screens, None);
        assert_eq!(at, [100., 25.]);
        // On the second screen, by the pointer.
        let (at, _) = place_on_screen(
            [3300., 900.],
            [400., 300.],
            [200., 160.],
            &screens,
            Some(Pos2::new(3000., 900.)),
        );
        assert_eq!(at, [3360. - 400., 1080. - 300. - TITLE_STRIP]);
        // Too big for the screen: it shrinks to fit, never below the minimum.
        let (_, size) = place_on_screen(
            [0., 0.],
            [3000., 2000.],
            [200., 160.],
            &screens,
            Some(Pos2::new(10., 30.)),
        );
        assert_eq!(size, [1440., 875. - TITLE_STRIP]);
        let (_, size) = place_on_screen([0., 0.], [50., 50.], [200., 160.], &screens, None);
        assert_eq!(size, [200., 160.]);
        // Nothing listed: as asked.
        assert_eq!(
            place_on_screen([5., 6.], [300., 200.], [1., 1.], &[], None),
            ([5., 6.], [300., 200.])
        );
    }

    #[test]
    fn a_window_saved_on_an_unplugged_screen_reopens_near_the_main_window() {
        // T15: saved on a second monitor that is gone now.
        let laptop = [screen(0., 25., 1440., 875.)];
        let size = [400., 300.];
        assert_eq!(
            reopen_place(Some([2000., 300.]), size, Some(&laptop), Some([100., 80.])),
            Some([160., 140.])
        );
        // Still connected: kept.
        assert_eq!(
            reopen_place(Some([200., 300.]), size, Some(&laptop), Some([100., 80.])),
            Some([200., 300.])
        );
        // Main window unknown: the system places it.
        assert_eq!(
            reopen_place(Some([2000., 300.]), size, Some(&laptop), None),
            None
        );
        // Screens unknown: trusted.
        assert_eq!(
            reopen_place(Some([2000., 300.]), size, None, None),
            Some([2000., 300.])
        );
        // Near a main window at the screen's bottom right: still fully on it.
        let at =
            reopen_place(Some([-5000., 0.]), size, Some(&laptop), Some([1300., 800.])).unwrap();
        assert_eq!(at, [1040., 900. - 300. - TITLE_STRIP]);
    }
}
