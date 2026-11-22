//! UV padding (dilation): fill the texels around UV islands from their
//! nearest covered texel, so filtering and mip-maps don't pull background
//! colour into the seams. Then, optionally, fill everything still empty
//! with a smooth pull-push average, so no texel is left at background.
//!
//! Adapted from TextureCraft `crates/texel/src/dilate.rs` and
//! `crates/bake/src/map/diffusion.rs` at 9c1b9cbc (MIT OR Apache-2.0,
//! Copyright (c) 2026 ArtCraft Team and the TextureCraft contributors; see
//! docs/licenses/texturecraft-LICENSE-MIT.txt). Changes: one square block
//! only (no UDIM tiles yet), diffusion over the whole image at once.

/// No source texel.
pub const NONE: u32 = u32::MAX;

/// For every texel, the index of the covered texel whose value it takes
/// (itself when covered, [`NONE`] when beyond the padding distance).
/// Computed once per UV layout and size, reused for every map.
#[derive(Clone, Debug, PartialEq)]
pub struct DilationMap {
    pub width: u32,
    pub height: u32,
    pub src: Vec<u32>,
}

impl DilationMap {
    /// `covered.len()` should be `width × height` (shorter input counts as
    /// uncovered). `max_dist` limits padding in texels (8-neighbour steps);
    /// `None` fills everything reachable.
    pub fn new(covered: &[bool], width: u32, height: u32, max_dist: Option<u32>) -> DilationMap {
        let n = (width as usize).saturating_mul(height as usize);
        let mut src = vec![NONE; n];
        let is_covered = |i: usize| covered.get(i).copied().unwrap_or(false);
        // Breadth first, one distance at a time; only island borders seed
        // it (interior texels have nothing to spread to).
        let mut frontier = Vec::new();
        for (i, s) in src.iter_mut().enumerate() {
            if is_covered(i) {
                *s = i as u32;
                if neighbours(i, width, height).any(|j| !is_covered(j)) {
                    frontier.push(i);
                }
            }
        }
        let limit = max_dist.unwrap_or(u32::MAX);
        let mut d = 0;
        while d < limit && !frontier.is_empty() {
            let mut next = Vec::new();
            for &i in &frontier {
                let from = src[i];
                for j in neighbours(i, width, height) {
                    if src[j] == NONE {
                        src[j] = from;
                        next.push(j);
                    }
                }
            }
            frontier = next;
            d += 1;
        }
        DilationMap { width, height, src }
    }

    /// Copies padded values in place: `data` holds `comps` values per
    /// texel. Texels with no source keep their value.
    pub fn apply<T: Copy>(&self, data: &mut [T], comps: usize) {
        for (i, &s) in self.src.iter().enumerate() {
            let s = s as usize;
            if s == i || s == NONE as usize {
                continue;
            }
            for c in 0..comps {
                if let Some(v) = data.get(s * comps + c).copied()
                    && let Some(d) = data.get_mut(i * comps + c)
                {
                    *d = v;
                }
            }
        }
    }

    /// Whether texel `i` has a value (covered or padded).
    pub fn has_value(&self, i: usize) -> bool {
        self.src.get(i).is_some_and(|s| *s != NONE)
    }
}

/// The 8-neighbours of texel `i` of a `w × h` image.
fn neighbours(i: usize, w: u32, h: u32) -> impl Iterator<Item = usize> {
    let wu = w as usize;
    let (x, y) = ((i % wu.max(1)) as u32, (i / wu.max(1)) as u32);
    let left = x > 0;
    let right = x + 1 < w;
    let up = y > 0;
    let down = y + 1 < h;
    [
        (left, i.wrapping_sub(1)),
        (right, i + 1),
        (up, i.wrapping_sub(wu)),
        (down, i + wu),
        (left && up, i.wrapping_sub(wu + 1)),
        (right && up, i.wrapping_sub(wu).wrapping_add(1)),
        (left && down, (i + wu).wrapping_sub(1)),
        (right && down, i + wu + 1),
    ]
    .into_iter()
    .filter_map(|(ok, j)| ok.then_some(j))
}

