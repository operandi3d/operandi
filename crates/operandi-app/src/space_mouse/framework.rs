//! The 3DxWare source (macOS): when 3Dconnexion's own driver is
//! installed, it holds the device exclusively, so raw HID can't open it.
//! Then we talk to the driver through the client framework it installs,
//! loaded at run time from /Library/Frameworks. Nothing of it is bundled
//! or linked, and no SDK file is used: the few declarations below are our
//! own, written from the framework's exported function names and their
//! public descriptions.
//!
//! The driver applies its own per-app settings (speed, axis flips, axes
//! turned off) before the values reach us; Operandi's Speed and Invert
//! preferences then apply on top (docs/SPACEMOUSE_GUI_QA.md).
use super::decode::{DeviceState, FULL_SCALE, RawAxes};
use super::source::{Event, Source};
use eframe::egui::Context;
use std::ffi::c_void;
use std::sync::Mutex;
use std::sync::mpsc::{Receiver, Sender, channel};

const FRAMEWORK: &str = "/Library/Frameworks/3DconnexionClient.framework/3DconnexionClient";

/// "Any client": the driver then matches the client by the process name
/// passed with it, and keeps per-app settings under that app.
const SIGNATURE_WILDCARD: u32 = u32::from_be_bytes(*b"****");
/// The client takes the device over while its app is in front.
const MODE_TAKE_OVER: u16 = 1;
/// Every axis and the button events.
const MASK_ALL: u32 = 0x3FFF;
const MASK_ALL_BUTTONS: u32 = 0xFFFF_FFFF;
/// The message carrying a device state.
const MSG_DEVICE_STATE: u32 = u32::from_be_bytes(*b"3dSR");
/// Axis values the driver sends at full deflection with its speed at the
/// middle (provisional, SM-Q05: to be checked on a live puck).
const DRIVER_FULL_SCALE: f32 = 1024.;

/// The device state the driver hands the message handler. Our own layout
/// declaration: 2-byte packed, 48 bytes.
#[repr(C, packed(2))]
#[derive(Clone, Copy)]
struct DriverState {
    version: u16,
    client: u16,
    command: u16,
    param: i16,
    value: i32,
    time: u64,
    report: [u8; 8],
    buttons8: u16,
    axis: [i16; 6],
    address: u16,
    buttons: u32,
}

type MessageHandler = extern "C" fn(product: u32, message: u32, argument: *mut c_void);
type DeviceHandler = extern "C" fn(product: u32);
type SetHandlers = unsafe extern "C" fn(MessageHandler, DeviceHandler, DeviceHandler, bool) -> i16;
type Register = unsafe extern "C" fn(u32, *const u8, u16, u32) -> u16;
type SetMask = unsafe extern "C" fn(u16, u32);
type Unregister = unsafe extern "C" fn(u16);
type Cleanup = unsafe extern "C" fn();

/// Where the driver's callbacks send their events (the C callbacks carry
/// no context pointer, so this is global; one source at a time).
struct Sink {
    send: Sender<Event>,
    ctx: Context,
    client: u16,
    connected: bool,
}
static SINK: Mutex<Option<Sink>> = Mutex::new(None);

fn with_sink(f: impl FnOnce(&mut Sink)) {
    if let Ok(mut sink) = SINK.lock()
        && let Some(sink) = sink.as_mut()
    {
        f(sink);
        sink.ctx.request_repaint();
    }
}

/// Driver axis values on the raw-HID scale, so both sources feel alike.
fn rescale(v: i16) -> i16 {
    let scaled = f32::from(v) * FULL_SCALE / DRIVER_FULL_SCALE;
    scaled.round().clamp(-32768., 32767.) as i16
}

fn state_of(s: &DriverState) -> DeviceState {
    let axis = s.axis;
    DeviceState {
        axes: RawAxes {
            t: [rescale(axis[0]), rescale(axis[1]), rescale(axis[2])],
            r: [rescale(axis[3]), rescale(axis[4]), rescale(axis[5])],
        },
        buttons: s.buttons,
    }
}

