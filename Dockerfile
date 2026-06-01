FROM rust:1-bookworm AS backend-builder

WORKDIR /app/backend
COPY backend/Cargo.toml backend/Cargo.lock ./
COPY backend/src ./src
COPY backend/migrations ./migrations
RUN cargo build --release --locked

FROM node:22-bookworm-slim AS frontend-builder

WORKDIR /app/frontend
COPY frontend/package.json frontend/package-lock.json ./
RUN npm ci
COPY frontend ./
RUN npm run build

FROM node:22-bookworm-slim

WORKDIR /app
COPY --from=backend-builder /app/backend/target/release/backend /app/backend
COPY --from=frontend-builder /app/frontend/build /app/frontend
COPY docker/entrypoint.sh /app/entrypoint.sh

RUN mkdir -p /data \
	&& chown node:node /data \
	&& chmod +x /app/entrypoint.sh

USER node

ENV DATABASE_URL=sqlite:///data/hearthledger.db \
	BACKEND_ORIGIN=http://127.0.0.1:3001 \
	BIND_ADDR=127.0.0.1:3001 \
	HOST=0.0.0.0 \
	PORT=3000

EXPOSE 3000

ENTRYPOINT ["/app/entrypoint.sh"]
