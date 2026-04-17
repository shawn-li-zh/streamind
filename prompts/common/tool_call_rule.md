## 函数调用规范
所有你能调用的函数工具包，都是使用ts来描述参数和类型定义。
当你需要调用某些函数时，输出一个<tool_call>标签，并将需要调用的函数放在标签中。
函数调用格式:包名.函数名(...参数列表)  
注意：参数列表必须符合JSON规范。

### 举例
假设有一个common工具包。

异步调用：
<tool_call>common.get_weather("shanghai")</tool_call>
<tool_call>common.println("hello world")</tool_call>
ps:以上两个函数会同时异步执行。

同步调用：
<tool_call>await common.get_weather("shanghai")</tool_call>
ps:当执行器遇到await时，会同步等待当前工具调用完成。

### 注意事项
你可以输出多个同步调用和异步调用，执行器会按顺序执行
