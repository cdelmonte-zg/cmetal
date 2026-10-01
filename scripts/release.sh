#!/usr/bin/env bash
# Drive a cmetal release from a local checkout.
#
# The pipeline itself lives in .github/workflows/release.yml: pushing a
# tag `vX.Y.Z` builds four tarballs (linux musl x86_64/aarch64, apple
# x86_64/aarch64), publishes the GitHub release with generated notes,
# bumps Formula/cmetal.rb in cdelmonte-zg/homebrew-tap and publishes the
# crate on crates.io. This script covers the manual part around it and
# checks the result:
#
#   scripts/release.sh status            what main looks like since the last tag
#   scripts/release.sh prepare X.Y.Z     bump Cargo.toml/Cargo.lock on a
#                                        release/X.Y.Z branch, run the local
#                                        checks, open the bump PR
#   scripts/release.sh tag X.Y.Z         after the PR is merged: tag main,
#                                        push the tag, watch the workflow,
#                                        then verify
#   scripts/release.sh verify X.Y.Z      check the release assets, the tap
#                                        formula and crates.io (and smoke
#                                        test the linux tarball if possible)
#
# Options:
#   --dry-run     prepare: bump, check and commit locally, but do not push
#                 or open the PR. tag: stop before creating the tag.
#   --skip-checks prepare: skip cargo build/test and the publish dry run
#                 (CI runs them on the PR anyway).
#   --no-watch    tag: push the tag and return without waiting for the
#                 workflow (run `verify` by hand later).
#
# The version must be greater than the one in Cargo.toml: `cmetal update`
# no-ops between equal versions, so a release that does not bump leaves
# every existing workspace on the old curriculum.
#
# A prerelease (`X.Y.Z-rc1`) still gets tarballs and a GitHub release, but
# release.yml skips the tap and crates.io jobs for it, and so does verify.

set -euo pipefail

REPO="cdelmonte-zg/cmetal"
TAP_REPO="cdelmonte-zg/homebrew-tap"
CRATE="cmetal"
TARGETS=(
  x86_64-unknown-linux-musl
  aarch64-unknown-linux-musl
  x86_64-apple-darwin
  aarch64-apple-darwin
)

usage() {
  sed -n '2,/^$/p' "$0" | sed 's/^# \{0,1\}//'
  exit "${1:-0}"
}

say()  { printf '\033[1;34m==>\033[0m %s\n' "$*"; }
ok()   { printf '\033[1;32m ok \033[0m %s\n' "$*"; }
warn() { printf '\033[1;33mwarn\033[0m %s\n' "$*" >&2; }
die()  { printf '\033[1;31merror\033[0m %s\n' "$*" >&2; exit 1; }

# ---------------------------------------------------------------- helpers

need_tools() {
  local t
  for t in git gh cargo curl jq; do
    command -v "$t" >/dev/null 2>&1 || die "missing tool: $t"
  done
}

check_version_syntax() {
  [[ "$1" =~ ^[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z.]+)?$ ]] \
    || die "version must look like 1.2.3 or 1.2.3-rc1 (no leading v): got '$1'"
}

is_prerelease() { [[ "$1" == *-* ]]; }

cargo_version() {
  # the only line starting with `version = ` is the package version;
  # dependencies spell it as `name = { version = ... }`
  local n
  n=$(grep -c '^version = "' Cargo.toml)
  [ "$n" -eq 1 ] || die "expected exactly one top-level 'version =' in Cargo.toml, found $n"
  sed -n 's/^version = "\([^"]*\)"$/\1/p' Cargo.toml
}

lock_version() {
  # the [[package]] block for cmetal, then its version line
  awk '/^\[\[package\]\]/{p=0} /^name = "'"$CRATE"'"$/{p=1} p && /^version = /{gsub(/"/,"",$3); print $3; exit}' Cargo.lock
}

# true when $1 > $2 in semver order: numeric on major.minor.patch, and a
# prerelease sorts before the release it precedes (0.5.0-rc1 < 0.5.0)
version_gt() {
  local a="${1%%-*}" b="${2%%-*}" pa="${1#"${1%%-*}"}" pb="${2#"${2%%-*}"}"
  local -a A B
  IFS=. read -r -a A <<<"$a"
  IFS=. read -r -a B <<<"$b"
  local i
  for i in 0 1 2; do
    [ "${A[$i]}" -gt "${B[$i]}" ] && return 0
    [ "${A[$i]}" -lt "${B[$i]}" ] && return 1
  done
  # same core: release > prerelease; two prereleases compare as strings
  [ -z "$pa" ] && [ -n "$pb" ] && return 0
  [ -n "$pa" ] && [ -z "$pb" ] && return 1
  [[ "$pa" > "$pb" ]]
}

