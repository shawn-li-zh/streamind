use chrono::Local;
#[derive(Debug, Clone)]
pub enum AgentEventData {
    //用户输入
    UserInput(String),
    //定时器
    Timer {
        attach: String,
        source_event_id: u64,
    },
    //工具调用结果
    ToolCallResult(Vec<String>),
    //中断信号
    Interupt(String),
}
impl AgentEventData {
    pub fn name(&self) -> &'static str {
        match self {
            AgentEventData::UserInput(_) => "用户输入",
            AgentEventData::Timer { .. } => "定时器",
            AgentEventData::ToolCallResult(_) => "工具调用结果",
            AgentEventData::Interupt(_) => "中断信号",
        }
    }
    pub fn content(&self) -> String {
        match self {
            AgentEventData::UserInput(content) => content.clone(),
            AgentEventData::Timer {
                attach,
                source_event_id,
            } => {
                format!("附加信息：{},源事件id：{}", attach, source_event_id)
            }
            AgentEventData::ToolCallResult(results) => {
               results.join("\n---\n")
            }
            AgentEventData::Interupt(reason) => format!("【中断信号:'{}'】", reason),
        }
    }
}
//事件
#[derive(Debug, Clone)]
pub struct AgentEvent {
    pub id: u64,
    pub agent_id: u32,
    pub data: AgentEventData,
    //是否强制执行
    pub force: bool,
    //agent的响应
    pub response: String,
    pub time: chrono::DateTime<Local>,
}

impl AgentEvent {
    pub fn to_message(&self) -> String {
        let time = self.time.format("%Y-%m-%d %H:%M:%S").to_string();
        let head = format!(
            "id={},type={},time={}",
            self.id,
            self.data.name(),
            time
        );
        let content = self.data.content();
        return format!(
            "{}\nContent:\n{}\nResponse:\n{}",
            head, content, self.response
        );
    }
}
