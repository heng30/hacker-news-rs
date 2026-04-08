# Hacker News RSS Generator with AI Summaries

A Rust-based web service that fetches top stories from Hacker News and generates comprehensive summaries using Large Language Models (LLM).

## Features

- Fetches top stories from Hacker News API
- **Keyword search via Algolia HN Search API** - search for specific topics (e.g., "rust", "go", "linux")
- Generates 200-300 word English or 400-500 character Chinese summaries using LLM
- SQLite database for persistent storage
- RESTful API for managing episodes and stories
- SSE streaming for real-time progress updates during fetch
- Configurable story count, LLM model, and API settings
- Episode-based organization (daily snapshots)
- Tag badges showing story source (top stories vs keyword search)
- Dark mode support in web UI

## Quick Start

### Prerequisites

- Rust 1.75+ (for building from source)
- An LLM API key (OpenAI, DeepSeek, or any OpenAI-compatible API)

### 1. Clone and Build

```bash
git clone https://github.com/heng30/hacker-news-rs.git
cd hacker-news-rs
cargo build --release
```

### 2. Configure Environment

Create a `.env` file in the project root:

```bash
# Required: LLM API key
HACKER_NEWS_OPENAI_API_KEY=your_api_key_here

# Optional: LLM API settings (defaults shown)
HACKER_NEWS_OPENAI_BASE_URL=https://api.deepseek.com/v1
HACKER_NEWS_MODEL=deepseek-chat
HACKER_NEWS_STORY_COUNT=30

# Optional: Enable keyword search
HACKER_NEWS_SEARCH_KEYWORDS=rust,go,linux

# Optional: Server settings
HACKER_NEWS_PORT=3000
HACKER_NEWS_DATABASE_URL=sqlite:data/hacker_news.db?mode=rwc
```

See [`.env.example`](.env.example) for all available options.

### 3. Run

```bash
cargo run --release
```

The server will start at `http://localhost:3000`. Open this URL in your browser to access the web UI.

## Deployment

### Option 1: Local Development

Build and run directly:

```bash
cargo build --release
cargo run --release
```

The executable is at `target/release/hacker-news-rs`.

### Option 2: Bundle for Deployment

Create a self-contained distribution directory:

```bash
./bundle.sh [output_dir]
```

This creates a `dist/` directory containing:
```
dist/
├── hacker-news-rs       # Executable
├── .env                 # Configuration (copied from project root)
└── src/static/
    └── index.html       # Web UI
```

To run the bundled version:
```bash
cd dist
./hacker-news-rs
```

### Option 3: Install to User Directory

Install to `$HOME/.local/bin/`:

```bash
./install.sh
```

This runs `bundle.sh` and copies the result to:
- `$HOME/.local/bin/hacker-news` - Executable
- `$HOME/.local/bin/hacker-news-dist/` - Distribution files

Run from anywhere (if `$HOME/.local/bin` is in your PATH):
```bash
cd ~/.local/bin/hacker-news-dist
./hacker-news
```

### Option 4: Systemd Service (Linux)

Create a systemd service for automatic startup:

```bash
# 1. Bundle and copy to system location
./bundle.sh
sudo cp -r dist /opt/hacker-news

# 2. Create systemd service file
sudo tee /etc/systemd/system/hacker-news.service << 'EOF'
[Unit]
Description=Hacker News RSS Generator
After=network.target

[Service]
Type=simple
User=www-data
WorkingDirectory=/opt/hacker-news
ExecStart=/opt/hacker-news/hacker-news-rs
Restart=on-failure
RestartSec=10

[Install]
WantedBy=multi-user.target
EOF

# 3. Enable and start
sudo systemctl daemon-reload
sudo systemctl enable hacker-news
sudo systemctl start hacker-news
```

### Option 5: Docker (Alternative)

Create a Dockerfile:

```dockerfile
FROM rust:1.75 as builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
WORKDIR /app
COPY --from=builder /app/target/release/hacker-news-rs /app/
COPY --from=builder /app/src/static /app/src/static
COPY --from=builder /app/.env.example /app/.env.example
CMD ["./hacker-news-rs"]
```

Build and run:
```bash
docker build -t hacker-news-rs .
docker run -d -p 3000:3000 -v $(pwd)/data:/app/data hacker-news-rs
```

## Usage

### Web UI

Open `http://localhost:3000` in your browser. The UI provides:

- **Story List**: View fetched stories with AI summaries
- **Calendar Navigation**: Browse episodes by date
- **Fetch Button**: Trigger new story fetch with progress indicator
- **Language Toggle**: Switch between English/Chinese summaries
- **Dark Mode**: Toggle in the navigation bar
- **Tag Badges**: Visual indicators for story source (top stories vs keyword search)

### API Endpoints

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/episode/latest` | Get latest episode with stories |
| GET | `/api/episode/{date}` | Get episode by specific date (YYYY-MM-DD) |
| DELETE | `/api/episode/{date}` | Delete episode by date |
| DELETE | `/api/episode/{date}/stories` | Delete stories for a specific episode |
| POST | `/api/fetch` | Fetch new stories (returns JSON) |
| GET | `/api/fetch/stream` | Fetch with SSE streaming progress |
| GET | `/api/stories` | Get all stored stories |
| DELETE | `/api/stories` | Delete all stories |
| DELETE | `/api/stories/read` | Delete read stories by hn_ids |
| PUT | `/api/story/{hn_id}/regenerate` | Regenerate summary for a story |
| GET | `/api/episodes` | Get list of all episodes |
| GET | `/api/config` | Get current configuration |
| PUT | `/api/config` | Update configuration |

### Fetch Stories via API

**Simple fetch (JSON response):**
```bash
curl -X POST http://localhost:3000/api/fetch \
  -H "Content-Type: application/json" \
  -d '{"lang": "zh"}'
