# Hacker News RSS Generator with AI Summaries

A Rust-based web service that fetches top stories from Hacker News and generates comprehensive Chinese summaries using Large Language Models (LLM).

## Features

- Fetches top stories from Hacker News API
- Generates 500-600 character Chinese summaries using LLM
- SQLite database for persistent storage
- RESTful API for managing episodes and stories
- Configurable story count, LLM model, and API settings
- Episode-based organization (daily snapshots)

## API Endpoints

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/episode/latest` | Get latest episode with stories |
| GET | `/api/episode/{date}` | Get episode by specific date |
| DELETE | `/api/episode/{date}` | Delete episode by date |
| DELETE | `/api/episode/{date}/stories` | Delete stories for a specific episode |
| POST | `/api/fetch` | Fetch new stories from Hacker News |
| GET | `/api/stories` | Get all stored stories |
| DELETE | `/api/stories` | Delete all stories |
| DELETE | `/api/stories/read` | Delete read stories by hn_ids |
| PUT | `/api/story/{hn_id}/regenerate` | Regenerate summary for a story |
| GET | `/api/episodes` | Get list of all episodes |

## Configuration

Configure via environment variables (prefix: `HACKER_NEW_`):

| Variable | Description | Default |
|----------|-------------|---------|
| `HACKER_NEW_DATABASE_URL` | SQLite database URL | `sqlite:data/hacker_news.db?mode=rwc` |
| `HACKER_NEW_PORT` | Server port | `3000` |
| `HACKER_NEW_OPENAI_API_KEY` | LLM API key | (required for summaries) |
| `HACKER_NEW_OPENAI_BASE_URL` | LLM API base URL | `https://api.deepseek.com/v1` |
| `HACKER_NEW_MODEL` | LLM model name | `deepseek-chat` |
| `HACKER_NEW_STORY_COUNT` | Number of stories per fetch | `10` |
| `HACKER_NEW_AUTO_UPDATE_INTERVAL` | Auto-update interval (minutes) | `0` (disabled) |

## Quick Start

1. Clone the repository:
   ```bash
   git clone https://github.com/your-username/hacker-new-rs.git
   cd hacker-new-rs
   ```

2. Create a `.env` file:
   ```bash
   HACKER_NEW_OPENAI_API_KEY=your_api_key_here
   HACKER_NEW_OPENAI_BASE_URL=https://api.deepseek.com/v1
   HACKER_NEW_MODEL=deepseek-chat
   ```

3. Build and run:
   ```bash
   cargo build --release
   cargo run --release
   ```

4. Access the API at `http://localhost:3000`

## Project Structure

```
hacker-new-rs/
├── src/
│   ├── main.rs          # Application entry point
│   ├── config.rs        # Environment configuration
│   ├── hn/
│   │   ├── api.rs       # Hacker News API client
│   │   └── mod.rs
│   ├── llm/
│   │   ├── client.rs    # LLM client for summaries
│   │   └── mod.rs
│   ├── db/
│   │   ├── models.rs    # Database models
│   │   └── mod.rs       # Database operations
│   └── routes/
│       ├── episode.rs   # Episode/story routes
│       ├── config.rs    # Configuration routes
│       └── mod.rs
├── lib/
│   └── bot/             # LLM chat library
└── Cargo.toml
```

## License

MIT License - see [LICENSE](LICENSE) for details.
