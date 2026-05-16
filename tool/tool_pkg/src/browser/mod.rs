pub use self::browser::build_package;
use tool_macro::package;
#[package("浏览器操作工具包")]
pub mod browser {
    use std::{
        collections::HashMap,
        io::Write,
        path::{self, PathBuf},
        str::FromStr,
        sync::Arc,
        time::Duration,
    };

    use chromiumoxide::{
        Browser, BrowserConfig, Page,
        cdp::browser_protocol::{
            browser::{SetDownloadBehaviorBehavior, SetDownloadBehaviorParams},
            page::CaptureScreenshotFormat,
        },
        page::ScreenshotParams,
    };
    use futures::StreamExt;
    use kernel::{
        agent::{AgentContext, AgentInfo},
        simple_id,
        tool::{ContextData, core::ToolReturn},
    };

    use anyhow::{Context as _, Ok};
    use chrono::Local;

    use kernel::tool::ToolCallContext;
    use serde::Serialize;
    use tokio::sync::RwLock;
    use tracing::error;

    use crate::browser::utils::{self, BrowserAction, get_available_port};

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
    #[context]
    pub fn new_context(agent: AgentContext) -> Context {
        Context {
            is_runing: false,
            tabs: HashMap::new(),
            browser: None,
            current_tab: None,
        }
    }
    #[shutdown]
    pub async fn shutdown(agent: AgentContext, ctx: ContextData) -> anyhow::Result<()> {
        let mut ctx = ctx.write().await;
        let ctx = ctx.downcast_mut::<Context>().unwrap();
        if ctx.is_runing {
            let browser = ctx.browser.as_mut().unwrap();
            browser.close().await?;
        }
        Ok(())
    }
    #[tool(anno = "启动浏览器", example = r#"lunch()"#)]
    pub async fn lunch(#[ctx] ctx: ToolCallContext) -> anyhow::Result<()> {
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

        let mut ctx = ctx.context_data.write().await;
        let ctx = ctx.downcast_mut::<Context>().unwrap();
        if ctx.is_runing {
            anyhow::bail!("浏览器已启动");
        }
        // let port = get_available_port().unwrap_or(19999);
        let config = BrowserConfig::builder()
            .user_data_dir(user_dir)
            .chrome_executable(chrome_path)
            .no_sandbox() // Windows 有时需要这个
            .with_head()
            // .port(port)
            .hide()
            .args(vec![
                "--disable-extensions",     // 禁用扩展
                "--disable-popup-blocking", // 禁用弹窗拦截
                "--disable-notifications",  // 禁用通知
                "--disable-translate",      // 禁用翻译
                "--disable-infobars",       // 禁用信息栏
                                            // "--disable-dev-shm-usage",         // 禁用共享内存
                                            // "--disable-gpu",                   // 禁用 GPU
                                            // "--disable-software-rasterizer",   // 禁用软件渲染器
                                            // "--disable-default-apps",          // 禁用默认应用
                                            // "--disable-background-networking", // 禁用后台网络
            ])
            .arg("--start-maximized") // 最大化
            // 隐藏“正受自动测试软件控制”提示条
            .arg("--exclude-switches=enable-automation")
            // 屏蔽其他干扰弹窗
            .arg("--no-first-run") // 跳过首次运行设置
            .arg("--no-default-browser-check") // 不检查是否为默认浏览器
            .launch_timeout(Duration::from_secs(5))
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
        let browser = ctx.browser.as_mut().unwrap();
        browser.close().await?;
        ctx.is_runing = false;
        ctx.browser = None;
        ctx.tabs.clear();
        ctx.current_tab = None;
        Ok(())
    }
    #[tool(
        anno = "打开网页,等dom解析完毕,返回tab id",
        example = r#"new_tab(url="https://www.baidu.com")"#,
        timeout = "10s"
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
                tokio::spawn(async move {
                    if let Err(page) = page.goto(&url).await {
                        error!("page goto error:{}", page);
                    };
                });
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
            utils::wait_for_dom(&page, "interactive", 100).await?;
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
        anno = "执行js脚本,脚本的返回值必须可以被转换成string或者不返回值",
        example = r#"exec(js="console.log(1+1)")"#,
        limit = "10s:5"
    )]
    pub async fn exec(
        #[arg("命令")] js: String,
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
        let res = tab.page.evaluate(js.as_str()).await?;
        let s = res.into_value::<Option<String>>()?;
        Ok(ToolReturn::raw(format!("{}", s.unwrap_or_default())))
    }
    #[tool(
        anno = "查看dom,返回精简后的节点树文档(每一行都是一个节点,节点id在头部，格式=[id]),在js中通过node_element(node_id)可以访问对应的元素对象",
        example = r#"look_dom()"#,
        limit = "10s:5"
    )]
    pub async fn look_dom(#[ctx] ctx: ToolCallContext) -> anyhow::Result<ToolReturn> {
        let mut ctx = ctx.context_data.write().await;
        let ctx = ctx.downcast_mut::<Context>().unwrap();
        if !ctx.is_runing {
            anyhow::bail!("浏览器未启动");
        }
        if ctx.current_tab.is_none() {
            anyhow::bail!("请先打开标签页");
        }
        let tab = ctx.tabs.get_mut(ctx.current_tab.as_ref().unwrap()).unwrap();
        let res = tab.page.evaluate("window['STREAMIND'].body()").await?;
        let s = res.into_value::<Option<String>>()?;
        Ok(ToolReturn::raw(format!("{}", s.unwrap_or_default())))
    }
    //截图
    #[tool(
        anno = "页面截图,返回图片路径",
        example = r#"screenshot()"#,
        limit = "10s:5"
    )]
    pub async fn screenshot(#[ctx] call_ctx: ToolCallContext) -> anyhow::Result<String> {
        let mut ctx = call_ctx.context_data.write().await;
        let ctx = ctx.downcast_mut::<Context>().unwrap();
        if !ctx.is_runing {
            anyhow::bail!("浏览器未启动");
        }
        if ctx.current_tab.is_none() {
            anyhow::bail!("请先打开标签页");
        }
        let (path, mut file) = call_ctx.new_temp_result_file("png")?;
        let tab = ctx.tabs.get_mut(ctx.current_tab.as_ref().unwrap()).unwrap();
        let bytes = tab
            .page
            .screenshot(
                ScreenshotParams::builder()
                    .format(CaptureScreenshotFormat::Png)
                    .full_page(true)
                    .omit_background(true)
                    .build(),
            )
            .await?;
        file.write_all(&bytes)?;
        let path = call_ctx.to_short_path(&path);
        Ok(path.to_string_lossy().to_string())
    }
    //指令流水线
    #[tool(
        anno = "执行指令流水线",
        example = r#"pipeline(cmds="node.click(x=100,y=100);")"#,
        limit = "10s:5"
    )]
    pub async fn pipeline(
        #[arg("指令流水线")] cmds: String,
        #[ctx] call_ctx: ToolCallContext,
    ) -> anyhow::Result<()> {
        let mut actions = vec![];
        for (i, s) in cmds.split(";").enumerate() {
            let action =
                BrowserAction::from_str(s).with_context(|| format!("第{}条指令解析错误", i))?;
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
        for action in actions {
            action.execute(&tab.page).await?;
        }
        Ok(())
    }
}

