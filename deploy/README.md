# Deploying the Nail website on a DigitalOcean droplet

One droplet hosts this site and any number of other services. Caddy sits on
ports 80/443 and routes by hostname; each app is a systemd service on its own
local port. Nothing is compiled on the droplet - `scripts/deploy.sh` transpiles
and builds on your machine and copies up a finished binary.

Setup splits in two, because the box outlives any one app:

| script | scope | run |
|---|---|---|
| `provision-base.sh` | the box: Caddy, ufw, fail2ban, swap | once per droplet |
| `add-app.sh` | one app: user, `/srv/<app>`, unit, Caddy fragment | once per app |

This `deploy/` directory is copied into each app's repo. No repo is the "lead" -
`provision-base.sh` is idempotent and refuses to overwrite an already-configured
box, so whichever repo runs first wins and the rest are no-ops.

Isolation each app gets:

- its own unix user and `/srv/<app>` at mode 0750 - other apps cannot read it
- `127.0.0.1` binding via `BIND_ADDR`, so the app has no publicly reachable
  socket; the proxy is the sole entrance and `ufw` only opens 22/80/443
- a systemd sandbox: `ProtectSystem=strict`, `ReadWritePaths=/srv/<app>`,
  `PrivateTmp`, `NoNewPrivileges`
- a memory ceiling (`MemoryMax`, default 192M) so one leak cannot take the
  others down

## One-time setup

1. Create the droplet: Ubuntu 24.04 or newer, Basic / Regular. The $4 512 MB
   size is enough - nothing is ever built here.

2. Prepare the box (once, ever):

   ```bash
   scp -r deploy root@<droplet-ip>:/tmp/deploy
   ssh root@<droplet-ip> 'bash /tmp/deploy/provision-base.sh'
   ```

3. Register this app on it:

   ```bash
   ssh root@<droplet-ip> \
     'bash /tmp/deploy/add-app.sh --name nail --port 8080 --bin nail_website_server --host nail.example.com'
   ```

   Without `--host` the app answers on the bare IP over HTTP; only one app on
   the box can hold that. With `--host`, the HTTPS certificate is automatic.

4. Create `.env` in the repo root (gitignored):

   ```
   DEPLOY_HOST=root@<droplet-ip>
   DEPLOY_PASSWORD=<ssh password, omit to use a key or type it>
   ```

5. `./scripts/deploy.sh`

## Everyday deploy

```bash
./scripts/deploy.sh
```

Transpiles `examples/nail_website.nail`, builds the server, uploads the binary
plus the runtime data files, restarts the service, and health-checks it -
dumping the last 30 log lines if the check fails. Set `SKIP_TRANSPILE=1` to
build the existing generated `main.rs` as-is.

**Runtime data files.** The server reads several paths relative to its working
directory. They are listed in `DATA_PATHS` in `scripts/deploy.sh`:
`examples/website_examples/`, `examples/nail_website.nail`, `tests/`,
`nail_language_spec.md`, `README.md`. If you add a `read_file` call to the
website, add its path there or the deployed site panics on startup.

## Port 8080

The port comes from the Nail source (`examples/nail_website.nail`), not from
the environment - the stdlib HTTP server takes its port from the program. If
you change it there, re-run `add-app.sh` with the matching `--port` and update
`APP_PORT` in `scripts/deploy.sh`.

## Adding another service to the same box

From the other repo, with this `deploy/` directory copied into it:

```bash
scp -r deploy root@<droplet-ip>:/tmp/deploy
ssh root@<droplet-ip> 'bash /tmp/deploy/provision-base.sh'   # no-op, box already set up
ssh root@<droplet-ip> 'bash /tmp/deploy/add-app.sh --name blog --port 3001 --bin blog --host blog.example.com'
```

Neither app can see the other's files. `add-app.sh` refuses a port another app
already claimed.

## Operating

```bash
systemctl status nail             # is it up
journalctl -u nail -f             # live logs
journalctl -u caddy -n 50         # cert / proxy problems
systemctl restart nail
```
