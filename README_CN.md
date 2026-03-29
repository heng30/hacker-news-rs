# Hacker News RSS 生成器（AI 摘要版）

一个基于 Rust 的 Web 服务，从 Hacker News 抓取热门故事并使用大语言模型（LLM）生成详细的中文摘要。

## 功能特性

- 从 Hacker News API 获取热门故事
- 使用 LLM 生成 500-600 字的中文摘要
- SQLite 数据库持久化存储
- RESTful API 管理期刊和故事
- 可配置的故事数量、LLM 模型和 API 设置
- 按期刊组织内容（每日快照）

## API 接口

| 方法 | 路径 | 说明 |
|------|------|------|
| GET | `/api/episode/latest` | 获取最新期刊及其故事 |
| GET | `/api/episode/{date}` | 按日期获取期刊 |
| DELETE | `/api/episode/{date}` | 按日期删除期刊 |
| DELETE | `/api/episode/{date}/stories` | 删除指定期刊的所有故事 |
| POST | `/api/fetch` | 从 Hacker News 抓取新故事 |
| GET | `/api/stories` | 获取所有已存储的故事 |
| DELETE | `/api/stories` | 删除所有故事 |
| DELETE | `/api/stories/read` | 按 hn_ids 删除已读故事 |
| PUT | `/api/story/{hn_id}/regenerate` | 重新生成故事摘要 |
| GET | `/api/episodes` | 获取所有期刊列表 |

## 配置说明

通过环境变量配置（前缀：`HACKER_NEW_`）：

| 变量 | 说明 | 默认值 |
|------|------|--------|
| `HACKER_NEW_DATABASE_URL` | SQLite 数据库 URL | `sqlite:data/hacker_news.db?mode=rwc` |
| `HACKER_NEW_PORT` | 服务端口 | `3000` |
| `HACKER_NEW_OPENAI_API_KEY` | LLM API 密钥 | （生成摘要必需） |
| `HACKER_NEW_OPENAI_BASE_URL` | LLM API 基础 URL | `https://api.deepseek.com/v1` |
| `HACKER_NEW_MODEL` | LLM 模型名称 | `deepseek-chat` |
| `HACKER_NEW_STORY_COUNT` | 每次抓取故事数量 | `10` |
| `HACKER_NEW_AUTO_UPDATE_INTERVAL` | 自动更新间隔（分钟） | `0`（禁用） |

## 快速开始

1. 克隆仓库：
   ```bash
   git clone https://github.com/your-username/hacker-new-rs.git
   cd hacker-new-rs
   ```

2. 创建 `.env` 文件：
   ```bash
   HACKER_NEW_OPENAI_API_KEY=你的API密钥
   HACKER_NEW_OPENAI_BASE_URL=https://api.deepseek.com/v1
   HACKER_NEW_MODEL=deepseek-chat
   ```

3. 构建并运行：
   ```bash
   cargo build --release
   cargo run --release
   ```

4. 访问 API：`http://localhost:3000`

## 项目结构

```
hacker-new-rs/
├── src/
│   ├── main.rs          # 应用入口
│   ├── config.rs        # 环境配置
│   ├── hn/
│   │   ├── api.rs       # Hacker News API 客户端
│   │   └── mod.rs
│   ├── llm/
│   │   ├── client.rs    # LLM 摘要生成客户端
│   │   └── mod.rs
│   ├── db/
│   │   ├── models.rs    # 数据库模型
│   │   └── mod.rs       # 数据库操作
│   └── routes/
│       ├── episode.rs   # 期刊/故事路由
│       ├── config.rs    # 配置路由
│       └── mod.rs
├── lib/
│   └── bot/             # LLM 聊天库
└── Cargo.toml
```

## 许可证

MIT 许可证 - 详见 [LICENSE](LICENSE)。

