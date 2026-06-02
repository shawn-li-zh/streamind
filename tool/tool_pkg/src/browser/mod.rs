mod action;
#[cfg(test)]
mod tests;
mod utils;

pub(crate) static JS: LazyLock<String> = LazyLock::new(|| {
    let path = Path::new(r"./public/dom_view.js");
    let dom_view = std::fs::read_to_string(path).expect("Failed to read file");
    let path = Path::new(r"./public/fingerprint_cheat.js");
    let fingerprint_cheat = std::fs::read_to_string(path).expect("Failed to read file");
    format!("{};{}", dom_view, fingerprint_cheat)
});
use std::{path::Path, sync::LazyLock};

pub use self::browser::build_package;
use tool_macro::package;
#[package("浏览器操作工具包")]
pub mod browser {
    use std::{
        collections::HashMap,
        io::Write,
        path::{self, Path, PathBuf},
        str::FromStr,
        sync::{Arc, LazyLock},
        time::Duration,
    };

    use chromiumoxide::{
        Browser, BrowserConfig, Page,
        cdp::{
            browser_protocol::{
                browser::{SetDownloadBehaviorBehavior, SetDownloadBehaviorParams},
                page::{CaptureScreenshotFormat, Frame, FrameTree, GetFrameTreeParams},
            },
            js_protocol::runtime::{EvaluateParams, ExecutionContextId},
        },
        handler::viewport::Viewport,
        page::ScreenshotParams,
    };
    use futures::StreamExt;
    use kernel::{
        agent::context::AgentContext,
        model::ModelModal,
        simple_id,
        tool::{ContextData, core::ToolReturn},
    };

    use anyhow::{Context as _, Ok};
    use chrono::Local;

    use kernel::tool::ToolCallContext;
    use serde::Serialize;

    use crate::browser::{
        JS,
        action::BrowserAction,
        utils::{self},
    };

    pub struct Context {
        pub is_runing: bool,
        pub tabs: HashMap<String, Tab>,
        pub browser: Option<Browser>,
        pub current_tab: Option<String>,
    }
    #[class("浏览器标签页")]
    #[derive(Debug, Clone, Serialize)]
    pub struct Tab {
        pub id: String,
        pub title: String,
        pub url: String,
        pub status: String,
        pub open_time: chrono::DateTime<Local>,
        //序列化忽略page
        #[serde(skip)]
        pub page: Page,
    }
    impl Tab {
        pub async fn get_frame_context_id(
            &self,
            iframe_id: Option<String>,
        ) -> anyhow::Result<Option<ExecutionContextId>> {
            match iframe_id {
                Some(id) => {
                    let id = self
                        .page
                        .frame_execution_context(id.into())
                        .await
                        .map_err(|e| anyhow::anyhow!("获取iframe执行环境失败: {e}"))?;
                    Ok(id)
                }
                None => Ok(None),
            }
        }
    }
    #[class("浏览器Iframe信息")]
    #[derive(Debug, Clone, Serialize)]
    pub struct Iframe {
        pub id: String,
        pub url: String,
        pub child_frames: Option<Vec<Iframe>>,
    }
    impl Iframe {
        pub fn from(tree: FrameTree) -> Self {
            let f = tree.frame;
            Iframe {
                id: f.id.inner().to_string(),
                url: f.url,
                child_frames: tree
                    .child_frames
                    .map(|c| c.into_iter().map(Iframe::from).collect()),
            }
        }
    }

