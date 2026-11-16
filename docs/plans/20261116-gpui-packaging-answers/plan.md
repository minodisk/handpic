<plan-guide>
When you start executing this plan, record the in-flight plan path in auto memory (`MEMORY.md`).
After every step's PR is merged, the `develop` skill's wrap-up phase (normally a chore PR, or the same PR as the implementation in single-PR mode) does the archive move and removes the auto memory entry.

Update this file as you go if problems or design changes come up.
Record additional important information (investigation results, design details) in separate files in this folder.
Record what you learn during implementation, and notable events (CI failures, review feedback, plan changes, user intervention), in `learnings.md` as they happen.
After finishing a step, continue to the next without asking the user.
lychee (`mise run lint`) resolves a relative link in `docs/plans/**` from the linking file's own directory, so write links relative to the plan folder (e.g. `../../humans/usage.md`) and put an example path that is not a real link target in backticks.

<pr-rules>
- Include the plan.md update (marking the step done + the Progress entry) in the implementation PR
- Include `Plan: [path]` and `Step: [number]` in the PR description
</pr-rules>
</plan-guide>

# Record the answers to the GPUI packaging and updater open questions

## Purpose

[`packaging-updater.md`](../20261109-gpui-port/packaging-updater.md), the
decision record for Step 10 of
[the gpui-port plan](../20261109-gpui-port/plan.md), was re-decided on
2026-11-16 along gpui-kit's guides and left six open questions. The user
answered all six on the same day, and also a seventh that came up while
planning (what the update check does on a deb install). This plan moves each
answer into the record's decision and adjusts Step 10's sub-milestones to
match, so that the packaging, updater and release sub-milestones can be
planned against a record with nothing left to ask about.

It is documentation only: no packaging or updater code, no workflow change.

## Steps

