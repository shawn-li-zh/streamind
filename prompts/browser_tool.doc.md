
# 浏览器指令流水线 (Pipeline) 操作规范

## 1. 工具概述
`pipeline` 工具允许你在单个工具调用中发送一系列有序的浏览器操作。这比多次调用单个工具效率更高，且能确保操作的连贯性。

*   **参数名称**: `cmds` (String)
*   **语法规则**: 指令之间使用分号 `;` 分隔。
*   **坐标系**: 屏幕左上角为 (0, 0)。

---

## 2. 指令定义详解

指令分为四大类：**节点操作 (node)**、**鼠标操作 (mouse)**、**键盘操作 (keyboard)** 和 **等待 (wait)**。

### A. 节点操作 (Node Actions)
节点操作依赖于 `look_dom()` 返回的 `[id]`。在调用这些指令前，必须先通过 `look_dom` 获取目标节点的 ID。

| 指令语法 | 参数说明 | 示例 |
| :--- | :--- | :--- |
| `node.click(id=ID, count=N, button="BTN")` | `id`: 节点ID (必填)<br>`count`: 点击次数 (默认1)<br>`button`: "Left"/"Right"/"Middle" (默认Left) | `node.click(id=101);` |
| `node.hover(id=ID)` | `id`: 节点ID (必填) | `node.hover(id=102);` |
| `node.type(id=ID, text="STR")` | `id`: 节点ID (必填)<br>`text`: 要输入的文本内容 | `node.type(id=103, text="Rust Programming");` |

### B. 鼠标操作 (Mouse Actions)
直接基于屏幕坐标进行操作。

| 指令语法 | 参数说明 | 示例 |
| :--- | :--- | :--- |
| `mouse.click(x=X, y=Y, count=N, button="BTN")` | `x, y`: 坐标 (必填)<br>`count`: 次数<br>`button`: 左右中键 | `mouse.click(x=500, y=300);` |
| `mouse.longClick(x=X, y=Y, ms=MS, button="BTN")` | `ms`: 长按持续毫秒数 (必填) | `mouse.longClick(x=100, y=100, ms=2000);` |
| `mouse.move(x=X, y=Y)` | 移动鼠标到指定位置 | `mouse.move(x=800, y=600);` |
| `mouse.wheel(x=X, y=Y)` | 滚动滚轮。`x`: 水平滚动, `y`: 垂直滚动 | `mouse.wheel(x=0, y=500);` |

### C. 键盘操作 (Keyboard Actions)

| 指令语法 | 参数说明 | 示例 |
| :--- | :--- | :--- |
| `keyboard.press(code="KEY")` | `code`: 键名 (如 "Enter", "Tab", "a") | `keyboard.press(code="Enter");` |
| `keyboard.longPress(code="KEY", ms=MS)` | `ms`: 持续毫秒数 | `keyboard.longPress(code="Control", ms=1000);` |

### D. 等待指令 (Wait)
用于处理页面动态加载或动画。

| 指令语法 | 参数说明 | 示例 |
| :--- | :--- | :--- |
| `wait(ms=MS)` | `ms`: 等待毫秒数 | `wait(ms=1000);` |

---

## 3. 组合示例

### 场景 1：搜索并提交
先在搜索框输入文字，然后按回车键。
```text
browser.pipeline(cmds='node.type(id=42, text="OpenAI"); wait(ms=200); keyboard.press(id="Enter");')
```

### 场景 2：模拟复杂交互（悬停后点击）
先移动到菜单触发悬停效果，等待菜单弹出，然后点击子选项。
```text
browser.pipeline(cmds='node.hover(id=15); wait(ms=500); node.click(id=18);')
```

### 场景 3：表单填充与滚动
输入信息，向下滚动页面以显露提交按钮，最后点击提交。
```text
browser.pipeline(cmds='node.type(id=10, text="UserA"); mouse.wheel(x=0, y=300); wait(ms=300); node.click(id=25);')
```

---

## 4. 注意事项 (Tips for Agent)
1.  **ID 实时性**: 节点 ID (`id`) 是通过 `look_dom()` 获取的。如果页面发生了跳转或刷新，之前的 ID 会失效，必须重新调用 `look_dom()`。
2.  **合理使用 Wait**: 在执行 `click` 或 `type` 之后，如果后续操作依赖于页面的反馈（如弹窗弹出），请务必加入 `wait(ms=300)` 或更长时间。
3.  **语法校验**: 确保每个小指令后面都有分号 `;`，参数必须使用 `key=value` 的格式。
4.  **坐标确认**: 如果使用 `mouse` 操作，请确保坐标在当前视口范围内（默认 1920x1080）。