//! Photometry files for the Photometric light (LP-PH-01..05): the IES
//! LM-63 text format (the public standard fixture makers publish), read
//! for the values loading a file sets and for the viewport's shape. Renders
//! hand the file itself to Cycles, which reads IES too.
//!
//! Provisional (LPROP-Q15, LPROP-Q16): EULUMDAT (.ldt) isn't read yet;
//! Radiant Intensity from a file is its peak candela times the multipliers,
//! converted with the same D65 luminous efficacy Cycles uses (177.83 lm/W),
//! so a freshly loaded light renders exactly the file's intensities.
use crate::Result;

/// Lumens per watt Cycles assumes for IES files (D65).
pub const LUMINOUS_EFFICACY: f64 = 177.83;

/// The parts of an IES file Operandi uses.
#[derive(Clone, Debug, PartialEq)]
pub struct Photometry {
    /// Vertical angles, degrees (0 = straight down the light's −Z).
    pub vertical: Vec<f64>,
    /// Horizontal angles, degrees.
    pub horizontal: Vec<f64>,
    /// Candela per horizontal angle, then per vertical angle, multipliers
    /// applied.
    pub candela: Vec<Vec<f64>>,
    /// Luminous opening width and length, metres.
    pub width: f64,
    pub length: f64,
}

/// Largest file read, bytes.
const MAX_BYTES: usize = 4 * 1024 * 1024;
/// Largest angle grid read.
const MAX_ANGLES: usize = 10_000;

impl Photometry {
    /// Parse an IES LM-63 file's text.
    pub fn parse(text: &str) -> Result<Photometry> {
        if text.len() > MAX_BYTES {
            return Err("Photometry file is too large".into());
        }
        let mut lines = text.lines();
        let mut tilt = None;
        for line in lines.by_ref() {
            if let Some(rest) = line.trim_start().strip_prefix("TILT=") {
                tilt = Some(rest.trim().to_string());
                break;
            }
        }
        let tilt = tilt.ok_or("Not an IES photometry file (no TILT= line)")?;
        let rest: String = lines.collect::<Vec<_>>().join(" ");
        let mut numbers = rest
            .split(|c: char| c.is_whitespace() || c == ',')
            .filter(|t| !t.is_empty())
            .map(|t| {
                t.parse::<f64>()
                    .map_err(|_| format!("Unreadable number {t:?} in photometry file"))
            });
        let mut next = || -> Result<f64> {
            numbers
                .next()
                .unwrap_or_else(|| Err("Photometry file ends early".into()))
        };
        match tilt.as_str() {
            "NONE" => {}
            "INCLUDE" => {
                // Lamp-to-luminaire geometry, then angle/factor pairs.
                next()?;
                let n = next()? as usize;
                if n > MAX_ANGLES {
                    return Err("Photometry file has too many tilt angles".into());
                }
                for _ in 0..2 * n {
                    next()?;
                }
            }
            _ => return Err("Photometry files with a separate tilt file aren't supported".into()),
        }
        let _lamps = next()?;
        let _lumens = next()?;
        let multiplier = next()?;
        let nv = next()?;
        let nh = next()?;
        let _photometric_type = next()?;
        let units = next()?;
        let width = next()?;
        let length = next()?;
        let _height = next()?;
        let ballast = next()?;
        let ballast_lamp = next()?;
        let _watts = next()?;
        let count = |v: f64| {
            (v.fract() == 0.0 && v >= 1.0 && v <= MAX_ANGLES as f64)
                .then_some(v as usize)
                .ok_or("Photometry file has a bad angle count")
        };
        let (nv, nh) = (count(nv)?, count(nh)?);
        if nv * nh > 1_000_000 {
            return Err("Photometry file is too large".into());
        }
        let vertical = (0..nv).map(|_| next()).collect::<Result<Vec<_>>>()?;
        let horizontal = (0..nh).map(|_| next()).collect::<Result<Vec<_>>>()?;
        let factor = multiplier * ballast * ballast_lamp;
        let candela = (0..nh)
            .map(|_| {
                (0..nv)
                    .map(|_| next().map(|c| c * factor))
                    .collect::<Result<Vec<_>>>()
            })
            .collect::<Result<Vec<_>>>()?;
        let finite = |v: &[f64]| v.iter().all(|x| x.is_finite());
        if !finite(&vertical)
            || !finite(&horizontal)
            || !candela
                .iter()
                .all(|c| finite(c) && c.iter().all(|x| *x >= 0.0))
            || !vertical.windows(2).all(|w| w[0] < w[1])
            || !horizontal.windows(2).all(|w| w[0] < w[1])
            || !factor.is_finite()
        {
            return Err("Photometry file has invalid angles or values".into());
        }
        // Units: 1 = feet, 2 = metres.
        let metres = if units == 1.0 { 0.3048 } else { 1.0 };
        Ok(Photometry {
            vertical,
            horizontal,
            candela,
            width: (width * metres).abs(),
            length: (length * metres).abs(),
        })
    }

    /// Read and parse `path`.
    pub fn load(path: &std::path::Path) -> Result<Photometry> {
        let meta =
            std::fs::metadata(path).map_err(|e| format!("Can't read {}: {e}", path.display()))?;
        if meta.len() > MAX_BYTES as u64 {
            return Err("Photometry file is too large".into());
        }
        let bytes =
            std::fs::read(path).map_err(|e| format!("Can't read {}: {e}", path.display()))?;
        Self::parse(&String::from_utf8_lossy(&bytes))
    }

    /// The brightest candela value.
    pub fn peak_candela(&self) -> f64 {
        self.candela.iter().flatten().cloned().fold(0.0, f64::max)
    }

    /// LP-PH-02: the Radiant Intensity loading the file sets, W/sr.
    pub fn radiant_intensity(&self) -> f64 {
        self.peak_candela() / LUMINOUS_EFFICACY
    }

    /// Candela toward vertical angle `v` and horizontal angle `h`
    /// (degrees), linear between the file's angles. Horizontal symmetry
    /// follows the file's last horizontal angle (0: all round, 90: four
    /// quadrants, 180: two halves, 360: none).
    pub fn sample(&self, v: f64, h: f64) -> f64 {
        let last = *self.horizontal.last().unwrap_or(&0.0);
        let h = h.rem_euclid(360.0);
        let h = if last <= 0.0 {
            0.0
        } else if last <= 90.0 {
            let q = h % 180.0;
            if q > 90.0 { 180.0 - q } else { q }
        } else if last <= 180.0 {
            if h > 180.0 { 360.0 - h } else { h }
        } else {
            h
        };
        let lerp = |angles: &[f64], x: f64| -> (usize, usize, f64) {
            if angles.len() == 1 || x <= angles[0] {
                return (0, 0, 0.0);
            }
            match angles.iter().position(|a| *a >= x) {
                None => (angles.len() - 1, angles.len() - 1, 0.0),
                Some(i) => {
                    let t = (x - angles[i - 1]) / (angles[i] - angles[i - 1]);
                    (i - 1, i, t)
                }
            }
        };
        let (h0, h1, th) = lerp(&self.horizontal, h);
        // Vertical angles past the file's range are dark.
        if v > *self.vertical.last().unwrap_or(&180.0) + 1e-9 || v < self.vertical[0] - 1e-9 {
            return 0.0;
        }
        let (v0, v1, tv) = lerp(&self.vertical, v);
        let at = |hi: usize| {
            let c = &self.candela[hi];
            c[v0] + (c[v1] - c[v0]) * tv
        };
        at(h0) + (at(h1) - at(h0)) * th
    }
}
