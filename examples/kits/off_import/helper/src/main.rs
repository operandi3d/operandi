//! The example kit's helper: turns an OFF file into an ASCII PLY file
//! Operandi reads itself (helper protocol 1, docs/KITS.md).
//!
//! Operandi runs `off-import-helper import <input.off> <output.ply>`. A
//! helper prints notes for the user as `note: ...` lines on stdout,
//! errors on stderr, and exits 0 only when it wrote the output.
use std::fmt::Write as _;
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 4 || args[1] != "import" {
        eprintln!("usage: off-import-helper import <input.off> <output.ply>");
        return ExitCode::from(2);
    }
    // Operandi says which protocol it speaks; refuse one we don't know.
    if let Ok(v) = std::env::var("OPERANDI_KIT_PROTOCOL")
        && v != "1"
    {
        eprintln!("This helper speaks kit helper protocol 1, not {v}");
        return ExitCode::from(2);
    }
    match convert(&args[2], &args[3]) {
        Ok((points, faces)) => {
            println!("note: Read {points} points and {faces} polygons");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("{e}");
            ExitCode::from(1)
        }
    }
}

fn convert(input: &str, output: &str) -> Result<(usize, usize), String> {
    let text = std::fs::read_to_string(input).map_err(|e| format!("Can't read {input}: {e}"))?;
    let (points, faces) = parse_off(&text)?;
    let mut ply = String::new();
    let _ = write!(
        ply,
        "ply\nformat ascii 1.0\ncomment written by the off_import example kit\nelement vertex {}\nproperty float x\nproperty float y\nproperty float z\nelement face {}\nproperty list uchar int vertex_indices\nend_header\n",
        points.len(),
        faces.len()
    );
    for [x, y, z] in &points {
        let _ = writeln!(ply, "{x} {y} {z}");
    }
    for face in &faces {
        let ids: Vec<String> = face.iter().map(usize::to_string).collect();
        let _ = writeln!(ply, "{} {}", face.len(), ids.join(" "));
    }
    std::fs::write(output, ply).map_err(|e| format!("Can't write {output}: {e}"))?;
    Ok((points.len(), faces.len()))
}

/// Points and polygons of an OFF file: an `OFF` line, then the point,
/// face and edge counts, the points, and one line per face (`n i0 i1 ...`,
/// optionally followed by a colour, which is ignored). `#` starts a comment.
fn parse_off(text: &str) -> Result<(Vec<[f64; 3]>, Vec<Vec<usize>>), String> {
    let mut lines = text
        .lines()
        .map(|l| l.split('#').next().unwrap_or("").trim())
        .filter(|l| !l.is_empty());
    let header = lines.next().ok_or("The file is empty")?;
    let rest = header
        .strip_prefix("OFF")
        .ok_or("Not an OFF file: it must start with OFF")?
        .trim();
    // The counts may share the OFF line or follow it.
    let counts_line = if rest.is_empty() {
        lines.next().ok_or("No counts after OFF")?
    } else {
        rest
    };
    let counts: Vec<usize> = counts_line
        .split_whitespace()
        .map(|n| n.parse().map_err(|_| format!("Bad count \"{n}\"")))
        .collect::<Result<_, _>>()?;
    let (n_points, n_faces) = match counts[..] {
        [p, f, ..] => (p, f),
        _ => return Err("The counts line needs point and face counts".into()),
    };
    let mut points = Vec::with_capacity(n_points.min(1 << 24));
    for i in 0..n_points {
        let line = lines.next().ok_or(format!("The file ends at point {i}"))?;
        let v: Vec<f64> = line
            .split_whitespace()
            .take(3)
            .map(|n| n.parse().map_err(|_| format!("Bad number \"{n}\" in point {i}")))
            .collect::<Result<_, _>>()?;
        let [x, y, z] = v[..] else {
            return Err(format!("Point {i} needs three numbers"));
        };
        points.push([x, y, z]);
    }
    let mut faces = Vec::with_capacity(n_faces.min(1 << 24));
    for i in 0..n_faces {
        let line = lines.next().ok_or(format!("The file ends at face {i}"))?;
        let mut words = line.split_whitespace();
        let n: usize = words
            .next()
            .and_then(|w| w.parse().ok())
            .ok_or(format!("Face {i} has no point count"))?;
        if !(3..=255).contains(&n) {
            return Err(format!("Face {i} has {n} points; this kit reads 3 to 255"));
        }
        let ids: Vec<usize> = words
            .take(n)
            .map(|w| w.parse().map_err(|_| format!("Bad index \"{w}\" in face {i}")))
            .collect::<Result<_, _>>()?;
        if ids.len() != n || ids.iter().any(|&id| id >= points.len()) {
            return Err(format!("Face {i} names a point the file doesn't have"));
        }
        faces.push(ids);
    }
    Ok((points, faces))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_a_cube_with_comments_and_counts_on_the_off_line() {
        let off = "OFF 8 6 12 # a cube\n0 0 0\n1 0 0\n1 1 0\n0 1 0\n0 0 1\n1 0 1\n1 1 1\n0 1 1\n4 0 3 2 1\n4 4 5 6 7\n4 0 1 5 4\n4 1 2 6 5\n4 2 3 7 6\n4 3 0 4 7 255 0 0\n";
        let (points, faces) = parse_off(off).unwrap();
        assert_eq!((points.len(), faces.len()), (8, 6));
        assert_eq!(faces[5], [3, 0, 4, 7]);
    }

    #[test]
    fn refuses_broken_files() {
        assert!(parse_off("").is_err());
        assert!(parse_off("PLY\n").is_err());
        assert!(parse_off("OFF\n3 1 0\n0 0 0\n1 0 0\n0 1 0\n3 0 1 9\n").is_err());
    }
}
