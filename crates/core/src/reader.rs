//! Read only as much of an ARW, DNG, NEF, CR3, RAF or ORF as the metadata and
//! the preview need. A JPEG file is its own preview and full-resolution JPEG.
//!
//! Every JPEG handed out has its Exif Orientation rewritten to 1, so
//! `Arw.orientation` is the only rotation a consumer applies, whatever the
//! container embeds.

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

use anyhow::{anyhow, Result};

use crate::arw::{self, Arw, Codec};
use crate::scan::is_jpeg_file;
use crate::{cr3, hevc, jpeg, nef, orf, raf};

/// How much of a file the bounded read takes.
///
/// On the α7 V files measured here every IFD, the ExifIFD, the MakerNote and
/// the end of the IFD0 preview sit within the first ~542 KB of a 48 MB ARW.
/// 1 MiB leaves close to twice that as margin while still reading ~1/48th of
/// the file; bodies that lay their preview out further in are covered by the
/// ranged re-read in `read_preview`.
pub const HEAD_LIMIT: usize = 1 << 20;

/// The error of a CR3 whose track is HEVC (shot with HDR PQ on) and whose
/// `PRVW` / `THMB` are neither a JPEG nor the HEVC `hevc::to_jpeg` decodes.
pub const HEVC_UNSUPPORTED: &str = "HDR PQ (HEIF) CR3: its HEVC preview is not supported yet";

/// Read at most `limit` bytes from the start of `path`.
pub fn read_head(path: &Path, limit: usize) -> Result<Vec<u8>> {
    head_of(&mut File::open(path)?, limit)
}

fn head_of(file: &mut File, limit: usize) -> Result<Vec<u8>> {
    let mut buf = Vec::new();
    file.take(limit as u64).read_to_end(&mut buf)?;
    Ok(buf)
}

/// Parse a file's metadata and return its IFD0 preview JPEG (an HEVC one
/// decoded to a JPEG), reading as little
/// as possible: a bounded prefix, plus a ranged read of the preview itself if
/// it lies beyond that prefix.
///
/// A prefix too short to parse is an error rather than a wrong answer, so in
/// that case the whole file is read and parsed instead.
pub fn read_preview(path: &Path) -> Result<(Arw, Vec<u8>)> {
    neutral(if is_jpeg_file(path) {
        read_jpeg(path)
    } else {
        read_embedded(path, Kind::Preview)
    })
}

/// Parse `buf` with the parser for `path`'s container, chosen by extension.
fn parse_raw(path: &Path, buf: &[u8]) -> Result<Arw> {
    let is = |ext: &str| {
        path.extension()
            .is_some_and(|e| e.eq_ignore_ascii_case(ext))
    };
    if is("nef") {
        nef::parse(buf)
    } else if is("cr3") {
        cr3::parse(buf)
    } else if is("raf") {
        raf::parse(buf)
    } else if is("orf") {
        orf::parse(buf)
    } else {
        arw::parse(buf)
    }
}

/// The same bounded read for the full-resolution JPEG (JpgFromRaw). Its
/// metadata is in the prefix but the JPEG itself is several MB long, so the
/// ranged read is the normal path here rather than a fallback.
pub fn read_full(path: &Path) -> Result<(Arw, Vec<u8>)> {
    neutral(if is_jpeg_file(path) {
        read_jpeg(path)
    } else {
        read_embedded(path, Kind::Full)
    })
}

/// The one exit of every JPEG `read_preview` and `read_full` hand out.
fn neutral(read: Result<(Arw, Vec<u8>)>) -> Result<(Arw, Vec<u8>)> {
    let (arw, mut jpeg) = read?;
    jpeg::neutralize_orientation(&mut jpeg);
    Ok((arw, jpeg))
}

fn read_jpeg(path: &Path) -> Result<(Arw, Vec<u8>)> {
    let buf = std::fs::read(path)?;
    Ok((jpeg::parse(&buf)?, buf))
}

#[derive(Clone, Copy)]
enum Kind {
    Preview,
    Full,
}

