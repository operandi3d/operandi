//! Operandi's paintable images (3D painting stage 1, PNT-01..39 and
//! PNT-300..309): pixel buffers at their own depth, the image commands'
//! pixel work (invert, clear, resize, flatten) and reading and writing
//! image files, Photoshop documents included.
//!
//! Nothing here knows about scenes or the shader tree; `operandi-core`'s
//! `paint` module (behind its `painting` feature) ties images to them.
//! The whole crate is only linked when the app is built with
//! `--features painting`, so release and tester builds don't carry it.
pub mod blend;
pub mod image;
pub mod io;
pub mod psd;

pub use blend::{Blend, Source, composite};
pub use image::{
    DEFAULT_SIZE, Depth, Format, Layout, MAX_SIZE, MIN_SIZE, PaintImage, Pixels, Result,
    SQUARE_POW2_NOTE, check_side, is_square_pow2,
};
pub use io::FileType;
