use chrono::Local;
#[derive(Debug, Clone)]
pub enum AgentEventType {
    //用户输入
    UserInput(String),
    //定时器
    Timer {
        attach: String,
        source_event_id: u64,
    },
}
impl AgentEventType {
    pub fn name(&self) -> &'static str {
        match self {
            AgentEventType::UserInput(_) => "用户输入",
            AgentEventType::Timer { .. } => "定时器",
        }
    }
    pub fn content(&self) -> String {
        match self {
            AgentEventType::UserInput(content) => content.clone(),
            AgentEventType::Timer {
                attach,
                source_event_id,
            } => {
                format!("附加信息：{},源事件id：{}", attach, source_event_id)
            }
        }
    }
}
//事件
#[derive(Debug, Clone)]
pub struct AgentEvent {
    pub id: u64,
    pub stream_id: u32,
    pub event_type: AgentEventType,
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
            self.event_type.name(),
            time
        );
        let content = self.event_type.content();
        return format!(
            "{}\nContent:\n{}\nResponse:\n{}",
            head, content, self.response
        );
    }
}