/// A product name for the status line.
pub fn product_name(product: u32) -> &'static str {
    match product {
        0xC626 => "SpaceNavigator",
        0xC627 => "SpaceExplorer",
        0xC628 => "SpaceNavigator for Notebooks",
        0xC62B => "SpaceMouse Pro",
        0xC631 | 0xC632 => "SpaceMouse Pro Wireless",
        0xC633 => "SpaceMouse Enterprise",
        0xC635 => "SpaceMouse Compact",
        0xC62E | 0xC62F | 0xC652 => "SpaceMouse Wireless",
        _ => "3D mouse",
    }
}

extern "C" fn on_message(_product: u32, message: u32, argument: *mut c_void) {
    if message != MSG_DEVICE_STATE || argument.is_null() {
        return;
    }
    // SAFETY: for a device-state message the driver passes a pointer to
    // its device state, valid for the call; read_unaligned copies it out
    // without assuming any alignment.
    let s = unsafe { std::ptr::read_unaligned(argument as *const DriverState) };
    log_state(&s);
    with_sink(|sink| {
        // Only our client's states (another app's never reach us, but the
        // client id also confirms the layout lines up).
        if s.client != sink.client {
            return;
        }
        if !sink.connected {
            sink.connected = true;
            let _ = sink
                .send
                .send(Event::Connected("3D mouse (via 3DxWare)".into()));
        }
        let _ = sink.send.send(Event::State(state_of(&s)));
    });
}

extern "C" fn on_added(product: u32) {
    with_sink(|sink| {
        sink.connected = true;
        let _ = sink.send.send(Event::Connected(format!(
            "{} (via 3DxWare)",
            product_name(product)
        )));
    });
}

extern "C" fn on_removed(_product: u32) {
    with_sink(|sink| {
        sink.connected = false;
        let _ = sink.send.send(Event::Disconnected);
    });
}

/// `OPERANDI_SPACEMOUSE_LOG=<file>`: the first device states, raw, for
/// checking the layout and ranges on a live puck.
fn log_state(s: &DriverState) {
    use std::sync::atomic::{AtomicU32, Ordering};
    static COUNT: AtomicU32 = AtomicU32::new(0);
    let Some(path) = std::env::var_os("OPERANDI_SPACEMOUSE_LOG") else {
        return;
    };
    if COUNT.fetch_add(1, Ordering::Relaxed) >= 400 {
        return;
    }
    let (version, client, command, axis, buttons) =
        (s.version, s.client, s.command, s.axis, s.buttons);
    let line = format!(
        "version={version:#x} client={client} command={command} axis={axis:?} buttons={buttons:#x}\n"
    );
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
    {
        use std::io::Write;
        let _ = f.write_all(line.as_bytes());
    }
}

pub struct FrameworkSource {
    events: Receiver<Event>,
    client: u16,
    unregister: Unregister,
    cleanup: Cleanup,
}

