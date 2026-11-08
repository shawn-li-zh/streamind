# 前端工程组织：web_ui / web_render

> 本文档记录仓库里两个前端目录的定位、契约与构建方式。
> 结论：**前后端同仓**，两个 Vue 项目直接放在根目录下，通过脚本构建后由后端托管。

## 1. 目录结构

```text
streamind/
├── Cargo.toml                 # Rust workspace
├── web_api/                   # 后端 HTTP 层（axum）
├── src/ kernel/ runtime/ ...  # 其余 Rust crate
├── web_ui/                    # 控制台（Vue3 + Vite + Pinia + Element Plus / TDesign）
├── web_render/                # 远程渲染组件（Vue3，Vite lib 模式）
├── public/                    # 运行时静态根目录（git 忽略，运行时生成）
│   ├── components/<Name>/     # ← web_render 的产物
│   ├── web/                   # ← web_ui 的产物（后端在 / 上托管）
│   ├── dom_view.js            # 浏览器工具注入脚本
│   └── fingerprint_cheat.js
├── dist/                      # scripts/package.ps1 的输出（git 忽略）
└── scripts/                   # package.ps1 / check-secrets.ps1
```

## 2. 三者之间的契约

只有三条，耦合很轻：

| # | 契约 | 体现 |
| --- | --- | --- |
| 1 | **HTTP**：控制台只依赖 `/api/**» 与 `/public/**» | 开发期 Vite 代理到 `http://localhost:3000»；生产同源，无需 CORS |
| 2 | **组件**：渲染组件是 ESM 模块，外部依赖交给宿主 | `vue» / `element-plus» / `@element-plus/icons-vue» 标记为 external，由宿主 importmap 注入 |
| 3 | **注册**：组件名 → 静态路径 | `render_component» 表登记 `public/components/<Name>/index.js»；`render_dependence» 表生成 `public/component-manifest.json» |

数据流：

```text
web_render/src/render/<Name>/index.vue
        │ pnpm run build-render
        ▼
web_render/dist/<Name>/index.js  ────► public/components/<Name>/index.js
                                              │
模型输出 <render><Name attrs={...} /></render>
                                              ▼
                                    前端「远程组件」按 importmap 加载并渲染

web_ui/src  ──pnpm run build-only──►  web_ui/dist  ────►  public/web
                                                          │
                                         后端 ServeDir("/") + index.html 回退
```

## 3. 日常开发

```powershell
# 控制台
pnpm -C web_ui install
pnpm -C web_ui dev            # http://localhost:5173，/api 与 /public 已代理到 :3000

# 渲染组件
pnpm -C web_render install
pnpm -C web_render run build-render -- --name=ImageList --path=ImageList
pnpm -C web_render run build-render -- --all

# 打包（前端 + 组件 + Rust release）
pwsh -File scripts/package.ps1
```

新增一个渲染组件的步骤：

1. 在 `web_render/src/render/<Name>/index.vue» 写组件（`defineProps<{ attrs: ... }>()» 约定见现有组件）；
2. `pnpm -C web_render run build-render -- --name=<Name> --path=<Name>» 构建；
3. 把 `web_render/dist/<Name>/» 复制到 `public/components/<Name>/»（打包脚本会自动做）；
4. 在 `render_component» 表登记名称、路径与 attrs 文档，模型才能在 `<render>» 里调用；
5. 如果用到新的第三方库，先在 `render_dependence» 里登记并提供 importmap 地址。

## 4. 为什么前后端同仓（而不是 submodule）

| 关注点 | 同仓（当前） | 独立仓库 / submodule |
| --- | --- | --- |
| 一个 PR 同时改接口与界面 | ✅ | ❌ 要跨仓协调 |
| 版本对齐（vue / element-plus） | ✅ 一眼可见 | ⚠️ 容易漂移 |
| 仓库体积 | ⚠️ 多约 1.3MB 源码（node_modules 与 dist 已忽略） | ✅ 主仓干净 |
| 独立发版 / 独立权限 | ⚠️ 需要路径过滤 | ✅ |
| 新人上手 | ✅ clone 一次全拿到 | ⚠️ 需要 `--recurse-submodules» |

这个项目的现状是**接口还在快速演进、前端跟着 Rust 侧一起改**，同仓的收益大于成本，所以选择直接入库。
两个前端原本各有独立 git 历史，迁移时以当前工作区快照并入（历史仍保留在原目录的仓库里）。

如果将来要拆出去，可以平滑迁移：

```powershell
# 方案 A：子模块
git submodule add <前端仓库地址> web_ui

# 方案 B：把现在的目录推成独立仓库（保留历史需要在迁移前用 git subtree split）
git subtree split --prefix web_ui -b web_ui-standalone
```

## 5. 约束与坑

- **忽略规则**：`web_ui/.gitignore»、`web_render/.gitignore» 负责各自目录；根 `.gitignore» 另加了 `node_modules/»、`web_ui/dist/»、`web_render/dist/» 兜底。
- **版本对齐**：两个项目的 `vue» 与 `element-plus» 大版本必须一致，否则远程组件在宿主里会因双实例崩溃。
- **构建脚本**：`web_ui/package.json» 原来的 `build» 依赖未声明的 `npm-run-all»（`run-p»/`run-s»），已改成 `vue-tsc --build && vite build»；打包脚本统一用 `build-only»。
- **静态资源路径**：Vite `base» 为默认 `/»，所以后端必须把控制台挂在站点根（`fallback_service»），不能挂在 `/app» 之类子路径下。
- **CORS**：开发期后端只放行 `http://localhost:5173»；换端口要同步改 `web_api/src/lib.rs»。

## 6. 自检清单

- [ ] `pnpm -C web_ui run build-only» 能出 `dist/index.html»
- [ ] `pnpm -C web_render run build-render -- --all» 能出 `dist/<Name>/index.js»
- [ ] `pwsh -File scripts/package.ps1» 产出的目录里 `public/web/index.html» 与控制台一致
- [ ] 启动后 `http://localhost:3000» 能打开控制台，刷新子路由不 404
- [ ] 对话里 `<render>» 的组件能正常渲染
