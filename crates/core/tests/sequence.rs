mod common;

use handpic_core::sequence::{self, output_dir, run, Summary};
use std::collections::BTreeMap;
use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

fn setup(names: &[&str]) -> tempfile::TempDir {
    let root = tempfile::tempdir().unwrap();
    let dir = src(&root);
    fs::create_dir(&dir).unwrap();
    for name in names {
        fs::write(dir.join(name), common::sample_jpeg()).unwrap();
    }
    root
}

/// The source folder inside a test's temporary root, so its `-sequenced`
/// sibling lands inside the root too.
fn src(root: &tempfile::TempDir) -> PathBuf {
    root.path().join("export")
}

fn out(root: &tempfile::TempDir) -> PathBuf {
    output_dir(&src(root)).unwrap()
}

fn write(root: &tempfile::TempDir, name: &str, bytes: Vec<u8>) {
    let dir = src(root);
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join(name), bytes).unwrap();
}

fn run_with(dir: &Path, dry_run: bool) -> Summary {
    run(dir, dry_run, &AtomicBool::new(false), |_, _, _, _| {}).unwrap()
}

fn run_all(dir: &Path) -> Summary {
    let summary = run_with(dir, false);
    for (path, result) in &summary.results {
        assert!(
            result.is_ok(),
            "failed to process {}: {:?}",
            path.display(),
            result.as_ref().err()
        );
    }
    summary
}

fn names(summary: &Summary) -> Vec<String> {
    summary
        .results
        .iter()
        .map(|(p, _)| p.file_name().unwrap().to_string_lossy().into_owned())
        .collect()
}

fn listing(dir: &Path) -> BTreeMap<String, Vec<u8>> {
    fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.is_file())
        .map(|p| {
            (
                p.file_name().unwrap().to_string_lossy().into_owned(),
                fs::read(&p).unwrap(),
            )
        })
        .collect()
}

/// Returns everything from the SOS marker on (the entropy-coded scan data and
/// EOI). Simplified implementation assuming 0xFFDA never appears inside APP1
/// in the sample JPEGs.
fn scan_data(buf: &[u8]) -> &[u8] {
    let pos = buf
        .windows(2)
        .position(|w| w == [0xFF, 0xDA])
        .expect("SOS marker not found");
    &buf[pos..]
}

fn exif_fields(buf: &[u8]) -> BTreeMap<String, String> {
    let exif = exif::Reader::new()
        .read_from_container(&mut Cursor::new(buf))
        .unwrap();
    exif.fields()
        .map(|f| {
            (
                format!("{:?}/{}", f.ifd_num, f.tag),
                f.display_value().to_string(),
            )
        })
        .collect()
}

fn read_ascii_tag(buf: &[u8], tag: exif::Tag) -> String {
    let exif = exif::Reader::new()
        .read_from_container(&mut Cursor::new(buf))
        .unwrap();
    let field = exif
        .get_field(tag, exif::In::PRIMARY)
        .unwrap_or_else(|| panic!("{tag} is missing"));
    match &field.value {
        exif::Value::Ascii(v) => String::from_utf8(v[0].clone()).unwrap(),
        other => panic!("{tag} is not ASCII: {other:?}"),
    }
}

fn output_dto(root: &tempfile::TempDir, name: &str) -> String {
    let buf = fs::read(out(root).join(name)).unwrap();
    read_ascii_tag(&buf, exif::Tag::DateTimeOriginal)
}

fn assert_exif_datetime_format(s: &str) {
    assert_eq!(s.len(), 19, "date-time is not 19 bytes long: {s:?}");
    for (i, c) in s.char_indices() {
        match i {
            4 | 7 | 13 | 16 => assert_eq!(c, ':', "position {i} is not a colon: {s:?}"),
            10 => assert_eq!(c, ' ', "position 10 is not a space: {s:?}"),
            _ => assert!(c.is_ascii_digit(), "position {i} is not a digit: {s:?}"),
        }
    }
}

