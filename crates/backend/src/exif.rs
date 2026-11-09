//! Display formatting of the shooting settings, shared by the metadata pane
//! and the index.

use riffle_core::arw::{excluded_dsc, Rational, Shot};

/// A numeric setting: `value` to sort by, `label` to show.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct Labeled {
    pub value: f64,
    pub label: String,
}

/// The shooting settings of one file, formatted for display. Fields the file
/// does not carry are `None`. The Maker note fields are left out of the JSON
/// when `None`: the folder listing sends one per file and the frontend never
/// reads them off it.
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize)]
pub struct Exif {
    pub camera: Option<String>,
    pub lens: Option<String>,
    pub aperture: Option<Labeled>,
    pub shutter: Option<Labeled>,
    pub iso: Option<Labeled>,
    pub focal_length: Option<Labeled>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub focus_mode: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub af_tracking: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub af_area: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub drive: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stabilization: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exposure_mode: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metering: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub creative_style: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dro: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw_type: Option<String>,
}

/// Format a rational as a decimal with at most `places` digits, with trailing
/// zeros dropped (2.80 -> "2.8", 50.0 -> "50").
pub fn decimal(r: Rational, places: usize) -> Option<String> {
    let v = r.value()?;
    let text = format!("{v:.places$}");
    let text = if text.contains('.') {
        text.trim_end_matches('0').trim_end_matches('.')
    } else {
        text.as_str()
    };
    Some(text.to_string())
}

/// Shutter speed the way a camera shows it: `1/250` below a second, `1.3"`
/// at or above one.
fn shutter(r: Rational) -> Option<String> {
    let v = r.value()?;
    if v <= 0.0 {
        return None;
    }
    if v >= 1.0 {
        return decimal(r, 1).map(|t| format!("{t}\""));
    }
    Some(format!("1/{}", (1.0 / v).round()))
}

/// Sony `FocusMode` (0x201b), labeled as ExifTool's Sony.pm `%Sony::Main`.
fn focus_mode(model: Option<&str>, v: u8) -> Option<&'static str> {
    if excluded_dsc(model) {
        return None;
    }
    match v {
        0 => Some("Manual"),
        2 => Some("AF-S"),
        3 => Some("AF-C"),
        4 => Some("AF-A"),
        6 => Some("DMF"),
        7 => Some("AF-D"),
        _ => None,
    }
}

/// Sony `AFTracking` (0x2021), labeled as ExifTool's Sony.pm `%Sony::Main`.
fn af_tracking(model: Option<&str>, v: u8) -> Option<&'static str> {
    if excluded_dsc(model) {
        return None;
    }
    match v {
        0 => Some("Off"),
        1 => Some("Face tracking"),
        2 => Some("Lock On AF"),
        _ => None,
    }
}

/// Sony `AFAreaModeSetting` (0x201c), labeled with ExifTool's Sony.pm
/// NEX/ILCE/ZV table. The SLT/HV and ILCA tables are intentionally not
/// mapped, and the `ILME-` and RX/HX `DSC-` bodies ExifTool also reads with
/// this table are deliberately left out.
fn af_area(model: Option<&str>, v: u8) -> Option<&'static str> {
    let model = model?;
    if !["ILCE-", "NEX-", "ZV-"]
        .iter()
        .any(|prefix| model.starts_with(prefix))
    {
        return None;
    }
    match v {
        0 => Some("Wide"),
        1 => Some("Center"),
        3 => Some("Flexible Spot"),
        4 => Some("Flexible Spot (LA-EA4)"),
        9 => Some("Center (LA-EA4)"),
        11 => Some("Zone"),
        12 => Some("Expanded Flexible Spot"),
        13 => Some("Custom AF Area"),
        _ => None,
    }
}

/// Sony `ReleaseMode` (0xb049), labeled as ExifTool's Sony.pm `%Sony::Main`.
/// 65535 (n/a) is `None`.
fn release_mode(v: u32) -> Option<&'static str> {
    match v {
        0 => Some("Normal"),
        2 => Some("Continuous"),
        5 => Some("Exposure Bracketing"),
        6 => Some("White Balance Bracketing"),
        8 => Some("DRO Bracketing"),
        _ => None,
    }
}

