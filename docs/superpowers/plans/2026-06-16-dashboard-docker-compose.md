# Dashboard Docker image + compose stack — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Publish a Docker image of the dashboard to GHCR (multi-arch) and ship a `docker-compose.yml` that runs the complete monitoring system (tracker + dashboard) from a clone with only `DMTR_API_KEY` set.

**Architecture:** A multi-stage Node image builds the Nitro SSR bundle and runs it in a slim runtime. A compose file wires the published tracker image and the dashboard image with the correct volume split — bind mounts (read-only) for `protocols/` and a Docker-specific `deploy/tracker.toml`, a named volume (read-write) for the shared `tracker.db` — and orders startup with a tracker healthcheck.

**Tech Stack:** Node 24, pnpm, TanStack Start / Nitro, better-sqlite3, Docker Compose, Docker buildx, GitHub Actions, GHCR.

**Spec:** `docs/superpowers/specs/2026-06-16-dashboard-docker-compose-design.md`

**Cross-repo dependency:** Consumes `ghcr.io/tx3-lang/tracker` and the tracker's `DMTR_API_KEY` env fallback (tx3-lift plan, Task 1). During development, build the tracker image locally; the committed compose pins the published image.

**Conventions for this plan:** Per the team's code-free-plan convention, steps state intent, exact files, and verification commands/expected output — not source content. Authoring follows the spec sections referenced in each task.

---

### Task 1: Dashboard Dockerfile

**Files:**
- Create: `frontend/Dockerfile`
- Create: `frontend/.dockerignore`

- [ ] **Step 1: Author the multi-stage Dockerfile** per spec §4.1: builder on `node:24` with the toolchain to compile `better-sqlite3`'s native binding, running `pnpm install` then `pnpm build` to produce `.output`; runtime on `node:24-slim` running `node .output/server/index.mjs`; same base/arch across stages; env `TRACKER_DB_PATH=/data/tracker.db`, `PORT=3000`. `.dockerignore` excludes `node_modules`, `.output`, `*.db*`, `.git`.
- [ ] **Step 2: Build the image** — `docker build -t dashboard:dev frontend`. Expected: success.
- [ ] **Step 3: Run against a seeded DB** — create a throwaway `tracker.db` (copy an existing one or let a local tracker create one), mount it at `/data/tracker.db`, run the container, hit `http://localhost:3000`. Expected: page renders (matches list or the "No matches yet" empty state) — proves the native binding loads in the slim runtime and `fileMustExist` is satisfied.
- [ ] **Step 4: Commit.**

---

### Task 2: Docker-specific tracker config

**Files:**
- Create: `deploy/tracker.toml`

- [ ] **Step 1: Author `deploy/tracker.toml`** per spec §4.3 + §4.4: absolute `database_path = /data/tracker.db`; mainnet `[[sources]]` with absolute `tii_path = /protocols/<name>.tii` for the chosen mainnet set (the five TIIs already in `protocols/`); keep the mainnet `endpoint`; **no** `api_key`; decide and set `[matching] mode` (`best` to mitigate over-matching across multiple protocols, per spec §4.4).
- [ ] **Step 2: Validate the referenced TIIs exist** — confirm each `tii_path` maps to a file present in `protocols/` (so the bind mount resolves). Expected: all present.
- [ ] **Step 3: Commit.**

---

### Task 3: Environment template

**Files:**
- Create: `.env.example`

- [ ] **Step 1: Author `.env.example`** with `DMTR_API_KEY=` (required) and optional `PORT` / `RUST_LOG`, each with a short comment.
- [ ] **Step 2:** Confirm `.env` is git-ignored (add to `.gitignore` if not) so a real key never gets committed.
- [ ] **Step 3: Commit.**

---

### Task 4: docker-compose.yml

**Files:**
- Create: `docker-compose.yml` (repo root)

