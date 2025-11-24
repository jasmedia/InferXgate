# Contributing to InferXgate

Thank you for your interest in contributing to InferXgate! This document provides guidelines and instructions for contributing.

## Getting Started

### Prerequisites

- Rust (latest stable)
- Node.js 18+ and Bun
- PostgreSQL 14+
- Redis (optional, for caching)
- Docker (optional)

### Development Setup

1. Clone the repository:
   ```bash
   git clone https://github.com/jasmedia/InferXgate.git
   cd InferXgate
   ```

2. Run initial setup:
   ```bash
   make setup
   ```

3. Start development servers:
   ```bash
   make dev
   ```

## Git Workflow

**Important: Never commit directly to main. Always use feature branches.**

### Branch Naming Convention

- `feature/` - New features (e.g., `feature/add-openai-provider`)
- `fix/` - Bug fixes (e.g., `fix/streaming-timeout`)
- `refactor/` - Code refactoring (e.g., `refactor/provider-abstraction`)
- `docs/` - Documentation updates (e.g., `docs/update-readme`)

### Pull Request Process

1. Create a feature branch from `main`
2. Make your changes with clear, atomic commits
3. Ensure all tests pass: `make test`
4. Ensure linting passes: `make lint`
5. Update documentation if needed
6. Submit a pull request with a clear description

## Code Standards

### Rust (Backend)

- Follow Rust idioms and best practices
- Use `cargo fmt` for formatting
- Run `cargo clippy` and address warnings
- Write tests for new functionality
- Document public APIs with doc comments

### TypeScript/React (Frontend)

- Use TypeScript strict mode
- Follow React best practices and hooks patterns
- Use Biome for linting and formatting
- Avoid `any` types - use proper interfaces
- Add accessibility attributes to interactive elements

### General Guidelines

- Write clear, self-documenting code
- Keep functions focused and small
- Add comments for complex logic
- Follow existing code patterns in the project

## Testing

Run all tests:
```bash
make test
```

Run backend tests only:
```bash
cd backend && cargo test
```

Run frontend tests only:
```bash
cd frontend && bun test
```

## Reporting Issues

When reporting issues, please include:

1. Clear description of the problem
2. Steps to reproduce
3. Expected vs actual behavior
4. Environment details (OS, versions)
5. Relevant logs or error messages

## Feature Requests

For feature requests:

1. Check existing issues to avoid duplicates
2. Clearly describe the use case
3. Explain the proposed solution
4. Consider backwards compatibility

## Code of Conduct

Please be respectful and constructive in all interactions. We are committed to providing a welcoming and inclusive environment for all contributors.

## License

By contributing to InferXgate, you agree that your contributions will be licensed under the AGPL-3.0 license.

## Questions?

If you have questions, feel free to:
- Open a discussion on GitHub
- Contact the maintainers at support@inferxgate.com

Thank you for contributing!
