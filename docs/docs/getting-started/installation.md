---
sidebar_position: 1
---

# Installation

This guide will help you set up Smally locally for development or production use.

## Prerequisites

Before installing Smally, ensure you have:

- **Rust** 1.70+ ([Install Rust](https://rustup.rs/))
- **PostgreSQL** 14+ ([Install PostgreSQL](https://www.postgresql.org/download/))
- **Redis** 6+ ([Install Redis](https://redis.io/download))
- **ONNX Model** (all-MiniLM-L6-v2)

## Quick Setup with Docker

The fastest way to get started is using Docker Compose:

```bash
git clone https://github.com/your-org/smally.git
cd smally
docker-compose up -d
```

This will start:
- PostgreSQL on port 5432
- Redis on port 6379
- Smally API on port 8000

## Manual Installation

### 1. Clone the Repository

```bash
git clone https://github.com/your-org/smally.git
cd smally/api
```

### 2. Set Up Database

Start PostgreSQL and create the database:

```bash
# Start PostgreSQL (macOS with Homebrew)
brew services start postgresql

# Create database
createdb smally_dev

# Run migrations
./scripts/init_db.sh
```

### 3. Set Up Redis

```bash
# Start Redis (macOS with Homebrew)
brew services start redis

# Verify Redis is running
redis-cli ping
# Should respond: PONG
```

### 4. Download ONNX Model

```bash
# Download the sentence-transformers model
mkdir -p models/all-MiniLM-L6-v2
cd models/all-MiniLM-L6-v2

# Download model files
# (Instructions for downloading from Hugging Face)
```

### 5. Configure Environment

Create a `.env` file in the `api` directory:

```bash
# Database
DATABASE_URL=postgresql://user:password@localhost:5432/smally_dev

# Redis
REDIS_URL=redis://localhost:6379

# Model
MODEL_PATH=./models/all-MiniLM-L6-v2
MODEL_NAME=sentence-transformers/all-MiniLM-L6-v2

# Server
HOST=127.0.0.1
PORT=8000

# Environment
RUST_ENV=development
```

### 6. Build and Run

```bash
# Build the project
cargo build --release

# Run the API server
cargo run --release
```

The API should now be running at `http://localhost:8000`!

## Verify Installation

Test that everything is working:

```bash
# Health check
curl http://localhost:8000/health

# Should respond with:
# {
#   "status": "ok",
#   "version": "0.1.0",
#   "build": { ... }
# }
```

## Next Steps

- [Quick Start](/docs/getting-started/quickstart) - Create your first API key and make requests
- [Authentication](/docs/getting-started/authentication) - Learn about API keys and tokens