- [ ] **Step 1: Author the compose** per spec §4.2: service `tracker` (`ghcr.io/tx3-lang/tracker`) with bind mounts `./protocols:/protocols:ro` and `./deploy/tracker.toml:/etc/tracker/tracker.toml:ro`, named volume `tracker-data:/data`, `DMTR_API_KEY` from env, command pointing at the mounted config, and a healthcheck `test -f /data/tracker.db` with enough retries for the first cursor advance; service `dashboard` (`ghcr.io/tx3-lang/dashboard`) with `tracker-data:/data`, `TRACKER_DB_PATH=/data/tracker.db`, `ports: 3000:3000`, `depends_on: tracker (condition: service_healthy)`; declare the `tracker-data` named volume.
- [ ] **Step 2: Validate config** — `docker compose config`. Expected: resolves with no errors; variables interpolate.
- [ ] **Step 3: End-to-end run** — build/pull the tracker image locally, `cp .env.example .env`, set a real `DMTR_API_KEY`, `docker compose up`. Expected: tracker becomes healthy after it creates the DB; dashboard starts only after (no `SQLITE_CANTOPEN`); `http://localhost:3000` shows real mainnet matches (spec D2, D3, B1).
- [ ] **Step 4: Restart/persistence check** — `docker compose down` then `up`; expected: `tracker.db` persists in the named volume and matches survive.
- [ ] **Step 5: Commit.**

---

### Task 5: CI workflow — publish dashboard image to GHCR

**Files:**
- Create: `.github/workflows/docker-dashboard.yml`

- [ ] **Step 1: Author the workflow** per spec §3/§4.1: trigger on tag/release; buildx; GHCR login with workflow token; build `linux/amd64,linux/arm64` from `frontend/`; push to `ghcr.io/tx3-lang/dashboard` with tags = version, SHA, `latest`; `permissions: packages: write`.
- [ ] **Step 2: Validate syntax** — `actionlint .github/workflows/docker-dashboard.yml`. Expected: no errors.
- [ ] **Step 3: Dry-run** — `docker buildx build --platform linux/amd64,linux/arm64 frontend` (no push). Expected: both arches build.
- [ ] **Step 4: Commit.**
- [ ] **Step 5: Post-merge verification (manual):** push a tag, confirm the image publishes and the GHCR package is **public** (spec D1).

---

### Task 6: Docs + demo alignment + secret cleanup

**Files:**
- Modify: `docs/running.md` (add "Running with Docker")
- Modify: `README.md` (point quickstart at the compose; align demo-protocol claim to mainnet)
- Modify: `tracker.toml` (remove the inline `api_key`; align active sources with the mainnet demo)

- [ ] **Step 1: Grep for committed secrets** — `git grep -nE 'api_key\s*=\s*"(utxorpc|dmtr_)'`. Record hits.
- [ ] **Step 2:** Remove the inline `api_key` from `tracker.toml`; re-grep, expected: no hits. (The key is in history and must be **rotated** out-of-band — note in the PR.)
- [ ] **Step 3:** Add a "Running with Docker" section to `docs/running.md` per spec §4.6: prerequisites (Docker + Demeter key), the clone → `.env` → `docker compose up` quickstart, the bind-mount-vs-named-volume model, and troubleshooting (empty list, startup race, WAL must be a local filesystem, where the DB lives).
- [ ] **Step 4:** Align README + `tracker.toml` so the documented demo matches the mainnet compose (spec §4.4 doc cleanup).
- [ ] **Step 5: Verify** the docs by following them from a clean clone (the Task 4 run already exercised the path). Expected: clone-to-running works using only documented steps (spec D5).
- [ ] **Step 6: Commit.**

---

### Task 7: Final verification & PR

- [ ] **Step 1:** Re-run `docker compose config` and a full `docker compose up` smoke; confirm the dashboard shows live matches.
- [ ] **Step 2:** Walk spec acceptance criteria D1–D5; confirm each maps to a completed task (D1 has a manual post-merge step).
- [ ] **Step 3:** Open a PR from `docs/docker-deploy-spec` (or a fresh feature branch) summarising the image, the compose stack, the demo config, and the **key-rotation** action item. Note the dependency on the tracker plan's Task 1 (env fallback) and that the compose pins `ghcr.io/tx3-lang/tracker` once published.

---

## Self-review notes

- **Spec coverage:** §4.1→Task 1+5; §4.2→Task 4; §4.3→Task 2; §4.4→Task 2+6; §4.5→Task 3; §4.6→Task 6; §6/§7 (secret/startup)→Task 4+6; D1 manual step flagged.
- **Cross-repo:** Tasks 2/4 depend on the tracker plan's env fallback (Task 1 there) and published image; called out in the header and Task 7.
- **TDD note:** This subsystem is infra (Dockerfile, compose, CI, docs); verification is build-and-run against the acceptance criteria rather than unit tests.