/// Lossless guarantee: the copy has the source's length and differs only in
/// the value areas of the target date-time tags (19 bytes x 3 locations);
/// the scan data is identical and the decoded pixels match bit for bit.
#[test]
fn test_lossless_bytes_and_pixels() {
    let root = setup(&["a.jpg", "b.jpg"]);
    let before = fs::read(src(&root).join("b.jpg")).unwrap();

    run_all(&src(&root));
    let after = fs::read(out(&root).join("b.jpg")).unwrap();

    // b.jpg comes second, so it gets +1 second and must differ
    assert_ne!(before, after, "no rewrite happened");
    assert_eq!(before.len(), after.len(), "file length changed");

    let offsets = sequence::find_datetime_offsets(&before).unwrap();
    let allowed: Vec<std::ops::Range<usize>> = offsets
        .all()
        .into_iter()
        .map(|o| o..o + sequence::DATETIME_LEN)
        .collect();
    for (i, (b, a)) in before.iter().zip(after.iter()).enumerate() {
        if b != a {
            assert!(
                allowed.iter().any(|r| r.contains(&i)),
                "diff outside the date-time tags at position {i} (0x{b:02X} -> 0x{a:02X})"
            );
        }
    }

    assert_eq!(
        scan_data(&before),
        scan_data(&after),
        "scan data changed (suspected re-encode)"
    );

    let pixels_before = image::load_from_memory(&before).unwrap().to_rgb8();
    let pixels_after = image::load_from_memory(&after).unwrap().to_rgb8();
    assert_eq!(
        pixels_before.as_raw(),
        pixels_after.as_raw(),
        "decoded pixels changed"
    );
}

/// EXIF tags other than the date-times (Make, Model, PixelX/YDimension, GPS,
/// MakerNote, SubSecTimeOriginal) do not change in the copy.
#[test]
fn test_other_exif_tags_preserved() {
    let root = tempfile::tempdir().unwrap();
    write(
        &root,
        "a.jpg",
        common::sample_jpeg_with_subsec(common::BASE_DATETIME, Some("1")),
    );
    write(
        &root,
        "b.jpg",
        common::sample_jpeg_with_subsec(common::BASE_DATETIME, Some("123456")),
    );
    let before_fields = exif_fields(&fs::read(src(&root).join("b.jpg")).unwrap());

    run_all(&src(&root));
    let after_fields = exif_fields(&fs::read(out(&root).join("b.jpg")).unwrap());

    let before_keys: Vec<_> = before_fields.keys().collect();
    let after_keys: Vec<_> = after_fields.keys().collect();
    assert_eq!(before_keys, after_keys, "tag set changed");

    for (key, before_value) in &before_fields {
        if key.contains("DateTime") {
            continue;
        }
        assert_eq!(
            before_value, &after_fields[key],
            "non-date-time tag {key} changed"
        );
    }

    let has = |name: &str| before_fields.keys().any(|k| k.contains(name));
    let value_of = |name: &str| {
        after_fields
            .iter()
            .find(|(k, _)| k.contains(name))
            .map(|(_, v)| v.clone())
            .unwrap_or_else(|| panic!("{name} is missing"))
    };
    assert!(value_of("Make").contains(common::MAKE));
    assert!(value_of("Model").contains(common::MODEL));
    assert_eq!(value_of("PixelXDimension"), common::WIDTH.to_string());
    assert_eq!(value_of("PixelYDimension"), common::HEIGHT.to_string());
    assert!(value_of("SubSecTimeOriginal").contains("123456"));
    assert!(has("GPSLatitude"), "GPSLatitude disappeared");
    assert!(has("MakerNote"), "MakerNote disappeared");
}

/// Burst shots collapsed into the same second are pushed to +0, +1, +2
/// seconds with minute carry-over, in natural filename order when the times
/// tie, and 0x9004 / 0x0132 follow 0x9003.
#[test]
fn test_datetime_updated_with_minute_carry_over_and_sync() {
    let root = setup(&["img_2.jpg", "img_10.jpg", "img_1.jpg"]);
    let summary = run_all(&src(&root));
    assert_eq!(names(&summary), ["img_1.jpg", "img_2.jpg", "img_10.jpg"]);

    let expected = [
        ("img_1.jpg", "2024:01:02 03:04:59"),
        ("img_2.jpg", "2024:01:02 03:05:00"),
        ("img_10.jpg", "2024:01:02 03:05:01"),
    ];
    for (name, want) in expected {
        let buf = fs::read(out(&root).join(name)).unwrap();
        let dto = read_ascii_tag(&buf, exif::Tag::DateTimeOriginal);
        assert_eq!(dto, want, "unexpected DateTimeOriginal for {name}");
        assert_exif_datetime_format(&dto);
        assert_eq!(read_ascii_tag(&buf, exif::Tag::DateTimeDigitized), want);
        assert_eq!(read_ascii_tag(&buf, exif::Tag::DateTime), want);
    }
}

