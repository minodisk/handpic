# Learnings: Windows GUI subsystem for the release app

## Step 1

All measured on Windows 11 Home (10.0.26200), rust 1.98.1, against a 12-file
ARW folder with temporary `--log`, `--index` and `--settings`.

- PE `Subsystem`: the release exe (`cargo build --release --locked` in
  `crates/app`) reads 2, the debug exe 3.
- No console: started by `Start-Process -PassThru`, the debug exe gets a
  `conhost.exe` child process and `AttachConsole(pid)` to it succeeds; the
  release exe has no child and `AttachConsole` fails, so it owns no console.
  A system-wide "new conhost since before the launch" check is useless as a
  signal: unrelated processes create conhosts all the time, and the first
  attempt was fooled by one. Look at the launched process's own children.
  The release run's `--log` held the `settings:` line and the scan lines
  (`scan list`, `scan extract`, `scan faces`).
- Panic: provoked in a throwaway release build (an env-gated panic on a
  spawned thread, then on the main thread, right after the panic hook's
  install; reverted, not committed), started by `Start-Process` with no
  console. Both `panic:` lines reached the log and the process exited 101,
  no abort and no secondary failure.
- Timing output reachability (`--auto 5`, its `page` lines and the quit
  summary), recorded in `docs/agents/app.md`'s "A Windows release build is a
  GUI-subsystem binary":
  - (a) Git Bash: in mintty, `handpic.exe` direct reaches the terminal, but
    `mise run app:release` does not (only cargo's own lines show); `| cat`
    or `> file` brings it back. Git Bash running inside a console window
    loses both. The agent's Bash tool (pipes) gets everything.
  - (b) PowerShell 5.1 and cmd, through `mise run app:release`: lost, in a
    classic conhost window and in the default (ConPTY) terminal alike;
    `| Out-Host` (PowerShell) and `> file` (cmd) carry it.
  - (c) `handpic.exe` direct: PowerShell 5.1 unredirected and `> file` both
    lose it and return the prompt at once (the file is created empty);
    `| Out-Host` waits and shows it. cmd unredirected loses it, cmd
    `> file` gets it.
  - So `docs/humans/performance.md` / `.ja.md` gained the Windows route.
- How the console was read: a new window per case
  (`Start-Process powershell|cmd`, or `Start-Process conhost.exe <shell>` for
  a classic window), then `$host.UI.RawUI.GetBufferContents` dumped to a
  file. In PowerShell 5.1 `$buf[$y,$x]` on the returned 2-D array is a parse
  error; use `$buf.GetValue($y,$x)`. Under the default ConPTY terminal, after
  `mise run` the buffer read came back blank or out of range, so the mise
  cases were read under a classic `conhost.exe` window. mintty was read with
  `mintty --log <file> -e bash <script>`.
- `mise run app:release` recompiled `handpic-app` once after a
  `mise exec rust@1.98.1 -- cargo build --release` of the same tree
  (different environment between the two); harmless, but expect it.
- pwsh (PowerShell 7) is not installed here, so only Windows PowerShell 5.1
  was measured; 7.x's native-command handling differs and may behave
  differently for `> file`.
- `mise run ci:local`'s lychee failed once on `plan.md`'s link to the
  native-updater plan, archived by the wrap-up (#1189) after this plan was
  written; the link now points into `_archived/`.

## Deferred issues (todo candidates)

- Pending manual check (Windows, the release build): double-click
  `crates/app/target/release/handpic.exe` (or the installed Handpic from the
  Start menu) in Explorer and confirm no console window appears next to the
  Handpic window. The automated equivalent passed (`Start-Process`, no
  `conhost.exe` child, `AttachConsole` failing); the step's checkbox was
  ticked on that. Basis: this plan's Step 1 Done-when, "opens no console
  window".
- Pending manual check (Windows, PowerShell 7 / pwsh): whether
  `handpic.exe ... > file` from pwsh captures the release build's stdout
  (PowerShell 5.1 does not: the prompt returns at once and the file stays
  empty). If it does, `docs/agents/app.md`'s table could name it. Basis:
  this step's measurement (c); related file `docs/agents/app.md`.
