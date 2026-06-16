# Stage 1: builder — full node:24 (Debian) for native compilation
FROM node:24 AS builder

WORKDIR /app

# Build toolchain for the better-sqlite3 native addon.
RUN apt-get update && apt-get install -y --no-install-recommends \
    python3 \
    make \
    g++ \
 && rm -rf /var/lib/apt/lists/*

# Enable corepack. The pnpm version is driven by the packageManager field in
# package.json and resolved on the first pnpm invocation below.
RUN corepack enable

# Copy manifests first for layer caching. package.json carries the
# pnpm.onlyBuiltDependencies allowlist that lets better-sqlite3 run its native
# build during install; .npmrc carries repo install settings.
COPY package.json pnpm-lock.yaml .npmrc ./

# Install dependencies (frozen lockfile).
RUN pnpm install --frozen-lockfile

# Copy source and build the Nitro bundle.
COPY . .
RUN pnpm build

# Stage 2: runtime — node:24-slim (same Debian family / arch → ABI-compatible binding)
FROM node:24-slim AS runtime

WORKDIR /app

ENV NODE_ENV=production
ENV TRACKER_DB_PATH=/data/tracker.db
ENV PORT=3000

EXPOSE 3000

# Copy the Nitro output bundle. Nitro traces better-sqlite3 — including its
# native .node binding — into .output/server/node_modules, so the bundle is
# self-contained; no separate node_modules copy is needed.
COPY --from=builder /app/.output ./.output

CMD ["node", ".output/server/index.mjs"]
