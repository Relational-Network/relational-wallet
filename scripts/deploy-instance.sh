#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-or-later
# Copyright (C) 2026 Relational Network
#
# Bootstrap a new Relational Wallet instance on this host.
#
#   • installs nginx + Let's Encrypt cert for INSTANCE.duckdns.org
#   • installs a systemd unit that runs the rust-server SGX container
#     (auto-pulls the configured image and restarts on reboot/crash)
#   • drops a per-instance env file at /etc/relational-wallet/rust-server.env
#
# Run as root.
#
# Usage:
#   sudo INSTANCE=wallet-001 \
#        DUCKDNS_TOKEN=xxxxxxxx-... \
#        bash scripts/deploy-instance.sh
#
# Optional env:
#   IMAGE       Container image (default: ghcr.io/relational-network/rust-server:main)
#   DATA_DIR    Host path bind-mounted to /data (default: /var/lib/relational-wallet/data)
#   ENV_FILE    Per-instance env file (default: /etc/relational-wallet/rust-server.env)
#   APP_UID     UID owning DATA_DIR (default: 10001 — matches Dockerfile)
#   APP_GID     GID owning DATA_DIR (default: 10001)
#   SKIP_CERT   Set to 1 to skip Let's Encrypt (use existing self-signed bootstrap)

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

INSTANCE="${INSTANCE:-}"
IMAGE="${IMAGE:-ghcr.io/relational-network/rust-server:main}"
DATA_DIR="${DATA_DIR:-/var/lib/relational-wallet/data}"
ENV_FILE="${ENV_FILE:-/etc/relational-wallet/rust-server.env}"
APP_UID="${APP_UID:-10001}"
APP_GID="${APP_GID:-10001}"
SKIP_CERT="${SKIP_CERT:-0}"

if [ -z "$INSTANCE" ]; then
    echo "ERROR: INSTANCE environment variable is required (e.g. INSTANCE=wallet-001)" >&2
    exit 1
fi

if [ "$EUID" -ne 0 ]; then
    echo "ERROR: must run as root (use sudo)" >&2
    exit 1
fi

DOMAIN="${INSTANCE}.duckdns.org"

echo "=== Relational Wallet — instance bootstrap ==="
echo "Instance:  $INSTANCE"
echo "Domain:    $DOMAIN"
echo "Image:     $IMAGE"
echo "Data dir:  $DATA_DIR  (uid=$APP_UID gid=$APP_GID)"
echo "Env file:  $ENV_FILE"
echo ""

# ── Preflight ────────────────────────────────────────────────────────
command -v docker >/dev/null || { echo "ERROR: docker not installed" >&2; exit 1; }
[ -e /dev/sgx/enclave ] || { echo "ERROR: /dev/sgx/enclave missing — SGX driver not loaded" >&2; exit 1; }
[ -e /dev/sgx/provision ] || { echo "ERROR: /dev/sgx/provision missing" >&2; exit 1; }
[ -d /var/run/aesmd ] || { echo "ERROR: /var/run/aesmd missing — install sgx-aesm-service" >&2; exit 1; }

if [ "$SKIP_CERT" != "1" ] && [ -z "${DUCKDNS_TOKEN:-}" ]; then
    echo "ERROR: DUCKDNS_TOKEN required (or set SKIP_CERT=1 to keep self-signed)" >&2
    exit 1
fi

# ── 1. nginx + bootstrap self-signed cert ───────────────────────────
echo "→ [1/5] Installing nginx reverse proxy for $DOMAIN..."
DOMAIN="$DOMAIN" bash "$REPO_ROOT/apps/proxy/scripts/setup.sh"

# ── 2. Let's Encrypt cert (DNS-01 via DuckDNS) ──────────────────────
if [ "$SKIP_CERT" = "1" ]; then
    echo "→ [2/5] SKIP_CERT=1 — keeping self-signed bootstrap cert"
else
    echo "→ [2/5] Requesting Let's Encrypt cert for $DOMAIN..."
    DOMAIN="$DOMAIN" DUCKDNS_TOKEN="$DUCKDNS_TOKEN" \
        bash "$REPO_ROOT/apps/proxy/scripts/get-cert.sh"
fi

# ── 3. Provision data dir + env file ────────────────────────────────
echo "→ [3/5] Provisioning $DATA_DIR (uid=$APP_UID gid=$APP_GID)..."
install -d -m 0750 -o "$APP_UID" -g "$APP_GID" "$DATA_DIR"

install -d -m 0755 "$(dirname "$ENV_FILE")"
if [ ! -f "$ENV_FILE" ]; then
    echo "→ Seeding env file from .env.example..."
    install -m 0640 "$REPO_ROOT/apps/rust-server/.env.example" "$ENV_FILE"
    echo "  ⚠ Edit $ENV_FILE to fill in CLERK_*, TRUELAYER_*, CORS_ALLOWED_ORIGINS"
    echo "    then run: systemctl restart rust-server"
else
    echo "  ✓ Env file already exists at $ENV_FILE — leaving untouched"
fi

# ── 4. Install systemd unit ─────────────────────────────────────────
echo "→ [4/5] Installing systemd unit..."
UNIT_PATH="/etc/systemd/system/rust-server.service"
sed \
    -e "s|__IMAGE__|$IMAGE|g" \
    -e "s|__DATA_DIR__|$DATA_DIR|g" \
    -e "s|__ENV_FILE__|$ENV_FILE|g" \
    "$REPO_ROOT/scripts/systemd/rust-server.service" > "$UNIT_PATH"

systemctl daemon-reload
systemctl enable rust-server.service
echo "  ✓ Unit installed at $UNIT_PATH and enabled at boot"

# ── 5. Start (only if env file looks configured) ────────────────────
echo "→ [5/5] Checking env file before starting..."
if grep -qE '<[^>]+>' "$ENV_FILE" 2>/dev/null; then
    echo "  ⚠ $ENV_FILE still contains <placeholder> values — NOT starting."
    echo "    Edit the file, then run: sudo systemctl start rust-server"
    NOT_STARTED=1
else
    echo "→ Starting rust-server..."
    systemctl restart rust-server.service
    sleep 2
    if systemctl is-active --quiet rust-server.service; then
        echo "  ✓ rust-server is running"
    else
        echo "  ✗ rust-server failed to start — check: journalctl -u rust-server -n 50"
        exit 1
    fi
    NOT_STARTED=0
fi

# ── Done ────────────────────────────────────────────────────────────
echo ""
echo "=== Instance ready ==="
echo ""
echo "  Backend URL:  https://$DOMAIN"
echo "  Health:       curl https://$DOMAIN/proxy/health"
echo "  Logs:         journalctl -u rust-server -f"
echo "  Restart:      sudo systemctl restart rust-server"
echo ""
echo "Vercel env (paste into apps/wallet-web project settings):"
echo "  WALLET_API_BASE_URL=https://$DOMAIN"
echo "  NODE_TLS_REJECT_UNAUTHORIZED=0   # only if backend still uses self-signed RA-TLS chain"
echo ""
echo "After Vercel deploy, add the Vercel URL to CORS_ALLOWED_ORIGINS in:"
echo "  $ENV_FILE"
echo "then: sudo systemctl restart rust-server"
echo ""
if [ "${NOT_STARTED:-0}" = "1" ]; then
    exit 2
fi
