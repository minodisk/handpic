# Per-language values

One JSON file per language, named by its language code (`en.json`,
`ja.json`, `pt-BR.json`). `crates/core/build.rs` picks up every `*.json`
here and embeds it at build time, so adding a language is adding a file; no
Rust change is needed.

## Shape

```json
{
  "meta": {
    "name": "日本語"
  },
  "lightroom": {
    "colorLabels": {
      "verified": "Lightroom Classic 15.5.1 (Windows, Japanese UI)",
      "red": "レッド",
      "yellow": "イエロー",
      "green": "グリーン",
      "blue": "ブルー",
      "purple": "パープル"
    }
  }
}
```

- The keys are namespaced, and `en.json` is the reference: every key in
  another file must also exist in `en.json`.
- `meta.name` is the language's name for itself (`English`, `日本語`), shown
  in the settings window's language dropdown. It is required.
- `lightroom.colorLabels` is Lightroom Classic's default color label set in
  that UI language (`Metadata > Color Label Set > Edit...`), offered as a
  preset in the settings window. Copy the five names from a real Lightroom
  running in that UI language; do not translate them yourself. `verified`
  names the Lightroom version, OS and UI language they were checked against.
  Give all five colors and `verified`, or leave `lightroom` out, in which case
  the language offers no preset.

## Checking

```sh
cargo test -p handpic-core
```

The tests check that each file parses, is named by a plausible language
code, has no key missing from `en.json`, and has a non-empty `meta.name` and,
when present, non-empty `lightroom.colorLabels` values.
