# Learnings: the native app's update flow

## Step 1: version mapping and the `Package` version input

- `version.sh [form] [version]` takes an optional version as its second
  argument, so `set-version.sh` validates its input through the same rule
  and the workflow's deb ordering check maps `2.0.0-rc.2` / `2.0.0-rc.10` /
  `2.0.0` through the script instead of writing the `~` forms by hand.
- The prerelease is restricted to dot-separated alphanumeric identifiers
  (no `-`, though SemVer allows one): a `-` in a deb upstream version is
  read as the revision separator, so `2.0.0~rc-1` would not be the version
  the control file meant. Leading zeros in `x.y.z` are refused as SemVer
  does. Build metadata (`+...`) is refused.
- `winresource` 0.1.34 writes `CARGO_PKG_VERSION` as the exe's
  `FileVersion` / `ProductVersion` strings and only the numeric
  major / minor / patch into the fixed `FILEVERSION`, so a prerelease
  version builds; the Windows leg checks the installed exe's
  `ProductVersion` against the SemVer string. Nothing in the binary reads
  `CARGO_PKG_VERSION` yet, so macOS and Linux have nothing to check until
  the updater reads it.
- The setup's file version is read as
  `FileMajorPart.FileMinorPart.FileBuildPart.FilePrivatePart`, not the
  `FileVersion` string, which Inno may write differently.
- `set-version.sh` writes the files back with `cat >` rather than `mv`, so
  `mktemp`'s `0600` mode does not replace the files' own.
- shellcheck's SC2016 fires on an awk program passed in single quotes that
  contains `$0`; passing the selectors as `awk -v` variables keeps the
  program a fixed single-quoted string.
- The "Set the version" step runs after `actions/cache`, so the cache key is
  the checked-in `Cargo.lock`'s hash and a versioned run still hits the
  cache; only `handpic-native` itself rebuilds.
- Inno Setup is not installed on this machine, so the `.iss` change
  (`VersionInfoVersion={#FileVersion}`) was first compiled on the `Package`
  runs.
- A change to `mise.toml` makes `mise run ci:local` select every check
  (all crates' clippy and tests); it took about 4 minutes here.
- `Package` runs on the branch at 2709745e, both green on the first try:
  - 44194426548 (`version=2.0.0-rc.1`): `CFBundleShortVersionString` and
    `CFBundleVersion` `2.0.0`, setup file version `2.0.0.0`,
    `DisplayVersion` and the exe's `ProductVersion` `2.0.0-rc.1`, deb
    `Version: 2.0.0~rc.1` in `handpic_2.0.0-rc.1_amd64.deb`, every asset
    named `Handpic-2.0.0-rc.1-...`. A prerelease `CARGO_PKG_VERSION` builds
    on Windows (`winresource`).
  - 44194431118 (no input): every asset at `0.0.0`.
  - A cold-ish release build plus the checks took about 75 minutes per run,
    the `macos-15-intel` leg queued longest.
