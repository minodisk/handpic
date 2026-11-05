# Learnings

## Step 1: Import settings

- chrono's `StrftimeItems` yields `Item::Error` for an unknown specifier or a
  lone `%`, and `NaiveDate::format` then fails its `Display` with
  `fmt::Error` for any specifier needing a time, an offset or a timestamp
  (`%H %M %S %p %I %f %z %Z %s %T %R %c %+`). `.to_string()` on that panics,
  so `format_date` writes it with `write!` and maps the error.
- `%n` and `%t` are valid chrono specifiers that emit a newline and a tab;
  the control-character check on each folder name is what refuses them.
- `validate_pattern` formats a fixed date (2026-01-02), and `format_date`
  runs the same folder-name checks on every real date, so a pattern whose
  validity depends on the date (none known among the date specifiers) would
  still be refused at the date that breaks it.
- The stored `importPattern` is re-validated at load (`pattern_setting`); an
  invalid stored value falls back to `%Y/%F` rather than reaching the run.
- `import_pattern_example` joins the formatted path onto the stored root in
  Rust (`PathBuf::extend` over the `/` segments), so the UI shows the OS
  separator without composing paths in TypeScript; with no root set it
  shows the relative path alone and the tab shows the "root not set" note.
- `commands::settings` became `pub(crate)` so `import.rs` reads and writes
  the store itself; `load_settings`'s tuple was left alone.
- The tab icon is Lucide `hard-drive-download` (lucide-static v1.48.0, not
  Feather-derived), added to `icons.ts`' license header.

## Deferred issues (todo candidates)

- Pending manual check (Step 1, the settings modal's `Import` tab): on
  Windows (and macOS if available) open Settings > Import, press `Choose…`
  and pick a folder; expect the root field to show it and the example line
  to read `<root>\<YYYY>\<YYYY-MM-DD>` for today. Type `%Y/%H` and expect the
  error in red under the input; blur and expect the stored pattern kept
  (reopen the app: the field shows the last valid pattern). Click each
  example button (`%Y/%F`, `%F`, `%Y/%m/%d`, `%Y/%m`) and expect the input,
  the example and, after a restart, the stored value to follow. The step
  was ticked on the automated criteria (unit tests, `mise run ci`); this
  GUI check was not run here. Related files: `crates/app/ui/src/settings.ts`,
  `crates/app/ui/index.html`, `crates/app/src/import.rs`.