/// The drive row: the release mode, followed by the frame number within the
/// burst when Sony `SequenceNumber` (0xb04a) records one (0 is a single shot,
/// 65535 n/a).
fn drive(release: Option<u32>, sequence: Option<u32>) -> Option<String> {
    let label = release_mode(release?)?;
    Some(match sequence {
        Some(n) if n != 0 && n != 65535 => format!("{label}, frame {n}"),
        _ => label.to_string(),
    })
}

/// Sony `ImageStabilization` (0xb026), labeled as ExifTool's Sony.pm
/// `%Sony::Main`. 0xffffffff (n/a) is `None`.
fn stabilization(v: u32) -> Option<&'static str> {
    match v {
        0 => Some("Off"),
        1 => Some("On"),
        _ => None,
    }
}

/// Sony `ExposureMode` (0xb041), labeled as ExifTool's Sony.pm
/// `%Sony::Main`. 65535 (n/a) is `None`.
fn exposure_mode(v: u32) -> Option<&'static str> {
    match v {
        0 => Some("Program AE"),
        1 => Some("Portrait"),
        2 => Some("Beach"),
        3 => Some("Sports"),
        4 => Some("Snow"),
        5 => Some("Landscape"),
        6 => Some("Auto"),
        7 => Some("Aperture-priority AE"),
        8 => Some("Shutter speed priority AE"),
        9 => Some("Night Scene / Twilight"),
        10 => Some("Hi-Speed Shutter"),
        11 => Some("Twilight Portrait"),
        12 => Some("Soft Snap/Portrait"),
        13 => Some("Fireworks"),
        14 => Some("Smile Shutter"),
        15 => Some("Manual"),
        18 => Some("High Sensitivity"),
        19 => Some("Macro"),
        20 => Some("Advanced Sports Shooting"),
        29 => Some("Underwater"),
        33 => Some("Food"),
        34 => Some("Sweep Panorama"),
        35 => Some("Handheld Night Shot"),
        36 => Some("Anti Motion Blur"),
        37 => Some("Pet"),
        38 => Some("Backlight Correction HDR"),
        39 => Some("Superior Auto"),
        40 => Some("Background Defocus"),
        41 => Some("Soft Skin"),
        42 => Some("3D Image"),
        _ => None,
    }
}

/// Sony `MeteringMode2` (0x202c), labeled as ExifTool's Sony.pm
/// `%Sony::Main`. It is finer than EXIF `MeteringMode` (0x9207), which
/// Riffle does not read.
fn metering(v: u32) -> Option<&'static str> {
    match v {
        0x100 => Some("Multi-segment"),
        0x200 => Some("Center-weighted average"),
        0x301 => Some("Spot (Standard)"),
        0x302 => Some("Spot (Large)"),
        0x400 => Some("Average"),
        0x500 => Some("Highlight"),
        _ => None,
    }
}

/// Sony `CreativeStyle` (0xb020), always recorded in English, passed through
/// with the renames ExifTool's Sony.pm `%Sony::Main` applies.
fn creative_style(v: &str) -> Option<String> {
    let label = match v {
        "" => return None,
        "AdobeRGB" => "Adobe RGB",
        "Nightview" => "Night View/Portrait",
        "BW" => "B&W",
        "Autumnleaves" => "Autumn Leaves",
        "VV2" => "Vivid 2",
        other => other,
    };
    Some(label.to_string())
}

/// Sony `DynamicRangeOptimizer` (0xb025), labeled as ExifTool's Sony.pm
/// `%Sony::Main`. The other `DynamicRangeOptimizer` tag, 0xb04f, is not
/// used: ExifTool gives it `Priority => 0`, and it reads Standard where the
/// body's menu (and 0xb025) says Auto.
fn dro(v: u32) -> Option<&'static str> {
    match v {
        0 => Some("Off"),
        1 => Some("Standard"),
        2 => Some("Advanced Auto"),
        3 => Some("Auto"),
        8 => Some("Advanced Lv1"),
        9 => Some("Advanced Lv2"),
        10 => Some("Advanced Lv3"),
        11 => Some("Advanced Lv4"),
        12 => Some("Advanced Lv5"),
        16 => Some("Lv1"),
        17 => Some("Lv2"),
        18 => Some("Lv3"),
        19 => Some("Lv4"),
        20 => Some("Lv5"),
        21 => Some("Lv6"),
        22 => Some("Lv7"),
        23 => Some("Lv8"),
        _ => None,
    }
}