    #[context]
    pub fn new_context(agent: Arc<AgentContext>) -> Context {
        Context {
            is_runing: false,
            tabs: HashMap::new(),
            browser: None,
            current_tab: None,
        }
    }
    #[shutdown]
    pub async fn shutdown(agent: Arc<AgentContext>, ctx: ContextData) -> anyhow::Result<()> {
        let mut ctx = ctx.write().await;
        let ctx = ctx.downcast_mut::<Context>().unwrap();
        if ctx.is_runing {
            let browser = ctx.browser.as_mut().unwrap();
            browser.close().await?;
        }
        Ok(())
    }
    #[tool(anno = "启动浏览器", example = r#"launch()"#)]
    pub async fn launch(
        #[arg("浏览器启动参数")] args: Option<Vec<String>>,
        #[ctx] ctx: ToolCallContext,
    ) -> anyhow::Result<()> {
        let chrome_path = std::env::var("CHROME_PATH")
            .map_err(|e| anyhow::anyhow!("请先设置浏览器执行文件地址"))?;
        let chrome_path = PathBuf::from_str(&chrome_path)?;
        if !chrome_path.exists() {
            anyhow::bail!("无效的浏览器执行文件地址");
        }
        let (user_dir, _) = ctx.get_real_path("browser/user_dir");
        if !user_dir.exists() {
            std::fs::create_dir_all(&user_dir)?;
        }
        let cur = std::env::current_dir().unwrap();
        let user_dir = cur.join(user_dir);
        let mut ctx = ctx.context_data.write().await;
        let ctx = ctx.downcast_mut::<Context>().unwrap();
        if ctx.is_runing {
            anyhow::bail!("浏览器已启动");
        }
        let view = Viewport {
            width: 1610,
            height: 880,
            device_scale_factor: Some(1.0), // 强制 1.0 比例，防止系统缩放导致偏移
            emulating_mobile: false,
            is_landscape: true,
            has_touch: false,
        };

        let config = BrowserConfig::builder()
            .user_data_dir(user_dir)
            .chrome_executable(chrome_path)
            .no_sandbox()
            .with_head() // 有头模式
            .args(args.unwrap_or_default())
            // 3. 强制设备缩放比为 1，解决高分屏偏移问题
            .arg("--force-device-scale-factor=1")
            // 4. 禁用一些可能改变 UI 布局的参数
            .arg("--disable-features=ViewsBrowser") // 某些版本下防止 UI 干扰
            .arg("--start-maximized")
            .viewport(view)
            .launch_timeout(Duration::from_secs(10))
            .build()
            .map_err(|e| anyhow::anyhow!("Browser Cdp:{e}"))?;

        let (browser, mut handler) = Browser::launch(config).await?;
        // 必须在一个单独的任务中运行 handler
        tokio::task::spawn(async move {
            while let Some(h) = handler.next().await {
                if h.is_err() {
                    break;
                }
            }
        });
        ctx.is_runing = true;
        ctx.browser = Some(browser);
        Ok(())
    }
    //关闭浏览器
    #[tool(anno = "关闭浏览器", example = r#"close()"#)]
    pub async fn close(#[ctx] ctx: ToolCallContext) -> anyhow::Result<()> {
        let mut ctx = ctx.context_data.write().await;
        let ctx = ctx.downcast_mut::<Context>().unwrap();
        if !ctx.is_runing {
            anyhow::bail!("浏览器未启动");
        }
        if let Some(browser) = ctx.browser.as_mut() {
            browser.close().await?;
        }
        ctx.is_runing = false;
        ctx.browser = None;
        ctx.tabs.clear();
        ctx.current_tab = None;
        Ok(())
    }
    #[tool(
        anno = "打开网页,等dom解析完毕,返回tab id",
        example = r#"new_tab(url="https://www.baidu.com")"#,
        timeout = "5s"
        limit = "10s:5"
    )]
    pub async fn new_tab(
        #[arg("网页地址")] url: String,
        #[ctx] ctx: ToolCallContext,
    ) -> anyhow::Result<ToolReturn> {
        let (downloads, _) = ctx.get_real_path("browser/downloads");
        if !downloads.exists() {
            std::fs::create_dir_all(&downloads)?;
        }
        let tab = {
            let ctx = ctx.context_data.read().await;
            let ctx = ctx.downcast_ref::<Context>().unwrap();
            if !ctx.is_runing {
                anyhow::bail!("浏览器未启动");
            }
            let browser = ctx.browser.as_ref().unwrap();

            let page = browser.new_page("about:blank").await?;

            page.evaluate_on_new_document(&*JS).await?;
            // 允许下载
            page.execute(
                SetDownloadBehaviorParams::builder()
                    .behavior(SetDownloadBehaviorBehavior::Allow)
                    .download_path(downloads.to_string_lossy().to_string()) // 指定路径
                    .build()
                    .unwrap(),
            )
            .await?;
            {
                let page = page.clone();
                let url = url.clone();
                page.goto(&url).await?;
            }
            tokio::time::timeout(Duration::from_secs(10), async {
                utils::wait_for_dom(&page, 100).await.unwrap();
                let js = format!("window['resolve_dom']()");
                let res = page.evaluate(js.as_str()).await?;
                let s = res.into_value::<Option<String>>()?;
                Ok(())
            })
            .await
            .map_err(|_| anyhow::anyhow!("加载网页超时"))??;

            Tab {
                id: simple_id(6),
                title: page.get_title().await?.unwrap_or_default(),
                url: url,
                status: "interactive".to_string(),
                open_time: Local::now(),
                page,
            }
        };
        let mut ctx = ctx.context_data.write().await;
        let ctx = ctx.downcast_mut::<Context>().unwrap();
        let id: String = tab.id.clone();
        ctx.tabs.insert(tab.id.clone(), tab);
        ctx.current_tab = Some(id.clone());
        Ok(ToolReturn::raw(format!("current tab id is '{}'", id)))
    }

    #[tool(
        anno = "切换标签页",
        example = r#"switch_tab(id="123")"#,
        limit = "10s:5"
    )]
    pub async fn switch_tab(
        #[arg("标签页id")] id: String,
        #[ctx] ctx: ToolCallContext,
    ) -> anyhow::Result<()> {
        let mut ctx = ctx.context_data.write().await;
        let ctx = ctx.downcast_mut::<Context>().unwrap();
        if !ctx.is_runing {
            anyhow::bail!("浏览器未启动");
        }
        if !ctx.tabs.contains_key(&id) {
            anyhow::bail!("标签页不存在");
        }
        ctx.current_tab = Some(id.clone());
        Ok(())
    }
    //关闭标签页
    #[tool(
        anno = "关闭标签页,返回打开的tab id",
        example = r#"close_tab(id="123")"#,
        limit = "10s:5"
    )]
    pub async fn close_tab(
        #[arg("标签页id")] id: String,
        #[ctx] ctx: ToolCallContext,
    ) -> anyhow::Result<ToolReturn> {
        let mut ctx = ctx.context_data.write().await;
        let ctx = ctx.downcast_mut::<Context>().unwrap();
        if !ctx.is_runing {
            anyhow::bail!("浏览器未启动");
        }
        if !ctx.tabs.contains_key(&id) {
            anyhow::bail!("标签页不存在");
        }
        let tab = ctx.tabs.remove(&id).unwrap();
        tab.page.close().await?;

        if ctx.current_tab != Some(id.clone()) {
            return Ok(ToolReturn::raw(format!(
                "current tab id is '{}'",
                ctx.current_tab.as_ref().unwrap()
            )));
        }
        if ctx.tabs.is_empty() {
            ctx.is_runing = false;
            ctx.browser = None;
            return Ok(ToolReturn::raw("browser closed"));
        }
        let tab = ctx.tabs.values().find(|_| true).unwrap();
        tab.page.bring_to_front().await?;
        Ok(ToolReturn::raw(format!("current tab id is '{}'", tab.id)))
    }

    //获取标签页列表
    #[tool(anno = "获取标签页列表", example = r#"list_tabs()"#, limit = "10s:5")]
    pub async fn list_tabs(#[ctx] ctx: ToolCallContext) -> anyhow::Result<Vec<Tab>> {
        let ctx = ctx.context_data.read().await;
        let ctx = ctx.downcast_ref::<Context>().unwrap();
        if !ctx.is_runing {
            anyhow::bail!("浏览器未启动");
        }
        let tabs = ctx.tabs.values().map(|v| v.clone()).collect();
        Ok(tabs)
    }
    //执行命令
    #[tool(
        anno = "执行js脚本,脚本的返回值必须可以被转换成string或者不返回值(最好不要用js来交互元素，容易触发反爬)",
        example = r#"exec(js="console.log(1+1)")"#,
        limit = "10s:5"
    )]
    pub async fn exec(
        #[arg("命令")] js: String,
        #[arg("iframe id,默认主网页")] iframe_id: Option<String>,
        #[ctx] ctx: ToolCallContext,
    ) -> anyhow::Result<ToolReturn> {
        let mut ctx = ctx.context_data.write().await;
        let ctx = ctx.downcast_mut::<Context>().unwrap();
        if !ctx.is_runing {
            anyhow::bail!("浏览器未启动");
        }
        if ctx.current_tab.is_none() {
            anyhow::bail!("请先打开标签页");
        }
        let tab = ctx.tabs.get_mut(ctx.current_tab.as_ref().unwrap()).unwrap();
        //
        let mut eval = EvaluateParams::new(js);
        eval.context_id = tab.get_frame_context_id(iframe_id).await?;
        let res = tab.page.evaluate(eval).await?;
        //
        let s = res.into_value::<Option<serde_json::Value>>()?;
        Ok(ToolReturn::json(s.unwrap_or_default())?)
    }
    #[tool(
        anno = "解析dom树，用在每次网页发生交互后，重新解析网页的节点树",
        example = r#"resolve_dom()"#,
        limit = "10s:5"
    )]
    pub async fn resolve_dom(
        #[arg(
            "每当页面发生变动，都需要重新解析元素，默认是document.body,也可以传入一段js代码,系统自动调用环境中的resolve_dom(ele?:HTMLElement)"
        )]
        element: Option<String>,
        #[arg("iframe id,默认主网页")] iframe_id: Option<String>,
        #[ctx] ctx: ToolCallContext,
    ) -> anyhow::Result<ToolReturn> {
        let mut ctx = ctx.context_data.write().await;
        let ctx = ctx.downcast_mut::<Context>().unwrap();
        if !ctx.is_runing {
            anyhow::bail!("浏览器未启动");
        }
        if ctx.current_tab.is_none() {
            anyhow::bail!("请先打开标签页");
        }
        let tab = ctx.tabs.get_mut(ctx.current_tab.as_ref().unwrap()).unwrap();
        let js = format!("window['resolve_dom']({})", element.unwrap_or_default());
        //
        let mut eval = EvaluateParams::new(js);
        eval.context_id = tab.get_frame_context_id(iframe_id).await?;
        let res = tab.page.evaluate(eval).await?;
        let s = res.into_value::<Option<String>>()?;
        Ok(ToolReturn::raw(format!("{}", s.unwrap_or_default())))
    }
    #[tool(
        anno = "查看dom树,返回精简后的节点树文档,如果区间内节点数大于100,会将相同层级相同标签的节点折叠omit(每一行都是一个节点,节点id在头部，格式=[id]),在js中通过node_element(node_id)可以访问对应的元素对象,)",
        example = r#"look_dom()"#,
        limit = "10s:5"
    )]
    pub async fn look_dom(
        #[arg("起始节点id,默认=0,代表body元素")] start: Option<u32>,
        #[arg("结束节点id")] end: Option<i32>,
        #[arg("iframe id,默认主网页")] iframe_id: Option<String>,
        #[ctx] ctx: ToolCallContext,
    ) -> anyhow::Result<ToolReturn> {
        let mut ctx = ctx.context_data.write().await;
        let ctx = ctx.downcast_mut::<Context>().unwrap();
        if !ctx.is_runing {
            anyhow::bail!("浏览器未启动");
        }
        if ctx.current_tab.is_none() {
            anyhow::bail!("请先打开标签页");
        }
        let tab = ctx.tabs.get_mut(ctx.current_tab.as_ref().unwrap()).unwrap();
        let start = start.unwrap_or_default();
        let end = end.unwrap_or(-1);
        let js = format!("window['look_dom']({},{})", start, end);
        //
        let mut eval = EvaluateParams::new(js);
        eval.context_id = tab.get_frame_context_id(iframe_id).await?;
        let res = tab.page.evaluate(eval).await?;
        let s = res.into_value::<Option<String>>()?;
        Ok(ToolReturn::raw(format!("{}", s.unwrap_or_default())))
    }
    #[tool(
        anno = "查看页面的iframe树",
        example = r#"look_iframes()"#,
        limit = "10s:5"
    )]
    pub async fn look_iframes(#[ctx] ctx: ToolCallContext) -> anyhow::Result<Iframe> {
        let mut ctx = ctx.context_data.write().await;
        let ctx = ctx.downcast_mut::<Context>().unwrap();
        if !ctx.is_runing {
            anyhow::bail!("浏览器未启动");
        }
        if ctx.current_tab.is_none() {
            anyhow::bail!("请先打开标签页");
        }
        let tab = ctx.tabs.get_mut(ctx.current_tab.as_ref().unwrap()).unwrap();
        let res = tab.page.execute(GetFrameTreeParams::default()).await?;
        let tree = res.frame_tree.clone();
        let root = Iframe::from(tree);
        Ok(root)
    }

    //截图
    #[tool(
        anno = "页面截图,返回图片路径(视觉模型返回图片内容)",
        example = r#"screenshot()"#,
        limit = "10s:5"
    )]
    pub async fn screenshot(#[ctx] call_ctx: ToolCallContext) -> anyhow::Result<ToolReturn> {
        let mut ctx = call_ctx.context_data.write().await;
        let ctx = ctx.downcast_mut::<Context>().unwrap();
        if !ctx.is_runing {
            anyhow::bail!("浏览器未启动");
        }
        if ctx.current_tab.is_none() {
            anyhow::bail!("请先打开标签页");
        }
        let (path, mut file) = call_ctx.new_temp_result_file("jpeg")?;
        let tab = ctx.tabs.get_mut(ctx.current_tab.as_ref().unwrap()).unwrap();
        let bytes = tab
            .page
            .screenshot(
                ScreenshotParams::builder()
                    .format(CaptureScreenshotFormat::Jpeg)
                    .quality(65)
                    .full_page(false)
                    .omit_background(true)
                    .build(),
            )
            .await?;
        file.write_all(&bytes)?;
        if call_ctx.agent_identity.modals.contains(&ModelModal::Image) {
            //只占1000,图片字节太多，免得计算上下文长度时出现一两句话就溢出了
            let image = ToolReturn::image(path.clone(), 1000, Some("网页截图".to_string()));
            Ok(image)
        } else {
            Ok(ToolReturn::raw(path.to_string_lossy().to_string()))
        }
    }
    //指令流水线
    #[tool(
        anno = r#"执行指令流水线,使用文档内容较多，请打开'系统工作目录+/prompts/browser_tool.doc.md' 文件查看具体使用方法。
        除了wait(ms=xxx)指令，其他指令超过3秒都会超时中断"#,
        example = r#"pipeline(cmds="node.click(id=1,x=100,y=100);node.type(id=1,text='hello world')")"#,
        limit = "10s:5"
    )]
    pub async fn pipeline(
        #[arg("指令流水线")] cmds: String,
        #[arg("指令分割符,默认是';'")] delimiter: Option<String>,
        // #[arg("iframe id,默认主网页")] iframe_id: Option<String>,
        #[ctx] call_ctx: ToolCallContext,
    ) -> anyhow::Result<()> {
        let mut actions = vec![];
        let delimiter = delimiter.unwrap_or_else(|| ";".into());
        let cmds: Vec<_> = cmds
            .split(&delimiter)
            .map(|v| v.trim())
            .filter(|v| !v.is_empty())
            .collect();
        for (i, s) in cmds.iter().enumerate() {
            let s = s.trim();
            if s.is_empty() {
                continue;
            }
            let action = BrowserAction::from_str(s)
                .map_err(|e| anyhow::anyhow!("第{}条指令解析错误: {}", i, e))?;
            actions.push(action);
        }

        let ctx = call_ctx.context_data.read().await;
        let ctx = ctx.downcast_ref::<Context>().unwrap();
        if !ctx.is_runing {
            anyhow::bail!("浏览器未启动");
        }
        if ctx.current_tab.is_none() {
            anyhow::bail!("请先打开标签页");
        }
        let tab = ctx.tabs.get(ctx.current_tab.as_ref().unwrap()).unwrap();
        for (i, action) in actions.into_iter().enumerate() {
            match &action {
                BrowserAction::Wait(_) => action.execute(&tab.page).await?,
                _ => {
                    let _ = tokio::time::timeout(Duration::from_secs(3), async move {
                        action.execute(&tab.page).await?;
                        Ok(())
                    })
                    .await
                    .map_err(|_| anyhow::anyhow!("执行超时:{},可能元素发生变化,请刷新页面,或者调用resolve_dom重新解析网页", &cmds[i]))??;
                }
            }
        }
        Ok(())
    }
}
