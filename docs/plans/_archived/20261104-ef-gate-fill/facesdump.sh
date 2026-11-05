#!/bin/sh
# Step 2: the extended per-face dump of the 33 folders, one folder at a time
# and resumable (a faces/<folder>.tsv that exists is skipped; each is written
# to .part and moved into place on exit 0). The library is only read: a size
# and mtime listing of the 33 folders is taken before the first run and after
# the last; per folder, the RAW count is logged next to the files the CLI read.
# Usage: facesdump.sh <riffle-cli.exe> <threads> [folder...]
cli=$1
threads=$2
shift 2
all="2026-06-05 2026-06-13 2026-06-14 2026-06-20 2026-07-02 2026-07-05
2026-07-11 2026-07-18 2026-07-21 2026-07-22 2026-07-23 2026-07-24 2026-07-26
2026-07-30 2026-07-31 2026-08-01 2026-08-02 2026-08-08 2026-08-15 2026-08-16
2026-08-22 2026-08-29 2026-09-13-b 2026-09-19 2026-09-27-a 2026-09-27-b
2026-10-03 2026-10-11 2026-10-12a 2026-10-12b 2026-10-17 2026-09-13-a
2026-09-27-c"
folders=${*:-$all}
lib=/d/Photos/2026
out=/d/Photos/tests/2026-11-04-ef-gate-fill
mkdir -p "$out/faces" "$out/logs"
log="$out/logs/wall.log"

listing() {
  # shellcheck disable=SC2086
  (cd "$lib" && find $all -type f -printf '%p\t%s\t%T@\n' | LC_ALL=C sort)
}

[ -e "$out/logs/listing.before.tsv" ] || listing > "$out/logs/listing.before.tsv"

for f in $folders; do
  t="$out/faces/$f.tsv"
  if [ -e "$t" ]; then
    echo "$f skip" | tee -a "$log"
    continue
  fi
  raws=$(find "$lib/$f" -maxdepth 1 -type f \( -iname '*.arw' -o -iname '*.cr3' -o -iname '*.dng' -o -iname '*.nef' -o -iname '*.orf' -o -iname '*.raf' \) | wc -l)
  start=$(date +%s)
  "$cli" facechoice "D:/Photos/2026/$f" "$threads" --check --out "$t.part" 2> "$out/logs/$f.err"
  code=$?
  end=$(date +%s)
  [ $code -eq 0 ] && mv "$t.part" "$t"
  echo "$f exit $code wall $((end - start)) s raws $raws" | tee -a "$log"
done

listing > "$out/logs/listing.after.tsv"
if cmp -s "$out/logs/listing.before.tsv" "$out/logs/listing.after.tsv"; then
  echo "library unchanged" | tee -a "$log"
else
  echo "library CHANGED" | tee -a "$log"
fi
echo "facesdump done" | tee -a "$log"
