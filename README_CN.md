# Hacker News RSS 生成器（AI 摘要版）

一个基于 Rust 的 Web 服务，从 Hacker News 抓取热门故事并使用大语言模型（LLM）生成详细摘要。

## 功能特性

- 从 Hacker News API 获取热门故事
- **关键词搜索（Algolia HN Search API）** - 搜索特定主题的故事（如 "rust"、"go"、"linux"）
- 使用 LLM 生成 200-300 词英文摘要或 400-500 字中文摘要
- SQLite 数据库持久化存储
- RESTful API 管理期刊和故事
- SSE 流式传输，实时显示抓取进度
- 可配置的故事数量、LLM 模型和 API 设置
- 按期刊组织内容（每日快照）
- 标签徽章显示故事来源（热门故事 vs 关键词搜索）
- Web UI 支持暗色模式

## 快速开始

### 前置要求

- Rust 1.75+（从源码构建）
- LLM API 密钥（OpenAI、DeepSeek 或任何 OpenAI 兼容 API）

### 1. 克隆并构建

```bash
git clone https://github.com/heng30/hacker-news-rs.git
cd hacker-news-rs
cargo build --release
```

### 2. 配置环境变量

在项目根目录创建 `.env` 文件：

```bash
# 必需：LLM API 密钥
HACKER_NEWS_OPENAI_API_KEY=你的API密钥

# 可选：LLM API 设置（默认值如下）
HACKER_NEWS_OPENAI_BASE_URL=https://api.deepseek.com/v1
HACKER_NEWS_MODEL=deepseek-chat
HACKER_NEWS_STORY_COUNT=30

# 可选：启用关键词搜索
HACKER_NEWS_SEARCH_KEYWORDS=rust,go,linux

# 可选：服务器设置
HACKER_NEWS_PORT=3000
HACKER_NEWS_DATABASE_URL=sqlite:data/hacker_news.db?mode=rwc
```

完整配置选项请参考 [`.env.example`](.env.example)。

### 3. 运行

```bash
cargo run --release
```

服务器将在 `http://localhost:3000` 启动。在浏览器中打开此地址即可访问 Web UI。

## 部署方式

### 方式一：本地开发

直接构建并运行：

```bash
cargo build --release
cargo run --release
```

可执行文件位于 `target/release/hacker-news-rs`。

### 方式二：打包部署

创建独立的分发目录：

```bash
./bundle.sh [输出目录]
```

这将创建 `dist/` 目录，包含：
```
dist/
├── hacker-news-rs       # 可执行文件
├── .env                 # 配置文件（从项目根目录复制）
└── src/static/
    └── index.html       # Web UI
```

运行打包版本：
```bash
cd dist
./hacker-news-rs
```

### 方式三：安装到用户目录

安装到 `$HOME/.local/bin/`：

```bash
./install.sh
```

此脚本会执行 `bundle.sh` 并将结果复制到：
- `$HOME/.local/bin/hacker-news` - 可执行文件
- `$HOME/.local/bin/hacker-news-dist/` - 分发文件

从任意位置运行（需确保 `$HOME/.local/bin` 在 PATH 中）：
```bash
cd ~/.local/bin/hacker-news-dist
./hacker-news
```

### 方式四：Systemd 服务（Linux）

创建 systemd 服务实现开机自启：

```bash
# 1. 打包并复制到系统位置
./bundle.sh
sudo cp -r dist /opt/hacker-news

# 2. 创建 systemd 服务文件
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

# 3. 启用并启动服务
sudo systemctl daemon-reload
sudo systemctl enable hacker-news
sudo systemctl start hacker-news
```

### 方式五：Docker（替代方案）

创建 Dockerfile：

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

构建并运行：
```bash
docker build -t hacker-news-rs .
docker run -d -p 3000:3000 -v $(pwd)/data:/app/data hacker-news-rs
```

## 使用说明

### Web UI

在浏览器中打开 `http://localhost:3000`。界面功能包括：

- **故事列表**：查看抓取的故事及 AI 摘要
- **日历导航**：按日期浏览期刊
- **抓取按钮**：触发新故事抓取，显示实时进度
- **语言切换**：切换中文/英文摘要
- **暗色模式**：在导航栏切换
- **标签徽章**：显示故事来源（热门故事 vs 关键词搜索）

### API 接口

| 方法 | 路径 | 说明 |
|------|------|------|
| GET | `/api/episode/latest` | 获取最新期刊及其故事 |
| GET | `/api/episode/{date}` | 按日期获取期刊（格式：YYYY-MM-DD） |
| DELETE | `/api/episode/{date}` | 按日期删除期刊 |
| DELETE | `/api/episode/{date}/stories` | 删除指定期刊的所有故事 |
| POST | `/api/fetch` | 抓取新故事（返回 JSON） |
| GET | `/api/fetch/stream` | SSE 流式抓取，实时显示进度 |
| GET | `/api/stories` | 获取所有已存储的故事 |
| DELETE | `/api/stories` | 删除所有故事 |
| DELETE | `/api/stories/read` | 按 hn_ids 删除已读故事 |
| PUT | `/api/story/{hn_id}/regenerate` | 重新生成故事摘要 |
| GET | `/api/episodes` | 获取所有期刊列表 |
| GET | `/api/config` | 获取当前配置 |
| PUT | `/api/config` | 更新配置 |

