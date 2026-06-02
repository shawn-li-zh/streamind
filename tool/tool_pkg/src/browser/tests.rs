use std::{
    path::{Path, PathBuf},
    str::FromStr,
    sync::LazyLock,
    time::Duration,
};

use anyhow::Context as _;
use chromiumoxide::{
    Browser, BrowserConfig, Page,
    cdp::browser_protocol::browser::{SetDownloadBehaviorBehavior, SetDownloadBehaviorParams},
    handler::viewport::Viewport,
};
use futures::StreamExt as _;
use tracing::error;

use crate::browser::{
    self,
    action::BrowserAction,
    utils::{self},
};
static JS: LazyLock<String> = LazyLock::new(|| {
    let path = Path::new(r"D:\project\rust\streamind/public/dom_view.js");
    let dom_view = std::fs::read_to_string(path).expect("Failed to read file");
    let path = Path::new(r"D:\project\rust\streamind/public/fingerprint_cheat.js");
    let fingerprint_cheat = std::fs::read_to_string(path).expect("Failed to read file");
    format!("{};{}", dom_view, fingerprint_cheat)
});
#[tokio::test]
pub async fn test_cdp() -> anyhow::Result<()> {
    let chrome_path = "C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe";
    let chrome_path = PathBuf::from_str(&chrome_path)?;
    if !chrome_path.exists() {
        anyhow::bail!("无效的浏览器执行文件地址");
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
        .user_data_dir(r"D:\project\rust\streamind\tempb")
        .chrome_executable(chrome_path)
        .no_sandbox() // Windows 有时需要这个
        .with_head()
        // 3. 强制设备缩放比为 1，解决高分屏偏移问题
        .arg("--force-device-scale-factor=1")
        // 4. 禁用一些可能改变 UI 布局的参数
        .arg("--disable-features=ViewsBrowser") // 某些版本下防止 UI 干扰
        .arg("--start-maximized")
        .viewport(view)
        .launch_timeout(Duration::from_secs(5))
        .build()
        .unwrap();
    let (mut browser, mut handler) = Browser::launch(config).await?;
    // 必须在一个单独的任务中运行 handler
    tokio::task::spawn(async move {
        while let Some(h) = handler.next().await {
            if h.is_err() {
                break;
            }
        }
    });
    let page = new_tab("https://www.taobao.com/".to_string(), &browser).await?;
    if let Ok(res) = page.evaluate("window['STREAMIND']?.look_dom()").await {
        if let Ok(s) = res.into_value::<Option<String>>() {
            println!("body: {:?}", s);
        };
    } else {
        println!("没有注入成功");
    }
    tokio::time::sleep(Duration::from_secs(5000)).await;
    browser.close().await?;

    Ok(())
}

async fn new_tab(url: String, browser: &Browser) -> anyhow::Result<Page> {
    let downloads = format!("D:\\project\\rust\\streamind\\tempb\\downloads");
    let page = browser.new_page("about:blank").await?;
    page.evaluate_on_new_document(&*JS).await?;
    // 允许下载
    page.execute(
        SetDownloadBehaviorParams::builder()
            .behavior(SetDownloadBehaviorBehavior::Allow)
            .download_path(downloads) // 指定路径
            .build()
            .unwrap(),
    )
    .await?;
    {
        let page = page.clone();
        let url = url.clone();
        if let Err(page) = page.goto(&url).await {
            error!("page goto error:{}", page);
        };
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
    if let Err(e) = tokio::time::timeout(Duration::from_secs(10), async {
        utils::wait_for_dom(&page, 100).await.unwrap();
    })
    .await
    {
        error!("timeout:{}", e);
    };
    println!("页面加载完成");
    Ok(page)
}
#[test]
pub fn pipeline() -> anyhow::Result<()> {

    let cmds = "node.click(id=43);wait(ms=300);node.type(id=43,text='迷你主机');wait(ms=300);node.click(id=38);wait(ms=2000);";
    let mut actions = vec![];
    for (i, s) in cmds.split(";").enumerate() {
        let s = s.trim();
        if s.is_empty() {
            continue;
        }
        let action = BrowserAction::from_str(s.trim())
            .map_err(|e| anyhow::anyhow!("第{}条指令解析错误: {}", i, e))?;
        actions.push(action);
    }

    print!("{:?}", actions);
    Ok(())
}
