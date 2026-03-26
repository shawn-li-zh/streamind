```ts

interface Agent{
    name:string,
    memory:string,
    goal:string
}
interface Event{
    type:string,
    time:Date,
    content:string,
    sourceEventId?:bigint
}
interface RuleGroup{
    name:string
}
interface TriggerRule{
    groupId:bigint,
    expr:string,
    //钩子，调用内置的钩子函数，示例:inject(用户的名字叫张三)，所有的参数都是字符串
    hook:string,
    create_time:Date,
    triger_times:number,
    expire_time:Date
}
```