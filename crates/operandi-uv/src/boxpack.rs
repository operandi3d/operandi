// SPDX-FileCopyrightText: 2023 Blender Authors
// SPDX-FileCopyrightText: 2026 Operandi contributors
// SPDX-License-Identifier: GPL-2.0-or-later
//
// Adapted from Blender `source/blender/blenlib/intern/boxpack_2d.cc`
// (`BLI_box_pack_2d`, with USE_MERGE, USE_FREE_STRIP and USE_PACK_BIAS) at
// commit b4eed6fd498e76a7e93404bac2326e9f84deba22.

//! Packs rectangles tightly toward the origin by trying every free corner of
//! the boxes already placed.

/// A rectangle to place; `x` and `y` are written.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct BoxPack {
    pub w: f64,
    pub h: f64,
    pub x: f64,
    pub y: f64,
}

const EPSILON: f64 = 0.0000001;
const EPSILON_MERGE: f64 = 0.00001;
const EPSILON_BIAS: f64 = 0.000001;

const BLF: u8 = 1;
const TRF: u8 = 2;
const TLF: u8 = 4;
const BRF: u8 = 8;
const CORNERFLAGS: u8 = BLF | TRF | TLF | BRF;

const BL: usize = 0;
const TR: usize = 1;
const TL: usize = 2;
const BR: usize = 3;

fn quad_flag(q: usize) -> u8 {
    1 << q
}

#[derive(Clone, Copy, Debug, Default)]
struct Vert {
    x: f64,
    y: f64,
    free: u8,
    used: bool,
    bias: f64,
    trb: Option<usize>,
    blb: Option<usize>,
    brb: Option<usize>,
    tlb: Option<usize>,
    isect_cache: [Option<usize>; 4],
}

struct Packer<'a> {
    boxes: &'a mut [BoxPack],
    /// Corner vertices of each box: BL, TR, TL, BR.
    bv: Vec<[usize; 4]>,
    verts: Vec<Vert>,
}

impl Packer<'_> {
    fn xmin(&self, b: usize) -> f64 {
        self.verts[self.bv[b][BL]].x
    }
    fn xmax(&self, b: usize) -> f64 {
        self.verts[self.bv[b][TR]].x
    }
    fn ymin(&self, b: usize) -> f64 {
        self.verts[self.bv[b][BL]].y
    }
    fn ymax(&self, b: usize) -> f64 {
        self.verts[self.bv[b][TR]].y
    }
    fn v34x(&mut self, b: usize) {
        let v = self.bv[b];
        self.verts[v[TL]].x = self.verts[v[BL]].x;
        self.verts[v[BR]].x = self.verts[v[TR]].x;
    }
    fn v34y(&mut self, b: usize) {
        let v = self.bv[b];
        self.verts[v[TL]].y = self.verts[v[TR]].y;
        self.verts[v[BR]].y = self.verts[v[BL]].y;
    }
    fn set_xmin(&mut self, b: usize, f: f64) {
        let v = self.bv[b];
        self.verts[v[TR]].x = f + self.boxes[b].w;
        self.verts[v[BL]].x = f;
        self.v34x(b);
    }
    fn set_xmax(&mut self, b: usize, f: f64) {
        let v = self.bv[b];
        self.verts[v[BL]].x = f - self.boxes[b].w;
        self.verts[v[TR]].x = f;
        self.v34x(b);
    }
    fn set_ymin(&mut self, b: usize, f: f64) {
        let v = self.bv[b];
        self.verts[v[TR]].y = f + self.boxes[b].h;
        self.verts[v[BL]].y = f;
        self.v34y(b);
    }
    fn set_ymax(&mut self, b: usize, f: f64) {
        let v = self.bv[b];
        self.verts[v[BL]].y = f - self.boxes[b].h;
        self.verts[v[TR]].y = f;
        self.v34y(b);
    }
    fn isect(&self, a: usize, b: usize) -> bool {
        !(self.xmin(a) + EPSILON >= self.xmax(b)
            || self.ymin(a) + EPSILON >= self.ymax(b)
            || self.xmax(a) - EPSILON <= self.xmin(b)
            || self.ymax(a) - EPSILON <= self.ymin(b))
    }
    fn bias_update(&mut self, v: usize) {
        self.verts[v].bias = self.verts[v].x * self.verts[v].y * EPSILON_BIAS;
    }

    /// USE_MERGE: two boxes meet flush; keep one shared corner vertex.
    /// `a` = (box, corner) of one side, `b` of the other.
    fn merge(&mut self, a: (usize, usize), b: (usize, usize), mask: u8) {
        let va = self.bv[a.0][a.1];
        let vb = self.bv[b.0][b.1];
        if self.verts[va].used {
            self.verts[va].free &= self.verts[vb].free & !mask;
            self.bv[b.0][b.1] = va;
        } else {
            self.verts[vb].free &= self.verts[va].free & !mask;
            self.bv[a.0][a.1] = vb;
        }
    }
}