impl Kind {
    fn pick(self, arw: &Arw) -> Option<arw::Embedded> {
        match self {
            Kind::Preview => arw.preview,
            Kind::Full => arw.full,
        }
    }

    fn name(self) -> &'static str {
        match self {
            Kind::Preview => "preview",
            Kind::Full => "JpgFromRaw",
        }
    }
}

fn read_embedded(path: &Path, kind: Kind) -> Result<(Arw, Vec<u8>)> {
    let mut file = File::open(path)?;
    let head = head_of(&mut file, HEAD_LIMIT)?;
    // Anything past the prefix exists only if the file is longer than it.
    let bounded = head.len() == HEAD_LIMIT;

    let found = match embedded_from(path, &head, &mut file, bounded, kind) {
        Ok(found) => found,
        Err(_) if bounded => {
            let buf = std::fs::read(path)?;
            // The prefix error only explains a truncated read; once the whole
            // file is in memory, an error there describes what is actually
            // wrong with the file, so surface that one instead.
            embedded_from(path, &buf, &mut file, false, kind)?
        }
        Err(e) => return Err(e),
    };
    // Decode once, after the read is settled: a decode error is not a
    // truncated-read error and must not trigger the whole-file retry.
    let (arw, bytes, codec) = found;
    let jpeg = match codec {
        Codec::Jpeg => bytes,
        Codec::Hevc => hevc::to_jpeg(&bytes)?,
    };
    Ok((arw, jpeg))
}

/// Parse just a file's metadata, reading the same bounded prefix as
/// `read_preview` and falling back to the whole file when that prefix is too
/// short to parse. A JPEG's prefix is too short when it holds no complete
/// Exif segment.
pub fn read_metadata(path: &Path) -> Result<Arw> {
    let head = read_head(path, HEAD_LIMIT)?;
    let bounded = head.len() == HEAD_LIMIT;
    if is_jpeg_file(path) {
        if bounded && !jpeg::has_exif(&head) {
            return jpeg::parse(&std::fs::read(path)?);
        }
        return jpeg::parse(&head);
    }
    match parse_raw(path, &head) {
        Ok(arw) => Ok(arw),
        Err(_) if bounded => parse_raw(path, &std::fs::read(path)?),
        Err(e) => Err(e),
    }
}