mod utils {
    use std::{collections::HashMap, net::TcpListener, str::FromStr, time::Duration};

    use anyhow::Context;
    use chromiumoxide::{
        Element, Page,
        cdp::browser_protocol::{
            input::{
                DispatchKeyEventParams, DispatchKeyEventParamsBuilder, DispatchKeyEventType,
                DispatchMouseEventParams, DispatchMouseEventType, MouseButton,
            },
            page,
        },
        error::CdpError,
        keys,
        layout::Point,
    };
    use kernel::tool::parser::parse_call;
    use tokio::time::sleep;

    use crate::browser::utils;

    pub fn get_available_port() -> Option<u16> {
        // 绑定到 127.0.0.1 的 0 端口
        // 操作系统会自动分配一个空闲端口
        TcpListener::bind("127.0.0.1:0")
            .and_then(|listener| listener.local_addr())
            .map(|addr| addr.port())
            .ok()
    }

    pub enum MouseEvent {
        Click(f64, f64, u32, MouseButton),
        LongClick(f64, f64, u64, MouseButton),
        Wheel(f64, f64),
        Move(f64, f64),
    }
    impl MouseEvent {
        pub async fn execute(self, page: &Page) -> anyhow::Result<()> {
            match self {
                MouseEvent::Click(x, y, count, btn) => {
                    utils::click(page, Point { x, y }, count, btn).await?;
                }
                MouseEvent::LongClick(x, y, duration, btn) => {
                    utils::long_click(page, Point { x, y }, duration, btn).await?;
                }
                MouseEvent::Wheel(x, y) => {
                    utils::scroll(page, x, y).await?;
                }
                MouseEvent::Move(x, y) => {
                    //@todo 优化移动轨迹
                    page.move_mouse(Point { x, y }).await?;
                }
            }
            Ok(())
        }
        //格式：click(x,y,count,btn)
        pub fn from_str(
            name: &str,
            mut args: HashMap<String, serde_json::Value>,
        ) -> anyhow::Result<MouseEvent> {
            let x = args
                .remove("x")
                .and_then(|v| v.as_f64())
                .context("x必填且必须是数字")?;
            let y = args
                .remove("y")
                .and_then(|v| v.as_f64())
                .context("y必填且必须是数字")?;
            //
            match name {
                "click" => {
                    let count = args.remove("count").and_then(|v| v.as_u64()).unwrap_or(1);

                    let button = args
                        .remove("button")
                        .and_then(|v| v.as_str().map(|s| s.to_string()))
                        .map(|s| {
                            MouseButton::from_str(&s)
                                .map_err(|_| anyhow::anyhow!("解析鼠标按钮失败: {}", s))
                        })
                        .transpose()?
                        .unwrap_or(MouseButton::Left);
                    Ok(MouseEvent::Click(x, y, count as u32, button))
                }
                "longClick" => {
                    let duration = args
                        .remove("ms")
                        .and_then(|v| v.as_u64())
                        .context("长按ms时长必填")?;

                    let button = args
                        .remove("button")
                        .and_then(|v| v.as_str().map(|s| s.to_string()))
                        .map(|s| {
                            MouseButton::from_str(&s)
                                .map_err(|_| anyhow::anyhow!("解析鼠标按钮失败: {}", s))
                        })
                        .transpose()?
                        .unwrap_or(MouseButton::Left);
                    Ok(MouseEvent::LongClick(x, y, duration, button))
                }
                "wheel" => Ok(MouseEvent::Wheel(x, y)),
                "move" => Ok(MouseEvent::Move(x, y)),
                _ => {
                    anyhow::bail!("invalid mouse event");
                }
            }
        }
    }
    pub enum KeyboardEvent {
        Press(String),
        LongPress(String, u64),
    }
    impl KeyboardEvent {
        pub async fn execute(self, page: &Page) -> anyhow::Result<()> {
            match self {
                KeyboardEvent::Press(key) => {
                    utils::press_key(page, key).await?;
                }
                KeyboardEvent::LongPress(key, ms) => {
                    utils::long_press_key(page, &key, ms).await?;
                }
            }
            Ok(())
        }
        //格式：press(key)
        pub fn from_str(
            name: &str,
            mut args: HashMap<String, serde_json::Value>,
        ) -> anyhow::Result<KeyboardEvent> {
            let code = args
                .remove("code")
                .and_then(|v| v.as_str().map(|s| s.to_string()))
                .context("keyboard code必填")?;

            //
            match name {
                "press" => Ok(KeyboardEvent::Press(code)),
                "longPress" => {
                    let duration = args
                        .remove("ms")
                        .and_then(|v| v.as_u64())
                        .context("长按ms时长必填")?;
                    Ok(KeyboardEvent::LongPress(code, duration))
                }
                _ => {
                    anyhow::bail!("invalid keyboard event");
                }
            }
        }
    }
    pub enum NodeEvent {
        Click(u64, u32, MouseButton),
        Hover(u64),
        Type(u64, String),
    }
    impl NodeEvent {
        pub async fn execute(self, page: &Page) -> anyhow::Result<()> {
            match self {
                NodeEvent::Click(id, count, btn) => {
                    let ele = utils::node_element(page, id).await?;
                    let point = ele.scroll_into_view().await?.clickable_point().await?;
                    utils::click(page, point, count, btn).await?;
                }
                NodeEvent::Hover(id) => {
                    let ele = utils::node_element(page, id).await?;
                    ele.hover().await?;
                }
                NodeEvent::Type(id, text) => {
                    let ele = utils::node_element(page, id).await?;
                    ele.type_str(text).await?;
                }
            }
            Ok(())
        }
        //格式：click(id,count,btn)
        pub fn from_str(
            name: &str,
            mut args: HashMap<String, serde_json::Value>,
        ) -> anyhow::Result<NodeEvent> {
            let id = args
                .remove("code")
                .and_then(|v| v.as_u64())
                .context("node id必填")?;
            //
            match name {
                "click" => {
                    let count = args.remove("count").and_then(|v| v.as_u64()).unwrap_or(1);

                    let button = args
                        .remove("button")
                        .and_then(|v| v.as_str().map(|s| s.to_string()))
                        .map(|s| {
                            MouseButton::from_str(&s)
                                .map_err(|_| anyhow::anyhow!("解析鼠标按钮失败: {}", s))
                        })
                        .transpose()?
                        .unwrap_or(MouseButton::Left);
                    Ok(NodeEvent::Click(id, count as _, button))
                }
                "hover" => Ok(NodeEvent::Hover(id)),
                "type" => {
                    let text = args
                        .remove("text")
                        .and_then(|v| v.as_str().map(|s| s.to_string()))
                        .context("text输入内容必填")?;

                    Ok(NodeEvent::Type(id, text))
                }
                _ => {
                    anyhow::bail!("invalid node event");
                }
            }
        }
    }