### 通过 API 抓取故事

**简单抓取（JSON 响应）：**
```bash
curl -X POST http://localhost:3000/api/fetch \
  -H "Content-Type: application/json" \
  -d '{"lang": "zh"}'
```

**流式抓取（SSE，实时进度）：**
```bash
curl -N http://localhost:3000/api/fetch/stream?lang=zh
```

SSE 事件类型：
- `story_added`：新故事已保存（摘要生成前）
- `summary_done`：某故事的摘要已生成
- `summary_error`：摘要生成失败
- `done`：所有故事处理完成

### 获取最新期刊

```bash
curl http://localhost:3000/api/episode/latest
```

响应示例：
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
        "title": "故事标题",
        "url": "https://example.com",
        "by": "作者",
        "score": 100,
        "summary_zh": "中文摘要...",
        "tag": "top"
      }
    ]
  }
}
```

## Hacker News API 来源

本项目使用两个 API：

### 官方 HN Firebase API（热门故事）

| 接口 | 说明 |
|------|------|
| `GET https://hacker-news.firebaseio.com/v0/topstories.json` | 返回最多 500 个热门故事 ID |
| `GET https://hacker-news.firebaseio.com/v0/item/{id}.json` | 返回故事详情（title, url, by, score, time） |

### Algolia HN Search API（关键词搜索）

| 接口 | 说明 |
|------|------|
| `GET https://hn.algolia.com/api/v1/search_by_date?query={keyword}&tags=story&hitsPerPage={limit}` | 按关键词搜索最新故事 |

API 参考：https://hn.algolia.com/api

### 故事标签（tag 字段）

`tag` 字段表示故事来源：
- `"top"` - 来自热门故事（UI 中隐藏）
- 关键词（如 `"rust"`、`"go"`）- 来自关键词搜索（显示为彩色徽章）

## 摘要生成流程

```
HN API → 故事元数据 → 抓取 URL 内容 → 提取文本 → LLM → 摘要 → SQLite → 前端
```

1. **内容抓取**：从故事 URL 获取 HTML（30 秒超时），使用 `scraper` 库提取纯文本
2. **内容限制**：超过 32,000 字符时截断
3. **失败回退**：内容抓取失败时，LLM 仅使用标题和 URL
4. **LLM 提示词**：标题 + 内容发送给 LLM，附带语言特定指令
5. **并行处理**：SSE 接口以 3 个并发批量生成摘要

## 配置说明

所有配置通过环境变量设置（前缀：`HACKER_NEWS_`）：

| 变量 | 说明 | 默认值 |
|------|------|--------|
| `HACKER_NEWS_OPENAI_API_KEY` | LLM API 密钥 | **（必需）** |
| `HACKER_NEWS_OPENAI_BASE_URL` | LLM API 基础 URL | `https://api.deepseek.com/v1` |
| `HACKER_NEWS_MODEL` | LLM 模型名称 | `deepseek-chat` |
| `HACKER_NEWS_STORY_COUNT` | 每次抓取故事数量 | `30` |
| `HACKER_NEWS_PORT` | 服务端口 | `3000` |
| `HACKER_NEWS_DATABASE_URL` | SQLite 数据库 URL | `sqlite:data/hacker_news.db?mode=rwc` |
| `HACKER_NEWS_AUTO_UPDATE_INTERVAL` | 自动更新间隔（分钟） | `0`（禁用） |
| `HACKER_NEWS_SEARCH_KEYWORDS` | Algolia 搜索关键词 | （为空则禁用） |
| `HACKER_NEWS_SOCKS5` | SOCKS5 代理 | （为空则禁用） |

### LLM API 兼容性

支持任何 OpenAI 兼容的 API：

| 服务商 | 基础 URL | 模型示例 |
|--------|----------|----------|
| OpenAI | `https://api.openai.com/v1` | `gpt-4o`, `gpt-4o-mini` |
| DeepSeek | `https://api.deepseek.com/v1` | `deepseek-chat` |
| 阿里云 Qwen | `https://dashscope.aliyuncs.com/compatible-mode/v1` | `qwen-plus`, `qwen-turbo` |
| Ollama（本地） | `http://localhost:11434/v1` | `llama3`, `mistral` |

## 项目结构

```
hacker-news-rs/
├── src/
│   ├── main.rs              # 入口，Axum 服务器配置
│   ├── config.rs            # 环境变量辅助函数
│   ├── hn/
│   │   └── api.rs           # HN Firebase + Algolia 客户端
│   ├── llm/
│   │   └── client.rs        # LLM 摘要生成封装
│   ├── db/
│   │   ├── models.rs        # Episode, Story, Config 结构体
│   │   └── mod.rs           # SQLite 操作（sqlx）
│   ├── fetcher/
│   │   └── content.rs       # URL 抓取 + HTML→Markdown
│   └── routes/
│       ├── episode.rs       # 故事/期刊 API 路由 + SSE
│       └── config.rs        # 配置 API 路由
│   └── static/
│       └── index.html       # Web UI（单页应用）
├── lib/
│   └── bot/                 # LLM 流式聊天库
├── bundle.sh                # 创建部署包
├── install.sh               # 安装到 ~/.local/bin
├── .env.example             # 配置模板
└── Cargo.toml               # 工作区配置
```

## 许可证

MIT 许可证 - 详见 [LICENSE](LICENSE)。