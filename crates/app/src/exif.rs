//! The filter menu's per-folder EXIF groups, a port of `exif.ts` and
//! `main.ts`'s `rebuildExifMenu`: each group's key of one file (the label to
//! show and match, the order to sort by), the focal length bucketed into a
//! range, and the folder's labels per group in display order.

use std::cmp::Ordering;

use handpic_engine::exif::Exif;
use icu_normalizer::DecomposingNormalizerBorrowed;

/// One EXIF section of the filter menu, in the menu's order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Group {
    Camera,
    Lens,
    Aperture,
    Shutter,
    Iso,
    Focal,
}

pub const GROUPS: [Group; 6] = [
    Group::Camera,
    Group::Lens,
    Group::Aperture,
    Group::Shutter,
    Group::Iso,
    Group::Focal,
];

impl Group {
    pub fn heading(self) -> &'static str {
        match self {
            Self::Camera => "Camera",
            Self::Lens => "Lens",
            Self::Aperture => "Aperture",
            Self::Shutter => "Shutter speed",
            Self::Iso => "ISO",
            Self::Focal => "Focal length",
        }
    }
}

/// Half-open `[lower, upper)`, so a frame falls in exactly one.
pub const FOCAL_RANGES: [(f64, &str); 7] = [
    (24., "<24 mm"),
    (35., "24–35 mm"),
    (50., "35–50 mm"),
    (85., "50–85 mm"),
    (135., "85–135 mm"),
    (200., "135–200 mm"),
    (f64::INFINITY, ">200 mm"),
];

/// The index into `FOCAL_RANGES` of `mm`.
pub fn focal_range(mm: f64) -> usize {
    FOCAL_RANGES
        .iter()
        .position(|&(upper, _)| mm < upper)
        .unwrap_or(FOCAL_RANGES.len() - 1)
}

#[derive(Clone, Debug, PartialEq)]
pub enum Order {
    Number(f64),
    Text(String),
}

/// A file's label in one group and what it sorts by.
#[derive(Clone, Debug, PartialEq)]
pub struct Key {
    pub label: String,
    pub order: Order,
}

/// The key of `group` for one file, or `None` if it has none.
pub fn exif_key(exif: Option<&Exif>, group: Group) -> Option<Key> {
    let exif = exif?;
    let text = |text: &Option<String>| {
        text.as_ref().map(|t| Key {
            label: t.clone(),
            order: Order::Text(t.clone()),
        })
    };
    let labeled = |labeled: &Option<handpic_engine::exif::Labeled>| {
        labeled.as_ref().map(|l| Key {
            label: l.label.clone(),
            order: Order::Number(l.value),
        })
    };
    match group {
        Group::Camera => text(&exif.camera),
        Group::Lens => text(&exif.lens),
        Group::Aperture => labeled(&exif.aperture),
        Group::Shutter => labeled(&exif.shutter),
        Group::Iso => labeled(&exif.iso),
        Group::Focal => exif.focal_length.as_ref().map(|l| {
            let at = focal_range(l.value);
            Key {
                label: FOCAL_RANGES[at].1.to_owned(),
                order: Order::Number(at as f64),
            }
        }),
    }
}

/// The folder's labels per group in display order, a group without any left
/// out.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Groups(pub Vec<(Group, Vec<String>)>);

impl Groups {
    pub fn labels(&self, group: Group) -> &[String] {
        self.0
            .iter()
            .find(|(g, _)| *g == group)
            .map_or(&[], |(_, labels)| labels.as_slice())
    }
}

