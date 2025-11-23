.PHONY: help build run test clean docker-build docker-up docker-down install dev

# Default target
help:
	@echo "LLM Gateway - Available commands:"
	@echo "  make install       - Install all dependencies"
	@echo "  make dev          - Start development servers"
	@echo "  make build        - Build both backend and frontend"
	@echo "  make run          - Run production builds"
	@echo "  make test         - Run all tests"
	@echo "  make clean        - Clean build artifacts"
	@echo "  make docker-build - Build Docker images"
	@echo "  make docker-up    - Start services with Docker Compose"
	@echo "  make docker-down  - Stop Docker services"

# Install dependencies
install:
	@echo "Installing backend dependencies..."
	cd backend && cargo fetch
	@echo "Installing frontend dependencies..."
	cd frontend && bun install
	@echo "Dependencies installed successfully!"

# Development mode
dev:
	@echo "Starting development servers..."
	@echo "Backend will run on http://localhost:3000"
	@echo "Frontend will run on http://localhost:5173"
	@trap 'kill 0' SIGINT; \
	(cd backend && cargo watch -x run) & \
	(cd frontend && bun run dev) & \
	wait

# Build for production
build: build-backend build-frontend

build-backend:
	@echo "Building backend..."
	cd backend && cargo build --release

build-frontend:
	@echo "Building frontend..."
	cd frontend && bun run build

# Run production builds
run:
	@echo "Starting production servers..."
	@trap 'kill 0' SIGINT; \
	(cd backend && ./target/release/llm-gateway) & \
	(cd frontend && bun run preview) & \
	wait

# Run tests
test: test-backend test-frontend

test-backend:
	@echo "Running backend tests..."
	cd backend && cargo test

test-frontend:
	@echo "Running frontend tests..."
	cd frontend && bun test

# Clean build artifacts
clean:
	@echo "Cleaning build artifacts..."
	cd backend && cargo clean
	cd frontend && rm -rf dist node_modules
	@echo "Clean complete!"

# Docker commands
docker-build:
	@echo "Building Docker images..."
	docker-compose build

docker-up:
	@echo "Starting Docker services..."
	docker-compose up -d
	@echo "Services are running!"
	@echo "Frontend: http://localhost"
	@echo "Backend: http://localhost:3000"

docker-down:
	@echo "Stopping Docker services..."
	docker-compose down

docker-logs:
	docker-compose logs -f

# Database migrations (for future use)
migrate:
	@echo "Running database migrations..."
	cd backend && cargo run --bin migrate

# Format code
fmt:
	@echo "Formatting code..."
	cd backend && cargo fmt
	cd frontend && bun run format

# Lint code
lint:
	@echo "Linting code..."
	cd backend && cargo clippy
	cd frontend && bun run lint

# Quick setup for new developers
setup: install
	@echo "Setting up development environment..."
	cp backend/.env.example backend/.env
	@echo "Please edit backend/.env with your API keys"
	@echo "Setup complete! Run 'make dev' to start development servers"
