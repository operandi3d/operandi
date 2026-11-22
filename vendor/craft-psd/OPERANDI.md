# craft-psd in Operandi

`src/` is craft-libs' `crates/psd/src` at commit
`d6b5b16bbb1448193e246ff640adf7ab7ae45037`
(<https://github.com/storytold/craft-libs>), copied unchanged. It came
from PhotoCraft. `LICENSE-MIT`, `LICENSE-APACHE` and `NOTICE` are the
craft-libs files, unchanged. Licence: MIT OR Apache-2.0, Copyright (c)
2026 ArtCraft Team and the craft-libs contributors; both licences fit
Operandi's GPL-3.0-or-later. The craft-libs tests and fixtures are not
copied. The ArtCraft name and logos are not part of the licence.

`Cargo.toml` is new: it names the dependencies the craft-libs workspace
manifest gave the crate (thiserror 2, flate2 with zlib-rs, encoding_rs,
rayon, optional serde) and drops its dev-dependencies. flate2 uses its
default backend (miniz_oxide) instead of zlib-rs, so adding this crate
doesn't change how core's 3MF writer compresses.

Only `operandi-paint` uses it (PSD/PSB images for 3D painting, PNT-300..309),
and only builds with the `painting` feature link it. It is kept out of
the workspace's members (root `Cargo.toml`, `exclude`), so the workspace's
clippy and tests don't run on it.

To update: copy `crates/psd/src` from a newer commit, compare the
craft-libs workspace manifest for dependency changes, and update the
commit above and in docs/DEPENDENCIES.md.

`rustfmt.toml` is craft-libs' own, so `cargo fmt --all` (which reaches path
dependencies) leaves the sources as they came.