```

**Streaming fetch (SSE, real-time progress):**
```bash
curl -N http://localhost:3000/api/fetch/stream?lang=en
```

SSE events:
- `story_added`: New story saved (before summary)
- `summary_done`: Summary generated for a story
- `summary_error`: Summary generation failed
- `done`: All stories processed

### Get Latest Episode

```bash
curl http://localhost:3000/api/episode/latest
```

Response:
```json
{
  "success": true,
  "data": {
    "episode": {
      "id": 1,
      "date": "2024-01-15",
      "created_at": "2024-01-15T10:30:00Z",
      "updated_at": "2024-01-15T10:30:00Z"
    },
    "stories": [
      {
        "hn_id": 12345,
        "title": "Story Title",
        "url": "https://example.com",
        "by": "author",
        "score": 100,
        "summary_zh": "中文摘要...",
        "tag": "top"
      }
    ]
  }
}
```

## Hacker News API Sources

This project uses two APIs:

### Official HN Firebase API (Top Stories)

| Endpoint | Description |
|----------|-------------|
| `GET https://hacker-news.firebaseio.com/v0/topstories.json` | Returns array of up to 500 top story IDs |
| `GET https://hacker-news.firebaseio.com/v0/item/{id}.json` | Returns story details (title, url, by, score, time) |

### Algolia HN Search API (Keyword Search)

| Endpoint | Description |
|----------|-------------|
| `GET https://hn.algolia.com/api/v1/search_by_date?query={keyword}&tags=story&hitsPerPage={limit}` | Search newest stories by keyword |

Reference: https://hn.algolia.com/api

### Story Tag Field

The `tag` field indicates the source:
- `"top"` - from top stories (hidden in UI)
- keyword (e.g., `"rust"`, `"go"`) - from keyword search (shown as colored badge)

## Summary Generation Flow

```
HN API → Story Metadata → Fetch URL Content → Extract Text → LLM → Summary → SQLite → Frontend
```

1. **Content Fetching**: Fetch HTML from story URL (30s timeout), extract plain text using `scraper` library
2. **Content Limit**: Truncate to 32,000 characters if too long
3. **Fallback**: If content fetch fails, LLM uses title and URL only
4. **LLM Prompt**: Title + content sent to LLM with language-specific instructions
5. **Parallel Processing**: SSE endpoint generates summaries in batches of 3 concurrently

## Configuration

All configuration via environment variables (prefix: `HACKER_NEWS_`):

| Variable | Description | Default |
|----------|-------------|---------|
| `HACKER_NEWS_OPENAI_API_KEY` | LLM API key | **(required)** |
| `HACKER_NEWS_OPENAI_BASE_URL` | LLM API base URL | `https://api.deepseek.com/v1` |
| `HACKER_NEWS_MODEL` | LLM model name | `deepseek-chat` |
| `HACKER_NEWS_STORY_COUNT` | Stories per fetch | `30` |
| `HACKER_NEWS_PORT` | Server port | `3000` |
| `HACKER_NEWS_DATABASE_URL` | SQLite database URL | `sqlite:data/hacker_news.db?mode=rwc` |
| `HACKER_NEWS_AUTO_UPDATE_INTERVAL` | Auto-update interval (minutes) | `0` (disabled) |
| `HACKER_NEWS_SEARCH_KEYWORDS` | Keywords for Algolia search | (disabled if empty) |
| `HACKER_NEWS_SOCKS5` | SOCKS5 proxy | (disabled if empty) |

### LLM API Compatibility

Supports any OpenAI-compatible API:

| Provider | Base URL | Model Examples |
|----------|----------|----------------|
| OpenAI | `https://api.openai.com/v1` | `gpt-4o`, `gpt-4o-mini` |
| DeepSeek | `https://api.deepseek.com/v1` | `deepseek-chat` |
| Alibaba Qwen | `https://dashscope.aliyuncs.com/compatible-mode/v1` | `qwen-plus`, `qwen-turbo` |
| Ollama (local) | `http://localhost:11434/v1` | `llama3`, `mistral` |

## Project Structure

```
hacker-news-rs/
├── src/
│   ├── main.rs              # Entry point, Axum server setup
│   ├── config.rs            # Environment variable helpers
│   ├── hn/
│   │   └── api.rs           # HN Firebase + Algolia client
│   ├── llm/
│   │   └── client.rs        # LLM summarization wrapper
│   ├── db/
│   │   ├── models.rs        # Episode, Story, Config structs
│   │   └── mod.rs           # SQLite operations (sqlx)
│   ├── fetcher/
│   │   └── content.rs       # URL fetch + HTML→Markdown
│   └── routes/
│       ├── episode.rs       # Story/episode API routes + SSE
│       └── config.rs        # Configuration API routes
│   └── static/
│       └── index.html       # Web UI (single-page app)
├── lib/
│   └── bot/                 # LLM streaming chat library
├── bundle.sh                # Create deployment bundle
├── install.sh               # Install to ~/.local/bin
├── .env.example             # Configuration template
└── Cargo.toml               # Workspace config
```

## License

MIT License - see [LICENSE](LICENSE) for details.