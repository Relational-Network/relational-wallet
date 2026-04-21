# proxy

TLS-terminating nginx in front of the rust-server. Holds a Let's Encrypt cert (DNS-01 via DuckDNS) on port 443 and forwards to the SGX backend on `127.0.0.1:8080`. The backend uses self-signed RA-TLS, so `proxy_ssl_verify off` is set on the upstream — clients that need to verify enclave identity should still validate the RA-TLS quote out-of-band.

```
external client ──► nginx :443 (LE cert) ──► rust-server :8080 (RA-TLS)
```

[`nginx.conf`](nginx.conf) is a template — `__DOMAIN__` is substituted at install time by the scripts below.

## Normal path: deploy with the host bootstrap

[`scripts/deploy-instance.sh`](../../scripts/deploy-instance.sh) at the repo root invokes both scripts here for you:

```bash
sudo INSTANCE=wallet-001 DUCKDNS_TOKEN=... bash scripts/deploy-instance.sh
```

That installs nginx + a self-signed bootstrap cert, then immediately upgrades to a Let's Encrypt cert for `wallet-001.duckdns.org`.

## Standalone usage

If you only want the proxy (e.g. retrofitting an existing host):

```bash
# 1. nginx + bootstrap self-signed cert for the chosen domain
sudo DOMAIN=wallet-001.duckdns.org bash scripts/setup.sh

# 2. real cert via DuckDNS DNS-01 (no port 80 needed)
sudo DOMAIN=wallet-001.duckdns.org \
     DUCKDNS_TOKEN=xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx \
     bash scripts/get-cert.sh
```

`get-cert.sh` also installs a cron job that runs `certbot renew` twice daily and reloads nginx on rotation. The DuckDNS token is persisted at `/etc/letsencrypt/duckdns/.env` (mode 600) for the renewal hook.

## Operate

```bash
sudo nginx -t                              # validate config
sudo systemctl reload nginx                # apply changes
sudo systemctl status nginx
sudo tail -f /var/log/nginx/{access,error}.log
curl https://wallet-001.duckdns.org/proxy/health
```

After editing [`nginx.conf`](nginx.conf), reinstall via the setup script (it re-substitutes `__DOMAIN__`); don't edit `/etc/nginx/nginx.conf` directly.

## What's in the config

- HTTP → HTTPS redirect (with `/.well-known/acme-challenge/` reserved)
- TLS 1.2/1.3 only, modern cipher suite, no session tickets
- `X-Content-Type-Options`, `X-Frame-Options` security headers
- `/proxy/health` returned by nginx itself (doesn't touch backend)
- Rate limit on `POST /v1/fiat/providers/truelayer/webhook`: 10 req/s burst 20 per IP
- All other paths proxied with `X-Real-IP`, `X-Forwarded-{For,Proto}`, `X-Request-ID`, and TrueLayer signature headers passed through

`client_max_body_size` is `1m` — bump if you add endpoints that need larger uploads.

---

SPDX-License-Identifier: AGPL-3.0-or-later · Copyright (C) 2026 Relational Network
