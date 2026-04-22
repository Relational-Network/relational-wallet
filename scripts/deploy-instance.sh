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
#        CLERK_JWKS_URL=https://your-app.clerk.accounts.dev/.well-known/jwks.json \
#        CLERK_ISSUER=https://your-app.clerk.accounts.dev \
#        CLERK_SECRET_KEY=sk_live_... \
#        CORS_ALLOWED_ORIGINS=https://your-frontend.vercel.app \
#        TRUELAYER_CLIENT_ID=... \
#        TRUELAYER_CLIENT_SECRET=... \
#        TRUELAYER_SIGNING_KEY_ID=... \
#        TRUELAYER_SIGNING_PRIVATE_KEY_PEM='-----BEGIN PRIVATE KEY-----\n...\n-----END PRIVATE KEY-----' \
#        TRUELAYER_MERCHANT_ACCOUNT_ID=... \
#        TRUELAYER_OFFRAMP_ACCOUNT_HOLDER_NAME='Jane Doe' \
#        TRUELAYER_OFFRAMP_IBAN=DE... \
#        DUCKDNS_TOKEN=xxxxxxxx-... \
#        bash scripts/deploy-instance.sh
#
# Optional env:
#   IMAGE       Container image (default: ghcr.io/relational-network/rust-server:main)
#   DATA_DIR    Host path bind-mounted to /data (default: /var/lib/relational-wallet/data)
#   ENV_FILE    Per-instance env file (default: /etc/relational-wallet/rust-server.env)
#   RATLS_PORT  Public host port exposing the raw rust-server RA-TLS endpoint for peer discovery (default: 8443)
#   APP_UID     UID owning DATA_DIR (default: 10001 — matches Dockerfile)
#   APP_GID     GID owning DATA_DIR (default: 10001)
#   SKIP_CERT   Set to 1 to skip Let's Encrypt (use existing self-signed bootstrap)
#
# Required runtime env:
#   CLERK_JWKS_URL
#   CLERK_ISSUER
#   CLERK_SECRET_KEY
#   CORS_ALLOWED_ORIGINS
#   TRUELAYER_CLIENT_ID
#   TRUELAYER_CLIENT_SECRET
#   TRUELAYER_SIGNING_KEY_ID
#   TRUELAYER_SIGNING_PRIVATE_KEY_PEM
#   TRUELAYER_MERCHANT_ACCOUNT_ID
#   TRUELAYER_OFFRAMP_ACCOUNT_HOLDER_NAME
#   TRUELAYER_OFFRAMP_IBAN

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
UBUNTU_CODENAME="focal"

APT_UPDATED=0

apt_update_once() {
    if [ "$APT_UPDATED" -eq 0 ]; then
        apt-get update
        APT_UPDATED=1
    fi
}

is_installed() {
    dpkg-query -W -f='${Status}' "$1" 2>/dev/null | grep -q '^install ok installed$'
}

ensure_packages() {
    local missing=()
    local pkg

    for pkg in "$@"; do
        if ! is_installed "$pkg"; then
            missing+=("$pkg")
        fi
    done

    if [ "${#missing[@]}" -gt 0 ]; then
        apt_update_once
        apt-get install -y "${missing[@]}"
    fi
}

