//! The native app's update engine: which install the running executable
//! belongs to (`detect`), the release a feed offers it under the prerelease
//! policy (`check`, `offered`), the install of a portable ZIP or tarball
//! update and the swap of a whole `Handpic.app` through `self_update`
//! (`install`), and an Inno Setup install's
//! setup staged (`stage_setup`) and started at quit (`run_setup`), every
//! archive checked against the release's `SHA256SUMS` and its zipsign
//! signature before anything is replaced or run. The feed and the trusted keys are parameters (`Source`); a
//! build's own are `source()`'s. Blocking: the caller runs it on a worker.

use std::path::{Path, PathBuf};
use std::time::Duration;

use self_update::backends::github;
use self_update::VerifyingKey;
use semver::Version;
use sha2::{Digest, Sha256};

/// The release asset whose digests every download is checked against.
const SUMS: &str = "SHA256SUMS";

/// The bound on each request of a check: a stalled connection fails the
/// check rather than holding it, so the next one can run.
pub const CHECK_TIMEOUT: Duration = Duration::from_secs(30);
/// The bound on each request of an install, the archive's download whole
/// (`ureq`'s global timeout covers reading the body), so longer.
pub const INSTALL_TIMEOUT: Duration = Duration::from_secs(10 * 60);

/// The production verifying key, whose private key the maintainer keeps
/// outside the repository.
const PRODUCTION_KEY: VerifyingKey = *include_bytes!("../../app/packaging/update.pub");
/// The checked-in test key a `HANDPIC_UPDATE_TEST_REPO` build trusts instead.
const TEST_KEY: VerifyingKey = *include_bytes!("../../app/packaging/test/update.pub");

/// Who installed the running executable, and so who updates it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InstallKind {
    /// The Windows portable ZIP's `Handpic/` folder.
    Portable,
    /// The Inno Setup per-user install.
    Inno,
    /// A macOS `Handpic.app`.
    App,
    /// `install.sh`'s copy of the Linux tarball under `~/.local`.
    Tarball,
    /// The deb package under `/usr/lib/handpic`.
    Deb,
    /// Anything else, a `cargo run` or a build's `target/` binary included.
    Unknown,
}

/// Where releases are listed: a GitHub API base URL and a repository.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Feed {
    pub api: String,
    pub owner: String,
    pub repo: String,
}

/// The feed a build reads and the keys it trusts. `test` is a
/// `HANDPIC_UPDATE_TEST_REPO` build, which must never be distributed.
#[derive(Clone, Debug)]
pub struct Source {
    pub feed: Feed,
    pub keys: Vec<VerifyingKey>,
    pub test: bool,
}

/// The release a check offers: its SemVer version (the tag without its `v`)
/// and its release page.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Offer {
    pub version: String,
    pub page: Option<String>,
}

/// The install kind of the executable at `exe`. Inno Setup leaves
/// `unins000.exe` beside the exe; the portable ZIP has only `licenses/`
/// there.
pub fn detect(exe: &Path) -> InstallKind {
    let Some(dir) = exe.parent() else {
        return InstallKind::Unknown;
    };
    if app_bundle(exe).is_some() {
        InstallKind::App
    } else if dir == Path::new("/usr/lib/handpic") {
        InstallKind::Deb
    } else if dir.ends_with(".local/bin") {
        InstallKind::Tarball
    } else if dir.join("unins000.exe").is_file() {
        InstallKind::Inno
    } else if dir.join("licenses").is_dir() {
        InstallKind::Portable
    } else {
        InstallKind::Unknown
    }
}

/// The `<x>.app` of an executable at `<x>.app/Contents/MacOS/<exe>`.
pub fn app_bundle(exe: &Path) -> Option<&Path> {
    exe.parent()
        .filter(|dir| dir.ends_with("Contents/MacOS"))
        .and_then(Path::parent)
        .and_then(Path::parent)
        .filter(|app| app.extension().is_some_and(|ext| ext == "app"))
}

/// This build's feed and keys: `minodisk/handpic` and the production key,
/// or, with `HANDPIC_UPDATE_TEST_REPO` (`owner/name`) set at build time,
/// that repository and the test key.
pub fn source() -> Source {
    let source = source_of(option_env!("HANDPIC_UPDATE_TEST_REPO"));
    log::info!(
        "update: feed {}/{}, {} key",
        source.feed.owner,
        source.feed.repo,
        if source.test { "test" } else { "production" }
    );
    source
}

fn source_of(test_repo: Option<&str>) -> Source {
    let (repo, key, test) = match test_repo {
        Some(repo) => (repo, TEST_KEY, true),
        None => ("minodisk/handpic", PRODUCTION_KEY, false),
    };
    let (owner, repo) = repo.split_once('/').unwrap_or((repo, ""));
    Source {
        feed: Feed {
            api: "https://api.github.com".to_string(),
            owner: owner.to_string(),
            repo: repo.to_string(),
        },
        keys: vec![key],
        test,
    }
}

/// Whether `candidate` is offered to an install at `current`: newer by
/// SemVer, and without a prerelease part unless `current` has one.
fn eligible(current: &Version, candidate: &Version) -> bool {
    candidate > current && (!current.pre.is_empty() || candidate.pre.is_empty())
}

/// The newest of `versions` offered to `current`, `None` when none is (an
/// unparsable version is skipped).
pub fn offered<'a>(current: &str, versions: impl IntoIterator<Item = &'a str>) -> Option<&'a str> {
    let current = Version::parse(current).ok()?;
    versions
        .into_iter()
        .filter_map(|v| Version::parse(v).ok().map(|parsed| (v, parsed)))
        .filter(|(_, parsed)| eligible(&current, parsed))
        .max_by(|(_, a), (_, b)| a.cmp(b))
        .map(|(v, _)| v)
}

fn configure(source: &Source, current: &str) -> github::UpdateBuilder {
    let mut builder = github::Update::configure();
    builder
        .api_base_url(source.feed.api.as_str())
        .repo_owner(source.feed.owner.as_str())
        .repo_name(source.feed.repo.as_str())
        .current_version(current)
        .checksum_from_asset(SUMS)
        .verifying_keys(source.keys.clone())
        .no_confirm(true)
        .show_output(false)
        .show_download_progress(false);
    builder
}