fn embedded_from(
    path: &Path,
    buf: &[u8],
    file: &mut File,
    bounded: bool,
    kind: Kind,
) -> Result<(Arw, Vec<u8>, Codec)> {
    let arw = parse_raw(path, buf)?;
    let e = kind.pick(&arw).ok_or_else(|| {
        if arw.hevc {
            anyhow!(HEVC_UNSUPPORTED)
        } else {
            anyhow!("no embedded {}", kind.name())
        }
    })?;
    let end = e
        .offset
        .checked_add(e.length)
        .ok_or_else(|| anyhow!("{} out of range", kind.name()))?;
    let bytes = if end <= buf.len() {
        arw.slice(buf, e).to_vec()
    } else {
        if !bounded {
            return Err(anyhow!("{} out of range", kind.name()));
        }
        if end as u64 > file.metadata()?.len() {
            return Err(anyhow!("{} out of range", kind.name()));
        }
        let mut bytes = vec![0u8; e.length];
        file.seek(SeekFrom::Start(e.offset as u64))?;
        file.read_exact(&mut bytes)?;
        bytes
    };
    Ok((arw, bytes, e.codec))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_file(name: &str, bytes: &[u8]) -> std::path::PathBuf {
        let path =
            std::env::temp_dir().join(format!("handpic-reader-{}-{name}", std::process::id()));
        std::fs::write(&path, bytes).unwrap();
        path
    }

    #[test]
    fn reads_at_most_the_limit() {
        let path = temp_file("head", &[7u8; 100]);
        assert_eq!(read_head(&path, 10).unwrap(), [7u8; 10]);
        assert_eq!(read_head(&path, 1000).unwrap().len(), 100);
        std::fs::remove_file(&path).unwrap();
    }

    /// A TIFF whose IFD0 points at a preview that starts after `at`.
    fn arw_with_preview(at: usize, jpeg: &[u8]) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.extend_from_slice(b"II\x2a\x00");
        buf.extend_from_slice(&8u32.to_le_bytes());
        buf.extend_from_slice(&2u16.to_le_bytes());
        for (tag, value) in [(0x0201u16, at as u32), (0x0202, jpeg.len() as u32)] {
            buf.extend_from_slice(&tag.to_le_bytes());
            buf.extend_from_slice(&4u16.to_le_bytes());
            buf.extend_from_slice(&1u32.to_le_bytes());
            buf.extend_from_slice(&value.to_le_bytes());
        }
        buf.extend_from_slice(&0u32.to_le_bytes());
        buf.resize(at, 0);
        buf.extend_from_slice(jpeg);
        buf
    }

    #[test]
    fn preview_inside_the_prefix_is_sliced() {
        let jpeg = [1u8, 2, 3, 4];
        let path = temp_file("near", &arw_with_preview(64, &jpeg));
        let (_, out) = read_preview(&path).unwrap();
        assert_eq!(out, jpeg);
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn preview_past_the_prefix_is_read_by_range() {
        let jpeg = [9u8; 32];
        let path = temp_file("far", &arw_with_preview(HEAD_LIMIT + 4096, &jpeg));
        let (_, out) = read_preview(&path).unwrap();
        assert_eq!(out, jpeg);
        std::fs::remove_file(&path).unwrap();
    }

    /// A TIFF whose IFD0 holds a tiny preview and whose IFD1 points at a
    /// larger JPEG, i.e. the JpgFromRaw, starting after `at`.
    fn arw_with_full(at: usize, jpeg: &[u8]) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.extend_from_slice(b"II\x2a\x00");
        buf.extend_from_slice(&8u32.to_le_bytes());
        let ifd1_at = 8 + 2 + 2 * 12 + 4;
        for (entries, next) in [
            ([(0x0201u16, 8u32), (0x0202, 1)], ifd1_at as u32),
            ([(0x0201, at as u32), (0x0202, jpeg.len() as u32)], 0),
        ] {
            buf.extend_from_slice(&2u16.to_le_bytes());
            for (tag, value) in entries {
                buf.extend_from_slice(&tag.to_le_bytes());
                buf.extend_from_slice(&4u16.to_le_bytes());
                buf.extend_from_slice(&1u32.to_le_bytes());
                buf.extend_from_slice(&value.to_le_bytes());
            }
            buf.extend_from_slice(&next.to_le_bytes());
        }
        buf.resize(at, 0);
        buf.extend_from_slice(jpeg);
        buf
    }

    #[test]
    fn the_full_jpeg_inside_the_prefix_is_sliced() {
        let jpeg = [3u8; 64];
        let path = temp_file("full-near", &arw_with_full(128, &jpeg));
        let (_, out) = read_full(&path).unwrap();
        assert_eq!(out, jpeg);
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn the_full_jpeg_past_the_prefix_is_read_by_range() {
        let jpeg = [4u8; 4096];
        let path = temp_file("full-far", &arw_with_full(HEAD_LIMIT - 1024, &jpeg));
        let (_, out) = read_full(&path).unwrap();
        assert_eq!(out, jpeg);
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn a_file_without_a_full_jpeg_is_an_error() {
        let path = temp_file("full-none", &arw_with_preview(64, &[1u8, 2, 3, 4]));
        assert!(read_full(&path).is_err());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn a_jpeg_is_its_own_preview_and_full_jpeg_with_a_neutral_orientation() {
        use crate::jpeg::tests::{plain_jpeg, with_exif, W};
        let w = W(false);
        let plain = plain_jpeg(64, 48);
        let file = with_exif(&plain, &w.tiff(&[w.short(0x0112, 8)], &[]));
        let neutral = with_exif(&plain, &w.tiff(&[w.short(0x0112, 1)], &[]));
        let path = temp_file("jpeg.JPG", &file);
        let (a, out) = read_preview(&path).unwrap();
        assert_eq!((a.orientation, out), (8, neutral.clone()));
        let (a, out) = read_full(&path).unwrap();
        assert_eq!((a.orientation, out), (8, neutral));
        assert_eq!(read_metadata(&path).unwrap().orientation, 8);
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn a_jpeg_whose_exif_is_past_the_prefix_reads_the_whole_file() {
        use crate::jpeg::tests::{plain_jpeg, with_exif, W};
        let w = W(true);
        let mut file = plain_jpeg(16, 16)[..2].to_vec();
        let mut filler = vec![0xFF, 0xE2, 0xFF, 0xFF];
        filler.resize(0xFFFF + 2, 0);
        while file.len() < HEAD_LIMIT {
            file.extend_from_slice(&filler);
        }
        let tail = with_exif(&plain_jpeg(16, 16), &w.tiff(&[w.short(0x0112, 6)], &[]));
        file.extend_from_slice(&tail[2..]);
        let path = temp_file("far.jpeg", &file);
        assert_eq!(read_metadata(&path).unwrap().orientation, 6);
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn a_file_named_jpg_that_is_not_a_jpeg_is_an_error() {
        let path = temp_file("fake.jpg", &arw_with_preview(64, &[1u8, 2, 3, 4]));
        assert!(read_preview(&path).is_err());
        assert!(read_metadata(&path).is_err());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn a_nef_reads_its_sub_ifd_jpegs_by_range_past_the_prefix() {
        use crate::jpeg::tests::W;
        use crate::nef::tests::{jpeg_sub, nef};
        let w = W(false);
        let (preview, full) = ([6u8; 64], [7u8; 4096]);
        let (preview_at, full_at) = (4096, HEAD_LIMIT + 4096);
        let mut file = nef(
            &w,
            &[w.short(0x0112, 6)],
            &[],
            &[
                jpeg_sub(&w, full_at, full.len()),
                jpeg_sub(&w, preview_at, preview.len()),
            ],
        );
        file.resize(preview_at, 0);
        file.extend_from_slice(&preview);
        file.resize(full_at, 0);
        file.extend_from_slice(&full);
        let path = temp_file("far.NEF", &file);
        let (a, out) = read_preview(&path).unwrap();
        assert_eq!((a.orientation, out), (6, preview.to_vec()));
        let (_, out) = read_full(&path).unwrap();
        assert_eq!(out, full);
        assert_eq!(read_metadata(&path).unwrap().orientation, 6);
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn a_nef_whose_sub_ifds_are_past_the_prefix_reads_the_whole_file() {
        use crate::jpeg::tests::W;
        use crate::nef::tests::{jpeg_sub, nef};
        let w = W(true);
        let jpeg = [8u8; 16];
        let jpeg_at = HEAD_LIMIT + 8192;
        let tiff = nef(
            &w,
            &[w.short(0x0112, 3)],
            &[],
            &[jpeg_sub(&w, jpeg_at, jpeg.len())],
        );
        // Move the lone SubIFD past the prefix and point IFD0's entry at it.
        let sub_at = tiff.len() - (2 + 12 * 4 + 4);
        let entry = tiff
            .windows(4)
            .position(|b| b == w.u32(sub_at as u32))
            .unwrap();
        let mut file = tiff[..sub_at].to_vec();
        file[entry..entry + 4].copy_from_slice(&w.u32((HEAD_LIMIT + 16) as u32));
        file.resize(HEAD_LIMIT + 16, 0);
        file.extend_from_slice(&tiff[sub_at..]);
        file.resize(jpeg_at, 0);
        file.extend_from_slice(&jpeg);
        let path = temp_file("deep.nef", &file);
        let (a, out) = read_full(&path).unwrap();
        assert_eq!((a.orientation, out), (3, jpeg.to_vec()));
        assert!(read_metadata(&path).unwrap().full.is_some());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn a_nef_whose_maker_note_is_past_the_prefix_reads_the_whole_file() {
        use crate::jpeg::tests::W;
        use crate::nef::tests::{af_info2, jpeg_sub, maker_note, nef};
        let w = W(true);
        let values = [5568, 3712, 3776, 1741, 552, 540];
        let note = maker_note(&w, &[af_info2(&w, b"0402", 1, values)]);
        let (_, _, note_len, note_bytes) = note.clone();
        let tiff = nef(
            &w,
            &[w.short(0x0112, 6)],
            &[note],
            &[jpeg_sub(&w, 1000, 900)],
        );
        // Find where IFD0's TIFF header (the entry's inline value field, four
        // bytes holding the MakerNote's offset) points, then move the
        // MakerNote past the prefix so `at + count` in `nef::af_point` runs
        // past the prefix `buf` while still inside the whole file.
        let old_at = tiff.windows(6).position(|b| b == b"Nikon\0").unwrap();
        let entry = tiff
            .windows(4)
            .position(|b| b == w.u32(old_at as u32))
            .unwrap();
        let new_at = HEAD_LIMIT + 4096;
        // Leave the original bytes at `old_at` in place (harmless, unused
        // once the pointer is retargeted) and grow the file so the note also
        // lives at `new_at`, past the prefix `read_metadata` bounds itself to.
        let mut file = tiff.clone();
        file[entry..entry + 4].copy_from_slice(&w.u32(new_at as u32));
        file.resize(new_at, 0);
        file.extend_from_slice(&note_bytes[..note_len as usize]);
        let path = temp_file("deep-note.nef", &file);
        assert!(read_metadata(&path).unwrap().shot.focus.is_some());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn a_cr3_reads_its_prvw_and_jpeg_track_by_range_past_the_prefix() {
        use crate::cr3::tests::{jpeg_bytes, Cr3};
        let c = Cr3 {
            preview: Some(jpeg_bytes(HEAD_LIMIT, 6)),
            ..Cr3::new()
        };
        let file = c.build();
        let path = temp_file("far.CR3", &file);
        let (_, out) = read_preview(&path).unwrap();
        assert_eq!(Some(out), c.preview);
        let (_, out) = read_full(&path).unwrap();
        assert_eq!(out, c.full);
        assert!(read_metadata(&path).unwrap().preview.is_some());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn a_cr3_whose_moov_is_past_the_prefix_reads_the_whole_file() {
        use crate::cr3::tests::Cr3;
        use crate::jpeg::tests::W;
        let w = W(true);
        let c = Cr3 {
            ifd0: vec![w.short(0x0112, 6)],
            pad: HEAD_LIMIT,
            ..Cr3::new()
        };
        let path = temp_file("deep.cr3", &c.build());
        let (a, out) = read_preview(&path).unwrap();
        assert_eq!((a.orientation, Some(out)), (6, c.preview.clone()));
        assert_eq!(read_metadata(&path).unwrap().orientation, 6);
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn an_hdr_pq_cr3_names_its_hevc_preview_and_keeps_its_metadata() {
        use crate::cr3::tests::Cr3;
        use crate::jpeg::tests::W;
        let w = W(true);
        let c = Cr3 {
            ifd0: vec![w.short(0x0112, 6)],
            exif: vec![w.ascii(0x9003, "2026:09:30 10:00:00")],
            thumbnail: Some(vec![0; 16]),
            preview: Some(vec![0; 64]),
            full: vec![0; 256],
            full_sub: *b"HEVC",
            ..Cr3::new()
        };
        let path = temp_file("hdr.CR3", &c.build());
        for err in [read_preview(&path), read_full(&path)] {
            assert_eq!(err.unwrap_err().to_string(), HEVC_UNSUPPORTED);
        }
        let a = read_metadata(&path).unwrap();
        assert_eq!(a.orientation, 6);
        assert_eq!(a.shot.capture_time.as_deref(), Some("2026:09:30 10:00:00"));
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn an_hdr_pq_cr3_hands_its_hevc_prvw_to_the_decoder_near_and_by_range() {
        use crate::cr3::tests::hdr_pq;
        for (name, pad) in [("hevc-near.CR3", 64), ("hevc-far.CR3", HEAD_LIMIT)] {
            // A lone `free` box: the whole payload reaches `hevc::to_jpeg`,
            // which finds no `hvcC` in it.
            let mut data = ((8 + pad) as u32).to_be_bytes().to_vec();
            data.extend_from_slice(b"free");
            data.resize(8 + pad, 0);
            let path = temp_file(name, &hdr_pq(None, Some(data)).build());
            for err in [read_preview(&path), read_full(&path)] {
                assert_eq!(err.unwrap_err().to_string(), "HEVC preview without hvcC");
            }
            assert!(read_metadata(&path).unwrap().hevc);
            std::fs::remove_file(&path).unwrap();
        }
    }

    #[test]
    fn a_cr3_without_any_image_has_no_embedded_preview() {
        use crate::cr3::tests::Cr3;
        let c = Cr3 {
            thumbnail: None,
            preview: None,
            full: vec![0; 256],
            full_sub: *b"CMP1",
            ..Cr3::new()
        };
        let path = temp_file("empty.CR3", &c.build());
        let err = read_preview(&path).unwrap_err().to_string();
        assert_eq!(err, "no embedded preview");
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn a_raf_reads_its_jpeg_by_range_past_the_prefix() {
        use crate::jpeg::tests::W;
        use crate::raf::tests::{exif_jpeg, raf, JPEG_AT};
        let far = |orientation| {
            let mut jpeg = exif_jpeg(&W(false), orientation);
            jpeg.resize(HEAD_LIMIT + 4096, 7);
            jpeg
        };
        let neutral = far(1);
        let path = temp_file("far.RAF", &raf(JPEG_AT, &far(6)));
        let (a, out) = read_preview(&path).unwrap();
        assert_eq!(a.orientation, 6);
        assert!(out == neutral);
        let (_, out) = read_full(&path).unwrap();
        assert!(out == neutral);
        assert_eq!(read_metadata(&path).unwrap().orientation, 6);
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn a_raf_whose_exif_is_past_the_prefix_reads_the_whole_file() {
        use crate::jpeg::tests::{plain_jpeg, W};
        use crate::raf::tests::{exif_jpeg, raf, JPEG_AT};
        let deep = |orientation| {
            let mut jpeg = plain_jpeg(16, 16)[..2].to_vec();
            let mut filler = vec![0xFF, 0xE2, 0xFF, 0xFF];
            filler.resize(0xFFFF + 2, 0);
            while jpeg.len() < HEAD_LIMIT {
                jpeg.extend_from_slice(&filler);
            }
            jpeg.extend_from_slice(&exif_jpeg(&W(true), orientation)[2..]);
            jpeg
        };
        let path = temp_file("deep.raf", &raf(JPEG_AT, &deep(8)));
        assert_eq!(read_metadata(&path).unwrap().orientation, 8);
        let (a, out) = read_preview(&path).unwrap();
        assert_eq!(a.orientation, 8);
        assert!(out == deep(1));
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn an_orf_reads_its_maker_note_preview_by_range_past_the_prefix() {
        use crate::jpeg::tests::W;
        use crate::orf::tests::{maker_note, note_at, orf, preview_fields};
        let w = W(true);
        let jpeg = [5u8; 4096];
        let jpeg_at = HEAD_LIMIT + 4096;
        let build = |start: u32| {
            orf(
                &w,
                &[w.short(0x0112, 6)],
                &[maker_note(
                    &w,
                    false,
                    0,
                    &preview_fields(&w, start, jpeg.len() as u32),
                )],
            )
        };
        let mut file = build((jpeg_at - note_at(&build(0))) as u32);
        file.resize(jpeg_at, 0);
        file.extend_from_slice(&jpeg);
        let path = temp_file("far.ORF", &file);
        let (a, out) = read_preview(&path).unwrap();
        assert_eq!((a.orientation, out), (6, jpeg.to_vec()));
        let (_, out) = read_full(&path).unwrap();
        assert_eq!(out, jpeg);
        assert!(read_metadata(&path).unwrap().preview.is_some());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn an_orf_whose_camera_settings_are_past_the_prefix_reads_the_whole_file() {
        use crate::jpeg::tests::W;
        use crate::orf::tests::{maker_note, note_at, orf, preview_fields};
        let w = W(false);
        let jpeg = [6u8; 64];
        let build = |start: u32| {
            orf(
                &w,
                &[w.short(0x0112, 3)],
                &[maker_note(
                    &w,
                    true,
                    HEAD_LIMIT,
                    &preview_fields(&w, start, jpeg.len() as u32),
                )],
            )
        };
        let probe = build(0);
        let mut file = build((probe.len() - note_at(&probe)) as u32);
        file.extend_from_slice(&jpeg);
        let path = temp_file("deep.orf", &file);
        let (a, out) = read_preview(&path).unwrap();
        assert_eq!((a.orientation, out), (3, jpeg.to_vec()));
        assert!(read_metadata(&path).unwrap().preview.is_some());
        std::fs::remove_file(&path).unwrap();
    }

    /// A JPEG whose Exif carries `orientation`, in `le`'s byte order.
    fn oriented_jpeg(le: bool, orientation: u16, width: usize, height: usize) -> Vec<u8> {
        use crate::jpeg::tests::{plain_jpeg, with_exif, W};
        let w = W(le);
        with_exif(
            &plain_jpeg(width, height),
            &w.tiff(&[w.short(0x0112, orientation)], &[]),
        )
    }

    /// Both JPEGs `path` hands out carry no rotation of their own (Orientation
    /// 1, or no Exif at all), while its orientation stays `orientation`.
    fn assert_neutral(path: &Path, orientation: u16) {
        type Read = fn(&Path) -> Result<(Arw, Vec<u8>)>;
        for (name, read) in [("preview", read_preview as Read), ("full", read_full)] {
            let (a, out) = read(path).unwrap();
            let what = format!("{} {name}", path.display());
            assert_eq!(a.orientation, orientation, "{what}");
            if let Ok((o, _)) = jpeg::read_exif(&out) {
                assert_eq!(o, 1, "{what}");
            }
        }
    }

    /// A TIFF in `w`'s byte order whose IFD0 holds `orientation` and
    /// `preview`, and whose IFD1 the larger `full`.
    fn arw_with_both(
        w: &crate::jpeg::tests::W,
        orientation: u16,
        preview: &[u8],
        full: &[u8],
    ) -> Vec<u8> {
        let mut buf = if w.0 { b"II".to_vec() } else { b"MM".to_vec() };
        buf.extend_from_slice(&w.u16(42));
        buf.extend_from_slice(&w.u32(8));
        let ifd1_at = 8 + 2 + 3 * 12 + 4;
        let preview_at = ifd1_at + 2 + 2 * 12 + 4;
        let full_at = preview_at + preview.len();
        let ifd0: &[(u16, u16, [u8; 4])] = &[
            (
                0x0112,
                3,
                [w.u16(orientation), [0, 0]].concat().try_into().unwrap(),
            ),
            (0x0201, 4, w.u32(preview_at as u32)),
            (0x0202, 4, w.u32(preview.len() as u32)),
        ];
        let ifd1: &[(u16, u16, [u8; 4])] = &[
            (0x0201, 4, w.u32(full_at as u32)),
            (0x0202, 4, w.u32(full.len() as u32)),
        ];
        for (entries, next) in [(ifd0, ifd1_at as u32), (ifd1, 0)] {
            buf.extend_from_slice(&w.u16(entries.len() as u16));
            for (tag, typ, value) in entries {
                buf.extend_from_slice(&w.u16(*tag));
                buf.extend_from_slice(&w.u16(*typ));
                buf.extend_from_slice(&w.u32(1));
                buf.extend_from_slice(value);
            }
            buf.extend_from_slice(&w.u32(next));
        }
        buf.extend_from_slice(preview);
        buf.extend_from_slice(full);
        buf
    }

    /// Every container, a future one included, must hand out its JPEGs
    /// through `neutral`: a list entry per supported format.
    #[test]
    fn every_format_hands_out_jpegs_without_an_exif_rotation() {
        use crate::cr3::tests::Cr3;
        use crate::jpeg::tests::W;
        use crate::nef::tests::{jpeg_sub, nef};
        use crate::orf::tests::{maker_note, note_at, orf, preview_fields};
        use crate::raf::tests::{exif_jpeg, raf, JPEG_AT};

        let mut files: Vec<(&str, Vec<u8>, u16)> = Vec::new();

        // A DNG goes through the same parser as an ARW.
        for (name, le) in [("inv.ARW", true), ("inv.DNG", false)] {
            let preview = oriented_jpeg(le, 8, 16, 16);
            let full = oriented_jpeg(le, 8, 64, 48);
            files.push((name, arw_with_both(&W(le), 6, &preview, &full), 6));
        }

        let w = W(false);
        let preview = oriented_jpeg(true, 6, 16, 16);
        let full = oriented_jpeg(false, 6, 64, 48);
        let (preview_at, full_at) = (4096, 8192);
        let mut file = nef(
            &w,
            &[w.short(0x0112, 6)],
            &[],
            &[
                jpeg_sub(&w, full_at, full.len()),
                jpeg_sub(&w, preview_at, preview.len()),
            ],
        );
        file.resize(preview_at, 0);
        file.extend_from_slice(&preview);
        file.resize(full_at, 0);
        file.extend_from_slice(&full);
        files.push(("inv.NEF", file, 6));

        let w = W(true);
        let c = Cr3 {
            ifd0: vec![w.short(0x0112, 8)],
            preview: Some(oriented_jpeg(false, 8, 16, 16)),
            full: oriented_jpeg(true, 8, 64, 48),
            ..Cr3::new()
        };
        files.push(("inv.CR3", c.build(), 8));

        files.push(("inv.RAF", raf(JPEG_AT, &exif_jpeg(&W(false), 8)), 8));

        let jpeg = oriented_jpeg(true, 6, 16, 16);
        let build = |start: u32| {
            orf(
                &w,
                &[w.short(0x0112, 6)],
                &[maker_note(
                    &w,
                    false,
                    0,
                    &preview_fields(&w, start, jpeg.len() as u32),
                )],
            )
        };
        let probe = build(0);
        let mut file = build((probe.len() - note_at(&probe)) as u32);
        file.extend_from_slice(&jpeg);
        files.push(("inv.ORF", file, 6));

        files.push(("inv.jpg", oriented_jpeg(false, 8, 16, 16), 8));

        for (name, file, orientation) in files {
            let path = temp_file(name, &file);
            assert_neutral(&path, orientation);
            std::fs::remove_file(&path).unwrap();
        }
    }

    /// The HEVC of a CR3 shot with HDR PQ on, decoded to a JPEG, holds to the
    /// same. No HEVC stream is synthesized here, so set `HANDPIC_HEVC_CR3` to
    /// such a file, then run with `--ignored`.
    #[test]
    #[ignore]
    fn an_hdr_pq_cr3_hands_out_jpegs_without_an_exif_rotation() {
        let path = std::env::var("HANDPIC_HEVC_CR3").expect("HANDPIC_HEVC_CR3");
        let path = Path::new(&path);
        let a = read_metadata(path).unwrap();
        assert!(a.hevc);
        assert_neutral(path, a.orientation);
    }

    #[test]
    fn a_preview_past_the_end_of_the_file_is_an_error() {
        let mut buf = arw_with_preview(HEAD_LIMIT + 4096, &[5u8; 16]);
        buf.truncate(HEAD_LIMIT + 4096);
        let path = temp_file("short", &buf);
        assert!(read_preview(&path).is_err());
        std::fs::remove_file(&path).unwrap();
    }
}
