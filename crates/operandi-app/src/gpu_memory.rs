//! Graphics memory watch (Jason, 2026-11-20; replaces Max GL Polygon Count
//! and its question, a deliberate departure from the reference). There is
//! no fixed triangle cap: Operandi reads the graphics card's own memory
//! use and budget, and when a scene comes close to the budget the heaviest
//! items draw as bounding-box stand-ins, their geometry released from the
//! card, instead of the app running out of memory. When there is room
//! again they come back, lightest first. A line in the view says how many
//! items are stand-ins.
//!
//! The budget is the Metal device's recommended working set on macOS.
//! Other platforms report no budget yet, so nothing is ever replaced there.
use crate::Operandi;
use std::collections::BTreeSet;
use std::time::{Duration, Instant};

/// Over this share of the budget, the heaviest items become stand-ins...
const HIGH: f64 = 0.85;
/// ...until the estimate is back under this one.
const LOW: f64 = 0.70;
/// A stand-in comes back only if the estimate with it stays under this.
const RETURN: f64 = 0.75;
/// How often the card is asked (cheap, but not every frame).
const EVERY: Duration = Duration::from_millis(250);

/// The card's memory now, in bytes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Sample {
    pub used: u64,
    pub budget: u64,
}

/// Which items are stand-ins, and the last reading.
#[derive(Default)]
pub struct MemoryWatch {
    pub stand_ins: BTreeSet<u64>,
    pub last: Option<Sample>,
    asked: Option<Instant>,
}

impl MemoryWatch {
    /// Bring the stand-ins up to date for `sample`, given each drawn
    /// item's triangle count (`items`, any order). Memory per triangle is
    /// estimated from the items the card holds now, so one reading can
    /// replace as many items as it takes. The meshes being edited
    /// (`keep`) are never stand-ins.
    pub fn update(&mut self, sample: Sample, items: &[(u64, u64)], keep: &BTreeSet<u64>) {
        self.last = Some(sample);
        let present: BTreeSet<u64> = items.iter().map(|&(id, _)| id).collect();
        self.stand_ins
            .retain(|id| present.contains(id) && !keep.contains(id));
        if sample.budget == 0 {
            self.stand_ins.clear();
            return;
        }
        let budget = sample.budget as f64;
        let used = sample.used as f64;
        let held: u64 = items
            .iter()
            .filter(|(id, _)| !self.stand_ins.contains(id))
            .map(|&(_, t)| t)
            .sum();
        // Bytes per triangle; nothing to go on with no triangles held.
        let per = if held > 0 { used / held as f64 } else { 0. };
        if used > HIGH * budget && per > 0. {
            let mut heaviest: Vec<(u64, u64)> = items
                .iter()
                .copied()
                .filter(|(id, t)| *t > 0 && !self.stand_ins.contains(id) && !keep.contains(id))
                .collect();
            heaviest.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
            let mut estimate = used;
            for (id, triangles) in heaviest {
                if estimate <= LOW * budget {
                    break;
                }
                self.stand_ins.insert(id);
                estimate -= triangles as f64 * per;
            }
        } else if used < LOW * budget && !self.stand_ins.is_empty() {
            // One back per reading, lightest first, if it fits.
            let per = if per > 0. { per } else { f64::INFINITY };
            let lightest = items
                .iter()
                .copied()
                .filter(|(id, _)| self.stand_ins.contains(id))
                .min_by(|a, b| a.1.cmp(&b.1).then(a.0.cmp(&b.0)));
            if let Some((id, triangles)) = lightest
                && used + triangles as f64 * per < RETURN * budget
            {
                self.stand_ins.remove(&id);
            }
        }
    }

    /// Time to ask the card again?
    fn due(&mut self) -> bool {
        let now = Instant::now();
        if self.asked.is_some_and(|t| now.duration_since(t) < EVERY) {
            return false;
        }
        self.asked = Some(now);
        true
    }

    /// The view's note while some items are stand-ins.
    pub fn note(&self) -> Option<String> {
        let n = self.stand_ins.len();
        (n > 0).then(|| {
            format!(
                "Graphics memory full: {n} item{} drawn as box{}",
                if n == 1 { "" } else { "s" },
                if n == 1 { "" } else { "es" }
            )
        })
    }

    /// Near the budget (the Bounding Box Threshold then boxes items while
    /// the view is navigated).
    pub fn pressed(&self) -> bool {
        self.last
            .is_some_and(|s| s.budget > 0 && s.used as f64 > LOW * s.budget as f64)
    }
}

/// The card's memory use and budget, where the platform says.
#[cfg(target_os = "macos")]
pub fn sample(device: &egui_wgpu::wgpu::Device) -> Option<Sample> {
    use egui_wgpu::wgpu::hal::api::Metal;
    // SAFETY: `as_hal` lends the Metal device wgpu owns for this call only.
    // We read two counters from it and keep no handle, issue no commands
    // and change no state, so wgpu's own use of the device is unaffected.
    let hal = unsafe { device.as_hal::<Metal>() }?;
    let raw = hal.raw_device().lock();
    Some(Sample {
        used: raw.current_allocated_size(),
        budget: raw.recommended_max_working_set_size(),
    })
}