/// The release the feed offers to an install at `current` (the app's own
/// version), `None` when it is up to date. The SemVer string decides the
/// channel, not GitHub's prerelease flag.
pub fn check(source: &Source, current: &str) -> Result<Option<Offer>, String> {
    Version::parse(current).map_err(|e| format!("version {current}: {e}"))?;
    let releases = configure(source, current)
        .bin_name("handpic")
        .timeout(CHECK_TIMEOUT)
        .build()
        .and_then(|update| update.get_newer_releases())
        .map_err(|e| e.to_string())?
        .into_vec();
    let Some(version) = offered(current, releases.iter().map(|r| r.version())) else {
        return Ok(None);
    };
    let release = releases.iter().find(|r| r.version() == version);
    Ok(Some(Offer {
        version: version.to_string(),
        page: release.and_then(|r| r.release_notes_url().map(str::to_string)),
    }))
}

/// The asset an install kind updates from and the executable's path inside
/// it, `None` for a kind `install` does not update.
fn portable_asset(kind: InstallKind, version: &str, exe_name: &str) -> Option<(String, String)> {
    match kind {
        InstallKind::Portable => Some((
            format!("Handpic-{version}-windows-x64.zip"),
            format!("Handpic/{exe_name}"),
        )),
        InstallKind::Tarball => Some((
            format!("Handpic-{version}-linux-x64.tar.gz"),
            format!("Handpic/bin/{exe_name}"),
        )),
        _ => None,
    }
}

/// Replace the executable at `exe` with the one in `offer`'s release, the
/// release pinned by its tag: a portable or tarball install, or for a `.app`
/// install the whole bundle (`install_app`). The archive is downloaded to a
/// temporary folder and checked against the release's `SHA256SUMS` and
/// `source`'s keys first; every `Err` leaves `exe` as it was. `progress` is
/// given the archive's bytes downloaded so far and its size when the server
/// sent one; the checks run after its last call.
pub fn install(
    source: &Source,
    offer: &Offer,
    kind: InstallKind,
    exe: &Path,
    current: &str,
    progress: impl Fn(u64, Option<u64>) + Send + Sync + 'static,
) -> Result<(), String> {
    let exe_name = exe
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| format!("{}: no file name", exe.display()))?;
    if kind == InstallKind::App {
        let bundle = app_bundle(exe)
            .ok_or_else(|| format!("{}: not inside a .app bundle", exe.display()))?;
        return install_app(
            source,
            offer,
            bundle,
            exe_name,
            current,
            progress,
            codesign_check,
        );
    }
    let (asset, path_in_archive) = portable_asset(kind, &offer.version, exe_name)
        .ok_or_else(|| format!("a {kind:?} install is not updated by the app"))?;
    let status = configure(source, current)
        .bin_name(exe_name)
        .bin_path_in_archive(path_in_archive)
        .bin_install_path(exe)
        .timeout(INSTALL_TIMEOUT)
        .progress_callback(progress)
        .release_tag(format!("v{}", offer.version))
        .asset_matcher(move |assets| assets.iter().find(|a| a.name() == asset).cloned())
        .build()
        .and_then(|update| update.update())
        .map_err(|e| e.to_string())?;
    if !status.is_updated() {
        return Err(format!("{} was not installed", offer.version));
    }
    Ok(())
}

/// The bundle's folder in the macOS tarball.
const APP_IN_ARCHIVE: &str = "Handpic.app";

/// What a `.app` install that cannot replace itself is told.
const MOVE_TO_APPLICATIONS: &str =
    "move Handpic.app to /Applications and open it from there, then check again";

/// The macOS tarball a `.app` install updates from, for this Mac's
/// architecture as `uname -m` names it (`arm64`, `x86_64`), as `macos.sh`
/// names the file.
fn app_asset(version: &str) -> String {
    let arch = match std::env::consts::ARCH {
        "aarch64" => "arm64",
        arch => arch,
    };
    format!("Handpic-{version}-macos-{arch}.tar.gz")
}

/// Why the bundle at `bundle` cannot be swapped where it is: run from a
/// translocated copy (a quarantined app opened outside `/Applications`), or
/// in a folder it cannot write (a read-only disk image, a folder of another
/// user). Checked before anything is downloaded.
fn misplaced(bundle: &Path) -> Option<String> {
    if bundle
        .components()
        .any(|c| c.as_os_str() == "AppTranslocation")
    {
        return Some(format!(
            "Handpic runs from a translocated copy: {MOVE_TO_APPLICATIONS}"
        ));
    }
    let parent = bundle.parent().filter(|p| !p.as_os_str().is_empty())?;
    let probe = parent.join(format!(".handpic-update-{}", std::process::id()));
    match std::fs::File::create_new(&probe) {
        Ok(_) => {
            let _ = std::fs::remove_file(&probe);
            None
        }
        Err(e) => Some(format!(
            "{} is not writable ({e}): {MOVE_TO_APPLICATIONS}",
            parent.display()
        )),
    }
}