/// The copies still decode as valid JPEGs.
#[test]
fn test_file_still_valid_jpeg() {
    let root = setup(&["a.jpg", "b.jpg", "c.jpg"]);
    run_all(&src(&root));

    for name in ["a.jpg", "b.jpg", "c.jpg"] {
        let buf = fs::read(out(&root).join(name)).unwrap();
        let img =
            image::load_from_memory(&buf).unwrap_or_else(|e| panic!("cannot decode {name}: {e}"));
        assert_eq!(img.width(), common::WIDTH);
        assert_eq!(img.height(), common::HEIGHT);
    }
}

/// A dry run computes the times but touches nothing on disk, not even the
/// output folder.
#[test]
fn test_dry_run_does_not_touch_disk() {
    let root = setup(&["a.jpg", "b.jpg"]);
    let before = listing(&src(&root));

    let summary = run_with(&src(&root), true);
    assert!(summary.results.iter().all(|(_, r)| r.is_ok()));
    let outcome = summary.results[1].1.as_ref().unwrap();
    assert_eq!(outcome.new.to_string(), "2024:01:02 03:05:00");
    assert!(outcome.changed);

    assert_eq!(before, listing(&src(&root)));
    assert!(
        !out(&root).exists(),
        "the dry run created the output folder"
    );
}

/// Zero targets is a clear error; a file without EXIF is a per-file error.
#[test]
fn test_error_cases() {
    let empty = tempfile::tempdir().unwrap();
    let err = run(
        empty.path(),
        false,
        &AtomicBool::new(false),
        |_, _, _, _| {},
    )
    .unwrap_err();
    assert!(err.contains("no target JPEG files"), "{err}");

    let root = tempfile::tempdir().unwrap();
    write(&root, "a.jpg", common::plain_jpeg());
    let summary = run_with(&src(&root), false);
    assert_eq!(summary.results.len(), 1);
    let err = summary.results[0].1.as_ref().unwrap_err();
    assert!(err.contains("Exif"), "{err}");
}

/// One file's failure does not stop the others; it is excluded from the
/// assignment and comes last.
#[test]
fn test_single_file_failure_does_not_stop_others() {
    let root = setup(&["a.jpg", "c.jpg"]);
    write(&root, "b.jpg", common::plain_jpeg());

    let summary = run_with(&src(&root), false);
    assert_eq!(names(&summary), ["a.jpg", "c.jpg", "b.jpg"]);
    assert!(summary.results[0].1.is_ok());
    assert!(summary.results[1].1.is_ok());
    assert!(summary.results[2].1.is_err());

    assert_eq!(output_dto(&root, "c.jpg"), "2024:01:02 03:05:00");
    assert!(!out(&root).join("b.jpg").exists());
}

/// Original times of separate scenes are preserved, and a scene is pushed
/// back minimally only when the push-out catches up.
#[test]
fn test_scene_times_preserved_and_pushed_only_when_caught_up() {
    let root = tempfile::tempdir().unwrap();
    for (name, dt) in [
        ("img_1.jpg", "2024:01:02 03:04:59"),
        ("img_2.jpg", "2024:01:02 03:04:59"),
        ("img_3.jpg", "2024:01:02 03:04:59"),
        ("img_4.jpg", "2024:01:02 03:05:00"),
        ("img_5.jpg", "2024:01:02 04:00:00"),
        ("img_6.jpg", "2024:01:02 04:00:00"),
    ] {
        write(&root, name, common::sample_jpeg_with(dt));
    }
    run_all(&src(&root));

    let expected = [
        ("img_1.jpg", "2024:01:02 03:04:59"),
        ("img_2.jpg", "2024:01:02 03:05:00"),
        ("img_3.jpg", "2024:01:02 03:05:01"),
        ("img_4.jpg", "2024:01:02 03:05:02"),
        ("img_5.jpg", "2024:01:02 04:00:00"),
        ("img_6.jpg", "2024:01:02 04:00:01"),
    ];
    for (name, want) in expected {
        assert_eq!(output_dto(&root, name), want, "{name}");
    }
}

