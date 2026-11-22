//! SYM-LOOP-01..04 (bug reports #3 and #4, Jason 2026-11-21): selection
//! commands that read the picks as ordered seeds (Loop, Ring, Close Loop,
//! Select More / Less / Between, Next / Previous Loop) work out their answer
//! from the picked side only when Symmetry is on, then add the mirror side.
//!
//! With Symmetry on, every click also selects its mirror (SYM-B2-10), so two
//! adjacent polygons arrive as four seeds: `a, a', b, b'`. Loop needs one or
//! two seeds and gave up. The rule (Jason): only the picks on the side you
//! clicked count.
//!
//! - SYM-LOOP-01: walking the picks in order, a pick that is the mirror of an
//!   earlier counted pick is set aside. Everything else counts, in order.
//! - SYM-LOOP-02: a pick that is its own mirror (it sits on the centre seam)
//!   always counts.
//! - SYM-LOOP-03: the command runs on the counted picks exactly as it does
//!   with Symmetry off, then the mirror of its result is added (as Select
//!   Symmetrical does), so the loop and its mirrored loop are both selected.
//!   A loop that is its own mirror (crossing the seam) is not doubled.
//! - SYM-LOOP-04: Symmetry off, or any other selection command, is unchanged.

use crate::symmetry::{match_tolerance, nearest, topology_counterparts};
use crate::{Mode, Result, Scene, SelectionOperation, Symmetry};
use std::collections::{BTreeSet, HashMap};

/// The commands that read the picks as ordered seeds (SYM-LOOP-04).
pub fn reads_seeds(operation: SelectionOperation) -> bool {
    use SelectionOperation::*;
    matches!(
        operation,
        Loop | Ring | CloseLoop | More | Less | Between | NextLoop | PreviousLoop
    )
}

impl Scene {
    /// SYM-LOOP-01..03 on the active mesh: drop the mirrored seeds, run
    /// `operation`, add the result's mirror side. Without Symmetry, or for a
    /// command that doesn't read seeds, it is `selection_operation`.
    pub fn selection_operation_symmetric(
        &mut self,
        operation: SelectionOperation,
        symmetry: Symmetry,
    ) -> Result<()> {
        if symmetry.axis.is_none() || !reads_seeds(operation) || self.selection.mode == Mode::Item {
            return self.selection_operation(operation);
        }
        symmetry.validate()?;
        let mesh = &self.active().ok_or("Select a visible mesh")?.mesh;
        let tolerance = match_tolerance(mesh);
        let topology = symmetry
            .topology
            .then(|| topology_counterparts(mesh, symmetry));
        let mirror = |v: u32| -> Option<u32> {
            if let Some(map) = &topology {
                return map.get(v as usize).copied().flatten();
            }
            let p = *mesh.vertices.get(v as usize)?;
            nearest(mesh, symmetry.mirror(p), tolerance)
        };
        let s = &self.selection;
        let mut probe = self.clone();
        match s.mode {
            Mode::Vertex => {
                probe.selection.vertices = counted(&s.vertices, |&v| mirror(v));
            }
            Mode::Edge => {
                // Compare edges by their sorted ends.
                let key = |a: u32, b: u32| (a.min(b), a.max(b));
                probe.selection.edges = counted(
                    &s.edges.iter().map(|&(a, b)| key(a, b)).collect::<Vec<_>>(),
                    |&(a, b)| Some(key(mirror(a)?, mirror(b)?)),
                );
            }
            Mode::Polygon => {
                // Only the picks' mirrors are looked up: one pass over the
                // polygons, keyed by their point sets.
                let wanted: Vec<Option<Vec<u32>>> = s
                    .faces
                    .iter()
                    .map(|&f| {
                        let face = mesh.faces.get(f as usize)?;
                        let mut set = face
                            .vertices
                            .iter()
                            .map(|&v| mirror(v))
                            .collect::<Option<Vec<u32>>>()?;
                        set.sort_unstable();
                        Some(set)
                    })
                    .collect();
                let keys: BTreeSet<&Vec<u32>> = wanted.iter().flatten().collect();
                let mut found: HashMap<Vec<u32>, u32> = HashMap::new();
                for (g, face) in mesh.faces.iter().enumerate() {
                    let mut set = face.vertices.clone();
                    set.sort_unstable();
                    if keys.contains(&set) {
                        found.entry(set).or_insert(g as u32);
                    }
                }
                let partner: HashMap<u32, u32> = s
                    .faces
                    .iter()
                    .zip(&wanted)
                    .filter_map(|(&f, w)| Some((f, *found.get(w.as_ref()?)?)))
                    .collect();
                probe.selection.faces = counted(&s.faces, |f| partner.get(f).copied());
            }
            Mode::Item => unreachable!(),
        }
        probe.selection_operation(operation)?;
        probe.select_symmetrical(symmetry)?;
        self.selection = probe.selection;
        Ok(())
    }
}

/// SYM-LOOP-01/02: the picks that count, in pick order. A pick is set aside
/// when it is the mirror of an earlier counted pick; a pick that is its own
/// mirror (on the seam) or has no mirror always counts.
fn counted<T: Copy + Ord>(picks: &[T], mirror: impl Fn(&T) -> Option<T>) -> Vec<T> {
    let mut mirrors_of_counted = BTreeSet::new();
    let mut out = Vec::with_capacity(picks.len());
    for pick in picks {
        if mirrors_of_counted.contains(pick) {
            continue;
        }
        if let Some(m) = mirror(pick)
            && m != *pick
        {
            mirrors_of_counted.insert(m);
        }
        out.push(*pick);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::counted;

    #[test]
    fn mirrored_picks_are_set_aside_in_pick_order() {
        // 1 <-> 11, 2 <-> 12, 5 is on the seam.
        let mirror = |v: &u32| {
            Some(match *v {
                5 => 5,
                v if v > 10 => v - 10,
                v => v + 10,
            })
        };
        assert_eq!(counted(&[1, 11, 2, 12], mirror), vec![1, 2]);
        assert_eq!(counted(&[11, 1, 2, 12], mirror), vec![11, 2]);
        assert_eq!(counted(&[5, 1, 11], mirror), vec![5, 1]);
        assert_eq!(counted(&[1, 2], mirror), vec![1, 2]);
    }
}