    pub enum BrowserAction {
        Node(NodeEvent),
        Mouse(MouseEvent),
        Keyboard(KeyboardEvent),
        Wait(u64),
    }
    impl BrowserAction {
        pub async fn execute(self, page: &Page) -> anyhow::Result<()> {
            match self {
                BrowserAction::Node(node) => node.execute(page).await,
                BrowserAction::Mouse(browser) => browser.execute(page).await,
                BrowserAction::Keyboard(keyboard) => keyboard.execute(page).await,
                BrowserAction::Wait(ms) => {
                    sleep(Duration::from_millis(ms)).await;
                    Ok(())
                }
            }
        }
        //格式：node.click(id,count,btn) 或者 wait(ms)
        pub fn from_str(s: &str) -> anyhow::Result<BrowserAction> {
            let (p, f, mut args) = parse_call(s)?;
            if s.starts_with("node.") {
                let event = NodeEvent::from_str(f, args)?;
                Ok(BrowserAction::Node(event))
            } else if s.starts_with("mouse.") {
                let event = MouseEvent::from_str(f, args)?;
                Ok(BrowserAction::Mouse(event))
            } else if s.starts_with("keyboard.") {
                let event = KeyboardEvent::from_str(f, args)?;
                Ok(BrowserAction::Keyboard(event))
            } else if f == "wait" {
                let duration = args
                    .remove("ms")
                    .and_then(|v| v.as_u64())
                    .context("长按ms时长必填")?;

                Ok(BrowserAction::Wait(duration))
            } else {
                anyhow::bail!("invalid action");
            }
        }
    }
    pub async fn node_element(page: &Page, id: u64) -> anyhow::Result<Element> {
        let selector = format!("[streamind-id='{}']", id);
        let a = page.find_element(selector).await?;
        Ok(a)
    }
    fn press_key_params(key: impl AsRef<str>) -> anyhow::Result<DispatchKeyEventParamsBuilder> {
        let key = key.as_ref();
        let key_definition: &keys::KeyDefinition = keys::get_key_definition(key)
            .ok_or_else(|| CdpError::msg(format!("Key not found: {key}")))?;
        let mut cmd: DispatchKeyEventParamsBuilder = DispatchKeyEventParams::builder();

        // See https://github.com/GoogleChrome/puppeteer/blob/62da2366c65b335751896afbb0206f23c61436f1/lib/Input.js#L114-L115
        // And https://github.com/GoogleChrome/puppeteer/blob/62da2366c65b335751896afbb0206f23c61436f1/lib/Input.js#L52
        let key_down_event_type: DispatchKeyEventType = if let Some(txt) = key_definition.text {
            cmd = cmd.text(txt);
            DispatchKeyEventType::KeyDown
        } else if key_definition.key.len() == 1 {
            cmd = cmd.text(key_definition.key);
            DispatchKeyEventType::KeyDown
        } else {
            DispatchKeyEventType::RawKeyDown
        };

        cmd = cmd
            .r#type(key_down_event_type)
            .key(key_definition.key)
            .code(key_definition.code)
            .windows_virtual_key_code(key_definition.key_code)
            .native_virtual_key_code(key_definition.key_code);
        Ok(cmd)
    }