/// The groups of the folder's files' EXIF, in the index's order: when two
/// files share a label with different orders, the last one's order counts.
/// Numbers sort by value, ties by label; text like ICU root collation
/// (`text_key`), ties by the plain compare.
pub fn groups_of<'a>(exifs: impl IntoIterator<Item = Option<&'a Exif>>) -> Groups {
    let exifs: Vec<&Exif> = exifs.into_iter().flatten().collect();
    let mut groups = Vec::new();
    for group in GROUPS {
        let mut found: Vec<Key> = Vec::new();
        for exif in &exifs {
            let Some(key) = exif_key(Some(exif), group) else {
                continue;
            };
            match found.iter_mut().find(|k| k.label == key.label) {
                Some(kept) => kept.order = key.order,
                None => found.push(key),
            }
        }
        if found.is_empty() {
            continue;
        }
        found.sort_by(compare);
        groups.push((group, found.into_iter().map(|k| k.label).collect()));
    }
    Groups(groups)
}

fn compare(a: &Key, b: &Key) -> Ordering {
    match (&a.order, &b.order) {
        (Order::Number(x), Order::Number(y)) => x.total_cmp(y).then_with(|| a.label.cmp(&b.label)),
        (Order::Text(x), Order::Text(y)) => text_key(x).cmp(&text_key(y)).then_with(|| x.cmp(y)),
        (Order::Number(_), Order::Text(_)) => Ordering::Less,
        (Order::Text(_), Order::Number(_)) => Ordering::Greater,
    }
}

/// ICU root collation's order of printable ASCII, lowercase.
const PRIMARY: &str = " _-,;:!?.'\"()[]{}@*/\\&#%`^+<=>|~$0123456789abcdefghijklmnopqrstuvwxyz";
/// ICU root collation's order of the common combining accents.
const ACCENTS: [char; 13] = [
    '\u{301}', '\u{300}', '\u{306}', '\u{302}', '\u{30C}', '\u{30A}', '\u{308}', '\u{30B}',
    '\u{303}', '\u{307}', '\u{327}', '\u{328}', '\u{304}',
];

/// An approximation of ICU root collation (`localeCompare`), not a collator:
/// the NFD string's primary, secondary (accent) and tertiary (case, lowercase
/// first) weights, each level compared as a whole sequence.
fn text_key(text: &str) -> (Vec<u32>, Vec<u32>, Vec<u32>) {
    let nfd = DecomposingNormalizerBorrowed::new_nfd().normalize(text);
    let (mut primary, mut secondary, mut tertiary) = (Vec::new(), Vec::new(), Vec::new());
    for c in nfd.chars() {
        if ('\u{300}'..='\u{36F}').contains(&c) {
            secondary.push(
                ACCENTS
                    .iter()
                    .position(|&a| a == c)
                    .map_or(0x100 + c as u32, |i| i as u32 + 1),
            );
            continue;
        }
        // After NFD no char lowercases to several (U+0130 decomposes), so the
        // first is the whole lowercase.
        let lower = c.to_lowercase().next().unwrap_or(c);
        primary.push(
            PRIMARY
                .find(lower)
                .map_or(0x10000 + lower as u32, |i| i as u32),
        );
        secondary.push(0);
        tertiary.push(u32::from(c.to_lowercase().ne([c])));
    }
    (primary, secondary, tertiary)
}

#[cfg(test)]
mod tests {
    //! `exif.test.ts`, then the group build `rebuildExifMenu` does, which has
    //! no Tauri test.

    use super::*;
    use handpic_engine::exif::Labeled;

    fn labeled(value: f64, label: &str) -> Option<Labeled> {
        Some(Labeled {
            value,
            label: label.to_owned(),
        })
    }

    fn exif() -> Exif {
        Exif {
            camera: Some("ILCE-7RM5".into()),
            lens: Some("FE 35mm F1.4 GM".into()),
            aperture: labeled(1.4, "f/1.4"),
            shutter: labeled(0.004, "1/250 s"),
            iso: labeled(100., "ISO 100"),
            focal_length: labeled(35., "35 mm"),
            ..Exif::default()
        }
    }

    fn key(label: &str, order: Order) -> Option<Key> {
        Some(Key {
            label: label.to_owned(),
            order,
        })
    }

