//! The update flow, kept pure: the launch check's throttle, the phases of
//! one check and the install it leads to, the Updates page's texts and
//! buttons per install kind, and the state `get_status` reports. The window
//! runs the backend's `update::check` and `update::install` on a thread of
//! their own and hands their answers here as `Message`s; the engine reports
//! no phase itself, so the phases are set around the two calls. An Inno
//! Setup install's "installed" is its setup staged (`update::stage_setup`),
//! which the quit runs (`Staged`); a `.app` install's is its bundle swapped.

use std::path::PathBuf;

use handpic_backend::update::{InstallKind, Offer};
use serde_json::{Value, json};

/// The settings store's key of the last automatic check, in Unix seconds.
pub const LAST_CHECK_KEY: &str = "updateCheckedAt";

/// At most one automatic check this often; the button always checks.
pub const CHECK_INTERVAL_SECS: u64 = 60 * 60;

pub const CHECK_LABEL: &str = "Check for Updates";
pub const RESTART_LABEL: &str = "Restart Now";
pub const RELEASE_PAGE_LABEL: &str = "Open Release Page";

/// Whether an automatic check is due at `now` after one at `last`. A time
/// in the future (a changed clock) counts as due.
pub fn due(last: Option<u64>, now: u64) -> bool {
    last.is_none_or(|last| now < last || now - last >= CHECK_INTERVAL_SECS)
}

/// Whether the launch checks: a release build, not under `--auto`, and due.
pub fn launch_checks(release_build: bool, auto: bool, last: Option<u64>, now: u64) -> bool {
    release_build && !auto && due(last, now)
}

/// Whether the app installs an update itself for `kind`. The others get the
/// release page.
pub fn installs(kind: InstallKind) -> bool {
    matches!(
        kind,
        InstallKind::Portable | InstallKind::Tarball | InstallKind::Inno | InstallKind::App
    )
}

/// An Inno Setup install's staged setup, shared with the quit handler that
/// starts it once the drains have run; `relaunch` is `Restart Now`'s.
#[derive(Debug, Default)]
pub struct Staged {
    pub setup: Option<PathBuf>,
    pub relaunch: bool,
}

pub fn kind_label(kind: InstallKind) -> &'static str {
    match kind {
        InstallKind::Portable => "portable ZIP",
        InstallKind::Inno => "installer",
        InstallKind::App => "Handpic.app",
        InstallKind::Tarball => "tarball under ~/.local",
        InstallKind::Deb => "deb package",
        InstallKind::Unknown => "not an install the app updates",
    }
}

fn kind_name(kind: InstallKind) -> &'static str {
    match kind {
        InstallKind::Portable => "portable",
        InstallKind::Inno => "inno",
        InstallKind::App => "app",
        InstallKind::Tarball => "tarball",
        InstallKind::Deb => "deb",
        InstallKind::Unknown => "unknown",
    }
}

/// What the worker threads hand back.
#[derive(Clone, Debug)]
pub enum Message {
    Checked(Result<Option<Offer>, String>),
    /// The archive's bytes downloaded so far and its size when known.
    Progress(u64, Option<u64>),
    Installed(Result<(), String>),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Phase {
    Idle,
    Checking,
    /// A release the app does not install itself, with its page.
    Available(Offer),
    Downloading {
        version: String,
        percent: Option<u8>,
    },
    Verifying(String),
    /// Installed on disk by this process; a restart runs it.
    ReadyToRestart(String),
    Failed(String),
}

/// What the window does after a check came back.
#[derive(Debug, PartialEq, Eq)]
pub enum Next {
    Nothing,
    Install(Offer),
}

/// A step of the download worth a message: the percent when the size is
/// known, else each MiB, so a download sends about a hundred, not one per
/// chunk.
pub fn progress_step(done: u64, total: Option<u64>) -> u64 {
    match total {
        Some(total) if total > 0 => done.saturating_mul(100) / total,
        _ => done >> 20,
    }
}

#[derive(Clone, Debug)]
pub struct Updates {
    /// The running version, `CARGO_PKG_VERSION`.
    pub version: String,
    pub kind: InstallKind,
    /// `owner/name` of a test build's feed, `None` for a production build.
    pub test_feed: Option<String>,
    phase: Phase,
    /// The last check found nothing newer.
    up_to_date: bool,
    /// The version this process installed, never installed twice.
    installed: Option<String>,
}

impl Updates {
    pub fn new(version: &str, kind: InstallKind, test_feed: Option<String>) -> Self {
        Self {
            version: version.to_owned(),
            kind,
            test_feed,
            phase: Phase::Idle,
            up_to_date: false,
            installed: None,
        }
    }

    #[cfg(test)]
    pub fn phase(&self) -> &Phase {
        &self.phase
    }

