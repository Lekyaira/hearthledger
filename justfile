# Default recipe - 'just' with no arguments lists available recipes.
default:
  @just --list 

# Run the backend server 
run-backend:
  cd backend && cargo run

# Run the frontend dev server
run-frontend:
  cd frontend && npm run dev

# Run the backend plus SvelteKit dev server
dev:
  mprocs

# Build everything for production
build:
  cd backend && cargo build --release
  cd frontend && npm run build 

# Type-check the SvelteKit app
check: 
  cd frontend && npm run check 

# Format everything 
fmt:
  cd backend && cargo fmt 
  cd frontend && npx prettier --write 

# Lint Rust 
lint:
  cd backend && cargo clippy -- -D warnings 

# Clean all build artifacts 
clean:
  rm -rf backend/target frontend/.svelte-kit frontend/build frontend/node_modules

# Run all SvelteKit tests in watch mode.
test-frontend:
    cd frontend && npm run test

# Run all SvelteKit tests once and exit (for CI / pre-commit).
test-frontend-run:
    cd frontend && npm run test:unit

# Run Rust tests for the backend 
test-backend:
  cd backend && cargo test 

# Run both Rust and SvelteKit tests.
test: test-backend test-frontend-run
