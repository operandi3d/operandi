//! Which graphics adapter (GPU and backend) the window opens with.
//!
//! Left to itself, wgpu ranks adapters by device type only, across every
//! backend, so on Windows the same GPU can come up through Vulkan,
//! DirectX 12 or OpenGL depending on what each driver reports. A Windows
//! tester saw freezes with that default pick and a crash at launch with
//! DirectX 12 forced, while Vulkan ran cleanly (2026-11-23). So the backend
//! decides first and the device type second, and Windows tries Vulkan,
//! then DirectX 12, then OpenGL.
//!
//! `WGPU_BACKEND=vulkan|dx12|gl` (read by egui-wgpu when it makes the
//! instance) still limits the choice to the backends it names.

use egui_wgpu::wgpu;

/// Graphics backends in the order the app tries them, best first.
pub fn backend_order() -> &'static [wgpu::Backend] {
    use wgpu::Backend::*;
    if cfg!(target_os = "windows") {
        &[Vulkan, Dx12, Gl]
    } else {
        &[Metal, Vulkan, Gl, Dx12]
    }
}

/// Rank of an adapter, lower is better: a real GPU before a software one,
/// then backend (`backend_order`), then discrete before integrated (egui's
/// HighPerformance default). `None` for a backend not in the list.
pub fn rank(backend: wgpu::Backend, device: wgpu::DeviceType) -> Option<(bool, usize, u8)> {
    let at = backend_order().iter().position(|&b| b == backend)?;
    let kind = match device {
        wgpu::DeviceType::DiscreteGpu => 0,
        wgpu::DeviceType::IntegratedGpu => 1,
        wgpu::DeviceType::Other => 2,
        wgpu::DeviceType::VirtualGpu => 3,
        wgpu::DeviceType::Cpu => 4,
    };
    // A software renderer (Microsoft Basic Render Driver, llvmpipe) only
    // when no backend offers a real GPU.
    Some((device == wgpu::DeviceType::Cpu, at, kind))
}

/// The index of the adapter to use among `(backend, device type, can draw
/// to the window)`, or `None` when none fits.
pub fn pick(candidates: &[(wgpu::Backend, wgpu::DeviceType, bool)]) -> Option<usize> {
    candidates
        .iter()
        .enumerate()
        .filter(|(_, (_, _, fits))| *fits)
        .filter_map(|(i, &(backend, device, _))| Some((rank(backend, device)?, i)))
        .min()
        .map(|(_, i)| i)
}

/// egui-wgpu's adapter selector: the best-ranked adapter that can draw into
/// the window. Logs every candidate, so a bug report says what the machine
/// offered and what was picked.
pub fn select(
    adapters: &[wgpu::Adapter],
    surface: Option<&wgpu::Surface<'_>>,
) -> Result<wgpu::Adapter, String> {
    let mut candidates = Vec::with_capacity(adapters.len());
    for adapter in adapters {
        let info = adapter.get_info();
        let fits = surface.is_none_or(|s| adapter.is_surface_supported(s));
        crate::log_ring::log_line!(
            "GPU adapter: {:?} {} ({:?}, driver {} {}){}",
            info.backend,
            info.name,
            info.device_type,
            info.driver,
            info.driver_info,
            if fits {
                ""
            } else {
                " - can't draw to the window"
            }
        );
        candidates.push((info.backend, info.device_type, fits));
    }
    let i = pick(&candidates).ok_or_else(|| {
        format!(
            "no graphics adapter can draw Operandi's window ({} found)",
            adapters.len()
        )
    })?;
    let info = adapters[i].get_info();
    crate::log_ring::log_line!("GPU picked: {:?} {}", info.backend, info.name);
    Ok(adapters[i].clone())
}

/// Logs it when the GPU device is lost (driver reset, out of memory,
/// removed GPU). wgpu reports a lost device only through this callback;
/// the next call on it then fails with a bare "is invalid" panic, so
/// without this line a crash report wouldn't say why.
pub fn watch_device_lost(device: &wgpu::Device) {
    device.set_device_lost_callback(|reason, message| {
        crate::log_ring::log_line!("GPU device lost ({reason:?}): {message}");
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use wgpu::{Backend::*, DeviceType::*};

    #[test]
    fn windows_prefers_vulkan_then_dx12_then_gl() {
        if !cfg!(target_os = "windows") {
            return;
        }
        let all = [
            (Gl, DiscreteGpu, true),
            (Dx12, DiscreteGpu, true),
            (Vulkan, DiscreteGpu, true),
        ];
        assert_eq!(pick(&all), Some(2));
        assert_eq!(pick(&all[..2]), Some(1));
        assert_eq!(pick(&all[..1]), Some(0));
    }

    #[test]
    fn backend_beats_device_type() {
        // The first backend's integrated GPU over the second's discrete one:
        // the same chip often reports differently through different drivers.
        let first = backend_order()[0];
        let second = backend_order()[1];
        let list = [(second, DiscreteGpu, true), (first, IntegratedGpu, true)];
        assert_eq!(pick(&list), Some(1));
    }

    #[test]
    fn discrete_beats_integrated_within_a_backend() {
        let b = backend_order()[0];
        let list = [(b, IntegratedGpu, true), (b, DiscreteGpu, true)];
        assert_eq!(pick(&list), Some(1));
    }

    #[test]
    fn software_renderer_only_as_last_resort() {
        let first = backend_order()[0];
        let last = *backend_order().last().unwrap();
        let list = [(first, Cpu, true), (last, IntegratedGpu, true)];
        assert_eq!(pick(&list), Some(1));
        assert_eq!(pick(&list[..1]), Some(0));
    }

    #[test]
    fn adapters_that_cannot_draw_to_the_window_are_skipped() {
        let first = backend_order()[0];
        let second = backend_order()[1];
        let list = [(first, DiscreteGpu, false), (second, IntegratedGpu, true)];
        assert_eq!(pick(&list), Some(1));
        assert_eq!(pick(&list[..1]), None);
        assert_eq!(pick(&[]), None);
    }
}
