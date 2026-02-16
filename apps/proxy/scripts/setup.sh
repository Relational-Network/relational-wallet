#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-or-later
# Copyright (C) 2026 Relational Network
#
# Setup Nginx reverse proxy for Relational Wallet.
# Run as root or with sudo.
#
# Usage:
#   sudo bash apps/proxy/scripts/setup.sh

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROXY_DIR="$(dirname "$SCRIPT_DIR")"
DOMAIN="relational-wallet.duckdns.org"
CERT_DIR="/etc/nginx/certs"
WEBROOT="/var/www/certbot"

echo "=== Relational Wallet — Nginx Proxy Setup ==="
echo "Domain:  $DOMAIN"
echo "Proxy:   $PROXY_DIR"
echo ""

# ── 1. Install Nginx ────────────────────────────────────────────────
if ! command -v nginx &>/dev/null; then
    echo "→ Installing Nginx..."
    apt-get update -qq
    apt-get install -y -qq nginx
    echo "  ✓ Nginx installed"
else
    echo "  ✓ Nginx already installed"
fi

# ── 2. Create directories ───────────────────────────────────────────
mkdir -p "$CERT_DIR" "$WEBROOT"

# ── 3. Generate self-signed cert (bootstrap — so Nginx can start) ──
if [ ! -f "$CERT_DIR/fullchain.pem" ]; then
    echo "→ Generating self-signed bootstrap certificate..."
    openssl req -x509 -nodes -newkey ec -pkeyopt ec_paramgen_curve:prime256v1 \
        -days 365 \
        -keyout "$CERT_DIR/privkey.pem" \
        -out "$CERT_DIR/fullchain.pem" \
        -subj "/CN=$DOMAIN" \
        -addext "subjectAltName=DNS:$DOMAIN" \
        2>/dev/null
    echo "  ✓ Self-signed cert generated at $CERT_DIR/"
    echo "  ⚠ Replace with Let's Encrypt cert for production!"
    echo "    Run:  sudo bash $PROXY_DIR/scripts/get-cert.sh"
else
    echo "  ✓ Certificate already exists at $CERT_DIR/"
fi

# ── 4. Install Nginx config ─────────────────────────────────────────
echo "→ Installing Nginx configuration..."
cp "$PROXY_DIR/nginx.conf" /etc/nginx/nginx.conf
echo "  ✓ Config installed"

# ── 5. Test and restart ─────────────────────────────────────────────
echo "→ Testing Nginx configuration..."
nginx -t
echo "  ✓ Config valid"

echo "→ Restarting Nginx..."
systemctl enable nginx
systemctl restart nginx
echo "  ✓ Nginx running"

# ── 6. Verify ───────────────────────────────────────────────────────
echo ""
echo "=== Setup complete ==="
echo ""
echo "Nginx is proxying:"
echo "  https://$DOMAIN → https://127.0.0.1:8080 (rust-server)"
echo ""
echo "Self-signed cert is active. To get a real Let's Encrypt cert:"
echo "  1. Get a DuckDNS token from https://www.duckdns.org"
echo "  2. Run:  sudo DUCKDNS_TOKEN=<token> bash $PROXY_DIR/scripts/get-cert.sh"
echo ""
echo "Quick test:"
echo "  curl -sk https://localhost/proxy/health"
echo ""
