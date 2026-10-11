FROM node:22-bookworm-slim AS deps
WORKDIR /app
COPY package.json pnpm-lock.yaml pnpm-workspace.yaml ./
RUN corepack enable && pnpm install --frozen-lockfile

FROM node:22-bookworm-slim AS build
WORKDIR /app
# next.config.ts (images.remotePatterns), robots.ts, sitemap.ts and
# metadataBase read DOMAIN at *build* time — standalone output saves the
# result, so a runtime-only DOMAIN is too late (Phase 19).
ARG DOMAIN
RUN test -n "$DOMAIN" || (echo "DOMAIN build arg is required (set the DOMAIN repo variable)" >&2 && exit 1)
ENV DOMAIN=$DOMAIN
COPY --from=deps /app/node_modules ./node_modules
COPY . .
RUN corepack enable && pnpm build

FROM node:22-bookworm-slim
WORKDIR /app
ENV NODE_ENV=production
ENV PORT=3000
ENV HOSTNAME=0.0.0.0
COPY --from=build --chown=node:node /app/.next/standalone ./
COPY --from=build --chown=node:node /app/.next/static ./.next/static
COPY --from=build --chown=node:node /app/public ./public
# Pre-created and node-owned so the web_cache volume mounted here
# (docker-compose.prod.yml) inherits writable ownership on first creation.
RUN mkdir -p .next/cache && chown -R node:node .next/cache
USER node
EXPOSE 3000
CMD ["node", "server.js"]
