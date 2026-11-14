#!/usr/bin/env bash

set -o errexit
set -o pipefail
set -o nounset

# Tests for local.sh's selection of checks from the changed paths.
#
# The selection decides what an agent checks before it pushes, so a path that
# silently selects nothing is the failure this guards against. Every case pins
# the whole list of checks, in order. No git repository is involved.
#
# Run it directly, or via `mise run ci`.

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
target="${script_dir}/local.sh"

failures=0
cases=0

# expect <mode> <paths, one per line> <expected checks, one per line>
function expect() {
	local mode="$1"
	local paths="$2"
	local expected="$3"
	local output status=0

	cases=$((cases + 1))
	output=$(bash "${target}" --select "${mode}" <<<"${paths}") || status=$?

	if [[ "${status}" -ne 0 || "${output}" != "${expected}" ]]; then
		echo "FAIL: [${mode}] ${paths//$'\n'/ }" >&2
		echo "  exit ${status}; expected:" >&2
		printf '    %s\n' "${expected//$'\n'/$'\n'    }" >&2
		echo "  got:" >&2
		printf '    %s\n' "${output//$'\n'/$'\n'    }" >&2
		failures=$((failures + 1))
	fi
}

every="fmt root
fmt native
vp-check
shellcheck-all
actionlint
lychee
bash-test .claude/skills/merge/scripts/check-merge-approval.test.sh
bash-test .claude/skills/merge/scripts/wait-post-merge-runs.test.sh
bash-test .claude/skills/merge-settings/scripts/merge.test.sh
bash-test tools/ci/local.test.sh
clippy riffle-core
clippy riffle-backend
clippy riffle-cli
test riffle-core
test riffle-backend
test riffle-cli
clippy native
test native"

# --- nothing ----------------------------------------------------------------
expect none "" ""
expect moved "" ""
expect none "tools/stats/foo.py" ""
expect none "crates/core/models/LICENSE-x" "clippy riffle-core
test riffle-core
clippy native"

# --- Markdown ---------------------------------------------------------------
expect none "README.md
docs/agents/native-app.md
.claude/agents/implementer.md" "lychee"
expect moved "docs/a.md
docs/b.md" "lychee"
expect moved "docs/old.md
docs/new.md" "lychee"
expect moved "crates/core/genres/old.py" "lychee
clippy riffle-core
test riffle-core
clippy native"
expect none "docs/plans/20260101-x/plan.md
docs/plans/20260101-x/data.json" "vp-check
lychee"
expect none ".claude/settings.json" ""

# --- frontend ---------------------------------------------------------------
expect none "crates/app/ui/src/x.ts" "vp-check"
expect none "crates/app/ui/index.html" "vp-check"
expect none "package.json" "vp-check"
expect none "pnpm-lock.yaml" "vp-check"
expect none "vite.config.ts" "vp-check"
expect none "tsconfig.json" "vp-check"
expect none "crates/app/tauri.conf.json" "vp-check"

# --- Rust -------------------------------------------------------------------
expect none "crates/app/src/x.rs" "fmt root"
expect none "crates/core/src/x.rs" "fmt root
clippy riffle-core
test riffle-core
clippy native"
expect none "crates/backend/src/x.rs" "fmt root
clippy riffle-backend
test riffle-backend
clippy native"
expect none "crates/cli/src/x.rs" "fmt root
clippy riffle-cli
test riffle-cli"
expect none "crates/native/src/x.rs" "fmt native
clippy native
test native"
expect none "crates/native/Cargo.lock" "clippy native
test native"
expect none "crates/core/Cargo.toml" "vp-check
clippy riffle-core
test riffle-core
clippy native"
expect none "Cargo.lock" "clippy riffle-core
clippy riffle-backend
clippy riffle-cli
test riffle-core
test riffle-backend
test riffle-cli
clippy native"
expect none "crates/cli/src/x.rs
crates/core/src/y.rs" "fmt root
clippy riffle-core
clippy riffle-cli
test riffle-core
test riffle-cli
clippy native"

# --- shell scripts and workflows ---------------------------------------------
expect none "tools/git/x.sh" "shellcheck tools/git/x.sh"
expect none "tools/stats/x.sh" "shellcheck tools/stats/x.sh"
expect none ".claude/skills/develop/scripts/commit-push.sh" "shellcheck .claude/skills/develop/scripts/commit-push.sh"
expect none "scripts/x.sh
tools/git/sub/x.sh
.claude/skills/x/scripts/sub/y.sh" ""
expect none "tools/ci/local.sh" "shellcheck tools/ci/local.sh
bash-test tools/ci/local.test.sh"
expect none ".github/workflows/ci.yml" "vp-check
actionlint"

# --- bash tests -------------------------------------------------------------
expect none ".claude/skills/merge/scripts/check-merge-approval.sh" "shellcheck .claude/skills/merge/scripts/check-merge-approval.sh
bash-test .claude/skills/merge/scripts/check-merge-approval.test.sh"
expect none ".claude/skills/merge/scripts/wait-post-merge-runs.test.sh" "shellcheck .claude/skills/merge/scripts/wait-post-merge-runs.test.sh
bash-test .claude/skills/merge/scripts/wait-post-merge-runs.test.sh"
expect none ".claude/skills/merge-settings/scripts/merge.js" "bash-test .claude/skills/merge-settings/scripts/merge.test.sh"

# --- everything -------------------------------------------------------------
expect all "" "${every}"
expect none "mise.toml" "${every}"
expect none ".shellcheckrc
docs/a.md" "${every}"

if [[ "${failures}" -gt 0 ]]; then
	echo "${failures} of ${cases} cases failed" >&2
	exit 1
fi
echo "local.test.sh: ${cases} cases passed"
