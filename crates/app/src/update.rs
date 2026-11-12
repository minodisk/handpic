//! The self-update flow: check, download and install in the background, with
//! the new version used on the next launch. On Windows the install exits the
//! process, so it is deferred to quit. The app is relaunched only when the user
//! picks `Restart Now` after a check from the menu.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

use tauri::{AppHandle, Manager};
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};
use tauri_plugin_updater::UpdaterExt;

use crate::index;

/// Guards against a startup check and a menu click running the updater at
/// once, and remembers the version installed (or, on Windows, downloaded) by
/// this process: the updater compares against the running binary, so a later
/// check would otherwise download the same update again.
#[derive(Default)]
pub struct UpdateRun {
    running: AtomicBool,
    installed: Mutex<Option<String>>,
    pending: Mutex<Option<(tauri_plugin_updater::Update, Vec<u8>)>>,
}

/// Install the update downloaded this session, if any. Called on quit after
/// the sidecar flush; on Windows `install` launches the installer and calls
/// `process::exit(0)` itself.
pub fn install_pending(app: &AppHandle) {
    let Some((update, bytes)) = index::lock(&app.state::<UpdateRun>().pending).take() else {
        return;
    };
    if let Err(e) = update.install(bytes) {
        log::warn!("installing the update failed: {e}");
    }
}

/// Run the update flow in the background. `interactive` reports the outcome
/// through native dialogs (the menu item); otherwise it is only logged.
pub fn spawn(app: AppHandle, interactive: bool) {
    tauri::async_runtime::spawn(run(app, interactive));
}

async fn run(app: AppHandle, interactive: bool) {
    let state = app.state::<UpdateRun>();
    if let Some(version) = index::lock(&state.installed).clone() {
        if interactive {
            offer_restart(&app, &version);
        }
        return;
    }
    if state
        .running
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .is_err()
    {
        if interactive {
            message(
                &app,
                "An update check is already running.",
                MessageDialogKind::Info,
            );
        }
        return;
    }
    let result = check_and_update(&app).await;
    if let Ok(Some(version)) = &result {
        *index::lock(&state.installed) = Some(version.clone());
    }
    state.running.store(false, Ordering::Release);
    match result {
        Ok(None) => {
            if interactive {
                message(&app, "Handpic is up to date.", MessageDialogKind::Info);
            }
        }
        Ok(Some(version)) => {
            log::info!("{}", installed_log(&version));
            if interactive {
                offer_restart(&app, &version);
            }
        }
        Err(e) => {
            log::warn!("update failed: {e}");
            if interactive {
                message(
                    &app,
                    &format!("Update failed: {e}"),
                    MessageDialogKind::Error,
                );
            }
        }
    }
}

async fn check_and_update(app: &AppHandle) -> Result<Option<String>, tauri_plugin_updater::Error> {
    let updater = app.updater_builder().restart_after_install(false).build()?;
    let Some(update) = updater.check().await? else {
        return Ok(None);
    };
    #[cfg(windows)]
    {
        let bytes = update.download(|_, _| {}, || {}).await?;
        *index::lock(&app.state::<UpdateRun>().pending) = Some((update.clone(), bytes));
    }
    #[cfg(not(windows))]
    update.download_and_install(|_, _| {}, || {}).await?;
    Ok(Some(update.version))
}

#[cfg(windows)]
fn installed_text(version: &str) -> String {
    format!(
        "Handpic {version} was downloaded. Restart now to install it, or later to install it when Handpic quits."
    )
}

#[cfg(not(windows))]
fn installed_text(version: &str) -> String {
    format!(
        "Handpic {version} was installed. Restart now to use it, or later to use it the next time Handpic launches."
    )
}

#[cfg(windows)]
fn installed_log(version: &str) -> String {
    format!("downloaded Handpic {version}; it is installed when Handpic quits")
}

#[cfg(not(windows))]
fn installed_log(version: &str) -> String {
    format!("installed Handpic {version}; it is used on the next launch")
}

fn offer_restart(app: &AppHandle, version: &str) {
    let app = app.clone();
    app.dialog()
        .message(installed_text(version))
        .title("Handpic")
        .kind(MessageDialogKind::Info)
        .buttons(MessageDialogButtons::OkCancelCustom(
            "Restart Now".into(),
            "Later".into(),
        ))
        .show(move |restart_now| {
            if restart_now {
                restart(&app);
            }
        });
}

/// Windows: the pending install is switched to relaunch the app and runs on
/// the exit's `ExitRequested`. The flag is set only here so that `Later`
/// followed by a normal quit does not relaunch.
#[cfg(windows)]
fn restart(app: &AppHandle) {
    {
        let state = app.state::<UpdateRun>();
        let mut pending = index::lock(&state.pending);
        *pending = pending
            .take()
            .map(|(update, bytes)| (update.restart_after_install(true), bytes));
    }
    app.exit(0);
}

/// Not `AppHandle::restart`: it never returns and parks this dialog-callback
/// thread, while `request_restart` goes through `ExitRequested` like a quit.
#[cfg(not(windows))]
fn restart(app: &AppHandle) {
    app.request_restart();
}

fn message(app: &AppHandle, text: &str, kind: MessageDialogKind) {
    app.dialog()
        .message(text)
        .title("Handpic")
        .kind(kind)
        .show(|_| {});
}
