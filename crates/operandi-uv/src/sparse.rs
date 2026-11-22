// SPDX-License-Identifier: GPL-2.0-or-later
//! Sparse linear solves for the unwrap solvers.
//!
//! Blender uses Eigen (`intern/eigen/linear_solver.cc`); here faer's sparse
//! Cholesky and LU do the same jobs. Only the parts the solvers need are
//! wrapped: build from triplets (duplicates are summed), solve, and a least
//! squares system with locked variables like Blender's
//! `EIG_linear_least_squares_solver`.

use faer::linalg::solvers::Solve;
use faer::sparse::{SparseColMat, Triplet};
use faer::{Mat, Side};

/// A square or rectangular matrix collected as (row, col, value) triplets.
/// Entries with the same row and column are added together.
#[derive(Default)]
pub(crate) struct Triplets {
    rows: usize,
    cols: usize,
    entries: Vec<Triplet<usize, usize, f64>>,
}

impl Triplets {
    pub(crate) fn new(rows: usize, cols: usize) -> Self {
        Self {
            rows,
            cols,
            entries: Vec::new(),
        }
    }

    pub(crate) fn add(&mut self, row: usize, col: usize, value: f64) {
        debug_assert!(row < self.rows && col < self.cols);
        if value != 0.0 {
            self.entries.push(Triplet::new(row, col, value));
        }
    }

    fn build(&self) -> Option<SparseColMat<usize, f64>> {
        SparseColMat::try_new_from_triplets(self.rows, self.cols, &self.entries).ok()
    }
}

fn finite(x: &[f64]) -> bool {
    x.iter().all(|v| v.is_finite())
}

fn solve_with(
    matrix: &SparseColMat<usize, f64>,
    rhs: &[Vec<f64>],
    spd: bool,
) -> Option<Vec<Vec<f64>>> {
    let n = matrix.nrows();
    let mut b = Mat::<f64>::zeros(n, rhs.len());
    for (c, col) in rhs.iter().enumerate() {
        for (r, v) in col.iter().enumerate() {
            b[(r, c)] = *v;
        }
    }
    if spd {
        let llt = matrix.sp_cholesky(Side::Lower).ok()?;
        llt.solve_in_place(b.as_mut());
    } else {
        let lu = matrix.sp_lu().ok()?;
        lu.solve_in_place(b.as_mut());
    }
    let out: Vec<Vec<f64>> = (0..rhs.len())
        .map(|c| (0..n).map(|r| b[(r, c)]).collect())
        .collect();
    out.iter().all(|c| finite(c)).then_some(out)
}

/// Solves the square system `A x = b` for each right-hand side. `spd` picks
/// Cholesky (symmetric positive definite, both triangles given) over LU.
/// Returns `None` when the factorization fails or the answer isn't finite.
pub(crate) fn solve(a: &Triplets, rhs: &[Vec<f64>], spd: bool) -> Option<Vec<Vec<f64>>> {
    if a.rows != a.cols || rhs.iter().any(|r| r.len() != a.rows) {
        return None;
    }
    if a.rows == 0 {
        return Some(rhs.iter().map(|_| Vec::new()).collect());
    }
    solve_with(&a.build()?, rhs, spd)
}

/// Least squares `min |A x|^2` over the free variables, with some variables
/// locked to given values, solved through the normal equations
/// (`EIG_linear_least_squares_solver` with a zero right-hand side).
pub(crate) struct LeastSquares {
    rows: usize,
    vars: usize,
    locked: Vec<Option<f64>>,
    entries: Vec<(usize, usize, f64)>,
}

impl LeastSquares {
    pub(crate) fn new(rows: usize, vars: usize) -> Self {
        Self {
            rows,
            vars,
            locked: vec![None; vars],
            entries: Vec::new(),
        }
    }

    pub(crate) fn lock(&mut self, var: usize, value: f64) {
        self.locked[var] = Some(value);
    }

    pub(crate) fn add(&mut self, row: usize, var: usize, value: f64) {
        debug_assert!(row < self.rows && var < self.vars);
        self.entries.push((row, var, value));
    }

    /// The value of every variable, or `None` when the system is singular.
    pub(crate) fn solve(&self) -> Option<Vec<f64>> {
        // Free variables get consecutive indices.
        let mut index = vec![usize::MAX; self.vars];
        let mut free = 0;
        for (v, lock) in self.locked.iter().enumerate() {
            if lock.is_none() {
                index[v] = free;
                free += 1;
            }
        }
        let mut result: Vec<f64> = self.locked.iter().map(|l| l.unwrap_or(0.0)).collect();
        if free == 0 {
            return Some(result);
        }

        // Group the entries by row, splitting each row into its free part
        // and the constant from locked variables.
        let mut by_row: Vec<Vec<(usize, f64)>> = vec![Vec::new(); self.rows];
        let mut constant = vec![0.0; self.rows];
        for &(r, v, value) in &self.entries {
            match self.locked[v] {
                Some(x) => constant[r] += value * x,
                None => by_row[r].push((index[v], value)),
            }
        }

        // Normal equations: (Af^T Af) x = -Af^T c.
        let mut ata = Triplets::new(free, free);
        let mut atb = vec![0.0; free];
        for (r, row) in by_row.iter().enumerate() {
            for &(i, a) in row {
                atb[i] -= a * constant[r];
                for &(j, b) in row {
                    ata.add(i, j, a * b);
                }
            }
        }
        let solved = solve(&ata, &[atb], true)?;
        for (v, &i) in index.iter().enumerate() {
            if i != usize::MAX {
                result[v] = solved[0][i];
            }
        }
        Some(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn duplicates_are_summed() {
        let mut a = Triplets::new(2, 2);
        a.add(0, 0, 1.0);
        a.add(0, 0, 1.0);
        a.add(1, 1, 4.0);
        let x = solve(&a, &[vec![2.0, 8.0]], true).unwrap();
        assert!((x[0][0] - 1.0).abs() < 1e-12 && (x[0][1] - 2.0).abs() < 1e-12);
        let x = solve(&a, &[vec![2.0, 8.0]], false).unwrap();
        assert!((x[0][0] - 1.0).abs() < 1e-12 && (x[0][1] - 2.0).abs() < 1e-12);
    }

    #[test]
    fn singular_matrix_is_none() {
        let mut a = Triplets::new(2, 2);
        a.add(0, 0, 1.0);
        a.add(0, 1, 1.0);
        a.add(1, 0, 1.0);
        a.add(1, 1, 1.0);
        assert!(solve(&a, &[vec![1.0, 1.0]], true).is_none());
    }

    #[test]
    fn least_squares_with_locks() {
        // x0 locked to 1, minimise (x1 - x0)^2 + (x2 - x1)^2 + (x2 - 3)^2 with x3 = 3 locked.
        let mut ls = LeastSquares::new(3, 4);
        ls.lock(0, 1.0);
        ls.lock(3, 3.0);
        ls.add(0, 1, 1.0);
        ls.add(0, 0, -1.0);
        ls.add(1, 2, 1.0);
        ls.add(1, 1, -1.0);
        ls.add(2, 2, 1.0);
        ls.add(2, 3, -1.0);
        let x = ls.solve().unwrap();
        assert!((x[1] - 5.0 / 3.0).abs() < 1e-9, "{x:?}");
        assert!((x[2] - 7.0 / 3.0).abs() < 1e-9, "{x:?}");
    }
}
