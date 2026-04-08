#!/usr/bin/env bash
# Run the same checks as rust-server-ci.yml locally before pushing.
# Usage: ./scripts/ci-local.sh [--skip-docker]
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
RUST_SERVER="$REPO_ROOT/apps/rust-server"
SKIP_DOCKER=false

for arg in "$@"; do
  case "$arg" in
    --skip-docker) SKIP_DOCKER=true ;;
    *) echo "Unknown argument: $arg"; exit 1 ;;
  esac
done

step() { echo; echo "==> $*"; }
ok()   { echo "    OK: $*"; }

# ---------------------------------------------------------------------------
# check job: fmt, clippy, tests
# ---------------------------------------------------------------------------
step "Check formatting"
(cd "$RUST_SERVER" && RUSTC_WRAPPER="" cargo fmt --check)
ok "cargo fmt"

step "Clippy"
(cd "$RUST_SERVER" && RUSTC_WRAPPER="" cargo clippy --all-targets --features dev --locked -- -D warnings)
ok "cargo clippy"

step "Tests"
(cd "$RUST_SERVER" && RUSTC_WRAPPER="" cargo test --features dev --locked --verbose)
ok "cargo test"

# ---------------------------------------------------------------------------
# validate-docker job
# ---------------------------------------------------------------------------
if [[ "$SKIP_DOCKER" == "true" ]]; then
  echo
  echo "Skipping Docker validation (--skip-docker)."
else
  step "Validate shell scripts"
  sh -n "$RUST_SERVER/docker/start.sh"
  sh -n "$RUST_SERVER/docker/restart_aesm.sh"
  ok "shell scripts"

  step "Lint Dockerfile"
  if command -v hadolint &>/dev/null; then
    hadolint --failure-threshold warning "$RUST_SERVER/docker/Dockerfile"
    ok "hadolint"
  else
    echo "    hadolint not found — skipping (install from https://github.com/hadolint/hadolint)."
  fi
fi

# ---------------------------------------------------------------------------
# build-docker job — requires signing key; skipped locally unless provided
# ---------------------------------------------------------------------------
if [[ -n "${ENCLAVE_SIGNING_KEY:-}" ]]; then
  step "Build signed Docker image"
  IMAGE_TAG="relationalnetwork/rust-server:ci-local"
  echo "ENCLAVE_SIGNING_KEY found — building signed image $IMAGE_TAG"
  TMPKEY=$(mktemp)
  trap 'shred -u "$TMPKEY" 2>/dev/null || rm -f "$TMPKEY"' EXIT
  printenv ENCLAVE_SIGNING_KEY > "$TMPKEY"
  chmod 600 "$TMPKEY"
  docker build \
    --file "$RUST_SERVER/docker/Dockerfile" \
    --tag "$IMAGE_TAG" \
    --secret "id=sgx-key,src=$TMPKEY" \
    "$RUST_SERVER"
  ok "docker build"
else
  echo
  echo "ENCLAVE_SIGNING_KEY not set — skipping signed Docker build (matches CI behaviour for forks)."
fi

echo
echo "All local CI checks passed."