impl FrameworkSource {
    /// Connect to 3DxWare, or `None` when it isn't installed or refuses
    /// (the caller falls back to raw HID).
    pub fn start(ctx: &Context) -> Option<Self> {
        if !std::path::Path::new(FRAMEWORK).exists() {
            return None;
        }
        // SAFETY: loading the driver's client framework runs only its own
        // initialisers. The library is never unloaded (leaked below), so
        // the callbacks it holds stay valid for the life of the process.
        let lib = unsafe { libloading::Library::new(FRAMEWORK) }.ok()?;
        let lib: &'static libloading::Library = Box::leak(Box::new(lib));
        // SAFETY: each symbol is a C function of the declared type.
        let (set_handlers, register, set_button_mask, unregister, cleanup) = unsafe {
            (
                *lib.get::<SetHandlers>(b"SetConnexionHandlers\0").ok()?,
                *lib.get::<Register>(b"RegisterConnexionClient\0").ok()?,
                *lib.get::<SetMask>(b"SetConnexionClientButtonMask\0").ok()?,
                *lib.get::<Unregister>(b"UnregisterConnexionClient\0").ok()?,
                *lib.get::<Cleanup>(b"CleanupConnexionHandlers\0").ok()?,
            )
        };
        let (send, events) = channel();
        if let Ok(mut sink) = SINK.lock() {
            *sink = Some(Sink {
                send,
                ctx: ctx.clone(),
                client: 0,
                connected: false,
            });
        }
        // SAFETY: the handlers are 'static extern "C" functions; the
        // driver calls them on its own thread (`true`).
        let error = unsafe { set_handlers(on_message, on_added, on_removed, true) };
        if error != 0 {
            Self::forget_sink();
            return None;
        }
        // The process name, as a length-prefixed (Pascal) string.
        let name = process_name();
        let mut pascal = vec![name.len().min(255) as u8];
        pascal.extend_from_slice(&name.as_bytes()[..pascal[0] as usize]);
        // SAFETY: `pascal` outlives the call; the driver copies the name.
        let client = unsafe {
            register(
                SIGNATURE_WILDCARD,
                pascal.as_ptr(),
                MODE_TAKE_OVER,
                MASK_ALL,
            )
        };
        if client == 0 {
            // SAFETY: undoes set_handlers above.
            unsafe { cleanup() };
            Self::forget_sink();
            return None;
        }
        // SAFETY: `client` is the id the driver just gave us.
        unsafe { set_button_mask(client, MASK_ALL_BUTTONS) };
        if let Ok(mut sink) = SINK.lock()
            && let Some(sink) = sink.as_mut()
        {
            sink.client = client;
        }
        Some(Self {
            events,
            client,
            unregister,
            cleanup,
        })
    }

    fn forget_sink() {
        if let Ok(mut sink) = SINK.lock() {
            *sink = None;
        }
    }
}

/// The running executable's file name (the name the driver knows the app
/// by).
fn process_name() -> String {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
        .unwrap_or_else(|| "Operandi".into())
}

impl Source for FrameworkSource {
    fn try_next(&mut self) -> Option<Event> {
        self.events.try_recv().ok()
    }
}

impl Drop for FrameworkSource {
    fn drop(&mut self) {
        // SAFETY: unregisters the client this source registered, then
        // removes its handlers; the library itself stays loaded.
        unsafe {
            (self.unregister)(self.client);
            (self.cleanup)();
        }
        Self::forget_sink();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_driver_state_layout_is_48_bytes_with_axes_at_30() {
        assert_eq!(std::mem::size_of::<DriverState>(), 48);
        assert_eq!(std::mem::offset_of!(DriverState, time), 12);
        assert_eq!(std::mem::offset_of!(DriverState, axis), 30);
        assert_eq!(std::mem::offset_of!(DriverState, buttons), 44);
    }

    #[test]
    fn driver_values_are_read_on_the_raw_hid_scale() {
        let mut bytes = [0u8; 48];
        bytes[2..4].copy_from_slice(&7u16.to_le_bytes());
        for (i, v) in [1024i16, -512, 0, 0, 0, -1024].iter().enumerate() {
            bytes[30 + 2 * i..32 + 2 * i].copy_from_slice(&v.to_le_bytes());
        }
        bytes[44..48].copy_from_slice(&3u32.to_le_bytes());
        // SAFETY: a 48-byte buffer read as the 48-byte packed struct.
        let s = unsafe { std::ptr::read_unaligned(bytes.as_ptr() as *const DriverState) };
        assert_eq!({ s.client }, 7);
        let state = state_of(&s);
        assert_eq!(state.axes.t, [350, -175, 0]);
        assert_eq!(state.axes.r, [0, 0, -350]);
        assert_eq!(state.buttons, 3);
    }

    #[test]
    fn known_products_are_named() {
        assert_eq!(product_name(0xC626), "SpaceNavigator");
        assert_eq!(product_name(0xC635), "SpaceMouse Compact");
        assert_eq!(product_name(0x1234), "3D mouse");
    }
}
