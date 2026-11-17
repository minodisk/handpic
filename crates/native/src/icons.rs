//! The Lucide icons the app draws, served to GPUI's `svg().path(..)` by
//! `Icons`, the application's one `AssetSource`: its own derivatives first,
//! then the catalog icons selected from `gpui-kit-assets` with
//! `icon_assets!`, then `gpui_kit::assets::Assets`, which holds `star`,
//! the tree's folders and the icons gpui-component's
//! controls draw. It also serves the
//! app's own icon PNG to the title bar's `img(..)`.

// The own SVGs below are derivatives of the Lucide `star` and `scan-face`
// icons, lucide-static v1.48.0
// (https://github.com/lucide-icons/lucide), ISC License, Copyright (c) 2026
// Lucide Icons and Contributors.
//
// Permission to use, copy, modify, and/or distribute this software for any
// purpose with or without fee is hereby granted, provided that the above
// copyright notice and this permission notice appear in all copies.
//
// The Face focus Low icon is `scan-face` with its mouth turned down, and the
// filled star is `star` with `fill="currentColor"`, derivatives.
//
// Full notice in `crates/native/LICENSE-lucide`. The unmodified icons come
// from `gpui-kit-assets`, under its own `LICENSE-LUCIDE`.

use std::borrow::Cow;

use gpui_kit::assets::Assets;
use gpui_kit::{AssetSource, Result, SharedString};

pub const STAR: &str = "icons/star.svg";
pub const STAR_FILLED: &str = "icons/star-filled.svg";
pub const RECTANGLE_VERTICAL: &str = "icons/rectangle-vertical.svg";
pub const RECTANGLE_HORIZONTAL: &str = "icons/rectangle-horizontal.svg";
pub const SCAN_FACE: &str = "icons/scan-face.svg";
pub const SCAN_FACE_FROWN: &str = "icons/scan-face-frown.svg";
pub const FOLDER: &str = "icons/folder.svg";
pub const FOLDER_OPEN: &str = "icons/folder-open.svg";
pub const APP: &str = "icons/app.png";

const APP_PNG: &[u8] = include_bytes!("../../app/icons/64x64.png");

const STAR_FILLED_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="currentColor" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M11.525 2.295a.53.53 0 0 1 .95 0l2.31 4.679a2.123 2.123 0 0 0 1.595 1.16l5.166.756a.53.53 0 0 1 .294.904l-3.736 3.638a2.123 2.123 0 0 0-.611 1.878l.882 5.14a.53.53 0 0 1-.771.56l-4.618-2.428a2.122 2.122 0 0 0-1.973 0L6.396 21.01a.53.53 0 0 1-.77-.56l.881-5.139a2.122 2.122 0 0 0-.611-1.879L2.16 9.795a.53.53 0 0 1 .294-.906l5.165-.755a2.122 2.122 0 0 0 1.597-1.16z"/></svg>"#;

const SCAN_FACE_FROWN_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M3 7V5a2 2 0 0 1 2-2h2"/><path d="M17 3h2a2 2 0 0 1 2 2v2"/><path d="M21 17v2a2 2 0 0 1-2 2h-2"/><path d="M7 21H5a2 2 0 0 1-2-2v-2"/><path d="M8 16s1.5-2 4-2 4 2 4 2"/><path d="M9 9h.01"/><path d="M15 9h.01"/></svg>"#;

const SVGS: [(&str, &str); 2] = [
    (STAR_FILLED, STAR_FILLED_SVG),
    (SCAN_FACE_FROWN, SCAN_FACE_FROWN_SVG),
];

// `star` and the folders are left out: the default
// `Assets` already holds them.
// If `every_icon_path_loads_an_svg` fails after a gpui-kit-assets bump, move
// the missing one into this list.
gpui_kit::assets::icon_assets!(Catalog, [RectangleVertical, RectangleHorizontal, ScanFace]);

pub struct Icons;

