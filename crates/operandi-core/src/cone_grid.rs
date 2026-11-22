//! Quad Grid base cap. Provisional layout (cone-2026-11-05 Q-CN-02): it
//! matches the measured 5-, 8- and 12-sided caps (cone-2026-11-05 and
//! primitives-2026-11-02 cases). Odd sides use the grid for one more side
//! with one corner cut off, which leaves a single triangle.
use crate::{ConeParameters, Mesh};

pub(crate) fn append_grid_cap(mesh: &mut Mesh, parameters: &ConeParameters) {
    let n = parameters.sides as usize;
    let inner_start = mesh.vertices.len() as u32;
    let mut center = parameters.position;
    center[parameters.axis] -= parameters.size[parameters.axis];
    let phase = n / 8;
    let rho = (1. - 1.8 * std::f64::consts::PI / n as f64).max(0.5);
    for i in 0..n {
        let rim = mesh.vertices[(phase + i) % n];
        mesh.vertices.push(center + rho * (rim - center));
    }
    let inner = |i: usize| inner_start + (i % n) as u32;
    for i in 0..n {
        mesh.faces.push(super::cone::cone_face(vec![
            ((phase + i) % n) as u32,
            inner(i),
            inner(i + 1),
            ((phase + i + 1) % n) as u32,
        ]));
    }
    // Odd sides: lay out the grid for `n + 1` boundary points, the first a
    // missing corner. It only steers the interior points (it sits between
    // its two neighbours); its cell becomes the cap's one triangle.
    const MISSING: u32 = u32::MAX;
    let odd = n % 2 == 1;
    let m = n + usize::from(odd);
    let boundary = |k: usize| {
        if odd && k.is_multiple_of(m) {
            MISSING
        } else {
            inner(k)
        }
    };
    let p = m / 4;
    let q = p + usize::from(m % 4 == 2);
    let mut grid = vec![0u32; (p + 1) * (q + 1)];
    let at = |i: usize, j: usize| j * (p + 1) + i;
    for j in 0..=q {
        for i in 0..=p {
            grid[at(i, j)] = if i == 0 {
                boundary(j)
            } else if j == q {
                boundary(q + i)
            } else if i == p {
                boundary(2 * q + p - j)
            } else if j == 0 {
                boundary(m - i)
            } else {
                // Set once the complete boundary is available below.
                0
            };
        }
    }
    let position = |mesh: &Mesh, id: u32| {
        if id == MISSING {
            (mesh.vertices[inner(1) as usize] + mesh.vertices[inner(0) as usize]) / 2.
        } else {
            mesh.vertices[id as usize]
        }
    };
    let k00 = position(mesh, grid[at(0, 0)]);
    let k10 = position(mesh, grid[at(p, 0)]);
    let k01 = position(mesh, grid[at(0, q)]);
    let k11 = position(mesh, grid[at(p, q)]);
    let weight = |z: f64| 0.125 + 0.75 * (3. * z * z - 2. * z * z * z);
    for j in 1..q {
        for i in 1..p {
            let a = weight(i as f64 / p as f64);
            let b = weight(j as f64 / q as f64);
            let left = position(mesh, grid[at(0, j)]);
            let right = position(mesh, grid[at(p, j)]);
            let down = position(mesh, grid[at(i, 0)]);
            let up = position(mesh, grid[at(i, q)]);
            let point = (1. - a) * left + a * right + (1. - b) * down + b * up
                - ((1. - a) * (1. - b) * k00
                    + a * (1. - b) * k10
                    + (1. - a) * b * k01
                    + a * b * k11);
            grid[at(i, j)] = mesh.vertices.len() as u32;
            mesh.vertices.push(point);
        }
    }
    for j in 0..q {
        for i in 0..p {
            let cell = [
                grid[at(i + 1, j)],
                grid[at(i + 1, j + 1)],
                grid[at(i, j + 1)],
                grid[at(i, j)],
            ];
            mesh.faces.push(super::cone::cone_face(
                cell.into_iter().filter(|&v| v != MISSING).collect(),
            ));
        }
    }
}
