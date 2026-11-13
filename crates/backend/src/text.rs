//! Display text both apps share: the message of an operation refused while a
//! scan runs and the byte sizes the dialogs show.

/// The base the size figure is divided by. The label mirrors what the user's
/// file manager shows next to the same file: Finder counts 1 GB as 1000^3,
/// Explorer (and the Linux file managers) as 1024^3, so the base follows the
/// platform rather than the labels changing to `GiB`.
pub const SIZE_BASE: u64 = if cfg!(target_os = "macos") {
    1000
} else {
    1024
};

/// The message shown when an operation is pressed while a scan is running. The
/// operation is refused rather than canceling the scan.
pub const SCAN_RUNNING: &str = "a scan is running; wait for it to finish";

/// Format `bytes` for display with plain `B`/`KB`/`MB`/`GB` labels: whole
/// bytes below `base`, one decimal above it. Steps up a unit once the
/// *rounded* value would reach `base` (e.g. 999_999_999 rounds to 1000.0 MB
/// at base 1000, so it must show as 1.0 GB, not 1000.0 MB), not just the raw
/// one, or a value one rounding step below a power of the base prints a
/// figure equal to the base.
pub fn format_bytes(bytes: u64, base: u64) -> String {
    if bytes < base {
        return format!("{bytes} B");
    }
    let base = base as f64;
    let round1 = |v: f64| (v * 10.0).round() / 10.0;
    let kb = bytes as f64 / base;
    if round1(kb) < base {
        return format!("{:.1} KB", kb);
    }
    let mb = kb / base;
    if round1(mb) < base {
        return format!("{:.1} MB", mb);
    }
    format!("{:.1} GB", mb / base)
}

#[cfg(test)]
mod tests {
    use super::{format_bytes, SIZE_BASE};

    #[test]
    fn format_bytes_uses_whole_bytes_below_the_base() {
        assert_eq!(format_bytes(0, 1000), "0 B");
        assert_eq!(format_bytes(0, 1024), "0 B");
        assert_eq!(format_bytes(999, 1000), "999 B");
        assert_eq!(format_bytes(1023, 1024), "1023 B");
    }

    #[test]
    fn format_bytes_steps_up_a_unit_at_each_power_of_the_base() {
        assert_eq!(format_bytes(1000, 1000), "1.0 KB");
        assert_eq!(format_bytes(1024, 1024), "1.0 KB");
        assert_eq!(format_bytes(1000 * 1000, 1000), "1.0 MB");
        assert_eq!(format_bytes(1024 * 1024, 1024), "1.0 MB");
        assert_eq!(format_bytes(1000 * 1000 * 1000, 1000), "1.0 GB");
        assert_eq!(format_bytes(1024 * 1024 * 1024, 1024), "1.0 GB");
    }

    #[test]
    fn format_bytes_shows_one_decimal_from_kb_up() {
        assert_eq!(format_bytes(1_200_000_000, 1000), "1.2 GB");
        assert_eq!(format_bytes(1288490189, 1024), "1.2 GB");
        assert_eq!(format_bytes(312_000, 1000), "312.0 KB");
    }

    #[test]
    fn format_bytes_steps_up_when_rounding_would_reach_the_base() {
        // Each of these divides to a rounded X.0 at the current unit, so it
        // must already step up rather than print e.g. "1000.0 MB".
        assert_eq!(format_bytes(999_999_999, 1000), "1.0 GB");
        assert_eq!(format_bytes(1_073_741_823, 1024), "1.0 GB");
        assert_eq!(format_bytes(999_999, 1000), "1.0 MB");
    }

    #[test]
    fn the_size_base_follows_the_platforms_file_manager() {
        // macOS Finder reports sizes in decimal units (1 GB = 1000^3 bytes);
        // Windows Explorer and most Linux file managers report binary units
        // (1 GB = 1024^3 bytes).
        let expected = if cfg!(target_os = "macos") {
            "1.1 GB"
        } else {
            "1.0 GB"
        };
        assert_eq!(format_bytes(1_073_741_824, SIZE_BASE), expected);
    }
}