/// A second run yields byte-identical output.
#[test]
fn test_idempotent() {
    let root = setup(&["a.jpg", "b.jpg", "c.jpg"]);
    run_all(&src(&root));
    let first = listing(&out(&root));
    run_all(&src(&root));
    assert_eq!(first, listing(&out(&root)));
}

/// Only .jpg / .jpeg (case-insensitive) are targeted, in natural order.
#[test]
fn test_collect_jpegs_filters_and_sorts() {
    let dir = tempfile::tempdir().unwrap();
    for name in ["b.JPG", "a.jpeg", "notes.txt", "raw.CR2", "img.png"] {
        fs::write(dir.path().join(name), b"dummy").unwrap();
    }
    let files = sequence::collect_jpegs(dir.path()).unwrap();
    let names: Vec<_> = files
        .iter()
        .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
        .collect();
    assert_eq!(names, vec!["a.jpeg", "b.JPG"]);
}

/// Two bodies in one folder come out in time order, not filename order.
#[test]
fn test_multi_body_mix_in_time_order() {
    let root = tempfile::tempdir().unwrap();
    for (name, dt) in [
        ("DSC00001.JPG", "2024:01:02 03:00:00"),
        ("L1000001.JPG", "2024:01:02 03:00:01"),
        ("DSC00002.JPG", "2024:01:02 03:00:02"),
        ("L1000002.JPG", "2024:01:02 03:00:02"),
        ("DSC00003.JPG", "2024:01:02 03:00:03"),
        ("L1000003.JPG", "2024:01:02 03:00:05"),
    ] {
        write(&root, name, common::sample_jpeg_with(dt));
    }
    let summary = run_all(&src(&root));
    assert_eq!(
        names(&summary),
        [
            "DSC00001.JPG",
            "L1000001.JPG",
            "DSC00002.JPG",
            "L1000002.JPG",
            "DSC00003.JPG",
            "L1000003.JPG"
        ]
    );
    for (name, want) in [
        ("DSC00001.JPG", "2024:01:02 03:00:00"),
        ("L1000001.JPG", "2024:01:02 03:00:01"),
        ("DSC00002.JPG", "2024:01:02 03:00:02"),
        ("L1000002.JPG", "2024:01:02 03:00:03"),
        ("DSC00003.JPG", "2024:01:02 03:00:04"),
        ("L1000003.JPG", "2024:01:02 03:00:05"),
    ] {
        assert_eq!(output_dto(&root, name), want, "{name}");
    }
}

/// Inside one second, SubSecTimeOriginal decides the order as a fraction:
/// "5" (0.5) is later than "12" (0.12), and an offset-stored value is read
/// like an inline one.
#[test]
fn test_subsec_tie_break() {
    let root = tempfile::tempdir().unwrap();
    let dt = common::BASE_DATETIME;
    write(
        &root,
        "a.jpg",
        common::sample_jpeg_with_subsec(dt, Some("5")),
    );
    write(
        &root,
        "b.jpg",
        common::sample_jpeg_with_subsec(dt, Some("12")),
    );
    write(
        &root,
        "c.jpg",
        common::sample_jpeg_with_subsec(dt, Some("300000")),
    );
    let summary = run_all(&src(&root));
    assert_eq!(names(&summary), ["b.jpg", "c.jpg", "a.jpg"]);
    let subsecs: Vec<_> = summary
        .results
        .iter()
        .map(|(_, r)| r.as_ref().unwrap().subsec.clone())
        .collect();
    assert_eq!(
        subsecs,
        [
            Some("12".to_string()),
            Some("300000".to_string()),
            Some("5".to_string())
        ]
    );
}

