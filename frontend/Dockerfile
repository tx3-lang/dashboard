# Stage 1: builder — full node:24 (Debian) for native compilation
FROM node:24 AS builder

WORKDIR /app

# Install build toolchain for better-sqlite3 native addon
RUN apt-get update && apt-get install -y --no-install-recommends \
    python3 \
    make \
    g++ \
 && rm -rf /var/lib/apt/lists/*

# Enable corepack so pnpm version is driven by packageManager in package.json
RUN corepack enable && corepack prepare --activate

# Copy manifests first for layer caching
COPY package.json pnpm-lock.yaml pnpm-workspace.yaml ./

# Install dependencies (frozen lockfile; pnpm-workspace.yaml enables better-sqlite3 build)
RUN pnpm install --frozen-lockfile

# Copy source and build
COPY . .
RUN pnpm build

# Stage 2: runtime — node:24-slim (same Debian family / arch → ABI-compatible binding)
FROM node:24-slim AS runtime

WORKDIR /app

ENV NODE_ENV=production
ENV TRACKER_DB_PATH=/data/tracker.db
ENV PORT=3000

EXPOSE 3000

# Copy the Nitro output bundle from the builder
COPY --from=builder /app/.output ./.output

# Nitro traces better-sqlite3 (including its native .node binding) into
# .output/server/node_modules — confirmed present at:
#   .output/server/node_modules/better-sqlite3/build/Release/better_sqlite3.node
# We also copy node_modules/better-sqlite3 as a belt-and-suspenders fallback in
# case the trace path changes across Nitro versions. Both are Linux binaries from
# the builder stage (ABI-compatible with the slim runtime — same Debian family).
COPY --from=builder /app/node_modules/better-sqlite3 ./node_modules/better-sqlite3

CMD ["node", ".output/server/index.mjs"]
