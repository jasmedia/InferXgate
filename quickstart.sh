#!/bin/bash

# LLM Gateway Quick Start Script
# This script helps you quickly set up and run the LLM Gateway

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${GREEN}🚀 LLM Gateway Quick Start${NC}"
echo "================================"

# Function to check if command exists
command_exists() {
    command -v "$1" >/dev/null 2>&1
}

# Check prerequisites
echo -e "\n${YELLOW}Checking prerequisites...${NC}"

if ! command_exists cargo; then
    echo -e "${RED}❌ Rust is not installed${NC}"
    echo "Please install Rust from https://rustup.rs/"
    exit 1
fi
echo -e "${GREEN}✅ Rust is installed${NC}"

if ! command_exists bun; then
    echo -e "${RED}❌ Bun is not installed${NC}"
    echo "Please install Bun from https://bun.sh/"
    exit 1
fi
echo -e "${GREEN}✅ Bun is installed${NC}"

# Create .env file if it doesn't exist
if [ ! -f backend/.env ]; then
    echo -e "\n${YELLOW}Creating .env file...${NC}"
    cp backend/.env.example backend/.env
    echo -e "${GREEN}✅ .env file created${NC}"
    echo -e "${YELLOW}⚠️  Please edit backend/.env and add your API keys${NC}"
fi

# Install dependencies
echo -e "\n${YELLOW}Installing dependencies...${NC}"

echo "Installing backend dependencies..."
cd backend
cargo fetch
cd ..

echo "Installing frontend dependencies..."
cd frontend
bun install
cd ..

echo -e "${GREEN}✅ Dependencies installed${NC}"

# Ask user what to do next
echo -e "\n${GREEN}Setup complete! What would you like to do?${NC}"
echo "1) Start development servers (recommended for development)"
echo "2) Build for production"
echo "3) Run with Docker"
echo "4) Exit"

read -p "Enter your choice (1-4): " choice

case $choice in
    1)
        echo -e "\n${GREEN}Starting development servers...${NC}"
        echo "Backend will run on http://localhost:3000"
        echo "Frontend will run on http://localhost:5173"
        echo -e "${YELLOW}Press Ctrl+C to stop${NC}\n"

        # Start both servers
        trap 'kill 0' SIGINT
        (cd backend && cargo run) &
        (cd frontend && bun run dev) &
        wait
        ;;

    2)
        echo -e "\n${GREEN}Building for production...${NC}"

        echo "Building backend..."
        cd backend
        cargo build --release
        cd ..

        echo "Building frontend..."
        cd frontend
        bun run build
        cd ..

        echo -e "${GREEN}✅ Build complete!${NC}"
        echo "To run: "
        echo "  Backend: ./backend/target/release/llm-gateway"
        echo "  Frontend: cd frontend && bun run preview"
        ;;

    3)
        if ! command_exists docker; then
            echo -e "${RED}❌ Docker is not installed${NC}"
            echo "Please install Docker from https://docker.com/"
            exit 1
        fi

        echo -e "\n${GREEN}Starting with Docker...${NC}"
        docker-compose up -d

        echo -e "${GREEN}✅ Services started!${NC}"
        echo "Frontend: http://localhost"
        echo "Backend: http://localhost:3000"
        echo "To stop: docker-compose down"
        ;;

    4)
        echo -e "${GREEN}Setup complete! Run this script again to start the servers.${NC}"
        exit 0
        ;;

    *)
        echo -e "${RED}Invalid choice${NC}"
        exit 1
        ;;
esac