    /// A check or an install is under way.
    pub fn busy(&self) -> bool {
        matches!(
            self.phase,
            Phase::Checking | Phase::Downloading { .. } | Phase::Verifying(_)
        )
    }

    /// `Check for Updates` is enabled: nothing under way and nothing
    /// installed that waits for the restart.
    pub fn can_check(&self) -> bool {
        !self.busy() && self.installed.is_none()
    }

    /// A check starts, `false` when one may not now.
    pub fn start_check(&mut self) -> bool {
        if !self.can_check() {
            return false;
        }
        self.phase = Phase::Checking;
        self.up_to_date = false;
        true
    }

    /// A message of the worker.
    pub fn apply(&mut self, message: Message) -> Next {
        match message {
            Message::Checked(result) => return self.checked(result),
            Message::Progress(done, total) => self.progress(done, total),
            Message::Installed(result) => self.installed(result),
        }
        Next::Nothing
    }

    fn checked(&mut self, result: Result<Option<Offer>, String>) -> Next {
        if self.phase != Phase::Checking {
            return Next::Nothing;
        }
        match result {
            Ok(None) => {
                self.phase = Phase::Idle;
                self.up_to_date = true;
                Next::Nothing
            }
            Ok(Some(offer)) if installs(self.kind) => {
                self.phase = Phase::Downloading {
                    version: offer.version.clone(),
                    percent: None,
                };
                Next::Install(offer)
            }
            Ok(Some(offer)) => {
                self.phase = Phase::Available(offer);
                Next::Nothing
            }
            Err(e) => {
                self.phase = Phase::Failed(e);
                Next::Nothing
            }
        }
    }

    /// The download is done once its known size is reached; the checks of
    /// the digest and the signature follow.
    fn progress(&mut self, done: u64, total: Option<u64>) {
        let Phase::Downloading { version, .. } = &self.phase else {
            return;
        };
        let version = version.clone();
        self.phase = match total {
            Some(total) if done >= total => Phase::Verifying(version),
            Some(total) if total > 0 => Phase::Downloading {
                version,
                percent: Some((done.saturating_mul(100) / total) as u8),
            },
            _ => Phase::Downloading {
                version,
                percent: None,
            },
        };
    }

    fn installed(&mut self, result: Result<(), String>) {
        let version = match &self.phase {
            Phase::Downloading { version, .. } | Phase::Verifying(version) => version.clone(),
            _ => return,
        };
        self.phase = match result {
            Ok(()) => {
                self.installed = Some(version.clone());
                Phase::ReadyToRestart(version)
            }
            Err(e) => Phase::Failed(e),
        };
    }

    /// The page's version line.
    pub fn version_line(&self) -> String {
        format!("Handpic {} ({})", self.version, kind_label(self.kind))
    }

    /// A test build says so on the page.
    pub fn test_note(&self) -> Option<String> {
        self.test_feed.as_ref().map(|feed| {
            format!(
                "Test build: updates come from {feed} and are trusted by the test key. \
                 Never distribute it."
            )
        })
    }

    /// The phase in words.
    pub fn status_text(&self) -> String {
        match &self.phase {
            Phase::Idle if self.up_to_date => "Handpic is up to date.".to_owned(),
            Phase::Idle => "Not checked yet.".to_owned(),
            Phase::Checking => "Checking for updates\u{2026}".to_owned(),
            Phase::Available(offer) => match self.kind {
                InstallKind::Deb => format!(
                    "Handpic {} is available. Install it with its deb package from the \
                     release page.",
                    offer.version
                ),
                _ => format!(
                    "Handpic {} is available. This copy is not updated by the app; download \
                     it from the release page.",
                    offer.version
                ),
            },
            Phase::Downloading {
                version,
                percent: Some(percent),
            } => format!("Downloading Handpic {version}\u{2026} {percent}%"),
            Phase::Downloading { version, .. } => {
                format!("Downloading Handpic {version}\u{2026}")
            }
            Phase::Verifying(version) => format!("Verifying Handpic {version}\u{2026}"),
            Phase::ReadyToRestart(version) if self.kind == InstallKind::Inno => format!(
                "Handpic {version} is downloaded and installs when Handpic quits. Restart to \
                 finish now."
            ),
            Phase::ReadyToRestart(version) => {
                format!("Handpic {version} is installed. Restart to finish.")
            }
            Phase::Failed(message) => format!("Update failed: {message}"),
        }
    }

    /// The pane's note while an installed update waits for the restart.
    pub fn notice(&self) -> Option<String> {
        match &self.phase {
            Phase::ReadyToRestart(version) if self.kind == InstallKind::Inno => Some(format!(
                "Handpic {version} installs when Handpic quits, or now with {RESTART_LABEL} on \
                 the Updates page of the settings."
            )),
            Phase::ReadyToRestart(version) => Some(format!(
                "Handpic {version} is installed. Restart to finish: {RESTART_LABEL} on the \
                 Updates page of the settings."
            )),
            _ => None,
        }
    }

