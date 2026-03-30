# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build and Run Commands

```bash
# Build
cargo build --release

# Run (requires .env with HACKER_NEWS_OPENAI_API_KEY)
cargo run --release

# Run tests (no test suite currently exists)
cargo test

# Check without building
cargo check
```

## Architecture Overview

This is a Rust web service that fetches Hacker News stories and generates AI summaries using LLM APIs. It uses a workspace structure with two crates:

- **Main crate** (`hacker-news-rs`): Axum web server with REST API and SSE streaming
- **`lib/bot`**: Reusable LLM chat library supporting OpenAI-compatible streaming APIs

### Data Flow

```
HN API → Story metadata → URL content fetch → HTML→Markdown → LLM → Summary → SQLite → REST API → Frontend
```

### Key Modules

- `src/hn/api.rs`: HN Firebase API + Algolia search API client
- `src/fetcher/content.rs`: URL content fetching with HTML→Markdown conversion (htmd library)
- `src/llm/client.rs`: LLM summarization wrapper using the `bot` library
- `src/db/`: SQLite operations with sqlx (models + CRUD)
- `src/routes/`: Axum routes (episode.rs for stories, config.rs for settings)

### Configuration Pattern

Environment variables (prefix `HACKER_NEWS_`) **take precedence** over database-stored config. This is critical for sensitive values:
- `HACKER_NEWS_OPENAI_API_KEY` - never stored in DB, always from env
- `HACKER_NEWS_OPENAI_BASE_URL`, `HACKER_NEWS_MODEL`, `HACKER_NEWS_STORY_COUNT` - env overrides DB

The `db::get_config_with_env_overrides()` function applies this hierarchy.

### Episode-Based Organization

Stories are grouped by date into "episodes". Each fetch creates/updates an episode for the current day. Stories have a `tag` field indicating source (`"top"` for top stories, or keyword like `"rust"` for Algolia search results).

### SSE Streaming Endpoint

`/api/fetch/stream` provides real-time updates during story fetching:
- Events: `story_added`, `summary_done`, `summary_error`, `done`
- Stories are saved immediately without summaries, then summaries are generated in parallel batches (3 concurrent)

## Key Dependencies

- `axum` + `tower-http`: Web framework with CORS and static file serving
- `sqlx` with SQLite: Async database
- `reqwest`: HTTP client for HN/Algolia/LLM APIs
- `htmd`: HTML to Markdown conversion
- `bot` (local): LLM streaming chat abstraction