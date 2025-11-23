# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

LLM Gateway is a high-performance Rust backend with React frontend that provides a unified OpenAI-compatible API interface for multiple LLM providers (Anthropic, Google Gemini, OpenAI, Azure OpenAI, and more).

## Quick Start

```bash
make setup              # Initial setup
make dev                # Start development servers (backend + frontend)
make test               # Run all tests
make lint               # Code quality checks
```

## Git Workflow

**CRITICAL: ALWAYS work on feature branches, NEVER commit directly to main.**

### Branch Naming Convention
- Features: `feature/add-openai-provider`
- Bug fixes: `fix/streaming-timeout`
- Refactoring: `refactor/provider-abstraction`
- Documentation: `docs/update-readme`

## Supported Providers

- **Anthropic** - Claude models
- **Google Gemini** - Gemini models
- **OpenAI** - GPT models
- **Azure OpenAI** - Azure-deployed OpenAI models

## Additional Instructions

See `.claude/instructions.md` for detailed development instructions including:
- MCP server usage (Context7, Chrome DevTools)
- Complete development commands
- Architecture details
- Provider implementation patterns
- Configuration and debugging