    /// `Restart Now` is shown.
    pub fn shows_restart(&self) -> bool {
        matches!(self.phase, Phase::ReadyToRestart(_))
    }

    /// The page `Open Release Page` opens, while it is shown.
    pub fn release_page(&self) -> Option<&str> {
        match &self.phase {
            Phase::Available(offer) => offer.page.as_deref(),
            _ => None,
        }
    }

    fn phase_name(&self) -> &'static str {
        match self.phase {
            Phase::Idle => "idle",
            Phase::Checking => "checking",
            Phase::Available(_) => "available",
            Phase::Downloading { .. } => "downloading",
            Phase::Verifying(_) => "verifying",
            Phase::ReadyToRestart(_) => "ready_to_restart",
            Phase::Failed(_) => "failed",
        }
    }

    /// What `get_status` reports: the state only, the check and the
    /// restart staying the user's.
    pub fn state(&self) -> Value {
        let target = match &self.phase {
            Phase::Available(offer) => Some(offer.version.as_str()),
            Phase::Downloading { version, .. }
            | Phase::Verifying(version)
            | Phase::ReadyToRestart(version) => Some(version.as_str()),
            _ => None,
        };
        let percent = match self.phase {
            Phase::Downloading { percent, .. } => percent,
            _ => None,
        };
        json!({
            "version": self.version,
            "install": kind_name(self.kind),
            "test_build": self.test_feed.is_some(),
            "phase": self.phase_name(),
            "target": target,
            "percent": percent,
            "message": self.status_text(),
            "release_page": self.release_page(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn offer(version: &str) -> Offer {
        Offer {
            version: version.to_owned(),
            page: Some(format!("https://github.com/o/r/releases/tag/v{version}")),
        }
    }

    fn checking(kind: InstallKind) -> Updates {
        let mut updates = Updates::new("1.0.0", kind, None);
        assert!(updates.start_check());
        updates
    }

    #[test]
    fn the_launch_checks_at_most_hourly_in_a_release_build_without_auto() {
        assert!(due(None, 10));
        assert!(!due(Some(1000), 1000 + CHECK_INTERVAL_SECS - 1));
        assert!(due(Some(1000), 1000 + CHECK_INTERVAL_SECS));
        assert!(due(Some(1000), 999));
        assert!(launch_checks(true, false, None, 10));
        assert!(!launch_checks(false, false, None, 10));
        assert!(!launch_checks(true, true, None, 10));
        assert!(!launch_checks(true, false, Some(10), 11));
    }

    #[test]
    fn a_portable_tarball_inno_or_app_install_downloads_verifies_and_waits_for_the_restart() {
        for kind in [
            InstallKind::Portable,
            InstallKind::Tarball,
            InstallKind::Inno,
            InstallKind::App,
        ] {
            let mut updates = checking(kind);
            assert_eq!(updates.status_text(), "Checking for updates\u{2026}");
            assert_eq!(
                updates.apply(Message::Checked(Ok(Some(offer("2.0.0"))))),
                Next::Install(offer("2.0.0"))
            );
            assert_eq!(updates.status_text(), "Downloading Handpic 2.0.0\u{2026}");
            updates.apply(Message::Progress(25, Some(100)));
            assert_eq!(
                updates.status_text(),
                "Downloading Handpic 2.0.0\u{2026} 25%"
            );
            updates.apply(Message::Progress(100, Some(100)));
            assert_eq!(updates.phase(), &Phase::Verifying("2.0.0".into()));
            assert!(updates.busy() && !updates.can_check());
            updates.apply(Message::Installed(Ok(())));
            assert_eq!(updates.phase(), &Phase::ReadyToRestart("2.0.0".into()));
            assert!(updates.shows_restart());
            assert!(updates.notice().is_some());
            assert_eq!(updates.release_page(), None);
        }
    }

    #[test]
    fn an_inno_install_says_its_setup_runs_at_quit() {
        let ready = |kind| {
            let mut updates = checking(kind);
            updates.apply(Message::Checked(Ok(Some(offer("2.0.0")))));
            updates.apply(Message::Installed(Ok(())));
            updates
        };
        assert_eq!(
            ready(InstallKind::Portable).status_text(),
            "Handpic 2.0.0 is installed. Restart to finish."
        );
        let inno = ready(InstallKind::Inno);
        assert_eq!(
            inno.status_text(),
            "Handpic 2.0.0 is downloaded and installs when Handpic quits. Restart to finish now."
        );
        assert!(
            inno.notice()
                .unwrap()
                .contains("installs when Handpic quits")
        );
    }

    #[test]
    fn an_installed_version_is_not_checked_or_installed_again() {
        let mut updates = checking(InstallKind::Portable);
        updates.apply(Message::Checked(Ok(Some(offer("2.0.0")))));
        updates.apply(Message::Installed(Ok(())));
        assert!(!updates.can_check());
        assert!(!updates.start_check());
        assert_eq!(
            updates.apply(Message::Checked(Ok(Some(offer("2.0.0"))))),
            Next::Nothing
        );
        assert!(updates.shows_restart());
    }

    #[test]
    fn a_download_of_unknown_size_stays_downloading_until_installed() {
        let mut updates = checking(InstallKind::Tarball);
        updates.apply(Message::Checked(Ok(Some(offer("2.0.0")))));
        updates.apply(Message::Progress(5 << 20, None));
        assert_eq!(updates.status_text(), "Downloading Handpic 2.0.0\u{2026}");
        updates.apply(Message::Installed(Ok(())));
        assert!(updates.shows_restart());
    }

    #[test]
    fn a_deb_or_unknown_install_gets_the_release_page_and_installs_nothing() {
        for kind in [InstallKind::Deb, InstallKind::Unknown] {
            let mut updates = checking(kind);
            assert_eq!(
                updates.apply(Message::Checked(Ok(Some(offer("2.0.0"))))),
                Next::Nothing
            );
            assert_eq!(updates.phase(), &Phase::Available(offer("2.0.0")));
            assert_eq!(
                updates.release_page(),
                Some("https://github.com/o/r/releases/tag/v2.0.0")
            );
            assert!(!updates.shows_restart());
            assert!(updates.can_check());
        }
        let mut deb = checking(InstallKind::Deb);
        deb.apply(Message::Checked(Ok(Some(offer("2.0.0")))));
        assert!(deb.status_text().contains("deb package"));
    }

    #[test]
    fn a_failed_check_or_install_is_retryable() {
        let mut updates = checking(InstallKind::Portable);
        updates.apply(Message::Checked(Err("offline".into())));
        assert_eq!(updates.status_text(), "Update failed: offline");
        assert!(updates.can_check());
        assert!(updates.start_check());
        updates.apply(Message::Checked(Ok(Some(offer("2.0.0")))));
        updates.apply(Message::Installed(Err("bad signature".into())));
        assert_eq!(updates.phase(), &Phase::Failed("bad signature".into()));
        assert!(!updates.shows_restart());
        assert!(updates.start_check());
    }

    #[test]
    fn never_two_checks_at_once() {
        let mut updates = checking(InstallKind::Portable);
        assert!(!updates.start_check());
        updates.apply(Message::Checked(Ok(Some(offer("2.0.0")))));
        assert!(!updates.start_check());
    }

    #[test]
    fn an_up_to_date_check_says_so() {
        let mut updates = Updates::new("1.0.0", InstallKind::Portable, None);
        assert_eq!(updates.status_text(), "Not checked yet.");
        updates.start_check();
        updates.apply(Message::Checked(Ok(None)));
        assert_eq!(updates.phase(), &Phase::Idle);
        assert_eq!(updates.status_text(), "Handpic is up to date.");
    }

    #[test]
    fn a_message_out_of_its_phase_changes_nothing() {
        let mut updates = Updates::new("1.0.0", InstallKind::Portable, None);
        updates.apply(Message::Checked(Ok(Some(offer("2.0.0")))));
        updates.apply(Message::Progress(1, Some(2)));
        updates.apply(Message::Installed(Ok(())));
        assert_eq!(updates.phase(), &Phase::Idle);
    }

    #[test]
    fn the_progress_steps_by_percent_else_by_mib() {
        assert_eq!(progress_step(50, Some(200)), 25);
        assert_eq!(progress_step(3 << 20, None), 3);
        assert_eq!(progress_step(3 << 20, Some(0)), 3);
    }

    #[test]
    fn the_page_names_the_version_the_install_and_a_test_build() {
        let updates = Updates::new("2.0.0-rc.1", InstallKind::Portable, Some("o/t".into()));
        assert_eq!(updates.version_line(), "Handpic 2.0.0-rc.1 (portable ZIP)");
        assert!(updates.test_note().unwrap().contains("o/t"));
        assert_eq!(
            Updates::new("1.0.0", InstallKind::Deb, None).test_note(),
            None
        );
    }

    #[test]
    fn the_state_reports_the_phase_and_its_version() {
        let mut updates = checking(InstallKind::Portable);
        updates.apply(Message::Checked(Ok(Some(offer("2.0.0")))));
        updates.apply(Message::Progress(40, Some(100)));
        assert_eq!(
            updates.state(),
            json!({
                "version": "1.0.0",
                "install": "portable",
                "test_build": false,
                "phase": "downloading",
                "target": "2.0.0",
                "percent": 40,
                "message": "Downloading Handpic 2.0.0\u{2026} 40%",
                "release_page": null,
            })
        );
    }
}
