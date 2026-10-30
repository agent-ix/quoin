FROM node:24.12-slim

ENV PNPM_HOME="/pnpm"
ENV PATH="$PNPM_HOME:$PATH"
ENV NPM_CONFIG_STORE_DIR=/tmp/pnpm-store
# Enable pnpm via corepack which is preferred for Node 22+
RUN corepack enable

WORKDIR /app

# Copy package config files for caching
COPY package.json pnpm-lock.yaml* .prettierrc .prettierignore eslint.config.js vite.config.ts tsconfig.json tsconfig.eslint.json ./

# Install dependencies (including dev deps)
# This layer will be cached unless package files change
RUN pnpm install --frozen-lockfile

# Copy the rest of the code
COPY . .

# Ensure typescript presence
# RUN pnpm list typescript

CMD ["pnpm", "test"]
