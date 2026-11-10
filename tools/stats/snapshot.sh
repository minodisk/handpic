#!/usr/bin/env bash
# snapshot.sh <dir> — record today's (UTC) promotion stats as CSV files in
# <dir>: the download count of every release asset, the daily views and
# clones of the 14-day traffic window, the 14-day top referrers and paths, and
# the star and fork counts. Every row starts with its date. The rows of the
# dates a run records are replaced, so a re-run on the same day does not
# duplicate them; rows of other dates are left as they are.

set -euo pipefail

if [ $# -ne 1 ]; then
	echo "usage: $0 <dir>" >&2
	exit 2
fi

dir="$1"
repo="${GH_REPO:-minodisk/handpic}"
today="$(date -u +%F)"

# Fetch everything before touching a file, so a failed call (a missing or
# under-scoped token gets a 403 on the traffic endpoints) leaves them intact.
# shellcheck disable=SC2016 # $t is a jq variable, not a shell one.
downloads="$(gh api --paginate "repos/$repo/releases" \
	--jq '.[] | .tag_name as $t | .assets[] | [$t, .name, .download_count] | @csv')"
views="$(gh api "repos/$repo/traffic/views" \
	--jq '.views[] | "\(.timestamp[:10]),\(.count),\(.uniques)"')"
clones="$(gh api "repos/$repo/traffic/clones" \
	--jq '.clones[] | "\(.timestamp[:10]),\(.count),\(.uniques)"')"
referrers="$(gh api "repos/$repo/traffic/popular/referrers" \
	--jq '.[] | [.referrer, .count, .uniques] | @csv')"
paths="$(gh api "repos/$repo/traffic/popular/paths" \
	--jq '.[] | [.path, .count, .uniques] | @csv')"
stars="$(gh api "repos/$repo" --jq '[.stargazers_count, .forks_count] | @csv')"

# upsert <file> <header> <dates> <rows>: drop the rows of <dates> (one per
# line) from <file>, add <rows> (each already starting with its date), and
# sort by date.
upsert() {
	local file="$dir/$1" header="$2" dates="$3" rows="$4" tmp
	tmp="$(mktemp)"
	{
		echo "$header"
		{
			if [ -f "$file" ]; then
				tail -n +2 "$file" |
					grep -v -f <(printf '%s\n' "$dates" | sed 's/^/^/; s/$/,/') || true
			fi
			if [ -n "$rows" ]; then
				printf '%s\n' "$rows"
			fi
		} | LC_ALL=C sort
	} >"$tmp"
	mv "$tmp" "$file"
}

# Prefix every row of a today-only snapshot with today's date.
dated() {
	if [ -n "$1" ]; then
		printf '%s\n' "$1" | sed "s/^/$today,/"
	fi
}

mkdir -p "$dir"
upsert downloads.csv "date,tag,asset,download_count" "$today" "$(dated "$downloads")"
upsert views.csv "date,count,uniques" "$(cut -d, -f1 <<<"$views")" "$views"
upsert clones.csv "date,count,uniques" "$(cut -d, -f1 <<<"$clones")" "$clones"
upsert referrers.csv "date,referrer,count,uniques" "$today" "$(dated "$referrers")"
upsert paths.csv "date,path,count,uniques" "$today" "$(dated "$paths")"
upsert stars.csv "date,stargazers_count,forks_count" "$today" "$(dated "$stars")"
