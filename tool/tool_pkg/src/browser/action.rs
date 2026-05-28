use std::{collections::HashMap, str::FromStr as _, time::Duration};

use anyhow::Context as _;
use chromiumoxide::{Page, cdp::browser_protocol::input::MouseButton, layout::Point};
use kernel::parser::parse_call;
use tokio::time::sleep;

use crate::browser::utils;

#[derive(Debug, Clone)]
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
#[derive(Debug, Clone)]
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
#[derive(Debug, Clone)]
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
            .remove("id")
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
#[derive(Debug, Clone)]
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
        let p = p.unwrap_or_default().to_lowercase();
        match p.as_str() {
            "node" => {
                let event = NodeEvent::from_str(f, args)?;
                Ok(BrowserAction::Node(event))
            }
            "keyboard" => {
                let event = KeyboardEvent::from_str(f, args)?;
                Ok(BrowserAction::Keyboard(event))
            }
            "mouse" => {
                let event = MouseEvent::from_str(f, args)?;
                Ok(BrowserAction::Mouse(event))
            }
            "" => {
                if f == "wait" {
                    let duration = args
                        .remove("ms")
                        .and_then(|v| v.as_u64())
                        .context("长按ms时长必填")?;

                    return Ok(BrowserAction::Wait(duration));
                }
                anyhow::bail!("invalid action");
            }
            _ => {
                anyhow::bail!("invalid action");
            }
        }
    }
}
