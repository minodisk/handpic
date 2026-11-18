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

# The host's `uname -s` the cases select for; the Windows cases set it.
uname=Linux

# expect <mode> <paths, one per line> <expected checks, one per line>
function expect() {
	local mode="$1"
	local paths="$2"
	local expected="$3"
	local output status=0

	cases=$((cases + 1))
	output=$(CI_LOCAL_UNAME="${uname}" bash "${target}" --select "${mode}" <<<"${paths}") || status=$?

	if [[ "${status}" -ne 0 || "${output}" != "${expected}" ]]; then
		echo "FAIL: [${mode}, ${uname}] ${paths//$'\n'/ }" >&2
		echo "  exit ${status}; expected:" >&2
		printf '    %s\n' "${expected//$'\n'/$'\n'    }" >&2
		echo "  got:" >&2
		printf '    %s\n' "${output//$'\n'/$'\n'    }" >&2
		failures=$((failures + 1))
	fi
}

every="fmt root
fmt native
shellcheck-all
actionlint
lychee
bash-test .claude/skills/merge/scripts/check-merge-approval.test.sh
bash-test .claude/skills/merge/scripts/wait-post-merge-runs.test.sh
bash-test .claude/skills/merge-settings/scripts/merge.test.sh
bash-test .claude/skills/pr/scripts/wait-pr-actionable.test.sh
bash-test tools/ci/local.test.sh
clippy handpic-core
clippy handpic-backend
clippy handpic-cli
test handpic-core
test handpic-backend
test handpic-cli
clippy native
test native"

# --- nothing ----------------------------------------------------------------
expect none "" ""
expect moved "" ""
expect none "tools/stats/foo.py" ""
expect none "crates/core/models/LICENSE-x" "clippy handpic-core
test handpic-core
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
clippy handpic-core
test handpic-core
clippy native"
expect none "docs/plans/20260101-x/plan.md
docs/plans/20260101-x/data.json" "lychee"
expect none ".claude/settings.json" ""

# --- Rust -------------------------------------------------------------------
expect none "crates/core/src/x.rs" "fmt root
clippy handpic-core
test handpic-core
clippy native"
expect none "crates/backend/src/x.rs" "fmt root
clippy handpic-backend
test handpic-backend
clippy native"
expect none "crates/cli/src/x.rs" "fmt root
clippy handpic-cli
test handpic-cli"
expect none "crates/native/src/x.rs" "fmt native
clippy native
test native"
expect none "crates/native/Cargo.lock" "clippy native
test native"
expect none "crates/core/Cargo.toml" "clippy handpic-core
test handpic-core
clippy native"
expect none "Cargo.lock" "clippy handpic-core
clippy handpic-backend
clippy handpic-cli
test handpic-core
test handpic-backend
test handpic-cli
clippy native"
expect none "crates/cli/src/x.rs
crates/core/src/y.rs" "fmt root
clippy handpic-core
clippy handpic-cli
test handpic-core
test handpic-cli
clippy native"

# --- shell scripts and workflows ---------------------------------------------
expect none "tools/git/x.sh" "shellcheck tools/git/x.sh"
expect none "tools/stats/x.sh" "shellcheck tools/stats/x.sh"
expect none "tools/package/x.sh" "shellcheck tools/package/x.sh"
expect none "crates/native/packaging/linux/install.sh" "shellcheck crates/native/packaging/linux/install.sh
clippy native
test native"
expect none ".claude/skills/develop/scripts/commit-push.sh" "shellcheck .claude/skills/develop/scripts/commit-push.sh"
expect none "scripts/x.sh
tools/git/sub/x.sh
tools/package/sub/x.sh
.claude/skills/x/scripts/sub/y.sh" ""
expect none "tools/ci/local.sh" "shellcheck tools/ci/local.sh
bash-test tools/ci/local.test.sh"
expect none ".github/workflows/ci.yml" "actionlint"
expect none "crates/native/packaging/NOTICE-gpui" "clippy native
test native"

# --- bash tests -------------------------------------------------------------
expect none ".claude/skills/merge/scripts/check-merge-approval.sh" "shellcheck .claude/skills/merge/scripts/check-merge-approval.sh
bash-test .claude/skills/merge/scripts/check-merge-approval.test.sh"
expect none ".claude/skills/merge/scripts/wait-post-merge-runs.test.sh" "shellcheck .claude/skills/merge/scripts/wait-post-merge-runs.test.sh
bash-test .claude/skills/merge/scripts/wait-post-merge-runs.test.sh"
expect none ".claude/skills/merge-settings/scripts/merge.js" "bash-test .claude/skills/merge-settings/scripts/merge.test.sh"
expect none ".claude/skills/pr/scripts/wait-pr-actionable.sh" "shellcheck .claude/skills/pr/scripts/wait-pr-actionable.sh
bash-test .claude/skills/pr/scripts/wait-pr-actionable.test.sh"
expect none ".claude/skills/pr/scripts/pr-status.sh" "shellcheck .claude/skills/pr/scripts/pr-status.sh
bash-test .claude/skills/pr/scripts/wait-pr-actionable.test.sh"
expect none ".claude/skills/pr/scripts/wait-pr-actionable.test.sh" "shellcheck .claude/skills/pr/scripts/wait-pr-actionable.test.sh
bash-test .claude/skills/pr/scripts/wait-pr-actionable.test.sh"

# --- everything -------------------------------------------------------------
expect all "" "${every}"
expect none "mise.toml" "${every}"
expect none ".shellcheckrc
docs/a.md" "${every}"

# --- Windows: the native app's screenshot feature build ----------------------
uname=MINGW64_NT-10.0-26200
expect none "crates/native/src/x.rs" "fmt native
clippy native
clippy native screenshot
test native"
expect none "crates/backend/src/x.rs" "fmt root
clippy handpic-backend
test handpic-backend
clippy native
clippy native screenshot"
expect none "crates/cli/src/x.rs" "fmt root
clippy handpic-cli
test handpic-cli"
expect none "README.md" "lychee"
expect all "" "${every/$'clippy native\n'/$'clippy native\nclippy native screenshot\n'}"
uname=Darwin
expect none "crates/native/src/x.rs" "fmt native
clippy native
test native"

if [[ "${failures}" -gt 0 ]]; then
	echo "${failures} of ${cases} cases failed" >&2
	exit 1
fi
echo "local.test.sh: ${cases} cases passed"