impl AssetSource for Icons {
    /// The own paths first, then the catalog selection, then the components'
    /// set, a path in none of them loading nothing (`Assets` errs on it).
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        if path == APP {
            return Ok(Some(Cow::Borrowed(APP_PNG)));
        }
        if let Some((_, svg)) = SVGS.iter().find(|(known, _)| *known == path) {
            return Ok(Some(Cow::Borrowed(svg.as_bytes())));
        }
        if let Some(svg) = Catalog.load(path)? {
            return Ok(Some(svg));
        }
        Ok(Assets.load(path).unwrap_or(None))
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        let mut paths: Vec<SharedString> = SVGS
            .iter()
            .filter(|(known, _)| known.starts_with(path))
            .map(|(known, _)| SharedString::from(*known))
            .collect();
        if APP.starts_with(path) {
            paths.push(SharedString::from(APP));
        }
        for other in Catalog.list(path)?.into_iter().chain(Assets.list(path)?) {
            if !paths.contains(&other) {
                paths.push(other);
            }
        }
        Ok(paths)
    }
}

#[cfg(test)]
mod tests {
    use gpui_kit::assets::IconName;

    use super::*;

    const PATHS: [&str; 8] = [
        STAR,
        STAR_FILLED,
        RECTANGLE_VERTICAL,
        RECTANGLE_HORIZONTAL,
        SCAN_FACE,
        SCAN_FACE_FROWN,
        FOLDER,
        FOLDER_OPEN,
    ];

    #[test]
    fn every_icon_path_loads_an_svg() {
        for path in PATHS {
            let bytes = Icons.load(path).unwrap().expect(path);
            assert!(bytes.starts_with(b"<svg"), "{path}");
            assert!(bytes.trim_ascii_end().ends_with(b"</svg>"), "{path}");
        }
    }

    #[test]
    fn the_app_icon_path_loads_png_bytes() {
        let bytes = Icons.load(APP).unwrap().expect(APP);
        assert!(bytes.starts_with(b"\x89PNG\r\n\x1a\n"));
    }

    #[test]
    fn the_catalog_paths_equal_their_icon_names() {
        for (path, name) in [
            (STAR, IconName::Star),
            (RECTANGLE_VERTICAL, IconName::RectangleVertical),
            (RECTANGLE_HORIZONTAL, IconName::RectangleHorizontal),
            (SCAN_FACE, IconName::ScanFace),
            (FOLDER, IconName::Folder),
            (FOLDER_OPEN, IconName::FolderOpen),
        ] {
            assert_eq!(name.path().as_ref(), path);
        }
    }

    #[test]
    fn a_derivative_loads_its_own_bytes() {
        let bytes = Icons.load(STAR_FILLED).unwrap().unwrap();
        assert_eq!(bytes.as_ref(), STAR_FILLED_SVG.as_bytes());
    }

    #[test]
    fn a_component_icon_loads_from_their_set() {
        let bytes = Icons.load("icons/check.svg").unwrap().expect("check");
        assert!(bytes.starts_with(b"<svg"));
        let bytes = Icons.load("icons/settings.svg").unwrap().expect("settings");
        assert!(bytes.starts_with(b"<svg"));
    }

    #[test]
    fn an_unknown_path_loads_nothing() {
        assert!(Icons.load("icons/no-such-icon.svg").unwrap().is_none());
        assert!(Icons.load("").unwrap().is_none());
    }

    #[test]
    fn the_listing_names_each_icon_under_the_prefix_once() {
        let listed = Icons.list("icons/").unwrap();
        for path in PATHS {
            assert_eq!(
                listed.iter().filter(|p| p.as_ref() == path).count(),
                1,
                "{path}"
            );
        }
        assert_eq!(listed.iter().filter(|p| p.as_ref() == APP).count(), 1);
        assert!(listed.iter().any(|p| p.as_ref() == "icons/check.svg"));
        assert!(Icons.list("fonts/").unwrap().is_empty());
    }
}