    pub async fn click(
        page: &Page,
        point: Point,
        count: u32,
        btn: MouseButton,
    ) -> anyhow::Result<()> {
        let cmd = DispatchMouseEventParams::builder()
            .x(point.x)
            .y(point.y)
            .button(btn)
            .click_count(count);

        page.move_mouse(point)
            .await?
            .execute(
                cmd.clone()
                    .r#type(DispatchMouseEventType::MousePressed)
                    .build()
                    .unwrap(),
            )
            .await?;
        page.execute(
            cmd.r#type(DispatchMouseEventType::MouseReleased)
                .build()
                .unwrap(),
        )
        .await?;
        Ok(())
    }
    pub async fn scroll(page: &Page, x: f64, y: f64) -> anyhow::Result<()> {
        page.execute(
            DispatchMouseEventParams::builder()
                .delta_x(x)
                .delta_y(y)
                .r#type(DispatchMouseEventType::MouseWheel)
                .build()
                .unwrap(),
        )
        .await?;
        Ok(())
    }
    pub async fn press_key(page: &Page, key: impl AsRef<str>) -> anyhow::Result<()> {
        let cmd = press_key_params(key)?;

        page.execute(cmd.clone().build().unwrap()).await?;
        page.execute(cmd.r#type(DispatchKeyEventType::KeyUp).build().unwrap())
            .await?;
        Ok(())
    }
    pub async fn long_press_key(
        page: &Page,
        key: impl AsRef<str>,
        duration_ms: u64,
    ) -> anyhow::Result<()> {
        let cmd = press_key_params(key)?;
        page.execute(cmd.clone().build().unwrap()).await?;
        let page = page.clone();
        tokio::spawn(async move {
            // 2. 保持状态 (这就是“长按”的持续时间)
            sleep(Duration::from_millis(duration_ms)).await;

            // 3. 发送 keyUp (告知浏览器键被松开了)
            page.execute(cmd.r#type(DispatchKeyEventType::KeyUp).build().unwrap())
                .await
                .unwrap();
        });
        Ok(())
    }

    pub async fn long_click(
        page: &Page,
        point: Point,
        duration_ms: u64,
        btn: MouseButton,
    ) -> anyhow::Result<()> {
        let cmd = DispatchMouseEventParams::builder()
            .x(point.x)
            .y(point.y)
            .button(btn)
            .click_count(1);

        page.move_mouse(point)
            .await?
            .execute(
                cmd.clone()
                    .r#type(DispatchMouseEventType::MousePressed)
                    .build()
                    .unwrap(),
            )
            .await?;
        let page = page.clone();
        tokio::spawn(async move {
            // 2. 保持状态 (这就是“长按”的持续时间)
            tokio::time::sleep(std::time::Duration::from_millis(duration_ms)).await;

            page.execute(
                cmd.clone()
                    .r#type(DispatchMouseEventType::MouseReleased)
                    .build()
                    .unwrap(),
            )
            .await
            .unwrap();
        });
        Ok(())
    }
    pub async fn wait_for_dom(page: &Page, status: &str, millis: u64) -> anyhow::Result<()> {
        loop {
            let ready_state = page
                .evaluate("document.readyState")
                .await?
                .into_value::<String>()?;
            if ready_state.as_str() == status {
                return Ok(());
            }
            tokio::time::sleep(std::time::Duration::from_millis(millis)).await;
        }
    }
}
#[cfg(test)]
mod test {
    use std::{path::PathBuf, str::FromStr, time::Duration};

    use chromiumoxide::{Browser, BrowserConfig};
    use futures::StreamExt as _;

    #[tokio::test]
    pub async fn test_cdp() -> anyhow::Result<()> {
        let chrome_path = "C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe";
        let chrome_path = PathBuf::from_str(&chrome_path)?;
        if !chrome_path.exists() {
            anyhow::bail!("无效的浏览器执行文件地址");
        }
        let config = BrowserConfig::builder()
            .user_data_dir(r"D:\project\rust\streamind\tempb")
            .chrome_executable(chrome_path)
            .no_sandbox() // Windows 有时需要这个
            .with_head()
            .launch_timeout(Duration::from_secs(5))
            .build()
            .unwrap();
        let (browser, mut handler) = Browser::launch(config).await?;
        // 必须在一个单独的任务中运行 handler
        tokio::task::spawn(async move {
            while let Some(h) = handler.next().await {
                if h.is_err() {
                    break;
                }
            }
        });
        browser
            .new_page("https://element.eleme.cn/#/zh-CN/component/card")
            .await?;
        Ok(())
    }
}