    #[test]
    fn focal_range_buckets_each_length() {
        for (mm, label) in [
            (0., "<24 mm"),
            (23.9, "<24 mm"),
            (24., "24–35 mm"),
            (35., "35–50 mm"),
            (50., "50–85 mm"),
            (85., "85–135 mm"),
            (135., "135–200 mm"),
            (199.9, "135–200 mm"),
            (200., ">200 mm"),
            (600., ">200 mm"),
        ] {
            assert_eq!(FOCAL_RANGES[focal_range(mm)].1, label, "{mm}");
        }
    }

    #[test]
    fn exif_key_returns_none_without_exif() {
        assert_eq!(exif_key(None, Group::Camera), None);
        assert_eq!(exif_key(None, Group::Iso), None);
    }

    #[test]
    fn exif_key_orders_camera_and_lens_by_their_text() {
        let e = exif();
        assert_eq!(
            exif_key(Some(&e), Group::Camera),
            key("ILCE-7RM5", Order::Text("ILCE-7RM5".into()))
        );
        assert_eq!(
            exif_key(Some(&e), Group::Lens),
            key("FE 35mm F1.4 GM", Order::Text("FE 35mm F1.4 GM".into()))
        );
    }

    #[test]
    fn exif_key_orders_labeled_values_by_their_numeric_value() {
        let e = exif();
        assert_eq!(
            exif_key(Some(&e), Group::Aperture),
            key("f/1.4", Order::Number(1.4))
        );
        assert_eq!(
            exif_key(Some(&e), Group::Shutter),
            key("1/250 s", Order::Number(0.004))
        );
        assert_eq!(
            exif_key(Some(&e), Group::Iso),
            key("ISO 100", Order::Number(100.))
        );
    }

    #[test]
    fn exif_key_buckets_the_focal_length_into_its_range() {
        assert_eq!(
            exif_key(Some(&exif()), Group::Focal),
            key("35–50 mm", Order::Number(2.))
        );
    }

    #[test]
    fn exif_key_returns_none_for_a_missing_field() {
        let empty = Exif::default();
        for group in GROUPS {
            assert_eq!(exif_key(Some(&empty), group), None, "{group:?}");
        }
    }

    fn shot(camera: &str, iso: f64, iso_label: &str) -> Exif {
        Exif {
            camera: Some(camera.into()),
            iso: labeled(iso, iso_label),
            ..Exif::default()
        }
    }

    fn strings(labels: &[&str]) -> Vec<String> {
        labels.iter().map(|l| l.to_string()).collect()
    }

    #[test]
    fn groups_come_in_menu_order_each_sorted() {
        let exifs = [
            exif(),
            Exif {
                camera: Some("ILCE-1".into()),
                lens: Some("FE 24-70mm F2.8 GM II".into()),
                aperture: labeled(8., "f/8"),
                shutter: labeled(1.3, "1.3\""),
                iso: labeled(6400., "ISO 6400"),
                focal_length: labeled(600., "600 mm"),
                ..Exif::default()
            },
            Exif {
                camera: Some("ilce-9".into()),
                aperture: labeled(2.8, "f/2.8"),
                iso: labeled(50., "ISO 50"),
                focal_length: labeled(14., "14 mm"),
                ..Exif::default()
            },
        ];
        let groups = groups_of(exifs.iter().map(Some));
        assert_eq!(groups.0.iter().map(|(g, _)| *g).collect::<Vec<_>>(), GROUPS);
        assert_eq!(
            groups.labels(Group::Camera),
            strings(&["ILCE-1", "ILCE-7RM5", "ilce-9"])
        );
        assert_eq!(
            groups.labels(Group::Lens),
            strings(&["FE 24-70mm F2.8 GM II", "FE 35mm F1.4 GM"])
        );
        assert_eq!(
            groups.labels(Group::Aperture),
            strings(&["f/1.4", "f/2.8", "f/8"])
        );
        assert_eq!(
            groups.labels(Group::Shutter),
            strings(&["1/250 s", "1.3\""])
        );
        assert_eq!(
            groups.labels(Group::Iso),
            strings(&["ISO 50", "ISO 100", "ISO 6400"])
        );
        assert_eq!(
            groups.labels(Group::Focal),
            strings(&["<24 mm", "35–50 mm", ">200 mm"])
        );
    }