/// With equal time and SubSec, or both without SubSec, the natural filename
/// order decides.
#[test]
fn test_natural_filename_tie_break() {
    let root = tempfile::tempdir().unwrap();
    let dt = common::BASE_DATETIME;
    for name in ["x_10.jpg", "x_2.jpg"] {
        write(&root, name, common::sample_jpeg_with_subsec(dt, Some("7")));
    }
    for name in ["y_10.jpg", "y_2.jpg"] {
        write(&root, name, common::sample_jpeg_with("2024:01:02 05:00:00"));
    }
    let summary = run_all(&src(&root));
    assert_eq!(
        names(&summary),
        ["x_2.jpg", "x_10.jpg", "y_2.jpg", "y_10.jpg"]
    );
}

/// A file without SubSec sorts before one with SubSec in the same second.
#[test]
fn test_missing_subsec_sorts_first() {
    let root = tempfile::tempdir().unwrap();
    let dt = common::BASE_DATETIME;
    write(
        &root,
        "a.jpg",
        common::sample_jpeg_with_subsec(dt, Some("1")),
    );
    write(&root, "b.jpg", common::sample_jpeg_with(dt));
    let summary = run_all(&src(&root));
    assert_eq!(names(&summary), ["b.jpg", "a.jpg"]);
}

/// A malformed SubSecTimeOriginal (an offset pointing outside the APP1
/// segment) must not fail the file: it is only a tie-break, so it reads as
/// absent (`None`) and the file is still assigned by `DateTimeOriginal`.
#[test]
fn test_corrupt_subsec_offset_does_not_fail_the_file() {
    let root = tempfile::tempdir().unwrap();
    let dt = common::BASE_DATETIME;
    // A SubSec value longer than 4 bytes is stored via an offset field
    // (rather than inline), so corrupting that offset is reachable.
    let mut buf = common::sample_jpeg_with_subsec(dt, Some("300000"));
    // Locate the 0x9291 (SubSecTimeOriginal), type=2 (ASCII) IFD entry and
    // corrupt its offset field (the last 4 bytes of the 12-byte entry) to
    // point far outside the APP1 segment.
    let pos = buf
        .windows(4)
        .position(|w| w == [0x91, 0x92, 0x02, 0x00])
        .expect("SubSecTimeOriginal entry not found");
    buf[pos + 8..pos + 12].copy_from_slice(&0xFFFF_FFFFu32.to_le_bytes());
    write(&root, "a.jpg", buf);
    let summary = run_all(&src(&root));
    assert_eq!(names(&summary), ["a.jpg"]);
    let (_, result) = &summary.results[0];
    assert_eq!(result.as_ref().unwrap().subsec, None);
    // The corrupted SubSec entry makes the whole file unparsable by a
    // strict EXIF reader, so read DateTimeOriginal with the crate's own
    // (lenient) walker instead of `output_dto`.
    let out_buf = fs::read(out(&root).join("a.jpg")).unwrap();
    assert_eq!(
        sequence::read_datetime_original(&out_buf)
            .unwrap()
            .to_string(),
        dt
    );
}

/// The source files are byte-identical after a run.
#[test]
fn test_source_untouched() {
    let root = setup(&["a.jpg", "b.jpg", "c.jpg"]);
    let before = listing(&src(&root));
    run_all(&src(&root));
    assert_eq!(before, listing(&src(&root)));
}

/// A file that keeps its time is copied too, byte for byte.
#[test]
fn test_unchanged_files_are_copied() {
    let root = tempfile::tempdir().unwrap();
    write(
        &root,
        "a.jpg",
        common::sample_jpeg_with("2024:01:02 03:04:59"),
    );
    write(
        &root,
        "b.jpg",
        common::sample_jpeg_with("2024:01:02 03:05:10"),
    );
    let summary = run_all(&src(&root));
    assert!(summary
        .results
        .iter()
        .all(|(_, r)| !r.as_ref().unwrap().changed));
    assert_eq!(listing(&src(&root)), listing(&out(&root)));
}

/// A rerun after adding a file equals a fresh run on the same set.
#[test]
fn test_rerun_after_adding_equals_fresh_run() {
    let root = setup(&["a.jpg", "c.jpg"]);
    run_all(&src(&root));
    write(&root, "b.jpg", common::sample_jpeg());
    run_all(&src(&root));

    let fresh = setup(&["a.jpg", "b.jpg", "c.jpg"]);
    run_all(&src(&fresh));
    assert_eq!(listing(&out(&fresh)), listing(&out(&root)));
}

