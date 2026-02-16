#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-or-later
# Copyright (C) 2026 Relational Network
#
# Obtain Let's Encrypt certificate via DuckDNS DNS-01 challenge.
# No port 80 needed — validation is DNS-based.
#
# Usage:
#   sudo DUCKDNS_TOKEN=xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx \
#       bash apps/proxy/scripts/get-cert.sh
#
# Prerequisites:
#   - DuckDNS account with "relational-wallet" subdomain
#   - DuckDNS token from https://www.duckdns.org

set -euo pipefail

DOMAIN="relational-wallet.duckdns.org"
CERT_DIR="/etc/nginx/certs"
EMAIL="${LETSENCRYPT_EMAIL:-admin@relational.network}"

if [ -z "${DUCKDNS_TOKEN:-}" ]; then
    echo "ERROR: DUCKDNS_TOKEN environment variable is required."
    echo ""
    echo "Usage:"
    echo "  sudo DUCKDNS_TOKEN=<your-token> bash $0"
    echo ""
    echo "Get your token at: https://www.duckdns.org"
    exit 1
fi

echo "=== Let's Encrypt cert via DuckDNS DNS-01 ==="
echo "Domain:  $DOMAIN"
echo ""

# ── 1. Install certbot + DuckDNS plugin ─────────────────────────────
if ! command -v certbot &>/dev/null; then
    echo "→ Installing certbot..."
    apt-get update -qq
    apt-get install -y -qq certbot python3-certbot-nginx
    echo "  ✓ certbot installed"
fi

# Install DuckDNS authenticator script
HOOK_DIR="/etc/letsencrypt/duckdns"
mkdir -p "$HOOK_DIR"

# DNS-01 auth hook: sets the TXT record via DuckDNS API
cat > "$HOOK_DIR/auth.sh" << 'AUTHEOF'
#!/usr/bin/env bash
# DuckDNS DNS-01 authenticator hook for certbot
DUCKDNS_DOMAIN="${CERTBOT_DOMAIN%%.*}"
curl -s "https://www.duckdns.org/update?domains=${DUCKDNS_DOMAIN}&token=${DUCKDNS_TOKEN}&txt=${CERTBOT_VALIDATION}" > /dev/null
# DNS propagation delay
sleep 30
AUTHEOF

# DNS-01 cleanup hook: clears the TXT record
cat > "$HOOK_DIR/cleanup.sh" << 'CLEANEOF'
#!/usr/bin/env bash
DUCKDNS_DOMAIN="${CERTBOT_DOMAIN%%.*}"
curl -s "https://www.duckdns.org/update?domains=${DUCKDNS_DOMAIN}&token=${DUCKDNS_TOKEN}&txt=removed&clear=true" > /dev/null
CLEANEOF

chmod +x "$HOOK_DIR/auth.sh" "$HOOK_DIR/cleanup.sh"

# ── 2. Request certificate ──────────────────────────────────────────
echo "→ Requesting certificate (DNS-01 challenge)..."
echo "  This will take ~30 seconds for DNS propagation..."

export DUCKDNS_TOKEN

certbot certonly \
    --manual \
    --preferred-challenges dns \
    --manual-auth-hook "$HOOK_DIR/auth.sh" \
    --manual-cleanup-hook "$HOOK_DIR/cleanup.sh" \
    --domain "$DOMAIN" \
    --email "$EMAIL" \
    --agree-tos \
    --manual-public-ip-logging-ok \
    --non-interactive \
    --keep-until-expiring

# ── 3. Copy certs to Nginx location ─────────────────────────────────
LIVE_DIR="/etc/letsencrypt/live/$DOMAIN"
if [ -d "$LIVE_DIR" ]; then
    echo "→ Installing certificates..."
    cp -L "$LIVE_DIR/fullchain.pem" "$CERT_DIR/fullchain.pem"
    cp -L "$LIVE_DIR/privkey.pem"   "$CERT_DIR/privkey.pem"
    chmod 600 "$CERT_DIR/privkey.pem"
    echo "  ✓ Certificates installed to $CERT_DIR/"
else
    echo "ERROR: Certificate directory not found at $LIVE_DIR"
    exit 1
fi

# ── 4. Reload Nginx ─────────────────────────────────────────────────
echo "→ Reloading Nginx..."
nginx -t && systemctl reload nginx
echo "  ✓ Nginx reloaded with Let's Encrypt certificate"

# ── 5. Setup auto-renewal ───────────────────────────────────────────
# Store token for renewal cron
echo "DUCKDNS_TOKEN=$DUCKDNS_TOKEN" > /etc/letsencrypt/duckdns/.env
chmod 600 /etc/letsencrypt/duckdns/.env

RENEW_SCRIPT="/etc/letsencrypt/renewal-hooks/deploy/nginx-copy.sh"
mkdir -p "$(dirname "$RENEW_SCRIPT")"
cat > "$RENEW_SCRIPT" << EOF
#!/usr/bin/env bash
# Auto-deploy renewed certs to Nginx
cp -L /etc/letsencrypt/live/$DOMAIN/fullchain.pem $CERT_DIR/fullchain.pem
cp -L /etc/letsencrypt/live/$DOMAIN/privkey.pem   $CERT_DIR/privkey.pem
chmod 600 $CERT_DIR/privkey.pem
systemctl reload nginx
EOF
chmod +x "$RENEW_SCRIPT"

# Add cron job for renewal (runs twice daily, certbot skips if not due)
CRON_LINE="0 3,15 * * * . /etc/letsencrypt/duckdns/.env && certbot renew --quiet"
(crontab -l 2>/dev/null | grep -v "certbot renew" || true; echo "$CRON_LINE") | crontab -

echo ""
echo "=== Certificate setup complete ==="
echo ""
echo "  Certificate: $CERT_DIR/fullchain.pem"
echo "  Key:         $CERT_DIR/privkey.pem"
echo "  Auto-renew:  cron every 12h (certbot skips if not due)"
echo ""
echo "Verify:"
echo "  curl https://$DOMAIN/proxy/health"
echo ""
