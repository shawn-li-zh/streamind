<div align="center">

# StreamInd

**用 Rust 写的事件驱动 AI Agent 运行时**

Rust 2024 · Axum · SQLite + sqlite-vec · SSE · 远程渲染组件

</div>

---

> **状态：早期开发中（0.1.0）。** 数据表由实体自动同步，HTTP 接口与提示词协议仍在快速演进，本文档以当前代码为准。
> 文档与代码不一致时，请以代码为第一手资料，并顺手修正文档。

## 目录

- [1. 这是什么](#1-这是什么)
- [2. 核心概念](#2-核心概念)
- [3. 输出协议：模型如何驱动运行时](#3-输出协议模型如何驱动运行时)
- [4. 架构与 crate 划分](#4-架构与-crate-划分)
- [5. 快速开始](#5-快速开始)
- [6. 配置项](#6-配置项)
- [7. HTTP / SSE 接口](#7-http--sse-接口)
- [8. 数据模型](#8-数据模型)
- [9. 目录结构](#9-目录结构)
- [10. 前端项目](#10-前端项目)
- [11. 安全须知](#11-安全须知)
- [12. 开发与调试](#12-开发与调试)
- [13. 已知问题](#13-已知问题)

---

## 1. 这是什么

StreamInd 把 LLM 当作一个**持续运行的进程**，而不是一问一答的接口。

一切输入都是**事件**：用户输入、定时器、工具结果、心跳、异常、规则召回……事件进入队列，Agent 在自己的「流」里消费事件、生成带标签的分节输出；输出又可以被解析成新的事件（工具调用、上下文栈操作、渲染组件），如此循环。

由此带来几个和普通聊天机器人不一样的能力：

- **过程可见**：输出按 `think` / `tool_call` / `reply` / `render` 分节，实时通过 SSE 推给前端。
- **上下文是栈**：模型可以用 `push` / `pop` / `reset` 自己管理临时注意力，长任务不丢主线。
- **条件反射**：用关键词/正则/向量相似度表达式绑定「触发器 → 注入内容」，命中后直接把经验反射进上下文，而不是靠 RAG 检索。
- **动态 UI**：模型输出 `render` 标签，运行时调用前端已注册的 Vue 组件，把结构化数据变成真正的界面。
- **可被中断与分叉**：主流运行中可派生支流响应新事件，支流还能请求打断主流。

## 2. 核心概念

| 概念 | 说明 | 主要代码 |
| --- | --- | --- |
| **事件 Event** | 运行时的一切输入。落库的 `event` 表 + 内存事件队列。 | `entity/src/event.rs`、`runtime/src/agent/event.rs` |
| **流 Mind** | 一次「思维活动」：消费事件上下文，产出分节流。 | `kernel/src/mind/stream.rs` |
| **分节 Section** | 模型输出的切片单元（think/reply/tool_call/...）。 | `kernel/src/mind/section.rs`、`entity/src/section.rs` |
| **栈式上下文 Stack** | 事件上下文以栈节点组织，支持 push/pop/reset。 | `runtime/src/context/stack.rs` |
| **条件反射 Rule** | 表达式（关键词/正则/相似度/时间算子）命中后注入内容。 | `reflection/`、`rule_script/`、`src/plugin/reflect.rs` |
| **记忆 Memory** | 主 Agent 之外的记忆 Agent，负责总结短期记忆、整理任务。 | `src/agent/memory.rs`、`src/plugin/memory.rs` |
| **工具 Tool** | 以工具包为单位注册的可调用函数，带类型与文档注释。 | `kernel/src/tool/`、`tool/tool_pkg/` |
| **渲染组件** | 前端注册的 Vue 组件，模型用 `render` 标签实例化。 | `entity/src/render_component.rs`、`public/components/` |
| **Agent** | 一条独立的运行时实例：模型 + 上下文栈 + 工具集 + 权限。 | `src/agent/app.rs`、`runtime/src/agent/` |

## 3. 输出协议：模型如何驱动运行时

模型的原始输出会被 `kernel/src/parser.rs` + `kernel/src/mind/section.rs` 切成「分节」，并流式推给前端。

| 标签 | 作用 | 处理位置 |
| --- | --- | --- |
| `<think>…</think>` | 思考过程，只展示不进上下文 | 分节直出 |
| `<reply>…</reply>` | 给用户的回复 | 分节直出 |
| `<tool_call id="a1">pkg.fn(args)</tool_call>` | 工具调用 | `src/runtime/tool_call.rs` + `kernel/src/tool/engine.rs` |
| `<recall time=… type=…>…</recall>` | 主动触发条件反射召回 | `src/runtime/reflect_call.rs` |
| `<render><Comp attrs={...} /></render>` | 实例化前端渲染组件 | `src/runtime/app.rs` → 前端 `RemoteComponent.vue` |
| `<push title="…">…</push>` | 创建临时栈节点（转移注意力） | `src/runtime/stack_call.rs` |
| `<pop/>` / `<reset/>` | 出栈 / 重置栈 | `src/runtime/stack_call.rs` |

一个完整回合的样子（节选自 `test.xml`）：

```xml
<think>用户想了解北京的天气，我需要调用天气查询工具…</think>
<tool_call id="a1">http.request(method="get",url="https://crates.io/crates/nanoid")</tool_call>
<push title="临时上下文">我需要调用天气查询工具来获取北京的天气信息。</push>
<reply>根据查询结果，北京现在是晴天，约 25 摄氏度。</reply>
<render>
  <ImageList title="动物摄影" attrs={"width":100,"images":[{"name":"鹿","path":"https://…"}]} />
</render>
```

提示词都放在 `prompts/` 目录，运行时按**当前工作目录**的相对路径读取（`prompts/common/*.md`、`prompts/memory/*.md`），改完重启即可生效。

## 4. 架构与 crate 划分

```text
             ┌──────────────────────────────────────────┐
   浏览器 ──▶│ web_api : axum router / SSE / JWT / CORS  │
             └───────────────────┬──────────────────────┘
                                 │
             ┌───────────────────▼──────────────────────┐
             │ src/  二进制 streamind                    │
             │  ├ agent/   AppAgent、MemoryAgent         │
             │  ├ runtime/ AppAgentRuntime 回合调度       │
             │  ├ plugin/  心跳、定时、权限、日志、记忆     │
             │  └ task/    向量化后台任务                 │
             └───┬────────────┬─────────────┬───────────┘
                 │            │             │
        ┌────────▼───┐ ┌──────▼─────┐ ┌─────▼──────────┐
        │  kernel    │ │  runtime   │ │ entity_service │
        │ 思维/工具/  │ │ LLM/上下文/ │ │ CRUD + 建表     │
        │ 解析/渲染   │ │ 嵌入/调度   │ │                │
        └─────┬──────┘ └──────┬─────┘ └─────┬──────────┘
              │               │             │
        ┌─────▼───────┐ ┌─────▼──────┐ ┌─────▼─────┐
        │ tool_pkg    │ │rule_script │ │  entity   │
        │ 内置工具包   │ │+reflection │ │  表定义    │
        └─────────────┘ └────────────┘ └───────────┘
```

| crate | 职责 | 说明 |
| --- | --- | --- |
| `src/`（bin `streamind`） | 装配与业务编排 | 初始化数据库/模型/Agent，回合调度、插件、SSE 出口 |
| `kernel` | 内核抽象 | 消息、解析器、分节、思维流、工具引擎、渲染上下文、权限 |
| `runtime` | 运行时实现 | LLM 客户端（DeepSeek/OpenAI/SiliconFlow）、嵌入、栈/线性上下文、定时器、Agent 事件 |
| `entity` | 数据表定义 | SeaORM 实体（agent/event/section/state/model/...） |
| `entity_service` | 数据访问层 | CRUD、sqlite-vec 初始化、渲染依赖清单重写 |
| `web_api` | HTTP 层 | axum 路由、SSE、JWT、限流、静态资源（原 `web/`） |
| `tool/tool_pkg` | 内置工具集 | tool / render / event / file / editor / http / browser / timer / reflect |
| `tool/tool_macro` | 工具宏 | `#[package]` / `#[tool]` 自动生成工具描述与参数类型 |
| `reflection` | 条件反射引擎 | 算子（includes/regex/count/similar/time…）与规则引擎 |
| `rule_script` | 规则表达式 | pest 语法解析 + 求值 |
| `mcp` | MCP 客户端 | 骨架，尚未接入主流程 |

## 5. 快速开始

### 5.1 前置条件

| 依赖 | 版本 | 备注 |
| --- | --- | --- |
| Rust | ≥ 1.85 | 子 crate 使用 edition 2024 |
| Chrome / Chromium | 任意较新版本 | `browser_*` 工具需要 |
| Node.js + pnpm | ≥ 20 / ≥ 9 | 只有跑前端时才需要 |
| 一个 OpenAI 兼容的模型服务 | — | DeepSeek / SiliconFlow / OpenAI 等 |

### 5.2 步骤

```powershell
git clone <repo> streamind
cd streamind

# 1) 配置环境变量（务必使用自己的密钥，见「安全须知」）
Copy-Item .env.example .env
notepad .env

# 2) 准备运行时目录（后端会往里写 component-manifest.json）
New-Item -ItemType Directory -Force public | Out-Null

# 3) 启动（首次运行自动建表；种子数据见 sql/init.data.sql）
cargo run
```

启动成功后会打印：

```text
INFO streamind: 存储服务初始化成功!
INFO streamind: 智能体运行时初始化成功!
INFO streamind: Starting StreamInd AI Client
INFO streamind: Generated login code:
 【 a1b2c3d4 】
```

用这个登录码换取 JWT：

```powershell
curl "http://localhost:3000/api/login?code=a1b2c3d4"
# {"code":200,"data":"<jwt>","message":"ok"}
```

> 所有 `prompts/`、`public/`、`workspace/`、`reader/` 路径都相对**进程当前工作目录**，请在仓库根目录启动。

### 5.3 前端联调

```powershell
# 另一个终端
cd ui/streamind-web      # 见「前端项目」一节
pnpm install
pnpm dev                 # http://localhost:5173，已代理 /api 与 /public 到 :3000
```

后端 CORS 仅放行 `http://localhost:5173`，改端口请同步修改 `web_api/src/lib.rs`。

## 6. 配置项

复制 `.env.example` 为 `.env`（`.env` 已在 `.gitignore` 中，**请勿提交**）。

| 变量 | 必填 | 默认 | 说明 |
| --- | --- | --- | --- |
| `PORT` | 是 | — | HTTP 监听端口，代码中 `unwrap()`，缺失会启动失败 |
| `DATABASE_URL` | 是 | — | `sqlite://streamind.db?mode=rwc` |
| `EMBEDDING_API_KEY` | 是 | — | 向量模型密钥 |
| `EMBEDDING_API_URL` | 是 | — | 向量接口地址，如火山方舟 `/api/v3/embeddings/multimodal` |
| `EMBEDDING_MODEL` | 是 | — | 如 `doubao-embedding-vision-251215` |
| `EMBEDDING_DIM` | 是 | — | **必须与 `sql/init.vec.sql` 的 `FLOAT[1024]` 一致** |
| `JWT_SECRET_KEY` | 是 | — | 签名密钥，建议 32 位以上随机串；缺失会启动失败 |
| `JWT_EXPIRATION_MINUTES` | | `600` | Token 有效期（分钟） |
| `BASE_URL` | | — | 外部可访问地址，用于把下载的渲染依赖改写成绝对 URL |
| `CHROME_PATH` | | 系统默认 | 浏览器工具使用的 Chrome 可执行文件 |
| `HEARTBEAT_INTERVAL` | | 代码内默认 | 心跳间隔（humantime，如 `30s`） |
| `CONTEXT_SIZE_LIMIT` | | 代码内默认 | 单次上下文体积上限 |
| `TOOL_CALL_TIME_OUT` | | 代码内默认 | 工具调用超时 |
| `TOOL_RESULT_SIZE_LIMIT` | | 代码内默认 | 工具返回值截断阈值 |

模型（`model` 表）与 Agent（`agent` 表）的密钥和参数存在数据库里，可从前端「模型」页维护，也可直接执行 SQL：

```sql
update model set api_key = 'sk-你的密钥' where id = 1;
```

## 7. HTTP / SSE 接口

统一响应体：

```json
{ "code": 200, "data": {}, "message": "ok" }
```

| 方法 | 路径 | 说明 |
| --- | --- | --- |
| GET | `/api/login?code=` | 用启动日志里的登录码换 JWT（带限流） |
| GET | `/api/agent/connect/{agent_id}` | **SSE** 事件流，连接即开始推送 |
| POST | `/api/agent/chat/{agent_id}` | 发送用户消息 `{content, attachments}` |
| POST | `/api/agent/user_interrupt/{agent_id}` | 打断当前回合 |
| POST | `/api/agent/ui_interaction/{agent_id}` | 组件交互回传 `{section_id, message}` |
| POST/GET | `/api/agent`、`/api/agent/{id}`、`/api/agent/tree` | Agent 增查 |
| POST/PUT/GET | `/api/model`、`/api/model/{id}`、`/api/model/list` | 模型配置 |
| GET | `/api/state/list` | 上下文栈状态 |
| GET | `/api/event/list`、`/api/event/load`、`/api/event/count_by_status/{id}` | 历史事件 |
| GET/POST | `/api/section/list`、`/api/section/list_in_range`、`/api/section/{id}`、`/api/section/update_value/{id}` | 分节（过程展示） |
| CRUD | `/api/render_component*` | 渲染组件注册表 |
| CRUD | `/api/render_dependence*` | 渲染依赖（vue/element-plus/自定义组件），含 `download_to_local` |
| POST | `/upload/{*path}` | 上传文件到 `public/` |
| GET | `/public/*` | 静态资源（组件、依赖、图片） |
| GET | `/api/local-image?path=` | 仅本机可访问的本地图片流 |

### SSE 帧格式

```json
{"bind_event_id": 12, "event": "section", "type": "chunk",
 "data": {"tag": "think", "chunk": "让我先看看…"}}
```

| `event`（类别） | `type`（动作） | 说明 |
| --- | --- | --- |
| `server` | `connected` / 注释帧 | 连接建立、心跳保活 |
| `section` | `start` / `open` / `chunk` / `close` / `end` | 分节流式输出 |
| `tool_call` | `start` / `active` / `success` / `failure` / `end` | 工具调用过程 |
| `env` | `userRequest` / `timer` / `error` / `interrupt` | 环境事件 |

## 8. 数据模型

SQLite，启用 `sqlite-vec` 扩展，向量表 `vec_concept` / `vec_scope` 维度固定 1024。

| 表 | 说明 |
| --- | --- |
| `agent` | Agent 实例：模型、父级、当前状态节点、workspace、工具包 |
| `model` | LLM 配置：api_url / api_key / 模态 / 成本档 |
| `state` | 上下文栈节点（`pos=0` 为根） |
| `event` | 事件流水（用户输入、回复、工具结果、定时器等） |
| `section` | 模型输出分节 |
| `item` | 结构化条目 |
| `rule_concept` / `rule_scope` / `rule_item` | 条件反射：概念、作用域、规则 |
| `render_component` / `render_dependence` / `render_state` | 远程渲染 |
| `agent_permission` / `agent_timer` / `task_queue` | 权限、定时、任务队列 |

建表方式：`entity_service::init()` 中 SeaORM `schema_registry.sync()` 自动对齐实体，再执行 `sql/init.vec.sql` 创建向量表。
`sql/init.data.sql` 是**种子数据**（示例模型 / Agent / 组件 / 依赖），`sql/reset.sql` 用于把 1 号 Agent 的会话状态清空。

## 9. 目录结构

```text
streamind/
├── src/                  # 二进制入口与业务编排
│   ├── main.rs           # 启动：dotenv → DB → runtime → HTTP
│   ├── global.rs         # AGENT_POOL / MODEL_POOL / 向量客户端 / 调度器
│   ├── agent_api.rs      # Agent 会话相关 HTTP 接口（chat/connect/interrupt）
│   ├── sse_event.rs      # WebEvent → SSE 帧
│   ├── agent/            # AppAgent、MemoryAgent
│   ├── runtime/          # 回合调度、工具/栈/召回调用
│   ├── plugin/           # 心跳、调度、权限、日志、记忆、反射
│   └── task/             # 向量化后台任务
├── kernel/               # 内核：mind / parser / tool / render / permission
├── runtime/              # LLM、嵌入、上下文、Agent 事件
├── entity/               # SeaORM 实体
├── entity_service/       # 数据访问 + DB 初始化
├── web_api/              # axum HTTP 层（原 web/）
├── tool/
│   ├── tool_macro/       # #[package] / #[tool] 过程宏
│   └── tool_pkg/         # 内置工具实现
├── reflection/           # 条件反射算子与规则引擎
├── rule_script/          # 规则表达式 DSL（pest）
├── mcp/                  # MCP 客户端（骨架）
├── prompts/              # 系统提示词与规则模板（cwd 相对路径）
├── reader/               # 编辑器工具的语言模式分隔符
├── sql/                  # 向量表 / 种子数据 / 重置脚本
├── public/               # 运行时静态资源（git 忽略，由运行时生成）
├── workspace/            # 各 Agent 的运行时目录（日志、浏览器 profile）
├── scripts/              # 开发辅助脚本（密钥自检等）
├── docs/                 # 设计与规划文档
└── ui/                   # 前端（规划中，见下一节）
```

## 10. 前端项目

后端只提供 API + 静态托管，界面由两个独立的 Vue 3 项目组成（当前位于仓库外，建议按 [docs/ui-monorepo.md](docs/ui-monorepo.md) 归置）：

| 项目 | 技术栈 | 作用 |
| --- | --- | --- |
| `streamind-web` | Vue 3 + Vite + Pinia + Element Plus / TDesign | 主控制台：对话、过程可视化、Agent/模型/组件管理 |
| `streamind-remote-render` | Vue 3 + Vite lib 模式 | 远程渲染组件的源码与构建脚本，产物 `dist/<Name>/index.js` 上传到后端 `public/components/` |

开发期约定：

- 主控制台把 `/api`、`/public` 代理到 `http://localhost:3000`（见其 `vite.config.ts`）。
- 组件用 `pnpm build-render -- --name=X --path=X` 单包构建，`--all` 全量构建。
- 组件以 ESM 输出，`vue` / `element-plus` / `@element-plus/icons-vue` 作为外部依赖，由宿主通过 `public/component-manifest.json` 的 importmap 注入。

## 11. 安全须知

- **本仓库此前提交过真实的模型 API Key。** 历史提交里的密钥请视为已泄露，务必到平台吊销并重新签发：DeepSeek、SiliconFlow（`sql/init.data.sql` 的种子数据）、火山方舟（`EMBEDDING_API_KEY`）。
- `.env`、`*.db`、`public/`、`workspace/`、`tempb/` 均已加入 `.gitignore`，且已从索引中移除。提交前请 `git status` 自查。
- 提交前自检：`pwsh -File scripts/check-secrets.ps1`（默认扫暂存区，`-All` 扫全部受跟踪文件），可挂到 `.git/hooks/pre-commit`。
- 若需要从历史中彻底抹除（会重写所有提交哈希，请先备份并通知协作者）：

```bash
git filter-repo --invert-paths --path .env --path streamind.db
```

- JWT 中间件目前在 `web_api/src/feature/auth.rs` 中**处于注释状态**，即使登录成功也不会强制校验；不要把它直接暴露到公网，启用前请自行打开校验。
- 默认监听 `0.0.0.0`。仅本机使用请改为 `127.0.0.1`。
- 浏览器工具会以你的身份操作本机 Chrome，`file_*` / `editor_*` 工具可以读写磁盘，请只在可信环境运行。

## 12. 开发与调试

```powershell
cargo check --workspace        # 全量类型检查
cargo run                      # 启动
cargo test --workspace         # 单元测试（部分 crate 尚无覆盖）
cargo nextest run              # 已在 dev-dependencies 中声明
$env:RUST_LOG="debug"; cargo run
```

- 单元测试覆盖规则引擎、分节解析、工具调用解析、栈上下文与上传路径，全仓库 33 项；另有 2 项依赖真实环境的集成测试默认 `#[ignore]`（真实数据库 / 本机 Chrome），用 `cargo test --workspace -- --ignored` 手动执行。
- 日志同时走 `tracing`（stdout）和 Agent 工作区文件（`workspace/<code>/log/*.txt`）。
- 想重置 1 号 Agent 的对话：执行 `sql/reset.sql`。
- 想重建向量索引：删掉 `vec_concept` / `vec_scope` 两张表后重启。

## 13. 已知问题

- `web_api` 的鉴权中间件未启用（见「安全须知」）。
- CORS 白名单与前端端口写死为 `localhost:5173`。
- `mcp` crate 只有骨架，未接入工具池。
- 部分工具在初始化时读取 `reader/modes`、`prompts/rule_scope.json`，依赖工作目录为仓库根目录。
- Agent 的 `workspace/` 会持续增长（浏览器 profile、日志、截图），需要定期清理。

## 许可证

尚未指定。默认保留所有权利，如需开源请补充 `LICENSE`。
