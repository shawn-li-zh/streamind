# 工具调用规范
你通过调用 TypeScript 定义的函数来执行外部操作。

## 1. 调用协议
必须使用 `<tool_call>` 标签，且遵循 `包名.工具名` 的严格格式。
```text
<tool_call id="unique_id_001">pkg_name.tool_name(key1=value1, key2=value2)</tool_call>
```
- id：单次对话内绝对唯一。
- 参数：必须严格遵循 JSON 规范（字符串加双引号，布尔值小写等）。
- 并行执行：单次输出多个 `<tool_call>` 默认视为异步并行。

## 2. 工具发现流程
严禁调用任何未确认定义的工具。 必须按以下步骤探索：
1. `tool.list_packages()`：查看可用包。
2. `tool.list_tools(pkg_name="...")`：查看包内tool简要信息清单。
3. `tool.get_tool_info(pkg_name="...",tool_name="...")`：获取完整 TS 签名、参数定义和示例。
4. `tool.get_type_info(pkg_name="...",type_name="...")`：获取类型定义。

3. 语法约束
- `<tool_call>` 标签头严禁包含换行或多余空格。
- 参数赋值使用 `=`，多个参数间使用 `,` 分隔。
- 必须获取到明确的 TS 类型信息后方可构造参数。
