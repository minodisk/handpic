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

## Step 2: the updater's assets and the signing tooling

- The production key pair is not generated here: the user runs `mise run
  native:keygen <private-key>` and checks in the
  `crates/native/packaging/update.pub` it writes, so that Done-when item and
  the Step 2 checkbox stay open until then. Nothing in this step reads
  `update.pub` (the test key signs); Step 3's `include_bytes!` will.
- zipsign 0.2.2 (`mise exec cargo:zipsign@0.2.2`, about 10 s to build here)
  writes a 64-byte private key (secret and public halves) and a 32-byte raw
  verifying key. `sign zip` / `sign tar` without `-o` sign in place, the
  `--context` defaulting to the input's file name: a renamed copy of a
  signed ZIP fails `verify` with "no matching key/signature pair found".
  Signing an already signed file again still verifies.
- Measured locally: Windows PowerShell 5.1's `Expand-Archive` opens a ZIP
  zipsign prepended its signature to (bsdtar-made, as `windows.sh` makes
  them), and GNU tar (`tar -tzvf`) and Python's `tarfile` (what `install.sh`
  uses) read a signed `.tar.gz` without a warning.
- The key files need `-text` in `.gitattributes`: under `* text=auto
  eol=lf`, 32 random bytes hold no NUL about 88 % of the time and git would
  treat them as text.
- zipsign is a task tool (`tools = { "cargo:zipsign" = "0.2.2" }` on
  `native:sign` and `native:keygen`), not a `[tools]` entry, so
  `mise-action` and every developer's `mise install` do not build it; it is
  built on the first `mise run native:sign` with the `[tools]` Rust.
  `sign.sh` itself refuses another `zipsign --version`, so the pin holds when
  it is run outside mise too.
- `mise run native:sign -- a b` hands `a b` to the task's `"$@"` (the `--`
  is dropped); a task's relative paths resolve against the repository root.
- Signing runs in each platform leg right after packaging and before the
  checks, so the checks and the uploads see the signed bytes; `sign.sh`
  verifies each file with the given verifying key right after signing, so
  `zipsign verify` with the test verifying key is proven in every leg. The
  `checksums` job installs the tarball with `install.sh` and the digest
  `SHA256SUMS` names.
- `HANDPIC_UPDATE_TEST_REPO` goes through `$GITHUB_ENV` from a conditional
  step, so a build without the input does not see the variable at all
  (rather than an empty one). Nothing reads it before Step 3.
