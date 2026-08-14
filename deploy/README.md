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

Runs the documentation tests (`cargo test --lib docs`, which compile every
Nail block in every markdown file and verify the paths the docs name), then
transpiles `examples/website/main.nail`, builds the wasm demos and the
playground, builds the server, uploads the binary plus the runtime data files,
restarts the service, and health-checks it - dumping the last 30 log lines if
the check fails. `deploy/releases.sh` runs the same gate before publishing a
compiler release. Set `SKIP_TRANSPILE=1` to build the existing generated
`main.rs` as-is.

**Runtime data files.** The server runs in `/srv/nail/examples/website`, the
same run-in-its-own-directory rule every Nail program follows. `add-app.sh`
registers units with `WorkingDirectory=/srv/<app>`, so `scripts/deploy.sh`
writes a systemd drop-in setting the website's working directory, idempotently
on every deploy. Reads are relative to that directory: site files by bare
name (`snippets/...`), repo files above it by `../../` (`../../README.md`).
The shipped paths are listed in `DATA_PATHS` in `scripts/deploy.sh`,
currently: `examples/website/`, `examples/mcp_dice_server.nail`, `tests/`,
`nail_language_spec.md`, `nail_for_agents.md` (served at /llms.txt),
`README.md`, `wasm_demos/`, and `bundle/install.sh`
(the script behind the `curl | sh` install one-liner). If you add an `fs_read`
call to the website, add its path there or the deployed site panics on
startup.

## Publishing a Nail release

`nail` fetches compilers from this box, and the box serves them itself. Caddy
reads the file straight off disk, so the bytes never pass through the website
process and its 192M limit is irrelevant. A bundle is roughly one to two
gigabytes against ~10GB of droplet disk and 500GB/month of transfer, which is
one or two versions live at a time and a few hundred downloads a month.

There is no source host in the path. What a user downloads is a compiled
artifact, so a git tag has nothing that corresponds to it and the two would
drift apart immediately.

```bash
./bundle/build_bundle.sh                                  # build it
./deploy/releases.sh bundle/nail-0.1.0-linux-x86_64.tar.xz  # ship it
```

`releases.sh` uploads the bundle beside its final name and renames it into
place, so a launcher that asks mid-upload gets a 404 rather than half a file.
It also uploads the launcher for `nail self-update`, writes
`/versions/latest`, and installs the Caddy fragment.

That fragment is `/etc/caddy/sites.d/nail.caddy`, **replacing** the one
`add-app.sh` wrote for the same hostname (Caddy allows one block per host), and
proxying the website as its fallback route. Re-running `add-app.sh` reverts to
plain proxying, and re-running `releases.sh` puts the release routes back.

Unpublishing an alpha, which is allowed before 1.0:

```bash
./deploy/releases.sh --withdraw 0.1.0
```

Releases are **not signed**. Being able to write to this box is the credential:
anyone who can publish here is already part of the project. Set `RELEASE_DOMAIN`
in `.env` to the hostname `nail` fetches releases from.

## Port 8080

The port comes from the Nail source (`examples/website/main.nail`), not from
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
