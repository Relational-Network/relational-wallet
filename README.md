# Relational Wallet

**TEE-backed non-custodial Avalanche wallet service.** Private keys and persistent state stay sealed inside an Intel SGX enclave (Gramine encrypted FS); clients verify the enclave identity via DCAP RA-TLS before sending requests.

## Architecture

```
Browser ─► Vercel (wallet-web, Next.js) ─► nginx (LE cert) ─► rust-server (SGX, RA-TLS)
                                                                    │
                                                              Avalanche C-Chain
```

| Component | Path | Hosting |
|-----------|------|---------|
| `wallet-web` | [apps/wallet-web](apps/wallet-web/) | Vercel (per instance) |
| `proxy` | [apps/proxy](apps/proxy/) | nginx on the SGX host |
| `rust-server` | [apps/rust-server](apps/rust-server/) | Docker on the SGX host |
| `contracts` (`rEUR`) | [apps/contracts](apps/contracts/) | Avalanche Fuji |

Container image: `ghcr.io/relational-network/rust-server:main` (built + signed by [.github/workflows/rust-server-ci.yml](.github/workflows/rust-server-ci.yml); `MRENCLAVE` pinned in [apps/rust-server/measurements.toml](apps/rust-server/measurements.toml)).

## Host prerequisites

A single instance lives on one Linux host. Required:

- **SGX hardware** with DCAP support and `/dev/sgx/{enclave,provision}` exposed
- **`sgx-aesm-service`** running (provides `/var/run/aesmd`)
- **Docker** (for the rust-server image)
- **Ports 80 + 443** reachable from the public internet (nginx + Let's Encrypt)

External accounts (one-time):

- **DuckDNS** subdomain + token — one DNS name per instance (e.g. `wallet-001.duckdns.org`)
- **Clerk** application — provides `CLERK_JWKS_URL` and `CLERK_ISSUER`
- **Vercel** project pointing at `apps/wallet-web`
- *(optional)* **TrueLayer** sandbox credentials for fiat on/off-ramp

## Deploy a new instance

```bash
sudo INSTANCE=wallet-001 \
     DUCKDNS_TOKEN=xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx \
     bash scripts/deploy-instance.sh
```

This installs nginx + Let's Encrypt cert for `wallet-001.duckdns.org`, drops a systemd unit that runs the SGX container (auto-restarts on reboot, pulls latest image on `systemctl restart`), and seeds an env file at `/etc/relational-wallet/rust-server.env` from [apps/rust-server/.env.example](apps/rust-server/.env.example).

Edit the env file (Clerk keys, CORS, optional TrueLayer creds), then start:

```bash
sudo $EDITOR /etc/relational-wallet/rust-server.env
sudo systemctl restart rust-server
```

Then deploy [apps/wallet-web](apps/wallet-web/) to Vercel and set:

```
WALLET_API_BASE_URL=https://wallet-001.duckdns.org
NEXT_PUBLIC_CLERK_PUBLISHABLE_KEY=pk_live_...
CLERK_SECRET_KEY=sk_live_...
```

Add the resulting Vercel URL to `CORS_ALLOWED_ORIGINS` in the host env file and restart `rust-server`.

## Operate

```bash
sudo systemctl restart rust-server      # also pulls latest image
sudo systemctl status rust-server
journalctl -u rust-server -f            # logs
curl https://wallet-001.duckdns.org/proxy/health
```

For reproducible deploys, pin the image to a digest:

```bash
sudo IMAGE=ghcr.io/relational-network/rust-server@sha256:<digest> \
     INSTANCE=wallet-001 DUCKDNS_TOKEN=... \
     bash scripts/deploy-instance.sh
```

## Further reading

- [apps/rust-server/README.md](apps/rust-server/README.md) — API, manifest, Gramine details
- [apps/rust-server/docker/README.md](apps/rust-server/docker/README.md) — image build + MRENCLAVE pinning
- [apps/proxy/README.md](apps/proxy/README.md) — nginx config + cert renewal
- [apps/wallet-web/README.md](apps/wallet-web/README.md) — frontend + Clerk wiring
- [apps/contracts/README.md](apps/contracts/README.md) — `rEUR` contract

## License

AGPL-3.0-or-later — see [LICENSE](LICENSE).