- [x] Step 1: Move the answers into `packaging-updater.md` and adjust Step 10 of the gpui-port plan
  - Done when:
    - `docs/plans/20261109-gpui-port/packaging-updater.md` states the seven
      answers below as decided, in "The decision", with no "open question N"
      reference left anywhere in the file, and no "Open questions" section.
    - Every claim keeps the **Source** / **Unverified** tagging, and the new
      unverified items listed below are in the "Unverified" section.
    - Step 10 of `docs/plans/20261109-gpui-port/plan.md` is consistent with
      the record: sub-milestones 2, 3, 4 and 6 no longer defer to an open
      question, name no OS code signing, no rpm and no AppImage, and describe
      the updater and the notice as decided. Step 10 stays unchecked.
    - The gpui-port plan's Progress section has a 2026-11-16 entry for these
      answers.
    - No file other than those two and this plan's own files changes.
    - `mise run ci:local` passes locally (for a Markdown-only change it runs
      lychee) and the PR's GitHub checks pass.
  - Implementation approach (as far as it is known; omit if unknown):
    - **Scope.** Only the two files above. Leave alone `crates/**`,
      `.github/workflows/**`, `mise.toml`, `README*.md`, `docs/humans/**`,
      `docs/agents/**`, `CONTRIBUTING.md`, `CLAUDE.md`, the `release` skill
      and `release-please-config.json`; Step 10's later sub-milestones own
      them.
    - **Form.** Edit the record in place. English, wrapped near 80 columns
      like the file today. `vite.config.ts` ignores `**/*.md` for `vp fmt`,
      so only lychee checks it. Keep the two gpui-kit URLs as plain URLs.
      Leave the section "What gpui-kit's guides say" as it is.
    - **The record's lead.** Add that the open questions of the re-decision
      were answered by the user on 2026-11-16 and recorded by this plan.
      Write this plan's path, and the re-decision plan's (stale today: it
      says `../20261116-gpui-packaging-redecision/plan.md`), under
      `../_archived/`, in backticks, since the wrap-up archives this plan.
    - **Section "The user's decision (2026-11-16)".** Keep the three existing
      bullets and add the answers, or put them in a second dated list under
      it. Reword "the minisign key pair" among the dropped things: what is
      dropped is the Tauri updater's use of it (`plugins.updater.pubkey`,
      `TAURI_SIGNING_PRIVATE_KEY` as the Tauri bundler's input); a
      project-owned signing key itself is kept by answer 2.
    - **The answers, as they go into "The decision".**
      1. **Who updates an installer-owned install: the app drives the
         owner.**
         - Windows, Inno Setup install: the app downloads the new setup
           `.exe` in the background and runs it at quit, silently
           (`/VERYSILENT /SUPPRESSMSGBOXES /NORESTART`). The install is per
           user under `%LOCALAPPDATA%`, so there is no UAC prompt. The next
           launch is the new version.
         - macOS, DMG install: the app stages the new version and swaps the
           whole `.app`, never the executable alone.
         - Linux, `.deb`: left to the package manager; no in-app install.
           There is no apt repository, so in effect a hand reinstall. Record:
           **Source** (`tauri-plugin-updater` 2.12.0's `updater.rs`,
           `install_deb`) today's updater runs `dpkg -i` through `pkexec` for
           a deb install; **Source** (`docs/humans/usage.md`, "Updating") the
           usage guide nevertheless says a deb install is updated by
           installing the newer package; **Unverified** whether a shipped deb
           install really updates itself, and so whether this is a
           regression.
         - The portable installs (the Windows ZIP, the Linux `.tar.gz`) are
           updated by `self_update`, as before.
         - gpui-kit's rule "do not silently switch an install from one owner
           to another" still holds.
      2. **Publisher authenticity: a project-owned signing key.** Every
         update the app installs is verified against a key the project owns,
         as minisign does for the Tauri app today. `SHA256SUMS` alone is not
         enough. For the portable archives the candidate is `self_update`'s
         `signatures` feature (`zipsign-api`). The mechanism for the setup
         `.exe` and the `.app` download is chosen in the updater
         sub-milestone and must use the project's own key. Whether that is
         the existing minisign key pair or a new one is chosen there too.
      3. **OS code signing: none.** No Apple Developer ID and no
         notarization (macOS stays ad hoc); no Authenticode. This deviates
         from gpui-kit's guide. **Source** (`README.md`, "Install"): the
         README tells users how to pass Gatekeeper and SmartScreen, and keeps
         doing so. Considered and not taken: paid certificates, and free
         signing for open source such as SignPath Foundation (tag it
         **Unverified**, nothing was read about it).
      4. **Linux formats: gpui-kit's only.** A `.tar.gz` with `install.sh`
         and a `.deb`. AppImage and rpm are dropped. **Source** (`README.md`
         line 32): the AppImage is the Linux download the README leads with
         today; sub-milestone 6 rewrites that.
      5. **The last Tauri release is v1.10.0** (still named Riffle). No
         further Tauri release and no in-app notice. The reinstall notice
         lives in the first native release's notes, in `README.md` /
         `README.ja.md` and in the usage guide under `docs/humans/`. No
         frozen legacy `latest.json` is attached to native releases, so a
         menu check on a Tauri install shows an update error from the first
         native release on. Consequences to write down: the Tauri-side
         changes on `main` since v1.10.0 (the Handpic rename among them)
         never ship; and no release is published from `main` until
         sub-milestone 4 has switched `release.yml` to the native artifacts.
      6. **The Windows portable ZIP ships**, next to the Inno Setup
         installer, as gpui-kit's guide has it; `self_update` updates it.
      7. **The update check on a deb install notifies.** The launch check and
         the menu item run on a deb install too; when a newer version exists
         the app says so and offers to open the release page. It installs
         nothing.
    - **Rewrite the per-platform bullets of "The decision"** so none says
      "open question": macOS (ad hoc signed, the app swaps the whole `.app`),
      Windows x64 (the per-user Inno Setup setup `.exe` run silently at quit,
      and the portable ZIP by `self_update`; no MSI, no NSIS), Linux x64
      (`.tar.gz` by `self_update`, `.deb` by hand after the app's notice; no
      rpm, no AppImage), "Every release" (`SHA256SUMS`, the signatures of the
      project's key, no `latest.json` of any kind), and "The update flow in
      the app". Add the options the user turned down to "Not taken", one line
      each with the reason given: notify only for installer installs; OS
      signatures or checksum-only as the authenticity answer; paid
      certificates; rpm and AppImage; a final Tauri release with or without
      an in-app notice; the frozen `latest.json`; no check or a manual-only
      check on a deb install.
    - **New items for the "Unverified" section.** None of these was read from
      a source at planning time; write each as **Unverified** and do not
      state an expected outcome.
      - Inno Setup: which directives give a per-user install under
        `%LOCALAPPDATA%` with no UAC prompt, and that an upgrade under
        `/VERYSILENT /SUPPRESSMSGBOXES /NORESTART` shows none either.
      - That the setup started at quit outlives the app's process and
        replaces the exe once it has exited (file locks, Inno's
        close-applications handling).
      - How `Restart Now` relaunches the app after a silent setup run.
      - Whether SmartScreen warns on a setup `.exe` the app downloaded
        itself.
      - macOS: where the staged `.app` comes from (the DMG mounted, or an
        archive of the `.app` published beside it). The updater
        sub-milestone chooses, and the packaging adds the asset it needs.
      - macOS: whether the swap works where the app is installed (a standard
        user's `/Applications`; an app run from the DMG or from Downloads
        under App Translocation).
      - macOS: whether Gatekeeper quarantines the swapped ad hoc bundle, and
        whether the folder-access grants survive a swap to a bundle with
        another ad hoc signature.
      - Which archive formats `zipsign` signs, and what verifies the setup
        `.exe` and the `.app` download with the project's key.
      - How the running app knows it is a deb install, for answer 7.
      - Extend the existing "hand reinstall over a Tauri install" item with
        the formats now dropped: an AppImage file and an rpm package of the
        Tauri app are left for the user to remove.
    - **"Open questions" section.** Remove it.
    - **Step 10 of the gpui-port plan.**
      - Lead: replace "with what is still unverified and what is still open"
        by wording that says the open questions were answered on 2026-11-16
        and what remains is the record's unverified list.
      - Sub-milestone 2: replace "signing as open question 3 is settled; rpm
        and AppImage as open question 4 is settled" with: no OS code signing
        (macOS ad hoc; no Developer ID, notarization or Authenticode), no
        rpm, no AppImage, the Inno Setup install per user. Keep the rest.
      - Sub-milestone 3: `self_update` with signature verification for the
        portable ZIP and the `.tar.gz`; the setup `.exe` downloaded in the
        background and run silently at quit; the whole `.app` staged and
        swapped; a deb install notified with the release page, not
        installed; every download verified with the project-owned key, its
        mechanism for the setup `.exe` and the `.app` chosen here, adding to
        the packaging any asset it needs; restart to finish after the sidecar
        drain. Keep "It replaces `tauri-plugin-updater` and reads no
        `latest.json`."
      - Sub-milestone 4: add the signing secret and the signatures the
        updater verifies to what `release.yml` attaches; say no `latest.json`
        of any kind, the frozen legacy copy included; replace "the notice to
        Tauri users as open question 5 is settled" with: the reinstall notice
        in the first native release's notes, v1.10.0 being the last Tauri
        release, and no release cut from `main` before this sub-milestone.
      - Sub-milestone 6: keep, and say the reinstall instructions replace the
        usage guide's "Updating" and "Upgrading from Riffle" text and the
        README's AppImage download row.
      - Sub-milestones 1 and 5: unchanged.
    - **Progress entry in the gpui-port plan**, appended: `(2026-11-16) Step
      10: the open questions of the packaging / updater record answered by
      the user and recorded, through` this plan's archived path in backticks
      `(Step 10 stays open)`.
    - Check that the gpui-port plan's "Trade-offs and risks" bullets that
      mention Step 10 still read true; no edit is expected.

## Trade-offs and risks

- **The deb regression contradicts the usage guide.** The plugin's source
  runs `pkexec dpkg -i`; the usage guide says a deb install is updated by
  hand. The step records both and tags the real behavior unverified. If the
  usage guide is right in practice, there is no regression.
- **The project key: reuse or new.** The re-decision listed the minisign key
  pair as dropped; answer 2 keeps a project-owned key. The step words it as
  "the Tauri updater's use of the key is dropped" and leaves reuse versus a
  new key (and minisign versus zipsign's own key format) to the updater
  sub-milestone.
- **Sub-milestones 2 and 3 are coupled.** The `.app` swap and the signature
  checks may need release assets the packaging does not produce yet (an
  archive of the `.app`, signature files). The step records that the updater
  sub-milestone chooses and adds them to the packaging; it does not reorder
  the sub-milestones.
- **No release from `main` until sub-milestone 4.** "No further Tauri
  release" only holds if no release is cut from `main` while `release.yml`
  still builds the Tauri app. The step writes that rule into the record;
  enforcing it (a held release PR) is outside this plan.
- **Stale plan paths in the gpui-port Progress list.** Two older entries
  point at plan folders that have since been archived. The step fixes the
  stale path in the record's lead, which it rewrites anyway, and leaves the
  Progress entries as history.
- **Risk: files outside this step keep describing the Tauri contract**
  (`README.md`'s AppImage row, the usage guide's "Updating" and "Upgrading
  from Riffle", the `release` skill, `verify-release.sh`). That stays correct
  while the Tauri app is what ships; sub-milestones 4 and 6 own them.

## Progress

- (2026-11-16) Step 1 complete