ensure_intel_sgx_repo() {
    local repo_file="/etc/apt/sources.list.d/intel-sgx.list"
    local keyring="/usr/share/keyrings/intel-sgx-deb.gpg"

    if ! grep -Rhsq 'download.01.org/intel-sgx/sgx_repo/ubuntu' /etc/apt/sources.list /etc/apt/sources.list.d/*.list 2>/dev/null; then
        echo "→ Adding Intel SGX apt repository..."
        install -d -m 0755 /usr/share/keyrings
        curl -fsSL https://download.01.org/intel-sgx/sgx_repo/ubuntu/intel-sgx-deb.key \
            | gpg --dearmor > "$keyring"
        printf 'deb [arch=amd64 signed-by=%s] https://download.01.org/intel-sgx/sgx_repo/ubuntu %s main\n' \
            "$keyring" "$UBUNTU_CODENAME" > "$repo_file"
        APT_UPDATED=0
    fi
}

ensure_microsoft_repo() {
    local repo_file="/etc/apt/sources.list.d/microsoft-prod.list"
    local keyring="/usr/share/keyrings/microsoft-prod.gpg"

    if ! grep -Rhsq 'packages.microsoft.com/ubuntu/20.04/prod' /etc/apt/sources.list /etc/apt/sources.list.d/*.list 2>/dev/null; then
        echo "→ Adding Microsoft apt repository..."
        install -d -m 0755 /usr/share/keyrings
        curl -fsSL https://packages.microsoft.com/keys/microsoft.asc \
            | gpg --dearmor > "$keyring"
        printf 'deb [arch=amd64 signed-by=%s] https://packages.microsoft.com/ubuntu/20.04/prod %s main\n' \
            "$keyring" "$UBUNTU_CODENAME" > "$repo_file"
        APT_UPDATED=0
    fi
}

ensure_docker() {
    if ! command -v docker >/dev/null 2>&1; then
        echo "→ Installing Docker..."
        ensure_packages docker.io
    fi

    systemctl enable docker
    systemctl start docker
}

ensure_sgx_userspace() {
    local sgx_packages=(
        apt-transport-https
        ca-certificates
        curl
        gnupg
        pkg-config
    )

    echo "→ Installing SGX/DCAP userspace prerequisites if missing..."
    ensure_packages "${sgx_packages[@]}"
    ensure_microsoft_repo
    ensure_intel_sgx_repo
    ensure_packages \
        az-dcap-client \
        sgx-aesm-service \
        libsgx-aesm-ecdsa-plugin \
        libsgx-aesm-quote-ex-plugin

    systemctl enable aesmd
    systemctl start aesmd
}

require_env() {
    local name="$1"
    if [ -z "${!name:-}" ]; then
        echo "ERROR: $name environment variable is required" >&2
        exit 1
    fi
}

validate_port() {
    local name="$1"
    local value="$2"

    if ! [[ "$value" =~ ^[0-9]+$ ]]; then
        echo "ERROR: $name must be a numeric TCP port" >&2
        exit 1
    fi

    if [ "$value" -lt 1 ] || [ "$value" -gt 65535 ]; then
        echo "ERROR: $name must be between 1 and 65535" >&2
        exit 1
    fi
}

normalize_env_value() {
    local value="$1"
    value="${value//$'\r'/}"
    value="${value//$'\n'/\\n}"
    printf '%s' "$value"
}

write_runtime_env_file() {
    local backup_file

    install -d -m 0755 "$(dirname "$ENV_FILE")"

    if [ -f "$ENV_FILE" ]; then
        backup_file="${ENV_FILE}.bak.$(date -u +%Y%m%dT%H%M%SZ)"
        cp "$ENV_FILE" "$backup_file"
        echo "  ✓ Backed up existing env file to $backup_file"
    fi

    cat > "$ENV_FILE" <<EOF
# SPDX-License-Identifier: AGPL-3.0-or-later
# Copyright (C) 2026 Relational Network
# Generated by scripts/deploy-instance.sh
CLERK_JWKS_URL=$(normalize_env_value "$CLERK_JWKS_URL")
CLERK_ISSUER=$(normalize_env_value "$CLERK_ISSUER")
CLERK_SECRET_KEY=$(normalize_env_value "$CLERK_SECRET_KEY")
CORS_ALLOWED_ORIGINS=$(normalize_env_value "$CORS_ALLOWED_ORIGINS")
TRUELAYER_CLIENT_ID=$(normalize_env_value "$TRUELAYER_CLIENT_ID")
TRUELAYER_CLIENT_SECRET=$(normalize_env_value "$TRUELAYER_CLIENT_SECRET")
TRUELAYER_SIGNING_KEY_ID=$(normalize_env_value "$TRUELAYER_SIGNING_KEY_ID")
TRUELAYER_SIGNING_PRIVATE_KEY_PEM=$(normalize_env_value "$TRUELAYER_SIGNING_PRIVATE_KEY_PEM")
TRUELAYER_MERCHANT_ACCOUNT_ID=$(normalize_env_value "$TRUELAYER_MERCHANT_ACCOUNT_ID")
TRUELAYER_OFFRAMP_ACCOUNT_HOLDER_NAME=$(normalize_env_value "$TRUELAYER_OFFRAMP_ACCOUNT_HOLDER_NAME")
TRUELAYER_OFFRAMP_IBAN=$(normalize_env_value "$TRUELAYER_OFFRAMP_IBAN")
TRUELAYER_API_BASE_URL=https://api.truelayer-sandbox.com
TRUELAYER_AUTH_BASE_URL=https://auth.truelayer-sandbox.com
TRUELAYER_HOSTED_PAYMENTS_BASE_URL=https://payment.truelayer-sandbox.com
TRUELAYER_CURRENCY=EUR
REUR_CONTRACT_ADDRESS_FUJI=0x76568BEd5Acf1A5Cd888773C8cAe9ea2a9131A63
FIAT_MIN_CONFIRMATIONS=1
APP_ENV=production
DEFAULT_NETWORK=fuji
TRUST_X_FORWARDED_FOR=false
EOF

    chmod 0640 "$ENV_FILE"
    echo "  ✓ Wrote runtime env file to $ENV_FILE"
}

INSTANCE="${INSTANCE:-}"
IMAGE="${IMAGE:-ghcr.io/relational-network/rust-server:main}"
DATA_DIR="${DATA_DIR:-/var/lib/relational-wallet/data}"
ENV_FILE="${ENV_FILE:-/etc/relational-wallet/rust-server.env}"
RATLS_PORT="${RATLS_PORT:-8443}"
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

validate_port RATLS_PORT "$RATLS_PORT"

if [ "$RATLS_PORT" = "80" ] || [ "$RATLS_PORT" = "443" ] || [ "$RATLS_PORT" = "8080" ]; then
    echo "ERROR: RATLS_PORT must not be 80, 443, or 8080 on this deployment" >&2
    exit 1
fi

require_env CLERK_JWKS_URL
require_env CLERK_ISSUER
require_env CLERK_SECRET_KEY
require_env CORS_ALLOWED_ORIGINS
require_env TRUELAYER_CLIENT_ID
require_env TRUELAYER_CLIENT_SECRET
require_env TRUELAYER_SIGNING_KEY_ID
require_env TRUELAYER_SIGNING_PRIVATE_KEY_PEM
require_env TRUELAYER_MERCHANT_ACCOUNT_ID
require_env TRUELAYER_OFFRAMP_ACCOUNT_HOLDER_NAME
require_env TRUELAYER_OFFRAMP_IBAN

DOMAIN="${INSTANCE}.duckdns.org"

echo "=== Relational Wallet — instance bootstrap ==="
echo "Instance:  $INSTANCE"
echo "Domain:    $DOMAIN"
echo "Image:     $IMAGE"
echo "Data dir:  $DATA_DIR  (uid=$APP_UID gid=$APP_GID)"
echo "Env file:  $ENV_FILE"
echo "RA-TLS:    https://$DOMAIN:$RATLS_PORT"
echo ""

# ── 1. Host prerequisites ────────────────────────────────────────────
echo "→ [1/6] Installing Docker and SGX/DCAP prerequisites if needed..."
ensure_packages apt-transport-https curl
ensure_docker
ensure_sgx_userspace

# ── Preflight ────────────────────────────────────────────────────────
[ -e /dev/sgx/enclave ] || { echo "ERROR: /dev/sgx/enclave missing — SGX driver not loaded" >&2; exit 1; }
[ -e /dev/sgx/provision ] || { echo "ERROR: /dev/sgx/provision missing" >&2; exit 1; }
[ -d /var/run/aesmd ] || { echo "ERROR: /var/run/aesmd missing — install sgx-aesm-service" >&2; exit 1; }

if [ "$SKIP_CERT" != "1" ] && [ -z "${DUCKDNS_TOKEN:-}" ]; then
    echo "ERROR: DUCKDNS_TOKEN required (or set SKIP_CERT=1 to keep self-signed)" >&2
    exit 1
fi

# ── 2. nginx + bootstrap self-signed cert ───────────────────────────
echo "→ [2/6] Installing nginx reverse proxy for $DOMAIN..."
DOMAIN="$DOMAIN" bash "$REPO_ROOT/apps/proxy/scripts/setup.sh"

# ── 3. Let's Encrypt cert (DNS-01 via DuckDNS) ──────────────────────
if [ "$SKIP_CERT" = "1" ]; then
    echo "→ [3/6] SKIP_CERT=1 — keeping self-signed bootstrap cert"
else
    echo "→ [3/6] Requesting Let's Encrypt cert for $DOMAIN..."
    DOMAIN="$DOMAIN" DUCKDNS_TOKEN="$DUCKDNS_TOKEN" \
        bash "$REPO_ROOT/apps/proxy/scripts/get-cert.sh"
fi

# ── 4. Provision data dir + env file ────────────────────────────────
echo "→ [4/6] Provisioning $DATA_DIR (uid=$APP_UID gid=$APP_GID)..."
install -d -m 0750 -o "$APP_UID" -g "$APP_GID" "$DATA_DIR"

echo "→ Writing runtime env file..."
write_runtime_env_file

# ── 5. Install systemd unit ─────────────────────────────────────────
echo "→ [5/6] Installing systemd unit..."
UNIT_PATH="/etc/systemd/system/rust-server.service"
sed \
    -e "s|__IMAGE__|$IMAGE|g" \
    -e "s|__DATA_DIR__|$DATA_DIR|g" \
    -e "s|__ENV_FILE__|$ENV_FILE|g" \
    -e "s|__RATLS_PORT__|$RATLS_PORT|g" \
    "$REPO_ROOT/scripts/systemd/rust-server.service" > "$UNIT_PATH"

systemctl daemon-reload
systemctl enable rust-server.service
echo "  ✓ Unit installed at $UNIT_PATH and enabled at boot"

# ── 6. Start (only if env file looks configured) ────────────────────
echo "→ [6/6] Starting rust-server..."
systemctl restart rust-server.service
sleep 2
if systemctl is-active --quiet rust-server.service; then
    echo "  ✓ rust-server is running"
else
    echo "  ✗ rust-server failed to start — check: journalctl -u rust-server -n 50"
    exit 1
fi

# ── Done ────────────────────────────────────────────────────────────
echo ""
echo "=== Instance ready ==="
echo ""
echo "  Backend URL:  https://$DOMAIN"
echo "  Peer RA-TLS:  https://$DOMAIN:$RATLS_PORT"
echo "  Peer URL:     https://$DOMAIN:$RATLS_PORT/v1"
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
echo "Discovery peers must use the raw RA-TLS /v1 URL above, not the nginx 443 proxy URL."
echo ""
