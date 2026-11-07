# 前端项目如何与 StreamInd 仓库共存

> 结论先行：**推荐方案 A —— 用 Git submodule 把两个 Vue 项目挂到 `ui/` 下**，
> 再把「构建组件产物 → 上传到后端 `public/`」这一步脚本化。
> 如果团队更看重「一次 clone 拿到全部代码、统一提交」，选方案 B（monorepo + subtree 合历史）。

## 1. 现状

```text
D:\project\ui\
├── streamind-web           # 主控制台（Vue3 + Vite + Pinia + Element Plus/TDesign）
│   └── .git                # 独立仓库
└── streamind-remote-render # 远程渲染组件源码 + 构建脚本（Vite lib 模式）
    └── .git                # 独立仓库

D:\project\rust\streamind\  # 本仓库（Rust workspace）
```

三者之间的契约只有三条，耦合很轻：

1. **HTTP 契约**：控制台通过 `/api/**` 调后端，通过 `/public/**` 取静态资源；开发期由 Vite 代理到 `http://localhost:3000`。
2. **组件契约**：`streamind-remote-render` 把每个组件打成 ESM（外部依赖 vue/element-plus 等），产物是 `dist/<Name>/index.js`。
3. **注册契约**：后端 `render_component` 表登记「组件名 → `public/components/<Name>/index.js`」，`render_dependence` 表生成 `public/component-manifest.json` 作为 importmap，供宿主解析依赖。

所以**不需要**把前端塞进 Cargo workspace，只需要一个「放在一起、能一键跑起来」的组织方式。

## 2. 三种方案对比

| | A. Git submodule | B. 单仓库（subtree / 合并历史） | C. 保持独立仓库 |
| --- | --- | --- | --- |
| 代码是否在同一目录树 | 是（`ui/` 下，指针文件） | 是（真实文件） | 否 |
| 是否保留独立仓库历史 | 保留 | 合并进主仓（subtree 可保留） | 保留 |
| 一次 clone 拿到全部代码 | 需要 `--recurse-submodules` | 是 | 否 |
| 原子提交（前后端同一个 PR） | 否（要分别在两个仓库提交，再更新指针） | 是 | 否 |
| 独立权限/独立 CI | 好 | 差（要按路径过滤） | 最好 |
| 新人上手成本 | 中 | 低 | 低（但要单独 clone 两个仓库） |
| Rust 仓库体积/噪音 | 小（只是 gitlink） | 大（node_modules 风险、CI 全量触发） | 无 |

**取舍要点**：这个项目的日常改动是「Rust 侧频繁重构 + 前端跟着改接口」，前端的发版节奏和 Rust 并不一致，
而且两个前端仓库各自都已经有提交历史。**A 的性价比最高**；等前后端接口稳定、需要频繁原子提交时，再平滑迁移到 B（`git subtree push/pull` 双向可用）。

## 3. 推荐目录结构（方案 A）

```text
streamind/
├── Cargo.toml                 # Rust workspace（保持现状）
├── src/  kernel/  runtime/  entity/  entity_service/  web_api/  tool/ ...
├── prompts/  reader/  sql/
├── public/                    # 运行时产物目录，git 忽略（后端启动时自动创建）
├── ui/
│   ├── README.md              # 说明如何初始化/更新子模块
│   ├── package.json           # 仅放「编排脚本」，可选
│   ├── pnpm-workspace.yaml    # 可选：让两个项目共享依赖与 lockfile
│   ├── streamind-web/         # submodule → git@…/streamind-web.git
│   └── streamind-remote-render/  # submodule → git@…/streamind-remote-render.git
├── docs/
│   └── ui-monorepo.md         # 本文
└── .gitignore                 # 增加 ui/**/node_modules、ui/**/dist
```

`.gitmodules` 形如：

```ini
[submodule "ui/streamind-web"]
	path = ui/streamind-web
	url = <你的 git 地址>/streamind-web.git
	branch = main
[submodule "ui/streamind-remote-render"]
	path = ui/streamind-remote-render
	url = <你的 git 地址>/streamind-remote-render.git
	branch = main
```

## 4. 落地步骤（可直接执行）

### 4.1 方案 A：submodule

```powershell
cd D:\project\rust\streamind

# 把已有本地仓库直接作为子模块挂进来（保留其 .git 与历史）
git submodule add <streamind-web 仓库地址> ui/streamind-web
git submodule add <streamind-remote-render 仓库地址> ui/streamind-remote-render

git commit -m "chore(ui): 以 submodule 方式引入两个前端项目"

# 其他人克隆时
git clone --recurse-submodules <本仓库地址>
# 已有克隆补拉
git submodule update --init --recursive
```

