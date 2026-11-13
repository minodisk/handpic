//! The meta pane's rows grouped by where each value comes from, a port of
//! `meta.ts`'s `metaGroups`: the standard EXIF/TIFF tags, the vendor
//! MakerNote, and the app's own analysis, whose rows the caller passes.

use riffle_backend::exif::Metadata;

pub const EXIF_HEADING: &str = "EXIF";
pub const MAKER_NOTE_HEADING: &str = "Maker note";
pub const ANALYSIS_HEADING: &str = "Analysis";

/// What the pane shows of the file whose preview is on screen.
pub enum Shown {
    Read(Box<Metadata>),
    /// Its preview failed: the file name and why.
    Failed {
        name: String,
        error: String,
    },
}

/// The pane's `n / total`: the pager's current file over the shown list, as
/// the Tauri `positionEl`, so a refilter that hides the file on screen moves
/// it at once rather than when the next preview arrives.
pub fn position(current: usize, total: usize) -> String {
    format!("{} / {}", current + 1, total)
}

#[derive(Debug, PartialEq)]
pub struct Row<'a> {
    pub label: &'a str,
    pub value: &'a str,
}

#[derive(Debug, PartialEq)]
pub struct Group<'a> {
    pub heading: &'static str,
    pub rows: Vec<Row<'a>>,
}

fn group<'a>(heading: &'static str, rows: &[(&'a str, &'a Option<String>)]) -> Group<'a> {
    Group {
        heading,
        rows: rows
            .iter()
            .filter_map(|&(label, value)| value.as_deref().map(|value| Row { label, value }))
            .collect(),
    }
}

/// A row is left out when its value is `None` and a group when it has no
/// row. The analysis group needs no `meta`, so a file whose metadata could
/// not be read still shows its analysis.
pub fn groups<'a>(
    meta: Option<&'a Metadata>,
    analysis: &'a [(&'a str, Option<String>)],
) -> Vec<Group<'a>> {
    let mut groups = Vec::new();
    if let Some(m) = meta {
        groups.push(group(
            EXIF_HEADING,
            &[
                ("Aperture", &m.aperture),
                ("Shutter", &m.shutter),
                ("ISO", &m.iso),
                ("Focal length", &m.focal_length),
                ("Exposure", &m.exposure_bias),
                ("Camera", &m.camera),
                ("Lens", &m.lens),
                ("Captured", &m.captured_at),
            ],
        ));
        groups.push(group(
            MAKER_NOTE_HEADING,
            &[
                ("Shutter type", &m.shutter_type),
                ("Focus mode", &m.focus_mode),
                ("AF area", &m.af_area),
                ("AF tracking", &m.af_tracking),
                ("Drive", &m.drive),
                ("Stabilization", &m.stabilization),
                ("Exposure mode", &m.exposure_mode),
                ("Metering", &m.metering),
                ("Creative style", &m.creative_style),
                ("DRO", &m.dro),
                ("RAW type", &m.raw_type),
                ("Focus distance", &m.focus_distance),
            ],
        ));
    }
    let analysis: Vec<_> = analysis.iter().map(|(l, v)| (*l, v)).collect();
    groups.push(group(ANALYSIS_HEADING, &analysis));
    groups.retain(|g| !g.rows.is_empty());
    groups
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `p` under `untagged` on `b` of `a b c`: the pager moves to `c`, at 1
    /// of 2, while `b`'s preview, shown at 1 of 3, is still on screen.
    #[test]
    fn the_position_follows_the_pager_after_a_refilter() {
        let (current, total) = (1, 2);
        assert_eq!(position(current, total), "2 / 2");
    }

    fn empty() -> Metadata {
        Metadata {
            name: "a.ARW".into(),
            camera: None,
            lens: None,
            aperture: None,
            shutter: None,
            shutter_type: None,
            focus_mode: None,
            af_area: None,
            af_tracking: None,
            drive: None,
            stabilization: None,
            exposure_mode: None,
            metering: None,
            creative_style: None,
            dro: None,
            raw_type: None,
            iso: None,
            focal_length: None,
            exposure_bias: None,
            focus_distance: None,
            captured_at: None,
        }
    }

    fn rows<'a>(group: &Group<'a>) -> Vec<(&'a str, &'a str)> {
        group.rows.iter().map(|r| (r.label, r.value)).collect()
    }

    #[test]
    fn only_exif_fields_give_one_group_and_no_analysis() {
        let meta = Metadata {
            aperture: Some("f/2.8".into()),
            shutter: Some("1/250".into()),
            camera: Some("ILCE-1".into()),
            ..empty()
        };
        let groups = groups(Some(&meta), &[]);
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].heading, EXIF_HEADING);
        assert_eq!(
            rows(&groups[0]),
            [
                ("Aperture", "f/2.8"),
                ("Shutter", "1/250"),
                ("Camera", "ILCE-1")
            ]
        );
    }

    #[test]
    fn keeps_meta_ts_order_across_groups() {
        let meta = Metadata {
            captured_at: Some("2026-11-10 10:00:00".into()),
            iso: Some("ISO 100".into()),
            focus_distance: Some("1.23 m".into()),
            shutter_type: Some("Electronic".into()),
            dro: Some("Off".into()),
            ..empty()
        };
        let groups = groups(Some(&meta), &[]);
        let headings: Vec<_> = groups.iter().map(|g| g.heading).collect();
        assert_eq!(headings, [EXIF_HEADING, MAKER_NOTE_HEADING]);
        assert_eq!(
            rows(&groups[0]),
            [("ISO", "ISO 100"), ("Captured", "2026-11-10 10:00:00")]
        );
        assert_eq!(
            rows(&groups[1]),
            [
                ("Shutter type", "Electronic"),
                ("DRO", "Off"),
                ("Focus distance", "1.23 m")
            ]
        );
    }

    #[test]
    fn empty_metadata_gives_no_group() {
        assert!(groups(Some(&empty()), &[]).is_empty());
        assert!(groups(None, &[]).is_empty());
    }

    #[test]
    fn analysis_rows_follow_and_drop_none() {
        let meta = Metadata {
            iso: Some("ISO 100".into()),
            ..empty()
        };
        let analysis = [
            ("AF eye in focus", Some("82%".to_owned())),
            ("Eyes", None),
            ("Yaw", Some("3°".to_owned())),
        ];
        let groups = groups(Some(&meta), &analysis);
        let headings: Vec<_> = groups.iter().map(|g| g.heading).collect();
        assert_eq!(headings, [EXIF_HEADING, ANALYSIS_HEADING]);
        assert_eq!(
            rows(&groups[1]),
            [("AF eye in focus", "82%"), ("Yaw", "3°")]
        );
    }

    #[test]
    fn analysis_shows_without_metadata() {
        let analysis = [("Yaw", Some("3°".to_owned()))];
        let groups = groups(None, &analysis);
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].heading, ANALYSIS_HEADING);
    }

    #[test]
    fn all_none_analysis_gives_no_group() {
        let analysis = [("Eyes", None), ("Yaw", None)];
        assert!(groups(None, &analysis).is_empty());
    }
}