last_tag() { git describe --tags --abbrev=0 --match 'v*' origin/main 2>/dev/null || true; }

require_clean_tree() {
  git diff --quiet && git diff --cached --quiet \
    || die "working tree is not clean; commit or stash first"
}

require_main_up_to_date() {
  say "fetching origin"
  git fetch --quiet --tags origin
  local branch
  branch=$(git rev-parse --abbrev-ref HEAD)
  [ "$branch" = "main" ] || die "run this from main (currently on '$branch')"
  [ "$(git rev-parse HEAD)" = "$(git rev-parse origin/main)" ] \
    || die "main is not at origin/main; pull (or push) first"
}

tag_exists() {
  git rev-parse -q --verify "refs/tags/$1" >/dev/null 2>&1 && return 0
  [ -n "$(git ls-remote --tags origin "refs/tags/$1")" ]
}

crates_io_has() {
  # crates.io answers without the `crate`/`version` fields unless a
  # User-Agent is sent
  curl -fsSL -A "cmetal-release-script" \
    "https://crates.io/api/v1/crates/$CRATE/$1" >/dev/null 2>&1
}

commits_since() {
  if [ -n "$1" ]; then git log --oneline "$1..origin/main"; else git log --oneline origin/main; fi
}

# ---------------------------------------------------------------- status

cmd_status() {
  git fetch --quiet --tags origin
  local cur tag n
  cur=$(cargo_version)
  tag=$(last_tag)
  say "Cargo.toml version: $cur (Cargo.lock: $(lock_version))"
  say "last tag on main:   ${tag:-none}"
  n=$(commits_since "$tag" | wc -l | tr -d ' ')
  say "commits on origin/main since ${tag:-the beginning}: $n"
  commits_since "$tag" | sed 's/^/     /'
  if [ -n "$tag" ] && [ "v$cur" = "$tag" ] && [ "$n" -gt 0 ]; then
    warn "Cargo.toml still says $cur: the next release needs a bump (prepare X.Y.Z)"
  elif [ -n "$tag" ] && version_gt "$cur" "${tag#v}"; then
    say "bump to $cur is on main but not tagged yet: run 'tag $cur' once the PR is merged"
  fi
}

# ---------------------------------------------------------------- prepare

