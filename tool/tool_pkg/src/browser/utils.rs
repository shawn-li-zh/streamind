
use std::time::Duration;

use anyhow::Context;
use chromiumoxide::{
    Element, Page,
    cdp::browser_protocol::input::{
        DispatchKeyEventParams, DispatchKeyEventParamsBuilder, DispatchKeyEventType,
        DispatchMouseEventParams, DispatchMouseEventType, MouseButton,
    },
    error::CdpError,
    keys,
    layout::Point,
};

use tokio::time::sleep;

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

pub async fn click(page: &Page, point: Point, count: u32, btn: MouseButton) -> anyhow::Result<()> {
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
pub async fn wait_for_dom(page: &Page, millis: u64) -> anyhow::Result<()> {
    loop {
        let ready_state = page
            .evaluate("document.readyState")
            .await?
            .into_value::<Option<String>>()?;
        if let Some(ready_state) = ready_state {
            if !ready_state.is_empty() && &ready_state != "loading" {
                return Ok(());
            }
        }
        tokio::time::sleep(std::time::Duration::from_millis(millis)).await;
    }
}
