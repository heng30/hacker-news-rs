# Hacker News RSS Generator with AI Summaries

A Rust-based web service that fetches top stories from Hacker News and generates comprehensive Chinese summaries using Large Language Models (LLM).

## Features

- Fetches top stories from Hacker News API
- Generates 500-600 character Chinese summaries using LLM
- SQLite database for persistent storage
- RESTful API for managing episodes and stories
- Configurable story count, LLM model, and API settings
- Episode-based organization (daily snapshots)

## Hacker News API

This project uses the official Hacker News Firebase API:

| Endpoint | Description |
|----------|-------------|
| `GET https://hacker-news.firebaseio.com/v0/topstories.json` | Returns array of up to 500 top story IDs |
| `GET https://hacker-news.firebaseio.com/v0/item/{id}.json` | Returns story details (title, url, by, score, time) |

### Story Data Structure

```json
{
  "id": 12345,
  "title": "Story Title",
  "url": "https://example.com",
  "by": "author_username",
  "score": 100,
  "time": 1234567890
}
```

API reference: https://github.com/HackerNews/API

## Summary Generation Flow

The LLM generates summaries based on story metadata and fetched article content:

```
Hacker News API → Story Metadata (title, url) → Fetch HTML → Convert to Markdown → LLM → Summary (Markdown) → Frontend Rendering
```

### Content Fetching

The system fetches the article HTML content from the story URL and converts it to Markdown using the `htmd` library:

1. Fetch HTML from URL (30 second timeout)
2. Convert HTML to Markdown (using [htmd](https://crates.io/crates/htmd))
3. Truncate to 32,000 characters if too long

If content fetching fails (timeout, invalid URL, conversion error), the LLM falls back to using only the title and URL.

### LLM Input Format

The prompt sent to the LLM includes the title and fetched content (or fallback to URL):

```
Title: {story_title}

Content:
{markdown_content}
```

The LLM generates a 500-600 character Chinese summary (or 200-300 word English summary) based on the actual article content when available.

### Markdown Rendering

Summaries support Markdown formatting (bold, lists, headers, code). The frontend renders these using [marked.js](https://marked.js.org/) with GitHub Flavored Markdown (GFM) support. Common Markdown elements in summaries:

- **Bold text** for emphasis
- Bullet lists for key points
- Headers for section organization
- Code blocks for technical terms

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

Configure via environment variables (prefix: `HACKER_NEWS_`):

| Variable | Description | Default |
|----------|-------------|---------|
| `HACKER_NEWS_DATABASE_URL` | SQLite database URL | `sqlite:data/hacker_news.db?mode=rwc` |
| `HACKER_NEWS_PORT` | Server port | `3000` |
| `HACKER_NEWS_OPENAI_API_KEY` | LLM API key | (required for summaries) |
| `HACKER_NEWS_OPENAI_BASE_URL` | LLM API base URL | `https://api.deepseek.com/v1` |
| `HACKER_NEWS_MODEL` | LLM model name | `deepseek-chat` |
| `HACKER_NEWS_STORY_COUNT` | Number of stories per fetch | `10` |
| `HACKER_NEWS_AUTO_UPDATE_INTERVAL` | Auto-update interval (minutes) | `0` (disabled) |

## Quick Start

1. Clone the repository:
   ```bash
   git clone https://github.com/your-username/hacker-new-rs.git
   cd hacker-new-rs
   ```

2. Create a `.env` file:
   ```bash
   HACKER_NEWS_OPENAI_API_KEY=your_api_key_here
   HACKER_NEWS_OPENAI_BASE_URL=https://api.deepseek.com/v1
   HACKER_NEWS_MODEL=deepseek-chat
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