若两个项目暂时没有远端，可先用本地路径：`git submodule add D:\project\ui\streamind-web ui/streamind-web`
（之后把 `.gitmodules` 里的 url 改成正式地址并 `git submodule sync`）。

### 4.2 方案 B：合并为一仓（保留历史）

```powershell
cd D:\project\rust\streamind
git remote add ui-web        D:\project\ui\streamind-web
git remote add ui-render     D:\project\ui\streamind-remote-render

git fetch ui-web main
git fetch ui-render main

git subtree add --prefix ui/streamind-web           ui-web/main --squash
git subtree add --prefix ui/streamind-remote-render ui-render/main --squash

git remote remove ui-web
git remote remove ui-render
```

之后前端改动可以直接在主仓提 PR；需要回推到独立仓库时用
`git subtree push --prefix ui/streamind-web ui-web main`。

### 4.3 通用：忽略规则

在根 `.gitignore` 追加：

```gitignore
# 前端依赖与构建产物
ui/**/node_modules
ui/**/dist
ui/**/.vite
ui/**/*.log
```

## 5. 开发编排（关键收益）

把「起后端 → 起前端 → 构建组件并上传」收敛成一条命令。在 `ui/package.json` 里放编排脚本：

```json
{
  "name": "streamind-ui",
  "private": true,
  "scripts": {
    "dev": "pnpm -C streamind-web dev",
    "build": "pnpm -C streamind-web build",
    "render:build": "pnpm -C streamind-remote-render build-render -- --all",
    "render:upload": "pwsh -File ../scripts/upload-render.ps1"
  }
}
```

配合 `ui/pnpm-workspace.yaml`（可选，能让两个前端共用一份 `pnpm-lock.yaml` 与依赖缓存）：

```yaml
packages:
  - streamind-web
  - streamind-remote-render
```

后端侧建议加一个 `scripts/dev.ps1`：先 `cargo run`，等 `:3000` 就绪后再起 Vite；
`scripts/upload-render.ps1` 遍历 `ui/streamind-remote-render/dist/*/index.js`，
按 `POST /upload/components/<Name>/index.js` 上传（对应后端 `web_api/src/feature/public.rs` 的 `/upload/{*path}`），
或直接复制到 `public/components/<Name>/index.js`，再在 `render_component` 表里登记。

## 6. 与后端的三个对接点（别踩坑）

1. **CORS 端口写死**：后端只放行 `http://localhost:5173`（`web_api/src/lib.rs`）。前端换端口要同步改，或把白名单做成环境变量。
2. **`public/` 是生成物**：组件产物、下载的依赖、`component-manifest.json` 都在这里，已在 `.gitignore` 中。**不要提交**，用上传接口或脚本同步。
3. **依赖版本要对齐**：组件以 ESM 输出并把 `vue` / `element-plus` / `@element-plus/icons-vue` 标记为 external，宿主通过
   `render_dependence` 表的版本记录注入 importmap。两个前端项目的 vue/element-plus 大版本要一致，否则远程组件会在宿主里崩。

## 7. CI 建议

| 仓库 | 触发 | 任务 |
| --- | --- | --- |
| 本仓库（Rust） | `src/`、`*/src/`、`Cargo.*` 变更 | `cargo fmt --check`、`cargo clippy`、`cargo test --workspace` |
| 本仓库（打包） | tag | 构建二进制 + 前端产物，产出可分发目录 |
| `streamind-web` | 自身变更 | `pnpm type-check`、`pnpm build` |
| `streamind-remote-render` | 自身变更 | `pnpm build-render -- --all`，产物作为 artifact 上传 |

若采用方案 A，主仓 CI 默认不构建前端（submodule 为空也能通过），需要在打包流水线里显式 `--recurse-submodules`。

## 8. 迁移后的自检清单

- [ ] `git clone --recurse-submodules` 后能直接 `cargo run`（或按方案 B 直接 `cargo run`）
- [ ] `ui/` 下两个项目能 `pnpm install && pnpm dev`
- [ ] Vite 代理指向 `:3000`，页面能拉到 `/api/model/list`
- [ ] `pnpm build-render -- --all` 产物能上传并在对话中通过 `<render>` 正常渲染
- [ ] 根 `.gitignore` 已忽略 `node_modules` / `dist`
- [ ] 没有任何 `.env`、密钥、`*.db` 被提交（见 README「安全须知」）
