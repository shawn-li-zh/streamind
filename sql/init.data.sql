-- 初始化测试模型
INSERT INTO "main"."model" ("id", "name", "remark", "api_url", "api_key", "input_modals", "output_modals", "consumed_tokens", "status", "cost") VALUES (1, 'deepseek-v4-pro', '测试', 'https://api.deepseek.com', 'sk-e847b4cab3f447b9a1132ef98bfcc509', '["text"]', '["text"]', 0, 'active', 'low');
INSERT INTO "main"."model" ("id", "name", "remark", "api_url", "api_key", "input_modals", "output_modals", "consumed_tokens", "status", "cost") VALUES (2, 'Pro/moonshotai/Kimi-K2.6', '测试', 'https://api.siliconflow.cn/v1', 'sk-kzxxmauuezefajpvtelyhmhsnetlocxwgfnjctmdwgqjopux', '["text","image"]', '["text"]', 0, 'active', 'low');
INSERT INTO "main"."agent" ("id", "model_id", "parent_id", "state_id", "name", "code", "persona", "workspace", "avatar", "tool_packages", "startup_time", "create_time","status") VALUES (1, 1, 0, 1, '助手', 'main', '', 'workspace/main', 'https://api.dicebear.com/9.x/bottts/svg?scale=90&seed=助手-0.30414842228662375', '["*"]', '2026-05-11T13:55:58.760881100+08:00', '2026-05-05T16:17:28.133669800+08:00','idle');
INSERT INTO "main"."state" ("id", "agent_id", "pos", "dumped", "title", "content", "time") VALUES (1, 1, 0, 0, '根节点', '- 工作机制:我是一个事件驱动的agent,使用rust构建
- 原生能力:异步处理事件、动态生成和渲染ui、创建分身、栈式上下文管理
- 自身状态:用户刚初始化我的环境
- 用户状态:未知
- 近期任务:暂无
- 值得关注的事件:暂无
- 驱动目标: 服务好我的用户，完美完成用户交代的任务，管理用户的事项、信息等，保持诚实稳重

当外部事件转移当前注意力时，我要使用<push>标签,创建一个临时栈节点。', '2026-05-11T17:27:36.351746+08:00');
-- 初始化依赖
INSERT INTO "main"."render_dependence" ("name", "version", "path","ext", "editable") VALUES ( 'vue', '3.2.25', 'https://unpkg.com/vue@3/dist/vue.esm-browser.js','js', false);
INSERT INTO "main"."render_dependence" ("name", "version", "path","ext", "editable") VALUES ( 'element-plus', '2.2.2', 'https://unpkg.com/element-plus/dist/index.full.mjs','js',false);
INSERT INTO "main"."render_dependence" ("name", "version", "path","ext", "editable") VALUES ( '@element-plus/icons-vue', '2.3.2', 'https://cdn.jsdelivr.net/npm/@element-plus/icons-vue@2.3.2/+esm', 'js',false);
INSERT INTO "main"."render_dependence" ("name", "version", "path","ext", "editable") VALUES ( '@/render/Grant', '1.0.0', 'http://localhost:3000/public/components/Grant/index.js', 'js',false);
INSERT INTO "main"."render_dependence" ("name", "version", "path","ext", "editable") VALUES ( '@/render/GrantList', '1.0.0', 'http://localhost:3000/public/components/GrantList/index.js', 'js',false);
INSERT INTO "main"."render_dependence" ("name", "version", "path","ext", "editable") VALUES ( 'dayjs', '1.11.0', 'https://unpkg.com/dayjs@1.11.0/dayjs.min.js','js',false);


-- 初始化组件
INSERT INTO "main"."render_component" ("id", "name", "path", "annotation", "doc", "create_time", "author_type", "author_name", "preview_attrs") VALUES (1, 'AuthRequest', 'AuthRequest/index.js', '向用户申请指定权限', 'interface Attrs {
  // 授权码
  code: string;
  // 子授权码
  subCode?: string;
  // 申请用途
  purpose: string;
}
', '2026-05-18T12:26:36.995581600+08:00', 'system', 'system', '{"code":"0","purpose":"需要保存您的个人基本信息以提供个性化服务。"}');
INSERT INTO "main"."render_component" ("id", "name", "path", "annotation", "doc", "create_time", "author_type", "author_name", "preview_attrs") VALUES (2, 'ImageList', 'ImageList/index.js', '向用户展示一组图片', 'interface Image {
  //url或者本地文件路径
  path: string;
  name?: string;
}

interface Attrs {
  title?: string;
  images: Image[];
  width?: number;
  height?: number;
}
', '2026-05-18T14:06:23.154057600+08:00', 'system', 'system', '{"height":100,"images":[{"name":"鹿","path":"https://fuss10.elemecdn.com/a/3f/3302e58f9a181d2509f3dc0fa68b0jpeg.jpeg"},{"name":"马","path":"https://fuss10.elemecdn.com/1/34/19aa98b1fcb2781c4fba33d850549jpeg.jpeg"},{"name":"母狮","path":"https://fuss10.elemecdn.com/0/6f/e35ff375812e6b0020b6b4e8f9583jpeg.jpeg"},{"name":"傻狍子","path":"https://fuss10.elemecdn.com/9/bb/e27858e973f5d7d3904835f46abbdjpeg.jpeg"},{"name":"狐狸","path":"https://fuss10.elemecdn.com/d/e6/c4d93a3805b3ce3f323f7974e6f78jpeg.jpeg"},{"name":"雪狐","path":"https://fuss10.elemecdn.com/3/28/bbf893f792f03a54408b3b7a7ebf0jpeg.jpeg"},{"name":"猴","path":"https://fuss10.elemecdn.com/2/11/6535bcfb26e4c79b48ddde44f4b6fjpeg.jpeg"}],"title":"动物摄影","width":100}');
INSERT INTO "main"."render_component" ("id", "name", "path", "annotation", "doc", "create_time", "author_type", "author_name", "preview_attrs") VALUES (3, 'FileList', 'FileList/index.js', '向用户展示一组文件', 'interface File {
  url: string;
  name: string;
  description?: string; // 描述
  time?: string;        // 时间
  size?: string;        // 大小
}

interface Attrs {
  files: File[];
}
', '2026-05-18T14:07:31.231423500+08:00', 'system', 'system', '"null"');