#[cfg(not(target_os = "macos"))]
pub fn sample(_device: &egui_wgpu::wgpu::Device) -> Option<Sample> {
    None
}

impl Operandi {
    /// Once a frame: read the card (every `EVERY`) and update the
    /// stand-ins from the items the views draw.
    pub(super) fn watch_gpu_memory(&mut self) {
        let Some(gpu) = &self.viewport_gpu else {
            return;
        };
        if !self.gpu_memory.due() {
            return;
        }
        let Some(sample) = sample(&gpu.state.device) else {
            return;
        };
        let items = self.item_triangles();
        let keep = self
            .editor
            .scene
            .shown_active_meshes()
            .into_iter()
            .collect();
        self.gpu_memory.update(sample, &items, &keep);
    }

    /// Each drawn item's triangles, counted as `drawn_triangles` counts
    /// them (a lookup per item for those on the card).
    fn item_triangles(&self) -> Vec<(u64, u64)> {
        let shown = self
            .editor
            .scene
            .items
            .iter()
            .any(|i| i.mesh_stack.is_some())
            .then(|| self.editor.evaluated_display())
            .flatten();
        let scene = shown.as_ref().unwrap_or(&self.editor.scene);
        let hidden = scene.display_hidden();
        scene
            .items
            .iter()
            .filter(|i| i.visible && !hidden.contains(&i.id))
            .map(|i| {
                let triangles = self.resident_cache.drawn_triangles(i).unwrap_or_else(|| {
                    operandi_core::subdivision::drawn_triangles(
                        &i.mesh,
                        &i.subdivision.clone().unwrap_or_default(),
                    )
                });
                (i.id, triangles)
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const GB: u64 = 1 << 30;

    fn none() -> BTreeSet<u64> {
        BTreeSet::new()
    }

    #[test]
    fn the_mesh_being_edited_is_never_a_stand_in() {
        let mut w = MemoryWatch {
            stand_ins: BTreeSet::from([2]),
            ..Default::default()
        };
        let items = [(1, 1_000_000), (2, 6_000_000), (3, 2_000_000)];
        w.update(
            Sample {
                used: 9 * GB,
                budget: 10 * GB,
            },
            &items,
            &BTreeSet::from([2]),
        );
        assert!(!w.stand_ins.contains(&2));
        assert!(w.stand_ins.contains(&3), "the next heaviest goes instead");
    }

    #[test]
    fn under_the_budget_nothing_changes() {
        let mut w = MemoryWatch::default();
        w.update(
            Sample {
                used: GB,
                budget: 10 * GB,
            },
            &[(1, 1_000), (2, 9_000)],
            &none(),
        );
        assert!(w.stand_ins.is_empty());
        assert_eq!(w.note(), None);
        assert!(!w.pressed());
    }

    #[test]
    fn near_the_budget_the_heaviest_items_become_stand_ins() {
        let mut w = MemoryWatch::default();
        // 9 of 10 GB in use; the 6M item alone brings it under 70%.
        let items = [(1, 1_000_000), (2, 6_000_000), (3, 2_000_000)];
        w.update(
            Sample {
                used: 9 * GB,
                budget: 10 * GB,
            },
            &items,
            &none(),
        );
        assert_eq!(w.stand_ins, BTreeSet::from([2]));
        assert_eq!(
            w.note().as_deref(),
            Some("Graphics memory full: 1 item drawn as box")
        );
        // Still high (the card hasn't freed it yet would read the same):
        // the next heaviest goes too.
        w.update(
            Sample {
                used: 9 * GB,
                budget: 10 * GB,
            },
            &items,
            &none(),
        );
        assert_eq!(w.stand_ins, BTreeSet::from([2, 3]));
    }

    #[test]
    fn with_room_again_stand_ins_come_back_lightest_first() {
        let mut w = MemoryWatch {
            stand_ins: BTreeSet::from([2, 3]),
            ..Default::default()
        };
        let items = [(1, 1_000_000), (2, 6_000_000), (3, 2_000_000)];
        // 1M triangles in 1 GB of 10: item 3 (2 GB more) fits under 75%.
        let sample = Sample {
            used: GB,
            budget: 10 * GB,
        };
        w.update(sample, &items, &none());
        assert_eq!(w.stand_ins, BTreeSet::from([2]));
        // Item 2 would need 6 GB more at 1 GB per million: 3 + 6 > 7.5.
        w.update(
            Sample {
                used: 3 * GB,
                budget: 10 * GB,
            },
            &items,
            &none(),
        );
        assert_eq!(w.stand_ins, BTreeSet::from([2]), "doesn't fit yet");
        // Deleted items stop being stand-ins.
        w.update(sample, &[(1, 1_000_000)], &none());
        assert!(w.stand_ins.is_empty());
    }

    #[test]
    fn no_budget_means_no_stand_ins() {
        let mut w = MemoryWatch {
            stand_ins: BTreeSet::from([2]),
            ..Default::default()
        };
        w.update(
            Sample {
                used: GB,
                budget: 0,
            },
            &[(2, 5)],
            &none(),
        );
        assert!(w.stand_ins.is_empty());
    }
}