/// The staged bundle's code signature, `codesign --verify --strict`, the
/// check before the swap. Only macOS has `codesign`.
fn codesign_check(bundle: &Path) -> Result<(), String> {
    if !cfg!(target_os = "macos") {
        return Err("a bundle's code signature is checked on macOS only".to_string());
    }
    let output = std::process::Command::new("codesign")
        .args(["--verify", "--strict"])
        .arg(bundle)
        .output()
        .map_err(|e| format!("codesign: {e}"))?;
    if output.status.success() {
        Ok(())
    } else {
        Err(format!(
            "codesign --verify --strict: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ))
    }
}

/// Replace the `.app` at `bundle` with the `Handpic.app` of `offer`'s macOS
/// tarball through `self_update`'s bundle mode: the archive checked as
/// `install`'s, the whole bundle extracted beside `bundle`, `verify`
/// (`codesign_check`) run on it, and the two folders swapped by renames, the
/// old one put back on a failed step. The executable inside is never
/// replaced alone. `exe_name` is `self_update`'s required `bin_name`.
fn install_app(
    source: &Source,
    offer: &Offer,
    bundle: &Path,
    exe_name: &str,
    current: &str,
    progress: impl Fn(u64, Option<u64>) + Send + Sync + 'static,
    verify: fn(&Path) -> Result<(), String>,
) -> Result<(), String> {
    if let Some(reason) = misplaced(bundle) {
        return Err(reason);
    }
    let asset = app_asset(&offer.version);
    let status = configure(source, current)
        .bin_name(exe_name)
        .bundle_path_in_archive(APP_IN_ARCHIVE)
        .bundle_install_path(bundle)
        .timeout(INSTALL_TIMEOUT)
        .progress_callback(progress)
        .release_tag(format!("v{}", offer.version))
        .asset_matcher(move |assets| assets.iter().find(|a| a.name() == asset).cloned())
        .verify_binary(move |staged| {
            verify(staged).map_err(self_update::Error::verification_rejected)
        })
        .build()
        .and_then(|update| update.update())
        .map_err(|e| match e {
            self_update::Error::AppTranslocated { .. }
            | self_update::Error::InstallPathNotWritable { .. } => {
                format!("{e}: {MOVE_TO_APPLICATIONS}")
            }
            e => e.to_string(),
        })?;
    if !status.is_updated() {
        return Err(format!("{} was not installed", offer.version));
    }
    Ok(())
}

/// The setup ZIP an Inno Setup install updates from and the setup's file
/// name inside it.
fn setup_asset(version: &str) -> (String, String) {
    let setup = format!("Handpic-{version}-windows-x64-setup.exe");
    (format!("{setup}.zip"), setup)
}

/// Download the setup of `offer`'s release, pinned by its tag, into `dir`
/// (emptied first), check the ZIP against the release's `SHA256SUMS` and its
/// zipsign signature against `source`'s keys, and extract the setup: its
/// path. Every `Err` leaves `dir` removed, so nothing is there to run.
/// `progress` is the ZIP's download, as `install`'s.
pub fn stage_setup(
    source: &Source,
    offer: &Offer,
    dir: &Path,
    current: &str,
    progress: impl Fn(u64, Option<u64>) + Send + Sync + 'static,
) -> Result<PathBuf, String> {
    let _ = std::fs::remove_dir_all(dir);
    let staged = stage(source, offer, dir, current, progress);
    if staged.is_err() {
        let _ = std::fs::remove_dir_all(dir);
    }
    staged
}

fn stage(
    source: &Source,
    offer: &Offer,
    dir: &Path,
    current: &str,
    progress: impl Fn(u64, Option<u64>) + Send + Sync + 'static,
) -> Result<PathBuf, String> {
    // `verify_signature` passes anything when given no key.
    if source.keys.is_empty() {
        return Err("no trusted key to verify the setup with".to_string());
    }
    let (zip_name, setup_name) = setup_asset(&offer.version);
    let tag = format!("v{}", offer.version);
    let release = configure(source, current)
        .bin_name("handpic")
        .timeout(CHECK_TIMEOUT)
        .build()
        .and_then(|update| update.get_release_version(&tag))
        .map_err(|e| e.to_string())?;
    let asset = |name: &str| {
        release
            .assets()
            .iter()
            .find(|a| a.name() == name)
            .ok_or_else(|| format!("release {tag} has no {name}"))
    };
    let (sums_asset, zip_asset) = (asset(SUMS)?, asset(&zip_name)?);
    let download = |url: &str, timeout: Duration| {
        let mut download = self_update::Download::from_url(url);
        download
            .request_header(
                self_update::http::header::ACCEPT,
                "application/octet-stream",
            )
            .timeout(timeout);
        download
    };
    let mut sums = Vec::new();
    download(sums_asset.download_url(), CHECK_TIMEOUT)
        .download_to(&mut sums)
        .map_err(|e| format!("{SUMS}: {e}"))?;
    let expected =
        match self_update::Checksum::from_sums_file(&String::from_utf8_lossy(&sums), &zip_name)
            .map_err(|e| e.to_string())?
        {
            self_update::Checksum::Sha256(hex) => hex.trim().to_ascii_lowercase(),
            _ => return Err(format!("{SUMS}: {zip_name} has no SHA-256 digest")),
        };
    std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let zip = dir.join(&zip_name);
    let file = std::fs::File::create(&zip).map_err(|e| format!("{}: {e}", zip.display()))?;
    download(zip_asset.download_url(), INSTALL_TIMEOUT)
        .progress_callback(progress)
        .download_to(file)
        .map_err(|e| format!("{zip_name}: {e}"))?;
    let bytes = std::fs::read(&zip).map_err(|e| format!("{}: {e}", zip.display()))?;
    let actual: String = Sha256::digest(&bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    if actual != expected {
        return Err(format!(
            "{zip_name}: checksum mismatch, expected {expected}, got {actual}"
        ));
    }
    self_update::verify_signature(&zip, &source.keys)
        .map_err(|e| format!("{zip_name}: signature: {e}"))?;
    self_update::Extract::from_source(&zip)
        .extract_file(dir, &setup_name)
        .map_err(|e| format!("{zip_name}: {e}"))?;
    Ok(dir.join(setup_name))
}

/// The switch `handpic.iss` reads to start the app once installed.
const RELAUNCH_SWITCH: &str = "/RELAUNCH";

/// The setup's command line: silent, without a reboot, waiting for the
/// process `pid` (the quitting app) to exit before it installs (`handpic.iss`'s
/// `/WAITPID`), and with `relaunch` starting the installed app at its end.
pub fn setup_args(pid: u32, relaunch: bool) -> Vec<String> {
    let mut args: Vec<String> = ["/VERYSILENT", "/SUPPRESSMSGBOXES", "/NORESTART"]
        .into_iter()
        .map(str::to_string)
        .collect();
    args.push(format!("/WAITPID={pid}"));
    if relaunch {
        args.push(RELAUNCH_SWITCH.to_string());
    }
    args
}

/// Start the staged `setup` detached, so it outlives the app that quits
/// right after: `setup_args` of `pid` and `relaunch`.
pub fn run_setup(setup: &Path, pid: u32, relaunch: bool) -> std::io::Result<()> {
    let mut command = std::process::Command::new(setup);
    command
        .args(setup_args(pid, relaunch))
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());
    if let Some(dir) = setup.parent() {
        command.current_dir(dir);
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const DETACHED_PROCESS: u32 = 0x0000_0008;
        const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
        command.creation_flags(DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP);
    }
    command.spawn().map(drop)
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::io::{BufRead, BufReader, Cursor, Write};
    use std::net::TcpListener;
    use std::sync::Arc;

    use zipsign_api::SigningKey;

    use super::*;

    const OLD: &[u8] = b"old executable";
    const NEW: &[u8] = b"new executable";

    fn temp_dir(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("handpic-update-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn key(seed: u8) -> SigningKey {
        SigningKey::from_bytes(&[seed; 32])
    }

    struct Release {
        version: &'static str,
        prerelease: bool,
        assets: Vec<(String, Vec<u8>)>,
    }

    fn release(version: &'static str) -> Release {
        Release {
            version,
            prerelease: false,
            assets: Vec::new(),
        }
    }

    /// A loopback GitHub API serving `releases`, each asset at
    /// `/assets/<version>/<name>`; the base URL.
    fn serve(releases: Vec<Release>) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let mut routes: HashMap<String, Vec<u8>> = HashMap::new();
        let json = |r: &Release| {
            serde_json::json!({
                "tag_name": format!("v{}", r.version),
                "name": r.version,
                "created_at": "2026-11-18T00:00:00Z",
                "prerelease": r.prerelease,
                "html_url": format!("https://github.com/o/r/releases/tag/v{}", r.version),
                "assets": r.assets.iter().map(|(name, _)| serde_json::json!({
                    "name": name,
                    "url": format!("{base}/assets/{}/{name}", r.version),
                })).collect::<Vec<_>>(),
            })
        };
        routes.insert(
            "/repos/o/r/releases".to_string(),
            serde_json::to_vec(&releases.iter().map(json).collect::<Vec<_>>()).unwrap(),
        );
        for r in &releases {
            routes.insert(
                format!("/repos/o/r/releases/tags/v{}", r.version),
                serde_json::to_vec(&json(r)).unwrap(),
            );
            for (name, bytes) in &r.assets {
                routes.insert(format!("/assets/{}/{name}", r.version), bytes.clone());
            }
        }
        let routes = Arc::new(routes);
        std::thread::spawn(move || {
            for stream in listener.incoming().flatten() {
                let mut reader = BufReader::new(stream);
                let mut request = String::new();
                if reader.read_line(&mut request).is_err() {
                    continue;
                }
                let mut line = String::new();
                while reader.read_line(&mut line).is_ok_and(|n| n > 2) {
                    line.clear();
                }
                let path = request.split(' ').nth(1).unwrap_or("");
                let path = path.split('?').next().unwrap_or("");
                let (status, body) = match routes.get(path) {
                    Some(body) => ("200 OK", body.as_slice()),
                    None => ("404 Not Found", &b"{\"message\":\"Not Found\"}"[..]),
                };
                let mut stream = reader.into_inner();
                let _ = write!(
                    stream,
                    "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    body.len()
                );
                let _ = stream.write_all(body);
            }
        });
        base
    }

    fn source(api: String, keys: Vec<VerifyingKey>) -> Source {
        Source {
            feed: Feed {
                api,
                owner: "o".to_string(),
                repo: "r".to_string(),
            },
            keys,
            test: true,
        }
    }

    fn zip_of(path: &str, bytes: &[u8]) -> Vec<u8> {
        let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
        zip.start_file(
            path,
            zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Stored),
        )
        .unwrap();
        zip.write_all(bytes).unwrap();
        zip.finish().unwrap().into_inner()
    }

    fn tar_gz_of(path: &str, bytes: &[u8]) -> Vec<u8> {
        let gz = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
        let mut tar = tar::Builder::new(gz);
        let mut header = tar::Header::new_gnu();
        header.set_size(bytes.len() as u64);
        header.set_mode(0o755);
        header.set_cksum();
        tar.append_data(&mut header, path, bytes).unwrap();
        tar.into_inner().unwrap().finish().unwrap()
    }

    /// `archive` signed by `key` under the file name `name`, as `sign.sh` does.
    fn signed(name: &str, archive: &[u8], key: &SigningKey) -> Vec<u8> {
        let mut out = Cursor::new(Vec::new());
        let keys = std::slice::from_ref(key);
        let context = Some(name.as_bytes());
        if name.ends_with(".zip") {
            zipsign_api::sign::copy_and_sign_zip(
                &mut Cursor::new(archive),
                &mut out,
                keys,
                context,
            )
            .unwrap();
        } else {
            zipsign_api::sign::copy_and_sign_tar(
                &mut Cursor::new(archive),
                &mut out,
                keys,
                context,
            )
            .unwrap();
        }
        out.into_inner()
    }

    /// A `SHA256SUMS` line for `bytes` under `name`.
    fn sums(name: &str, bytes: &[u8]) -> (String, Vec<u8>) {
        let digest: String = Sha256::digest(bytes)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        (SUMS.to_string(), format!("{digest}  {name}\n").into_bytes())
    }

    /// `archive` and the `SHA256SUMS` naming it.
    fn with_sums(archive: (String, Vec<u8>)) -> Vec<(String, Vec<u8>)> {
        vec![sums(&archive.0, &archive.1), archive]
    }

    const ZIP: &str = "Handpic-2.0.0-windows-x64.zip";
    const TAR: &str = "Handpic-2.0.0-linux-x64.tar.gz";

    fn portable_archive(signer: Option<&SigningKey>) -> (String, Vec<u8>) {
        let archive = zip_of("Handpic/handpic.exe", NEW);
        let bytes = match signer {
            Some(key) => signed(ZIP, &archive, key),
            None => archive,
        };
        (ZIP.to_string(), bytes)
    }

    /// A portable install at 1.0.0 in a temp folder; its exe.
    fn portable_install(name: &str) -> PathBuf {
        let dir = temp_dir(name).join("Handpic");
        std::fs::create_dir_all(dir.join("licenses")).unwrap();
        let exe = dir.join("handpic.exe");
        std::fs::write(&exe, OLD).unwrap();
        exe
    }

    fn offer(version: &str) -> Offer {
        Offer {
            version: version.to_string(),
            page: Some(format!("https://github.com/o/r/releases/tag/v{version}")),
        }
    }

    /// Install the 2.0.0 the feed of `assets` holds over a portable 1.0.0,
    /// trusting `key(1)`; the result and the exe's bytes afterwards.
    fn install_portable(
        name: &str,
        assets: Vec<(String, Vec<u8>)>,
    ) -> (Result<(), String>, Vec<u8>) {
        let mut r = release("2.0.0");
        r.assets = assets;
        let source = source(serve(vec![r]), vec![key(1).verifying_key().to_bytes()]);
        let exe = portable_install(name);
        let result = install(
            &source,
            &offer("2.0.0"),
            InstallKind::Portable,
            &exe,
            "1.0.0",
            |_, _| {},
        );
        (result, std::fs::read(&exe).unwrap())
    }

    #[test]
    fn the_policy_follows_semver_and_keeps_rcs_from_stable_installs() {
        let rc2 = Version::parse("2.0.0-rc.2").unwrap();
        let rc10 = Version::parse("2.0.0-rc.10").unwrap();
        let final_ = Version::parse("2.0.0").unwrap();
        assert!(rc2 < rc10 && rc10 < final_);

        let cases: &[(&str, &[&str], Option<&str>)] = &[
            ("1.9.0", &["2.0.0-rc.1", "2.0.0-rc.2"], None),
            ("1.9.0", &["2.0.0", "2.1.0-rc.1"], Some("2.0.0")),
            ("1.9.0", &["1.9.0", "1.8.0"], None),
            (
                "2.0.0-rc.1",
                &["2.0.0-rc.2", "2.0.0-rc.10"],
                Some("2.0.0-rc.10"),
            ),
            ("2.0.0-rc.2", &["2.0.0-rc.10", "2.0.0"], Some("2.0.0")),
            ("2.0.0-rc.1", &["2.1.0-rc.1"], Some("2.1.0-rc.1")),
            ("2.0.0-rc.10", &["2.0.0-rc.2", "2.0.0-rc.10"], None),
            ("2.0.0", &["2.0.0-rc.10", "nightly"], None),
        ];
        for (current, versions, expected) in cases {
            assert_eq!(
                offered(current, versions.iter().copied()),
                *expected,
                "{current} over {versions:?}"
            );
        }
    }

    #[test]
    fn a_check_reads_the_channel_from_the_version_not_the_prerelease_flag() {
        let mut flagged = release("2.0.0");
        flagged.prerelease = true;
        let base = serve(vec![release("2.1.0-rc.1"), flagged, release("2.0.0-rc.2")]);
        let source = source(base, vec![]);
        assert_eq!(check(&source, "1.9.0").unwrap(), Some(offer("2.0.0")));
        assert_eq!(
            check(&source, "2.0.0-rc.1").unwrap(),
            Some(offer("2.1.0-rc.1"))
        );
        assert_eq!(check(&source, "2.0.0").unwrap(), None);
        assert_eq!(check(&source, "2.1.0-rc.1").unwrap(), None);
    }

    #[test]
    fn an_unreachable_feed_fails_the_check_and_installs_nothing() {
        let port = TcpListener::bind("127.0.0.1:0")
            .unwrap()
            .local_addr()
            .unwrap()
            .port();
        let source = source(format!("http://127.0.0.1:{port}"), vec![]);
        assert!(check(&source, "1.0.0").is_err());
        let exe = portable_install("unreachable");
        assert!(install(
            &source,
            &offer("2.0.0"),
            InstallKind::Portable,
            &exe,
            "1.0.0",
            |_, _| {}
        )
        .is_err());
        assert_eq!(std::fs::read(&exe).unwrap(), OLD);
    }

    #[test]
    fn a_signed_zip_replaces_the_portable_exe() {
        let archive = portable_archive(Some(&key(1)));
        let assets = with_sums(archive);
        let (result, exe) = install_portable("zip", assets);
        result.unwrap();
        assert_eq!(exe, NEW);
    }

    #[test]
    fn a_signed_tarball_replaces_the_tarball_exe() {
        let archive = signed(TAR, &tar_gz_of("Handpic/bin/handpic", NEW), &key(1));
        let size = archive.len() as u64;
        let archive = (TAR.to_string(), archive);
        let mut r = release("2.0.0");
        r.assets = with_sums(archive);
        let source = source(serve(vec![r]), vec![key(1).verifying_key().to_bytes()]);
        let bin = temp_dir("tar").join(".local").join("bin");
        std::fs::create_dir_all(&bin).unwrap();
        let exe = bin.join("handpic");
        std::fs::write(&exe, OLD).unwrap();
        assert_eq!(detect(&exe), InstallKind::Tarball);
        let last = Arc::new(std::sync::Mutex::new(None));
        let seen = last.clone();
        install(
            &source,
            &offer("2.0.0"),
            InstallKind::Tarball,
            &exe,
            "1.0.0",
            move |done, total| *seen.lock().unwrap() = Some((done, total)),
        )
        .unwrap();
        assert_eq!(*last.lock().unwrap(), Some((size, Some(size))));
        assert_eq!(std::fs::read(&exe).unwrap(), NEW);
    }

    #[test]
    fn a_missing_asset_installs_nothing() {
        let tar = (TAR.to_string(), b"not this one".to_vec());
        let (result, exe) = install_portable("missing", with_sums(tar));
        let error = result.unwrap_err();
        assert!(error.to_lowercase().contains("no release"), "{error}");
        assert_eq!(exe, OLD);
    }

    #[test]
    fn a_wrong_digest_installs_nothing() {
        let archive = portable_archive(Some(&key(1)));
        let (result, exe) = install_portable("digest", vec![sums(ZIP, b"other bytes"), archive]);
        assert!(result.unwrap_err().to_lowercase().contains("checksum"));
        assert_eq!(exe, OLD);
    }

    #[test]
    fn an_unsigned_or_foreign_signed_archive_installs_nothing() {
        for (name, signer) in [("unsigned", None), ("foreign", Some(key(2)))] {
            let archive = portable_archive(signer.as_ref());
            let assets = with_sums(archive);
            let (result, exe) = install_portable(name, assets);
            let error = result.unwrap_err();
            assert!(
                error.to_lowercase().contains("signature"),
                "{name}: {error}"
            );
            assert_eq!(exe, OLD, "{name}");
        }
    }

    #[test]
    fn a_release_without_sums_installs_nothing() {
        let (result, exe) = install_portable("nosums", vec![portable_archive(Some(&key(1)))]);
        assert!(result.unwrap_err().contains(SUMS));
        assert_eq!(exe, OLD);
    }

    #[test]
    fn a_deb_or_unknown_install_installs_nothing() {
        let archive = portable_archive(Some(&key(1)));
        let mut r = release("2.0.0");
        r.assets = with_sums(archive);
        let source = source(serve(vec![r]), vec![key(1).verifying_key().to_bytes()]);
        let exe = portable_install("deb");
        for kind in [InstallKind::Deb, InstallKind::Unknown] {
            assert!(install(&source, &offer("2.0.0"), kind, &exe, "1.0.0", |_, _| {}).is_err());
            assert_eq!(std::fs::read(&exe).unwrap(), OLD);
        }
    }

    const SETUP: &str = "Handpic-2.0.0-windows-x64-setup.exe";
    const SETUP_ZIP: &str = "Handpic-2.0.0-windows-x64-setup.exe.zip";

    fn setup_archive(signer: Option<&SigningKey>) -> (String, Vec<u8>) {
        let archive = zip_of(SETUP, NEW);
        let bytes = match signer {
            Some(key) => signed(SETUP_ZIP, &archive, key),
            None => archive,
        };
        (SETUP_ZIP.to_string(), bytes)
    }

    /// Stage the 2.0.0 setup the feed of `assets` holds into a temp folder,
    /// trusting `key(1)`; the result and the staging folder.
    fn stage_from(
        name: &str,
        assets: Vec<(String, Vec<u8>)>,
    ) -> (Result<PathBuf, String>, PathBuf) {
        let mut r = release("2.0.0");
        r.assets = assets;
        let source = source(serve(vec![r]), vec![key(1).verifying_key().to_bytes()]);
        let dir = temp_dir(name).join("update");
        let result = stage_setup(&source, &offer("2.0.0"), &dir, "1.0.0", |_, _| {});
        (result, dir)
    }

    #[test]
    fn a_signed_setup_zip_is_downloaded_checked_and_extracted() {
        let archive = setup_archive(Some(&key(1)));
        let size = archive.1.len() as u64;
        let mut r = release("2.0.0");
        r.assets = with_sums(archive);
        let source = source(serve(vec![r]), vec![key(1).verifying_key().to_bytes()]);
        let dir = temp_dir("setup").join("update");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("stale.exe"), OLD).unwrap();
        let last = Arc::new(std::sync::Mutex::new(None));
        let seen = last.clone();
        let setup = stage_setup(
            &source,
            &offer("2.0.0"),
            &dir,
            "1.0.0",
            move |done, total| *seen.lock().unwrap() = Some((done, total)),
        )
        .unwrap();
        assert_eq!(setup, dir.join(SETUP));
        assert_eq!(std::fs::read(&setup).unwrap(), NEW);
        assert!(!dir.join("stale.exe").exists());
        assert_eq!(*last.lock().unwrap(), Some((size, Some(size))));
    }

    #[test]
    fn a_setup_failing_a_check_leaves_nothing_to_run() {
        let cases = [
            (
                "setup-digest",
                vec![
                    sums(SETUP_ZIP, b"other bytes"),
                    setup_archive(Some(&key(1))),
                ],
                "checksum",
            ),
            (
                "setup-unsigned",
                with_sums(setup_archive(None)),
                "signature",
            ),
            (
                "setup-foreign",
                with_sums(setup_archive(Some(&key(2)))),
                "signature",
            ),
            ("setup-nosums", vec![setup_archive(Some(&key(1)))], SUMS),
            (
                "setup-missing",
                with_sums(portable_archive(Some(&key(1)))),
                SETUP_ZIP,
            ),
        ];
        for (name, assets, reason) in cases {
            let (result, dir) = stage_from(name, assets);
            let error = result.unwrap_err();
            assert!(
                error.to_lowercase().contains(&reason.to_lowercase()),
                "{name}: {error}"
            );
            assert!(!dir.exists(), "{name}");
        }
    }

    #[test]
    fn a_setup_is_not_staged_without_a_trusted_key() {
        let mut r = release("2.0.0");
        r.assets = with_sums(setup_archive(None));
        let source = source(serve(vec![r]), vec![]);
        let dir = temp_dir("setup-nokey").join("update");
        assert!(stage_setup(&source, &offer("2.0.0"), &dir, "1.0.0", |_, _| {}).is_err());
        assert!(!dir.exists());
    }

    #[test]
    fn the_setup_runs_silently_waits_for_the_app_and_relaunches_on_request() {
        assert_eq!(
            setup_args(42, false),
            [
                "/VERYSILENT",
                "/SUPPRESSMSGBOXES",
                "/NORESTART",
                "/WAITPID=42"
            ]
        );
        assert_eq!(setup_args(42, true).last().unwrap(), RELAUNCH_SWITCH);
        let iss = include_str!("../../app/packaging/windows/handpic.iss");
        assert!(iss.contains("{param:WAITPID|0}"));
        assert!(iss.contains(&format!("'{RELAUNCH_SWITCH}'")));
    }

    /// The files of a `.app`, relative to it, with the executable at
    /// `Contents/MacOS/handpic` holding `exe`.
    fn app_files(exe: &[u8]) -> Vec<(&'static str, Vec<u8>)> {
        vec![
            ("Contents/Info.plist", b"<plist/>".to_vec()),
            ("Contents/MacOS/handpic", exe.to_vec()),
            ("Contents/Resources/icon.icns", exe.to_vec()),
        ]
    }

    /// A `.tar.gz` holding `Handpic.app` with `files`.
    fn app_tar_gz(files: &[(&str, Vec<u8>)]) -> Vec<u8> {
        let gz = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
        let mut tar = tar::Builder::new(gz);
        for (path, bytes) in files {
            let mut header = tar::Header::new_gnu();
            header.set_size(bytes.len() as u64);
            header.set_mode(0o755);
            header.set_cksum();
            tar.append_data(&mut header, format!("Handpic.app/{path}"), bytes.as_slice())
                .unwrap();
        }
        tar.into_inner().unwrap().finish().unwrap()
    }

    /// `Handpic.app` at 1.0.0 under `Applications` in a temp folder; the
    /// bundle.
    fn app_install(name: &str) -> PathBuf {
        let bundle = temp_dir(name).join("Applications").join("Handpic.app");
        for (path, bytes) in app_files(OLD) {
            let file = bundle.join(path);
            std::fs::create_dir_all(file.parent().unwrap()).unwrap();
            std::fs::write(file, bytes).unwrap();
        }
        bundle
    }

    /// Swap in the 2.0.0 bundle of a feed of `assets` trusting `key(1)`,
    /// checking the staged bundle with `verify`.
    fn install_bundle(
        bundle: &Path,
        assets: Vec<(String, Vec<u8>)>,
        verify: fn(&Path) -> Result<(), String>,
    ) -> Result<(), String> {
        let mut r = release("2.0.0");
        r.assets = assets;
        let source = source(serve(vec![r]), vec![key(1).verifying_key().to_bytes()]);
        install_app(
            &source,
            &offer("2.0.0"),
            bundle,
            "handpic",
            "1.0.0",
            |_, _| {},
            verify,
        )
    }

    fn app_archive(tar_gz: &[u8], signer: Option<&SigningKey>) -> (String, Vec<u8>) {
        let name = app_asset("2.0.0");
        let bytes = match signer {
            Some(key) => signed(&name, tar_gz, key),
            None => tar_gz.to_vec(),
        };
        (name, bytes)
    }

    fn exe_of(bundle: &Path) -> Vec<u8> {
        std::fs::read(bundle.join("Contents/MacOS/handpic")).unwrap()
    }

    type Verify = fn(&Path) -> Result<(), String>;
    /// A case: its name, the feed's assets, the verifier and the error's words.
    type Case = (&'static str, Vec<(String, Vec<u8>)>, Verify, &'static str);

    #[test]
    fn a_signed_tarball_swaps_the_whole_bundle() {
        let bundle = app_install("app");
        std::fs::write(bundle.join("Contents/Resources/stale"), OLD).unwrap();
        let tar = app_tar_gz(&app_files(NEW));
        install_bundle(
            &bundle,
            with_sums(app_archive(&tar, Some(&key(1)))),
            |staged| {
                assert!(staged.ends_with(APP_IN_ARCHIVE));
                Ok(())
            },
        )
        .unwrap();
        assert_eq!(exe_of(&bundle), NEW);
        assert_eq!(
            std::fs::read(bundle.join("Contents/Resources/icon.icns")).unwrap(),
            NEW
        );
        assert!(!bundle.join("Contents/Resources/stale").exists());
        let left: Vec<_> = std::fs::read_dir(bundle.parent().unwrap())
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .collect();
        assert_eq!(left, ["Handpic.app"]);
    }

    #[test]
    fn a_bundle_failing_a_check_leaves_the_old_bundle_in_place() {
        let tar = app_tar_gz(&app_files(NEW));
        let other = tar_gz_of("Other.app/Contents/MacOS/handpic", NEW);
        let cases: [Case; 5] = [
            (
                "app-unsigned",
                with_sums(app_archive(&tar, None)),
                |_| Ok(()),
                "signature",
            ),
            (
                "app-foreign",
                with_sums(app_archive(&tar, Some(&key(2)))),
                |_| Ok(()),
                "signature",
            ),
            (
                "app-digest",
                vec![
                    sums(&app_asset("2.0.0"), b"other bytes"),
                    app_archive(&tar, Some(&key(1))),
                ],
                |_| Ok(()),
                "checksum",
            ),
            (
                "app-codesign",
                with_sums(app_archive(&tar, Some(&key(1)))),
                |_| Err("code signature invalid".to_string()),
                "code signature invalid",
            ),
            (
                "app-nobundle",
                with_sums(app_archive(&other, Some(&key(1)))),
                |_| Ok(()),
                "no bundle directory",
            ),
        ];
        for (name, assets, verify, reason) in cases {
            let bundle = app_install(name);
            let error = install_bundle(&bundle, assets, verify).unwrap_err();
            assert!(error.to_lowercase().contains(reason), "{name}: {error}");
            assert_eq!(exe_of(&bundle), OLD, "{name}");
            let left = std::fs::read_dir(bundle.parent().unwrap()).unwrap().count();
            assert_eq!(left, 1, "{name}");
        }
    }

    #[test]
    fn a_translocated_bundle_is_told_to_move_to_applications() {
        let bundle = temp_dir("translocated")
            .join("AppTranslocation")
            .join("d")
            .join("Handpic.app");
        std::fs::create_dir_all(&bundle).unwrap();
        let tar = app_tar_gz(&app_files(NEW));
        let error = install_bundle(&bundle, with_sums(app_archive(&tar, Some(&key(1)))), |_| {
            Ok(())
        })
        .unwrap_err();
        assert!(error.contains("/Applications"), "{error}");
        assert!(!bundle.join("Contents").exists());
    }

    #[cfg(unix)]
    #[test]
    fn a_bundle_in_a_folder_it_cannot_write_is_told_to_move_to_applications() {
        use std::os::unix::fs::PermissionsExt;
        let bundle = app_install("readonly");
        let parent = bundle.parent().unwrap().to_path_buf();
        std::fs::set_permissions(&parent, std::fs::Permissions::from_mode(0o555)).unwrap();
        let tar = app_tar_gz(&app_files(NEW));
        let result = install_bundle(&bundle, with_sums(app_archive(&tar, Some(&key(1)))), |_| {
            Ok(())
        });
        std::fs::set_permissions(&parent, std::fs::Permissions::from_mode(0o755)).unwrap();
        let error = result.unwrap_err();
        assert!(error.contains("/Applications"), "{error}");
        assert_eq!(exe_of(&bundle), OLD);
    }

    /// A real, ad hoc signed bundle passes `codesign_check` and is swapped
    /// in; one changed after signing is refused and the old bundle stays.
    #[cfg(target_os = "macos")]
    #[test]
    fn codesign_checks_the_staged_bundle_before_the_swap() {
        fn signed_bundle(dir: &Path) -> PathBuf {
            let app = dir.join(APP_IN_ARCHIVE);
            std::fs::create_dir_all(app.join("Contents/MacOS")).unwrap();
            std::fs::copy("/usr/bin/true", app.join("Contents/MacOS/handpic")).unwrap();
            std::fs::write(
                app.join("Contents/Info.plist"),
                concat!(
                    r#"<?xml version="1.0" encoding="UTF-8"?>"#,
                    "\n<plist version=\"1.0\"><dict>",
                    "<key>CFBundleExecutable</key><string>handpic</string>",
                    "<key>CFBundleIdentifier</key><string>dev.minodisk.handpic.test</string>",
                    "<key>CFBundlePackageType</key><string>APPL</string>",
                    "</dict></plist>\n"
                ),
            )
            .unwrap();
            let status = std::process::Command::new("codesign")
                .args(["--force", "--sign", "-"])
                .arg(&app)
                .status()
                .unwrap();
            assert!(status.success());
            app
        }
        fn tar_gz_dir(app: &Path) -> Vec<u8> {
            let gz = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
            let mut tar = tar::Builder::new(gz);
            tar.append_dir_all(APP_IN_ARCHIVE, app).unwrap();
            tar.into_inner().unwrap().finish().unwrap()
        }

        let app = signed_bundle(&temp_dir("codesign-src"));
        codesign_check(&app).unwrap();
        let good = tar_gz_dir(&app);
        let mut plist = std::fs::OpenOptions::new()
            .append(true)
            .open(app.join("Contents/Info.plist"))
            .unwrap();
        plist.write_all(b"\n").unwrap();
        drop(plist);
        assert!(codesign_check(&app).is_err());
        let tampered = tar_gz_dir(&app);

        let bundle = app_install("codesign-bad");
        let error = install_bundle(
            &bundle,
            with_sums(app_archive(&tampered, Some(&key(1)))),
            codesign_check,
        )
        .unwrap_err();
        assert!(error.contains("codesign"), "{error}");
        assert_eq!(exe_of(&bundle), OLD);

        let bundle = app_install("codesign-good");
        install_bundle(
            &bundle,
            with_sums(app_archive(&good, Some(&key(1)))),
            codesign_check,
        )
        .unwrap();
        assert_eq!(exe_of(&bundle), std::fs::read("/usr/bin/true").unwrap());
        codesign_check(&bundle).unwrap();
    }

    #[test]
    fn the_macos_tarball_is_named_for_this_macs_architecture() {
        let name = app_asset("2.0.0");
        assert!(name.starts_with("Handpic-2.0.0-macos-") && name.ends_with(".tar.gz"));
        assert!(!name.contains("aarch64"));
    }

    #[test]
    fn the_install_kind_follows_the_executables_place() {
        let exe = portable_install("detect");
        assert_eq!(detect(&exe), InstallKind::Portable);
        std::fs::write(exe.with_file_name("unins000.exe"), b"").unwrap();
        assert_eq!(detect(&exe), InstallKind::Inno);
        let cases = [
            (
                "/Applications/Handpic.app/Contents/MacOS/handpic",
                InstallKind::App,
            ),
            ("/usr/lib/handpic/handpic", InstallKind::Deb),
            ("/home/u/.local/bin/handpic", InstallKind::Tarball),
            (
                "/home/u/handpic/target/release/handpic",
                InstallKind::Unknown,
            ),
            (
                "/Applications/Handpic/Contents/MacOS/handpic",
                InstallKind::Unknown,
            ),
        ];
        assert_eq!(
            app_bundle(Path::new(
                "/Applications/Handpic.app/Contents/MacOS/handpic"
            )),
            Some(Path::new("/Applications/Handpic.app"))
        );
        assert_eq!(app_bundle(Path::new("/usr/lib/handpic/handpic")), None);
        for (path, kind) in cases {
            assert_eq!(detect(Path::new(path)), kind, "{path}");
        }
    }

    #[test]
    fn a_test_build_reads_its_repository_with_the_test_key() {
        let production = source_of(None);
        assert_eq!(
            (
                production.feed.owner.as_str(),
                production.feed.repo.as_str()
            ),
            ("minodisk", "handpic")
        );
        assert_eq!(production.keys, vec![PRODUCTION_KEY]);
        assert!(!production.test);
        let test = source_of(Some("someone/scratch"));
        assert_eq!(
            (test.feed.owner.as_str(), test.feed.repo.as_str()),
            ("someone", "scratch")
        );
        assert_eq!(test.keys, vec![TEST_KEY]);
        assert!(test.test);
        assert_ne!(PRODUCTION_KEY, TEST_KEY);
    }
}