/// Fills every texel `padding` left without a value with a smooth average
/// of the texels around it (pull-push: average down to 1×1, then
/// interpolate back up), so the whole image has plausible values.
pub fn fill_rest(pixels: &mut [f32], components: usize, padding: &DilationMap) {
    let (width, height) = (padding.width as usize, padding.height as usize);
    let valid: Vec<bool> = (0..width * height).map(|i| padding.has_value(i)).collect();
    if !valid.iter().any(|v| *v) || valid.iter().all(|v| *v) {
        return;
    }
    let mut values = pixels.to_vec();
    fill(&mut values, &valid, width, height, components);
    for (i, ok) in valid.iter().enumerate() {
        if !ok {
            let range = i * components..(i + 1) * components;
            if let (Some(dst), Some(v)) = (pixels.get_mut(range.clone()), values.get(range)) {
                dst.copy_from_slice(v);
            }
        }
    }
}

fn fill(values: &mut [f32], valid: &[bool], width: usize, height: usize, components: usize) {
    if width <= 1 && height <= 1 {
        return;
    }
    let (sw, sh) = (width.div_ceil(2), height.div_ceil(2));
    let mut small = vec![0.0; sw * sh * components];
    let mut small_valid = vec![false; sw * sh];
    for (index, covered) in small_valid.iter_mut().enumerate() {
        let (x, y) = (index % sw * 2, index / sw * 2);
        let mut count = 0;
        for cy in y..(y + 2).min(height) {
            for cx in x..(x + 2).min(width) {
                let child = cy * width + cx;
                if valid.get(child) != Some(&true) {
                    continue;
                }
                count += 1;
                for c in 0..components {
                    small[index * components + c] += values[child * components + c];
                }
            }
        }
        *covered = count > 0;
        for v in &mut small[index * components..(index + 1) * components] {
            *v /= count.max(1) as f32;
        }
    }
    fill(&mut small, &small_valid, sw, sh, components);
    for (index, covered) in valid.iter().enumerate() {
        if *covered {
            continue;
        }
        let (x, y) = (
            (index % width) as f32 * 0.5 - 0.25,
            (index / width) as f32 * 0.5 - 0.25,
        );
        let (fx, fy) = (x - x.floor(), y - y.floor());
        let sample = |ox: i32, oy: i32, c: usize| {
            let sx = (x.floor() as i32 + ox).clamp(0, sw as i32 - 1) as usize;
            let sy = (y.floor() as i32 + oy).clamp(0, sh as i32 - 1) as usize;
            small[(sy * sw + sx) * components + c]
        };
        for c in 0..components {
            let top = sample(0, 0, c) * (1.0 - fx) + sample(1, 0, c) * fx;
            let bottom = sample(0, 1, c) * (1.0 - fx) + sample(1, 1, c) * fx;
            values[index * components + c] = top * (1.0 - fy) + bottom * fy;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uncovered_texels_take_the_nearest_covered_value() {
        let covered = [false, true, false, false, false];
        let m = DilationMap::new(&covered, 5, 1, Some(2));
        assert_eq!(m.src, vec![1, 1, 1, 1, NONE]);
        let mut data = [0u8, 9, 0, 0, 0];
        m.apply(&mut data, 1);
        assert_eq!(data, [9, 9, 9, 9, 0]);
    }

    #[test]
    fn unlimited_padding_fills_everything_and_keeps_covered_values() {
        let covered = [true, false, false, false, false, false, false, false, true];
        let m = DilationMap::new(&covered, 3, 3, None);
        assert!(m.src.iter().all(|s| *s != NONE));
        assert_eq!((m.src[0], m.src[8]), (0, 8));
    }

    #[test]
    fn nothing_covered_is_a_no_op() {
        let m = DilationMap::new(&[], 4, 4, None);
        let mut data = [3u16; 16];
        m.apply(&mut data, 1);
        assert_eq!(data, [3u16; 16]);
        let mut f = [0.5f32; 16];
        fill_rest(&mut f, 1, &m);
        assert_eq!(f, [0.5; 16]);
    }

    #[test]
    fn fill_rest_reaches_every_texel_between_the_two_values() {
        let mut covered = vec![false; 16 * 8];
        covered[0] = true;
        covered[16 * 8 - 1] = true;
        let m = DilationMap::new(&covered, 16, 8, Some(1));
        let mut px = vec![0.0f32; 16 * 8];
        px[16 * 8 - 1] = 1.0;
        m.apply(&mut px, 1);
        fill_rest(&mut px, 1, &m);
        assert!(px.iter().all(|v| (0.0..=1.0).contains(v)));
        assert!(px[4 * 16 + 8] > 0.0 && px[4 * 16 + 8] < 1.0);
    }
}