/// `BLI_box_pack_2d` with boxes in the given order. Returns the total width
/// and height used.
pub(crate) fn box_pack_2d(boxes: &mut [BoxPack]) -> (f64, f64) {
    let len = boxes.len();
    if len == 0 {
        return (0.0, 0.0);
    }
    let mut verts = Vec::with_capacity(4 * len);
    let mut bv = Vec::with_capacity(len);
    for b in 0..len {
        let base = verts.len();
        let v = |free: u8| Vert {
            free,
            ..Default::default()
        };
        let mut bl = v(CORNERFLAGS & !TRF);
        bl.trb = Some(b);
        let mut tr = v(CORNERFLAGS & !BLF);
        tr.blb = Some(b);
        let mut tl = v(CORNERFLAGS & !BRF);
        tl.brb = Some(b);
        let mut br = v(CORNERFLAGS & !TLF);
        br.tlb = Some(b);
        verts.extend([bl, tr, tl, br]);
        bv.push([base, base + 1, base + 2, base + 3]);
    }
    let mut p = Packer { boxes, bv, verts };

    // The first box goes at the origin.
    let v0 = p.bv[0];
    p.verts[v0[BL]].free = 0;
    p.verts[v0[BR]].free &= !(BLF | BRF);
    p.verts[v0[TL]].free &= !(BLF | TLF);
    let mut tot_x = p.boxes[0].w;
    let mut tot_y = p.boxes[0].h;
    p.set_xmin(0, 0.0);
    p.set_ymin(0, 0.0);
    p.boxes[0].x = 0.0;
    p.boxes[0].y = 0.0;
    for k in 0..4 {
        let v = p.bv[0][k];
        p.verts[v].used = true;
        p.bias_update(v);
    }
    let mut pack: Vec<usize> = (1..4).map(|k| p.bv[0][k]).collect();

    for b in 1..len {
        let (bw, bh) = (p.boxes[b].w, p.boxes[b].h);
        // Sort corners from lower left to top right, free ones last.
        pack.sort_by(|&i, &j| {
            let (v1, v2) = (&p.verts[i], &p.verts[j]);
            match (v1.free == 0, v2.free == 0) {
                (true, true) => return std::cmp::Ordering::Equal,
                (true, false) => return std::cmp::Ordering::Greater,
                (false, true) => return std::cmp::Ordering::Less,
                _ => {}
            }
            let a1 = (v1.x + bw).max(v1.y + bh) + v1.bias;
            let a2 = (v2.x + bw).max(v2.y + bh) + v2.bias;
            a1.total_cmp(&a2)
        });
        // Strip used-up corners from the end.
        let mut i = pack.len() - 1;
        while i != 0 && p.verts[pack[i]].free == 0 {
            i -= 1;
        }
        pack.truncate(i + 1);

        let mut isect = true;
        let mut i = 0;
        while i < pack.len() && isect {
            let vert = pack[i];
            let mut j = 0;
            while j < 4 && isect {
                if p.verts[vert].free & quad_flag(j) != 0 {
                    let (vx, vy) = (p.verts[vert].x, p.verts[vert].y);
                    match j {
                        BL => {
                            p.set_xmax(b, vx);
                            p.set_ymax(b, vy);
                        }
                        TR => {
                            p.set_xmin(b, vx);
                            p.set_ymin(b, vy);
                        }
                        TL => {
                            p.set_xmax(b, vx);
                            p.set_ymin(b, vy);
                        }
                        _ => {
                            p.set_xmin(b, vx);
                            p.set_ymax(b, vy);
                        }
                    }
                    isect = false;
                    let cached = p.verts[vert].isect_cache[j];
                    if p.xmin(b) < 0.0 || p.ymin(b) < 0.0 || cached.is_some_and(|c| p.isect(b, c)) {
                        isect = true;
                    } else {
                        for t in 0..b {
                            if p.isect(b, t) {
                                p.verts[vert].isect_cache[j] = Some(t);
                                isect = true;
                                break;
                            }
                        }
                    }
                    if !isect {
                        tot_x = tot_x.max(p.xmax(b));
                        tot_y = tot_y.max(p.ymax(b));
                        p.verts[vert].free &= !quad_flag(j);
                        match j {
                            TR => {
                                p.bv[b][BL] = vert;
                                p.verts[vert].trb = Some(b);
                            }
                            TL => {
                                p.bv[b][BR] = vert;
                                p.verts[vert].tlb = Some(b);
                            }
                            BR => {
                                p.bv[b][TL] = vert;
                                p.verts[vert].brb = Some(b);
                            }
                            _ => {
                                p.bv[b][TR] = vert;
                                p.verts[vert].blb = Some(b);
                            }
                        }
                        // Keep boxes inside the positive quadrant.
                        if p.xmin(b) <= 0.0 {
                            let (tl, bl) = (p.bv[b][TL], p.bv[b][BL]);
                            p.verts[tl].free &= !(TLF | BLF);
                            p.verts[bl].free &= !(TLF | BLF);
                        } else if p.ymin(b) <= 0.0 {
                            let (bl, br) = (p.bv[b][BL], p.bv[b][BR]);
                            p.verts[bl].free &= !(BRF | BLF);
                            p.verts[br].free &= !(BRF | BLF);
                        }
                        let vv = p.verts[vert];
                        // Vertical neighbours.
                        if let (Some(tlb), Some(trb)) = (vv.tlb, vv.trb)
                            && (b == tlb || b == trb)
                        {
                            let (htl, htr) = (p.boxes[tlb].h, p.boxes[trb].h);
                            if (htl - htr).abs() < EPSILON_MERGE {
                                p.merge((trb, TL), (tlb, TR), BLF | BRF);
                            } else if htl > htr {
                                let v = p.bv[trb][TL];
                                p.verts[v].free &= !(TLF | BLF);
                            } else {
                                let v = p.bv[tlb][TR];
                                p.verts[v].free &= !(TRF | BRF);
                            }
                        } else if let (Some(blb), Some(brb)) = (vv.blb, vv.brb)
                            && (b == blb || b == brb)
                        {
                            let (hbl, hbr) = (p.boxes[blb].h, p.boxes[brb].h);
                            if (hbl - hbr).abs() < EPSILON_MERGE {
                                p.merge((blb, BR), (brb, BL), TRF | TLF);
                            } else if hbl > hbr {
                                let v = p.bv[brb][BL];
                                p.verts[v].free &= !(TLF | BLF);
                            } else {
                                let v = p.bv[blb][BR];
                                p.verts[v].free &= !(TRF | BRF);
                            }
                        }
                        // Horizontal neighbours.
                        if let (Some(tlb), Some(blb)) = (vv.tlb, vv.blb)
                            && (b == tlb || b == blb)
                        {
                            let (wtl, wbl) = (p.boxes[tlb].w, p.boxes[blb].w);
                            if (wtl - wbl).abs() < EPSILON_MERGE {
                                p.merge((blb, TL), (tlb, BL), TRF | BRF);
                            } else if wtl > wbl {
                                let v = p.bv[blb][TL];
                                p.verts[v].free &= !(TLF | TRF);
                            } else {
                                let v = p.bv[tlb][BL];
                                p.verts[v].free &= !(BLF | BRF);
                            }
                        } else if let (Some(trb), Some(brb)) = (vv.trb, vv.brb)
                            && (b == trb || b == brb)
                        {
                            let (wtr, wbr) = (p.boxes[trb].w, p.boxes[brb].w);
                            if (wtr - wbr).abs() < EPSILON_MERGE {
                                p.merge((brb, TR), (trb, BR), TLF | BLF);
                            } else if wtr > wbr {
                                let v = p.bv[brb][TR];
                                p.verts[v].free &= !(TLF | TRF);
                            } else {
                                let v = p.bv[trb][BR];
                                p.verts[v].free &= !(BLF | BRF);
                            }
                        }
                        for k in 0..4 {
                            let v = p.bv[b][k];
                            if !p.verts[v].used {
                                p.verts[v].used = true;
                                p.bias_update(v);
                                pack.push(v);
                            }
                        }
                        p.boxes[b].x = p.xmin(b);
                        p.boxes[b].y = p.ymin(b);
                    }
                }
                j += 1;
            }
            i += 1;
        }
    }
    (tot_x, tot_y)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn overlap(a: &BoxPack, b: &BoxPack) -> bool {
        a.x + 1e-9 < b.x + b.w
            && b.x + 1e-9 < a.x + a.w
            && a.y + 1e-9 < b.y + b.h
            && b.y + 1e-9 < a.y + a.h
    }

    #[test]
    fn packs_without_overlap() {
        let mut boxes: Vec<BoxPack> = [
            (3.0, 2.0),
            (2.0, 2.0),
            (2.0, 1.0),
            (1.0, 1.0),
            (1.0, 1.0),
            (0.5, 2.0),
        ]
        .iter()
        .map(|&(w, h)| BoxPack {
            w,
            h,
            ..Default::default()
        })
        .collect();
        let (tx, ty) = box_pack_2d(&mut boxes);
        for i in 0..boxes.len() {
            assert!(boxes[i].x >= 0.0 && boxes[i].y >= 0.0);
            assert!(boxes[i].x + boxes[i].w <= tx + 1e-9 && boxes[i].y + boxes[i].h <= ty + 1e-9);
            for j in 0..i {
                assert!(!overlap(&boxes[i], &boxes[j]), "{i} {j} {boxes:?}");
            }
        }
        // Reasonably tight: total area 13.
        assert!(tx * ty < 13.0 * 1.6, "{tx} {ty}");
    }

    #[test]
    fn many_equal_boxes() {
        let mut boxes = vec![
            BoxPack {
                w: 1.0,
                h: 1.0,
                ..Default::default()
            };
            16
        ];
        let (tx, ty) = box_pack_2d(&mut boxes);
        assert!((tx * ty) <= 20.0, "{tx} {ty}");
        for i in 0..boxes.len() {
            for j in 0..i {
                assert!(!overlap(&boxes[i], &boxes[j]));
            }
        }
    }
}