/// A rerun removes an output file whose source was deleted.
#[test]
fn test_rerun_removes_deleted_source() {
    let root = setup(&["a.jpg", "b.jpg"]);
    run_all(&src(&root));
    fs::remove_file(src(&root).join("b.jpg")).unwrap();
    run_all(&src(&root));
    let names: Vec<_> = listing(&out(&root)).into_keys().collect();
    assert_eq!(names, ["a.jpg"]);
}

/// A rebuild deletes only the JPEG files directly in the output folder.
#[test]
fn test_rebuild_keeps_other_files() {
    let root = setup(&["a.jpg"]);
    let out = out(&root);
    fs::create_dir_all(out.join("sub")).unwrap();
    fs::write(out.join("sub").join("keep.jpg"), b"keep").unwrap();
    fs::write(out.join("notes.txt"), b"notes").unwrap();
    fs::write(out.join("stale.JPEG"), b"stale").unwrap();

    run_all(&src(&root));

    assert_eq!(fs::read(out.join("notes.txt")).unwrap(), b"notes");
    assert_eq!(fs::read(out.join("sub").join("keep.jpg")).unwrap(), b"keep");
    assert!(!out.join("stale.JPEG").exists());
    assert!(out.join("a.jpg").exists());
}

/// If the output folder resolves to the source folder (a symlink or, on
/// Windows, a junction), the run must refuse rather than delete the source
/// files.
#[test]
fn test_refuses_when_output_resolves_to_source() {
    let root = setup(&["a.jpg"]);
    let source = src(&root);
    let output = out(&root);
    fs::remove_dir_all(&output).ok();

    #[cfg(unix)]
    std::os::unix::fs::symlink(&source, &output).unwrap();
    #[cfg(windows)]
    std::os::windows::fs::symlink_dir(&source, &output).unwrap();

    let before = listing(&source);
    let result = run(&source, false, &AtomicBool::new(false), |_, _, _, _| {});
    assert!(result.is_err(), "expected an error, got {result:?}");
    assert_eq!(before, listing(&source));
}

/// Canceling mid-run leaves only complete JPEGs in the output folder, with no
/// temporary file behind.
#[test]
fn test_cancel_leaves_only_complete_files() {
    let names: Vec<String> = (1..=200).map(|i| format!("img_{i}.jpg")).collect();
    let refs: Vec<&str> = names.iter().map(String::as_str).collect();
    let root = setup(&refs);
    let cancel = AtomicBool::new(false);

    let summary = run(&src(&root), false, &cancel, |_, _, _, _| {
        cancel.store(true, Ordering::Relaxed)
    })
    .unwrap();

    assert!(summary.canceled);
    assert!(summary.results.len() < summary.total);
    let written = listing(&out(&root));
    assert_eq!(written.len(), summary.results.len());
    for (name, bytes) in written {
        assert!(name.ends_with(".jpg"), "leftover file {name}");
        image::load_from_memory(&bytes).unwrap_or_else(|e| panic!("{name} is incomplete: {e}"));
    }
}

/// A cancel set before the run touches nothing on disk.
#[test]
fn test_cancel_before_run_touches_nothing() {
    let root = setup(&["a.jpg"]);
    let summary = run(&src(&root), false, &AtomicBool::new(true), |_, _, _, _| {}).unwrap();
    assert!(summary.canceled);
    assert!(summary.results.is_empty());
    assert!(!out(&root).exists());
}

/// Progress counts up to the total, once per file.
#[test]
fn test_progress_reaches_total() {
    let root = setup(&["a.jpg", "b.jpg", "c.jpg"]);
    let calls = AtomicUsize::new(0);
    let last = AtomicUsize::new(0);
    let summary = run(
        &src(&root),
        false,
        &AtomicBool::new(false),
        |done, total, _, r| {
            assert_eq!(total, 3);
            assert!(r.is_ok());
            calls.fetch_add(1, Ordering::Relaxed);
            last.fetch_max(done, Ordering::Relaxed);
        },
    )
    .unwrap();
    assert!(!summary.canceled);
    assert_eq!(calls.load(Ordering::Relaxed), 3);
    assert_eq!(last.load(Ordering::Relaxed), 3);
}
