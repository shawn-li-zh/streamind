# web_ui — StreamInd 控制台

Vue 3 + Vite + Pinia + Element Plus / TDesign 的单页应用，负责对话、过程可视化、Agent / 模型 / 渲染组件管理。

## 开发

```powershell
pnpm install
pnpm dev        # http://localhost:5173，/api 与 /public 已代理到后端 :3000
```

后端需要先在仓库根目录 `cargo run` 起来。

## 构建

```powershell
pnpm run build          # 类型检查 + 构建
pnpm run build-only     # 只构建（打包脚本用这个）
```

产物 `dist/` 会被 `scripts/package.ps1` 复制到后端的 `public/web`，由 `web_api` 在站点根托管。

## 环境变量

| 变量 | 默认 | 说明 |
| --- | --- | --- |
| `VITE_API_URL` | `/` | 后端地址前缀；同源部署保持默认即可 |

更多背景见仓库根目录的 `README.md` 与 `docs/ui-monorepo.md`。
