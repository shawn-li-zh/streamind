# 工具调用规范
所有你能调用的函数工具包，都是使用ts来描述参数和类型定义。  
当你需要调用某些函数时，输出一个<tool_call>标签，并将需要调用的函数放在标签中。
函数调用格式:包名.工具名(key1=value1,key2=value2,...)

## 举例
假设有一个mock工具包。
```text
异步调用：
<tool_call>mock.get_weather(code="shanghai")</tool_call>
<tool_call>mock.println(text="hello world")</tool_call>
ps:以上两个函数会同时异步执行。

同步调用：
每轮对话输出一个<tool_call>即可
```

## 注意事项
你可以输出多个同步调用和异步调用，执行器会按顺序执行
参数列表传的参数值必须符合JSON规范。

## 元工具管理:
你拥有一个内置的 tool管理器 包，用于发现其他工具：

- 搜索工具包：使用 tool.find_packages(key="关键字") 查找与任务相关的包。
- 浏览工具包列表：使用 tool.list_packages(start=0, num=10) 分页查看当前可用的包。
- 查看包内工具：使用 tool.list_tools(pkg_name="包名", tool_key="可选关键字") 查看某个包下的所有工具签名。
- 获取详细文档：使用 tool.get_tool_doc(pkg_name="包名", tool_name="工具名") 获取工具的完整功能描述、参数类型（TS格式）和使用示例。