/// Sony `RAWFileType` (0x2029), labeled as ExifTool's Sony.pm `%Sony::Main`.
/// 65535 (n/a) is `None`.
fn raw_type(v: u32) -> Option<&'static str> {
    match v {
        0 => Some("Compressed RAW"),
        1 => Some("Uncompressed RAW"),
        2 => Some("Lossless Compressed RAW"),
        3 => Some("Compressed RAW 2"),
        _ => None,
    }
}

pub fn exif(shot: &Shot) -> Exif {
    // The model usually already starts with the make ("SONY" / "ILCE-7M5"),
    // so the two are joined rather than one being dropped.
    let camera = match (shot.make.as_deref(), shot.model.as_deref()) {
        (Some(make), Some(model)) => Some(format!("{make} {model}")),
        (make, model) => make.or(model).map(str::to_string),
    };
    let aperture = match (shot.f_number, shot.estimated_f_number) {
        (Some(r), _) => r.value().zip(decimal(r, 1)).map(|(value, t)| Labeled {
            value,
            label: format!("f/{t}"),
        }),
        (None, Some(f)) => decimal(
            Rational {
                num: (f * 10.0).round() as i64,
                den: 10,
            },
            1,
        )
        .map(|t| Labeled {
            value: f,
            label: format!("f/{t} (est.)"),
        }),
        (None, None) => None,
    };
    Exif {
        camera,
        lens: shot.lens_model.clone(),
        aperture,
        shutter: shot.exposure_time.and_then(|r| {
            Some(Labeled {
                label: shutter(r)?,
                value: r.value()?,
            })
        }),
        iso: shot.iso.map(|v| Labeled {
            value: f64::from(v),
            label: v.to_string(),
        }),
        focal_length: shot.focal_length.and_then(|r| {
            Some(Labeled {
                label: format!("{} mm", decimal(r, 1)?),
                value: r.value()?,
            })
        }),
        focus_mode: shot
            .focus_mode
            .and_then(|v| focus_mode(shot.model.as_deref(), v))
            .map(str::to_string),
        af_tracking: shot
            .af_tracking
            .and_then(|v| af_tracking(shot.model.as_deref(), v))
            .map(str::to_string),
        af_area: shot
            .af_area_mode
            .and_then(|v| af_area(shot.model.as_deref(), v))
            .map(str::to_string),
        drive: drive(shot.release_mode, shot.sequence_number),
        stabilization: shot
            .image_stabilization
            .and_then(stabilization)
            .map(str::to_string),
        exposure_mode: shot
            .exposure_mode
            .and_then(exposure_mode)
            .map(str::to_string),
        metering: shot.metering_mode.and_then(metering).map(str::to_string),
        creative_style: shot.creative_style.as_deref().and_then(creative_style),
        dro: shot
            .dynamic_range_optimizer
            .and_then(dro)
            .map(str::to_string),
        raw_type: shot.raw_file_type.and_then(raw_type).map(str::to_string),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn r(num: i64, den: i64) -> Rational {
        Rational { num, den }
    }

    fn camera(make: Option<&str>, model: Option<&str>) -> Option<String> {
        exif(&Shot {
            make: make.map(str::to_string),
            model: model.map(str::to_string),
            ..Shot::default()
        })
        .camera
    }

    #[test]
    fn shutter_reads_as_a_fraction_below_a_second_and_seconds_above() {
        assert_eq!(shutter(r(1, 250)).as_deref(), Some("1/250"));
        assert_eq!(shutter(r(13, 10)).as_deref(), Some("1.3\""));
        assert_eq!(shutter(r(4, 1)).as_deref(), Some("4\""));
        assert_eq!(shutter(r(0, 1)), None);
        assert_eq!(shutter(r(1, 0)), None);
    }

    #[test]
    fn decimals_drop_their_trailing_zeros() {
        assert_eq!(decimal(r(28, 10), 1).as_deref(), Some("2.8"));
        assert_eq!(decimal(r(500, 10), 1).as_deref(), Some("50"));
        assert_eq!(decimal(r(1, 0), 1), None);
        assert_eq!(decimal(r(100, 1), 0).as_deref(), Some("100"));
    }

    #[test]
    fn the_numeric_settings_carry_their_value_and_label() {
        let e = exif(&Shot {
            exposure_time: Some(r(1, 250)),
            f_number: Some(r(28, 10)),
            iso: Some(800),
            focal_length: Some(r(500, 10)),
            lens_model: Some("FE 50mm F1.4 GM".to_string()),
            ..Shot::default()
        });
        let label = |l: &Option<Labeled>| l.as_ref().map(|l| (l.value, l.label.clone()));
        assert_eq!(label(&e.shutter), Some((0.004, "1/250".to_string())));
        assert_eq!(label(&e.aperture), Some((2.8, "f/2.8".to_string())));
        assert_eq!(label(&e.iso), Some((800.0, "800".to_string())));
        assert_eq!(label(&e.focal_length), Some((50.0, "50 mm".to_string())));
        assert_eq!(e.lens.as_deref(), Some("FE 50mm F1.4 GM"));
    }

    #[test]
    fn an_estimated_aperture_is_marked() {
        let e = exif(&Shot {
            estimated_f_number: Some(2.0),
            ..Shot::default()
        });
        let a = e.aperture.unwrap();
        assert_eq!(a.value, 2.0);
        assert_eq!(a.label, "f/2 (est.)");
    }

    #[test]
    fn the_camera_joins_make_and_model() {
        assert_eq!(
            camera(Some("SONY"), Some("ILCE-7M5")).as_deref(),
            Some("SONY ILCE-7M5")
        );
        assert_eq!(camera(Some("SONY"), None).as_deref(), Some("SONY"));
        assert_eq!(camera(None, Some("ILCE-7M5")).as_deref(), Some("ILCE-7M5"));
        assert_eq!(camera(None, None), None);
    }

    #[test]
    fn the_sony_focus_mode_maps_to_its_label() {
        for (v, label) in [
            (0, "Manual"),
            (2, "AF-S"),
            (3, "AF-C"),
            (4, "AF-A"),
            (6, "DMF"),
            (7, "AF-D"),
        ] {
            assert_eq!(focus_mode(Some("ILCE-7M5"), v), Some(label));
        }
        for v in [1, 5, 255] {
            assert_eq!(focus_mode(Some("ILCE-7M5"), v), None);
        }
    }

    #[test]
    fn the_sony_af_tracking_maps_to_its_label() {
        assert_eq!(af_tracking(Some("ILCE-7M5"), 0), Some("Off"));
        assert_eq!(af_tracking(Some("ILCE-7M5"), 1), Some("Face tracking"));
        assert_eq!(af_tracking(Some("ILCE-7M5"), 2), Some("Lock On AF"));
        assert_eq!(af_tracking(Some("ILCE-7M5"), 3), None);
    }

    #[test]
    fn the_sony_focus_mode_and_af_tracking_are_excluded_on_older_dsc_bodies() {
        assert_eq!(focus_mode(Some("DSC-RX100M3"), 3), None);
        assert_eq!(af_tracking(Some("DSC-RX100M3"), 1), None);
        assert_eq!(focus_mode(Some("DSC-RX100M7"), 3), Some("AF-C"));
        assert_eq!(af_tracking(Some("DSC-RX100M7"), 1), Some("Face tracking"));
        assert_eq!(focus_mode(None, 3), Some("AF-C"));
        assert_eq!(af_tracking(None, 1), Some("Face tracking"));
    }

    #[test]
    fn the_sony_af_area_maps_to_its_label_on_ilce_nex_and_zv_bodies() {
        for model in ["ILCE-7M5", "NEX-7", "ZV-E10"] {
            for (v, label) in [
                (0, "Wide"),
                (1, "Center"),
                (3, "Flexible Spot"),
                (4, "Flexible Spot (LA-EA4)"),
                (9, "Center (LA-EA4)"),
                (11, "Zone"),
                (12, "Expanded Flexible Spot"),
                (13, "Custom AF Area"),
            ] {
                assert_eq!(af_area(Some(model), v), Some(label));
            }
            for v in [2, 8, 255] {
                assert_eq!(af_area(Some(model), v), None);
            }
        }
    }

    #[test]
    fn the_sony_af_area_is_left_out_on_other_bodies() {
        for model in [
            Some("ILME-FX3"),
            Some("ILCA-99M2"),
            Some("SLT-A99V"),
            Some("DSC-RX100M7"),
            Some("LEICA M11-P"),
            None,
        ] {
            assert_eq!(af_area(model, 0), None);
        }
    }

    #[test]
    fn the_af_fields_are_formatted_from_the_shot() {
        let e = exif(&Shot {
            model: Some("ILCE-7M5".to_string()),
            focus_mode: Some(3),
            af_tracking: Some(1),
            af_area_mode: Some(13),
            ..Shot::default()
        });
        assert_eq!(e.focus_mode.as_deref(), Some("AF-C"));
        assert_eq!(e.af_tracking.as_deref(), Some("Face tracking"));
        assert_eq!(e.af_area.as_deref(), Some("Custom AF Area"));
    }

    #[test]
    fn the_sony_release_mode_maps_to_its_label() {
        for (v, label) in [
            (0, "Normal"),
            (2, "Continuous"),
            (5, "Exposure Bracketing"),
            (6, "White Balance Bracketing"),
            (8, "DRO Bracketing"),
        ] {
            assert_eq!(release_mode(v), Some(label));
        }
        for v in [1, 3, 65535] {
            assert_eq!(release_mode(v), None);
        }
    }

    #[test]
    fn the_drive_row_adds_the_frame_number_within_a_burst() {
        assert_eq!(drive(Some(0), None).as_deref(), Some("Normal"));
        assert_eq!(drive(Some(0), Some(0)).as_deref(), Some("Normal"));
        assert_eq!(drive(Some(2), None).as_deref(), Some("Continuous"));
        assert_eq!(drive(Some(2), Some(65535)).as_deref(), Some("Continuous"));
        assert_eq!(
            drive(Some(2), Some(1)).as_deref(),
            Some("Continuous, frame 1")
        );
        assert_eq!(
            drive(Some(2), Some(2)).as_deref(),
            Some("Continuous, frame 2")
        );
        assert_eq!(drive(None, Some(2)), None);
        assert_eq!(drive(Some(65535), Some(2)), None);
        assert_eq!(drive(None, None), None);
    }

    #[test]
    fn the_sony_stabilization_maps_to_its_label() {
        assert_eq!(stabilization(0), Some("Off"));
        assert_eq!(stabilization(1), Some("On"));
        assert_eq!(stabilization(2), None);
        assert_eq!(stabilization(0xffff_ffff), None);
    }

    #[test]
    fn the_sony_exposure_mode_maps_to_its_label() {
        for (v, label) in [
            (0, "Program AE"),
            (1, "Portrait"),
            (2, "Beach"),
            (3, "Sports"),
            (4, "Snow"),
            (5, "Landscape"),
            (6, "Auto"),
            (7, "Aperture-priority AE"),
            (8, "Shutter speed priority AE"),
            (9, "Night Scene / Twilight"),
            (10, "Hi-Speed Shutter"),
            (11, "Twilight Portrait"),
            (12, "Soft Snap/Portrait"),
            (13, "Fireworks"),
            (14, "Smile Shutter"),
            (15, "Manual"),
            (18, "High Sensitivity"),
            (19, "Macro"),
            (20, "Advanced Sports Shooting"),
            (29, "Underwater"),
            (33, "Food"),
            (34, "Sweep Panorama"),
            (35, "Handheld Night Shot"),
            (36, "Anti Motion Blur"),
            (37, "Pet"),
            (38, "Backlight Correction HDR"),
            (39, "Superior Auto"),
            (40, "Background Defocus"),
            (41, "Soft Skin"),
            (42, "3D Image"),
        ] {
            assert_eq!(exposure_mode(v), Some(label));
        }
        for v in [16, 30, 50, 65535] {
            assert_eq!(exposure_mode(v), None);
        }
    }

    #[test]
    fn the_sony_metering_mode_maps_to_its_label() {
        for (v, label) in [
            (0x100, "Multi-segment"),
            (0x200, "Center-weighted average"),
            (0x301, "Spot (Standard)"),
            (0x302, "Spot (Large)"),
            (0x400, "Average"),
            (0x500, "Highlight"),
        ] {
            assert_eq!(metering(v), Some(label));
        }
        for v in [0, 0x300, 0x600, 65535] {
            assert_eq!(metering(v), None);
        }
    }

    #[test]
    fn the_sony_creative_style_passes_through_with_exiftool_renames() {
        for (v, label) in [
            ("Standard", "Standard"),
            ("ST", "ST"),
            ("AdobeRGB", "Adobe RGB"),
            ("Nightview", "Night View/Portrait"),
            ("BW", "B&W"),
            ("Autumnleaves", "Autumn Leaves"),
            ("VV2", "Vivid 2"),
        ] {
            assert_eq!(creative_style(v).as_deref(), Some(label));
        }
        assert_eq!(creative_style(""), None);
    }

    #[test]
    fn the_sony_dro_maps_to_its_label() {
        for (v, label) in [
            (0, "Off"),
            (1, "Standard"),
            (2, "Advanced Auto"),
            (3, "Auto"),
            (8, "Advanced Lv1"),
            (9, "Advanced Lv2"),
            (10, "Advanced Lv3"),
            (11, "Advanced Lv4"),
            (12, "Advanced Lv5"),
            (16, "Lv1"),
            (17, "Lv2"),
            (18, "Lv3"),
            (19, "Lv4"),
            (20, "Lv5"),
            (21, "Lv6"),
            (22, "Lv7"),
            (23, "Lv8"),
        ] {
            assert_eq!(dro(v), Some(label));
        }
        for v in [4, 7, 13, 24, 0xffff_ffff] {
            assert_eq!(dro(v), None);
        }
    }

    #[test]
    fn the_sony_raw_type_maps_to_its_label() {
        assert_eq!(raw_type(0), Some("Compressed RAW"));
        assert_eq!(raw_type(1), Some("Uncompressed RAW"));
        assert_eq!(raw_type(2), Some("Lossless Compressed RAW"));
        assert_eq!(raw_type(3), Some("Compressed RAW 2"));
        assert_eq!(raw_type(4), None);
        assert_eq!(raw_type(65535), None);
    }

    #[test]
    fn the_drive_and_picture_fields_are_formatted_from_the_shot() {
        let e = exif(&Shot {
            model: Some("ILCE-7M5".to_string()),
            release_mode: Some(2),
            sequence_number: Some(3),
            image_stabilization: Some(1),
            exposure_mode: Some(15),
            metering_mode: Some(0x100),
            creative_style: Some("Standard".to_string()),
            dynamic_range_optimizer: Some(3),
            raw_file_type: Some(2),
            ..Shot::default()
        });
        assert_eq!(e.drive.as_deref(), Some("Continuous, frame 3"));
        assert_eq!(e.stabilization.as_deref(), Some("On"));
        assert_eq!(e.exposure_mode.as_deref(), Some("Manual"));
        assert_eq!(e.metering.as_deref(), Some("Multi-segment"));
        assert_eq!(e.creative_style.as_deref(), Some("Standard"));
        assert_eq!(e.dro.as_deref(), Some("Auto"));
        assert_eq!(e.raw_type.as_deref(), Some("Lossless Compressed RAW"));
    }

    #[test]
    fn a_shot_without_settings_formats_to_nothing() {
        assert_eq!(exif(&Shot::default()), Exif::default());
    }

    #[test]
    fn absent_maker_note_fields_are_left_out_of_the_json() {
        let json = serde_json::to_value(exif(&Shot::default())).unwrap();
        let object = json.as_object().unwrap();
        for key in [
            "focus_mode",
            "af_tracking",
            "af_area",
            "drive",
            "stabilization",
            "exposure_mode",
            "metering",
            "creative_style",
            "dro",
            "raw_type",
        ] {
            assert!(!object.contains_key(key), "{key}");
        }
        assert_eq!(object.get("camera"), Some(&serde_json::Value::Null));
    }
}
