## 函数调用规范
所有你能调用的函数工具包，都是使用ts来描述参数和类型定义。
当你需要调用某些函数时，输出一个<function_call>标签，并将需要调用的函数放在标签中。
函数调用格式:包名.函数名(...参数列表)  
注意：参数列表必须符合JSON规范。

## 举例
假设有一个common工具包。
<function_call>
common.get_weather("shanghai")
common.print({"a":1})
</function_call>