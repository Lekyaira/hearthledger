#!/bin/sh
set -eu

: "${DATABASE_URL:=sqlite:///data/hearthledger.db}"
: "${BACKEND_ORIGIN:=http://127.0.0.1:3001}"
: "${BIND_ADDR:=127.0.0.1:3001}"
: "${HOST:=0.0.0.0}"
: "${PORT:=3000}"

export DATABASE_URL BACKEND_ORIGIN BIND_ADDR HOST PORT

cleanup() {
	kill "${backend_pid:-}" "${frontend_pid:-}" 2>/dev/null || true
	wait "${backend_pid:-}" "${frontend_pid:-}" 2>/dev/null || true
}

trap cleanup EXIT INT TERM

/app/backend &
backend_pid=$!

node /app/frontend/index.js &
frontend_pid=$!

while kill -0 "$backend_pid" 2>/dev/null && kill -0 "$frontend_pid" 2>/dev/null; do
	sleep 1
done

exit 1