cmd_prepare() {
  local version="$1" dry_run="$2" skip_checks="$3"
  local tag="v$version" branch="release/$version" current prev n body

  check_version_syntax "$version"
  require_clean_tree
  require_main_up_to_date

  current=$(cargo_version)
  version_gt "$version" "$current" \
    || die "$version is not greater than the current version $current"
  tag_exists "$tag" && die "tag $tag already exists"
  git rev-parse -q --verify "refs/heads/$branch" >/dev/null \
    && die "branch $branch already exists locally"
  [ -z "$(git ls-remote --heads origin "refs/heads/$branch")" ] \
    || die "branch $branch already exists on origin"
  if ! is_prerelease "$version" && crates_io_has "$version"; then
    die "$CRATE $version is already on crates.io"
  fi
  gh auth status >/dev/null 2>&1 || die "gh is not authenticated (gh auth login)"

  prev=$(last_tag)
  n=$(commits_since "$prev" | wc -l | tr -d ' ')
  [ "$n" -gt 0 ] || die "no commits on main since $prev; nothing to release"
  say "releasing $version (current $current, last tag ${prev:-none}, $n commits since)"

  say "creating branch $branch"
  git switch --quiet -c "$branch"

  say "bumping Cargo.toml $current -> $version"
  sed -i -E "s/^version = \"[^\"]+\"/version = \"$version\"/" Cargo.toml
  [ "$(cargo_version)" = "$version" ] || die "Cargo.toml bump failed"

  say "syncing Cargo.lock"
  cargo update --quiet --workspace --offline
  [ "$(lock_version)" = "$version" ] || die "Cargo.lock does not carry $version after cargo update"
  local changed
  changed=$(git diff --name-only | sort | tr '\n' ' ')
  [ "$changed" = "Cargo.lock Cargo.toml " ] \
    || die "unexpected files changed by the bump: $changed"

  if [ "$skip_checks" = 1 ]; then
    warn "skipping local checks (--skip-checks); CI runs them on the PR"
  else
    say "cargo build --release --locked"
    cargo build --release --locked
    say "cargo test --locked"
    cargo test --locked
    say "cargo publish --dry-run (validates the crates.io package)"
    cargo publish --dry-run --locked --allow-dirty
  fi

  say "committing"
  git add Cargo.toml Cargo.lock
  git commit --quiet -F - <<EOF
Release $version

Bumps the version so the next tag ships the $n commits that have
landed on main since ${prev:-the first commit}. \`cmetal update\` no-ops between equal
versions, so a release that does not bump leaves every existing
workspace on the old curriculum.
EOF
  ok "committed $(git rev-parse --short HEAD) on $branch"

  body=$(cat <<EOF
Version bump for the next tag. Since ${prev:-the first commit}, main has accumulated $n commits:

$(commits_since "$prev" | sed -E 's/^[0-9a-f]+ /- /')

\`cmetal update\` no-ops between equal versions, so the bump is what delivers all of this to existing workspaces. After merge, tagging \`$tag\` on main triggers release.yml: 4 tarballs, the GitHub release, the Homebrew tap bump, and the crates.io publish.
EOF
)

  if [ "$dry_run" = 1 ]; then
    local body_file
    body_file=$(mktemp "${TMPDIR:-/tmp}/cmetal-release-$version-XXXX.md")
    printf '%s\n' "$body" > "$body_file"
    warn "dry run: not pushing. To continue by hand:"
    printf '     git push -u origin %s\n' "$branch"
    printf '     gh pr create --base main --title "Release %s" --body-file %s\n' "$version" "$body_file"
    return
  fi

  say "pushing $branch"
  git push --quiet -u origin "$branch"
  say "opening the PR"
  gh pr create --base main --head "$branch" --title "Release $version" --body "$body"
  ok "once CI is green and the PR is merged, run: scripts/release.sh tag $version"
}

# ---------------------------------------------------------------- tag

cmd_tag() {
  local version="$1" dry_run="$2" watch="$3"
  local tag="v$version"

  check_version_syntax "$version"
  require_clean_tree
  if [ "$(git rev-parse --abbrev-ref HEAD)" != "main" ]; then
    say "switching to main"
    git switch --quiet main
  fi
  git fetch --quiet --tags origin
  git pull --quiet --ff-only origin main
  require_main_up_to_date

  [ "$(cargo_version)" = "$version" ] \
    || die "Cargo.toml on main says $(cargo_version), not $version; has the bump PR been merged?"
  [ "$(lock_version)" = "$version" ] \
    || die "Cargo.lock on main says $(lock_version), not $version"
  tag_exists "$tag" && die "tag $tag already exists"
  is_prerelease "$version" \
    && warn "$version is a prerelease: release.yml will skip the tap and crates.io jobs"

  say "tagging $(git rev-parse --short HEAD) ($(git log -1 --format=%s)) as $tag"
  if [ "$dry_run" = 1 ]; then
    warn "dry run: not tagging. To continue by hand:"
    printf '     git tag -a %s -m "cmetal %s" && git push origin %s\n' "$tag" "$version" "$tag"
    return
  fi
  git tag -a "$tag" -m "$CRATE $version"
  git push --quiet origin "$tag"
  ok "pushed $tag; release.yml is starting: https://github.com/$REPO/actions/workflows/release.yml"

  if [ "$watch" = 0 ]; then
    warn "not watching (--no-watch); later run: scripts/release.sh verify $version"
    return
  fi
  watch_workflow "$tag"
  cmd_verify "$version"
}

watch_workflow() {
  local tag="$1" run_id="" i
  say "waiting for the Release run on $tag"
  for _ in $(seq 1 30); do
    run_id=$(gh run list --repo "$REPO" --workflow=release.yml --branch "$tag" \
               --json databaseId --limit 1 -q '.[0].databaseId' 2>/dev/null || true)
    [ -n "$run_id" ] && break
    sleep 5
  done
  [ -n "$run_id" ] || die "no Release run appeared for $tag after 150s; check the Actions tab"
  say "watching run $run_id"
  gh run watch --repo "$REPO" --exit-status "$run_id" \
    || die "the Release run failed: gh run view --repo $REPO $run_id --log-failed"
  ok "Release run $run_id succeeded"
}

# ---------------------------------------------------------------- verify

cmd_verify() {
  local version="$1" failures=0
  local tag="v$version"
  check_version_syntax "$version"

  say "GitHub release $tag"
  local assets expected a
  assets=$(gh release view --repo "$REPO" "$tag" --json assets,isDraft \
             -q 'if .isDraft then error("release is still a draft") else .assets[].name end') \
    || { warn "release $tag not found or still a draft"; failures=$((failures+1)); assets=""; }
  for a in "${TARGETS[@]}"; do
    for expected in "cmetal-$tag-$a.tar.gz" "cmetal-$tag-$a.tar.gz.sha256"; do
      if grep -qx "$expected" <<<"$assets"; then
        ok "$expected"
      else
        warn "missing asset: $expected"; failures=$((failures+1))
      fi
    done
  done

  if is_prerelease "$version"; then
    warn "prerelease: skipping tap and crates.io checks"
  else
    say "Homebrew tap Formula/cmetal.rb"
    local formula_version
    formula_version=$(gh api "repos/$TAP_REPO/contents/Formula/cmetal.rb" -q .content \
                        | base64 -d | sed -n 's/^ *version "\([^"]*\)"$/\1/p') || formula_version=""
    if [ "$formula_version" = "$version" ]; then
      ok "formula at $version"
    else
      warn "formula says '${formula_version:-?}', expected $version"; failures=$((failures+1))
    fi

    say "crates.io"
    local crate_json max
    if crate_json=$(curl -fsSL -A "cmetal-release-script" "https://crates.io/api/v1/crates/$CRATE/$version"); then
      ok "$CRATE $(jq -r .version.num <<<"$crate_json") published $(jq -r .version.created_at <<<"$crate_json")"
    else
      warn "$CRATE $version is not on crates.io"; failures=$((failures+1))
    fi
    max=$(curl -fsSL -A "cmetal-release-script" "https://crates.io/api/v1/crates/$CRATE" | jq -r .crate.max_version)
    [ "$max" = "$version" ] || warn "crates.io max_version is $max (fine if $version is a backport)"
  fi

  if [ "$(uname -s)" = Linux ] && [ "$(uname -m)" = x86_64 ] && [ -n "$assets" ]; then
    say "smoke test of the linux x86_64 tarball"
    local target="x86_64-unknown-linux-musl" tmp archive
    archive="cmetal-$tag-$target.tar.gz"
    tmp=$(mktemp -d)
    (
      cd "$tmp"
      gh release download --repo "$REPO" "$tag" -p "$archive" -p "$archive.sha256" >/dev/null
      sha256sum --quiet -c "$archive.sha256"
      tar -xzf "$archive"
      out=$("./cmetal-$tag-$target/cmetal" --version)
      [[ "$out" == *"$version"* ]] || { echo "cmetal --version printed '$out'"; exit 1; }
      "./cmetal-$tag-$target/cmetal" init smoke >/dev/null
      (cd smoke && "../cmetal-$tag-$target/cmetal" list >/dev/null)
    ) && ok "sha256 ok, --version reports $version, init/list work" \
      || { warn "smoke test failed (see above)"; failures=$((failures+1)); }
    rm -rf "$tmp"
  fi

  if [ "$failures" -eq 0 ]; then
    ok "release $tag verified"
  else
    die "$failures check(s) failed for $tag"
  fi
}

# ---------------------------------------------------------------- main

main() {
  need_tools
  cd "$(git rev-parse --show-toplevel)"

  local cmd="${1:-}" version="" dry_run=0 skip_checks=0 watch=1
  [ -n "$cmd" ] || usage 1
  shift
  while [ $# -gt 0 ]; do
    case "$1" in
      --dry-run) dry_run=1 ;;
      --skip-checks) skip_checks=1 ;;
      --no-watch) watch=0 ;;
      -h|--help) usage ;;
      -*) die "unknown option: $1" ;;
      *) [ -z "$version" ] || die "unexpected argument: $1"; version="${1#v}" ;;
    esac
    shift
  done

  case "$cmd" in
    status) cmd_status ;;
    prepare|tag|verify)
      [ -n "$version" ] || die "$cmd needs a version: scripts/release.sh $cmd X.Y.Z"
      case "$cmd" in
        prepare) cmd_prepare "$version" "$dry_run" "$skip_checks" ;;
        tag)     cmd_tag "$version" "$dry_run" "$watch" ;;
        verify)  cmd_verify "$version" ;;
      esac ;;
    -h|--help|help) usage ;;
    *) die "unknown command: $cmd (status, prepare, tag, verify)" ;;
  esac
}

main "$@"