    #[test]
    fn text_differing_in_case_puts_lowercase_first() {
        let exifs = [shot("b", 1., "1"), shot("B", 1., "1"), shot("a", 1., "1")];
        let groups = groups_of(exifs.iter().map(Some));
        assert_eq!(groups.labels(Group::Camera), strings(&["a", "b", "B"]));
    }

    /// `want`, `localeCompare`'s order, sorted from its reverse and a shuffle.
    fn assert_text_order(want: &[&str]) {
        let n = want.len();
        let reversed: Vec<&str> = want.iter().rev().copied().collect();
        let shuffled: Vec<&str> = (0..n).map(|i| want[(i * 7 + 3) % n]).collect();
        for given in [reversed, shuffled] {
            let mut keys: Vec<Key> = given
                .iter()
                .map(|t| Key {
                    label: t.to_string(),
                    order: Order::Text(t.to_string()),
                })
                .collect();
            keys.sort_by(compare);
            let got: Vec<&str> = keys.iter().map(|k| k.label.as_str()).collect();
            assert_eq!(got, want, "from {given:?}");
        }
    }

    #[test]
    fn text_orders_punctuation_like_icu_root() {
        assert_text_order(&[
            "a b", "a_b", "a-b", "a:b", "a.b", "a(b", "a)b", "a/b", "a&b", "a+b", "a|b", "a1",
            "ab", "aB", "Ab", "AB",
        ]);
    }

    #[test]
    fn text_orders_accents_after_the_base_letter_like_icu_root() {
        assert_text_order(&["e", "E", "é", "É", "è", "ê", "ë", "f"]);
        assert_text_order(&["resume", "Resume", "résume", "résumé"]);
    }

    #[test]
    fn text_orders_model_suffixes_like_icu_root() {
        assert_text_order(&["Z5", "Z5 II", "Z5_2", "Z50", "Z50_2", "Z5II"]);
        assert_text_order(&[
            "NIKON CORPORATION NIKON Z5_2",
            "NIKON CORPORATION NIKON Z50_2",
        ]);
        assert_text_order(&[
            "24-70mm F2.8 DG DN | Art 019",
            "24-70mm F2.8 DG DN II | Art 024",
        ]);
    }

    #[test]
    fn numeric_ties_sort_by_label() {
        let exifs = [shot("X", 100., "ISO b"), shot("X", 100., "ISO a")];
        let groups = groups_of(exifs.iter().map(Some));
        assert_eq!(groups.labels(Group::Iso), strings(&["ISO a", "ISO b"]));
    }

    #[test]
    fn a_shared_label_takes_the_last_entrys_value() {
        let exifs = [
            shot("X", 100., "ISO 100"),
            shot("X", 200., "ISO 200"),
            shot("X", 300., "ISO 100"),
        ];
        let groups = groups_of(exifs.iter().map(Some));
        assert_eq!(groups.labels(Group::Iso), strings(&["ISO 200", "ISO 100"]));
    }

    #[test]
    fn a_group_no_file_has_is_left_out() {
        let exifs = [shot("X", 100., "ISO 100")];
        let groups = groups_of(exifs.iter().map(Some).chain([None]));
        assert_eq!(
            groups.0.iter().map(|(g, _)| *g).collect::<Vec<_>>(),
            [Group::Camera, Group::Iso]
        );
        assert!(groups.labels(Group::Lens).is_empty());
        assert_eq!(groups_of([None]), Groups::default());
    }
}
