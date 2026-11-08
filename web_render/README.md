# web_render — 远程渲染组件

模型在对话里通过 `<render>` 标签实例化的 Vue 组件源码与构建脚本。

每个组件是一个独立入口：`src/render/<Name>/index.vue`，用 Vite lib 模式打包成 ESM，
`vue` / `element-plus` / `@element-plus/icons-vue` 作为外部依赖，由宿主通过
`public/component-manifest.json` 的 importmap 注入。

## 构建

```powershell
pnpm install

# 单个组件
pnpm run build-render -- --name=ImageList --path=ImageList

# 全部组件
pnpm run build-render -- --all
```

产物：`dist/<Name>/index.js`（打包脚本会复制到 `public/components/<Name>/`）。

## 新增组件的完整流程

1. 在 `src/render/<Name>/index.vue` 写组件，props 约定见现有组件；
2. 构建出 `dist/<Name>/index.js`；
3. 同步到后端 `public/components/<Name>/`（或直接用 `/upload/<path>` 接口上传）；
4. 在数据库 `render_component` 表登记名称、路径与 attrs 文档 —— 不登记模型就不知道有这个组件；
5. 若引入了新的第三方库，先在 `render_dependence` 表登记并给出可访问的 importmap 地址。

注意：`vue` / `element-plus` 的大版本必须与 `web_ui` 保持一致，否则组件在宿主里会因双实例崩溃。
