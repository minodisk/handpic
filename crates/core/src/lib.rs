//! Reusable ARW parsing and JPEG decoding shared by the CLI and the app.

pub mod arw;
pub mod candidate;
pub mod copy;
pub mod cr3;
pub mod decode;
pub mod dop;
pub mod embed;
mod exif;
pub mod eyes;
pub mod faces;
pub mod fullsharp;
pub mod genre;
mod hevc;
pub mod i18n;
pub mod jpeg;
pub mod nef;
pub mod orf;
pub mod partial;
pub mod pose;
pub mod raf;
pub mod rank;
pub mod reader;
pub mod scan;
pub mod sequence;
pub mod sharpness;
pub mod xmp;

/// A pick / reject judgment, independent of the `0`-`5` stars, as both
/// Lightroom (`xmpDM:good`) and DxO PhotoLab (`ShouldProcess`) model it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Flag {
    #[default]
    None,
    Pick,
    Reject,
}
