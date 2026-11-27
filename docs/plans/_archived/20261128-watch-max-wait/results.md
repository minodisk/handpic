# Results: the open folder fills during an import

## Setup

- Machine: Windows 11 Home (10.0.26200), right after a reboot (cold file cache).
- Build: `crates/app` at `969ee7ed` (branch `fix/watch-max-wait`), debug, `--features screenshot`, `cargo build -j 2`.
- Launch: `handpic.exe --index V:\handpic-watch-max-wait\run\index.sqlite --settings V:\handpic-watch-max-wait\run\settings.sqlite --log V:\handpic-watch-max-wait\run\Handpic.log --import-settings V:\handpic-watch-max-wait\run\seed.json --mcp-port 47830`, the seed `{"mcpEnabled": true, "mcpAllowDestructive": true, "importRoot": "V:\\handpic-watch-max-wait\\dst"}`.
- Material: the 100 ARWs (5.2 GB, shot 2026-08-22) of `D:\Photos\tests\2026-08-22-focus-sample`, hard-linked into `D:\handpic-watch-max-wait\src`; imported into `V:\handpic-watch-max-wait\dst\2026\2026-08-22`.
- Driver: a Python MCP client calling `import_folder` (preview, then `confirm: true`), then `get_view` and `get_status` about once a second, with `screenshot`s at about 3 s and 9 s.

## The strip fills while an import into the open folder runs: PASS

| t (s) | copied | listed | scan |
| ----: | -----: | -----: | ---- |
| 1.0 | 8 | 0 | none |
| 2.0 | 18 | 0 | none |
| 3.1 | 25 | 19 | analyzing 1/19 |
| 5.5 | 45 | 19 | checking faces 0/10 |
| 9.6 | 76 | 70 | analyzing 1/51 |
| 13.0 | finished | 70 | analyzing 34/51 |
| 27.2 | finished | 100 | analyzing 0/0 |

The log (UTC):

```
22:33:16 scan list: dir=...\2026-08-22 files=0
22:33:19 resync: dir=...\2026-08-22
22:33:19 scan list: dir=...\2026-08-22 files=19
22:33:21 resync: ... deferred
22:33:23 resync: ... deferred
22:33:25 resync: ... deferred
22:33:25 resync: dir=...\2026-08-22
22:33:25 scan list: dir=...\2026-08-22 files=70
22:33:27 resync: ... deferred
22:33:29 resync: ... deferred
22:33:29 import: files=100 bytes=5197660160 mb_per_s=413.5 skipped=0 failed=0 canceled=false ... in 12571ms
22:33:29 resync: ... deferred
22:33:43 resync: dir=...\2026-08-22
22:33:43 scan list: dir=...\2026-08-22 files=100
```

The watcher reported about every 2 s (`MAX_WAIT`) while the copy ran; the
reports during a scan were deferred and collapsed into one re-listing at the
scan's end, so pass 1 was never restarted mid-scan. The screenshot at 9 s
showed 70 cells in the strip, the tree's `2026-08-22` row at 70 and the pane
at `importing 76 / 100`. The cadence is bound by the scan: the last 30 files
appeared 14 s after the run ended, when the scan under way finished.

Not done: a 6 GB, 125-file run as in the original report (100 files lasted
12.6 s, several `MAX_WAIT`s, which is what the fix needs); a release build.
