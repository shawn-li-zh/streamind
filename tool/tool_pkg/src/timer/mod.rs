pub use self::timer::build_package;
use tool_macro::package;
#[package("定时管理器")]
pub mod timer {
    use chrono::Local;
    use kernel::{
        agent::{AgentHandler, context::AgentTimer},
        tool::{ToolCallContext, core::ToolReturn},
    };

    #[tool(
        anno = "列出没过期的定时任务",
        example = r#"list_timers(start=0,num=100)"#,
        limit = "30s:10"
    )]
    pub async fn list_timers(
        #[arg("起始位置")] start: u64,
        #[arg("数量")] num: u64,
        #[arg("关键字")] key: Option<String>,
        #[ctx] ctx: ToolCallContext,
    ) -> anyhow::Result<ToolReturn> {
        let agent_context = ctx.agent_handler.context();
        let agent_id = agent_context.agent_id;
        let agent_timers = entity_service::agent_timer::range(agent_id, start, num, key).await?;
        let mut res = vec![];
        res.push(format!("结果总数:{}\n---", agent_timers.len()));
        for timer in agent_timers {
            let info = format!(
                "ID={},周期={},次数限制={},已触发次数={}\n附加信息:\n{}",
                timer.id, timer.corn, timer.times, timer.trigger_times, timer.payload
            );
            res.push(info);
        }
        Ok(ToolReturn::raws(res))
    }
    #[tool(anno = "取消定时任务")]
    pub async fn cancel(
        #[arg("任务id")] id: i64,
        #[ctx] ctx: ToolCallContext,
    ) -> anyhow::Result<ToolReturn> {
        let agent: std::sync::Arc<dyn AgentHandler> = ctx.agent_handler.clone();
        let _ = agent.timer_handler().cancel(id).await?;

        Ok(ToolReturn::raw("取消成功"))
    }

    #[tool(
        anno = "添加定时任务",
        example = r#"list_timers(start=0,num=100)"#,
        limit = "30s:10"
    )]
    pub async fn add(
        #[arg("任务周期corn表达式")] corn: String,
        #[arg("触发次数限制,0表示可以无限次触发")] times: u32,
        #[arg("当触发时,发送给智能体的附加信息")] payload: String,
        #[ctx] ctx: ToolCallContext,
    ) -> anyhow::Result<ToolReturn> {
        let agent: std::sync::Arc<dyn AgentHandler> = ctx.agent_handler.clone();
        let timer = AgentTimer::new(corn, times, payload);
        let id = agent.timer_handler().add(timer).await?;
        Ok(ToolReturn::raw(format!("创建成功,定时器ID={}", id)))
    }

    //睡眠
    #[tool(
        anno = "睡眠,等待指定时间后继续执行",
        example = r#"sleep(time="1h 30m 30s")"#,
        limit = "30s:10"
    )]
    pub async fn sleep(
        #[arg("睡眠时间,格式:数字+时间单位,")] time: String,
        #[ctx] ctx: ToolCallContext,
    ) -> anyhow::Result<ToolReturn> {
        let dur = match humantime::parse_duration(&time) {
            Ok(v) => v,
            Err(e) => {
                anyhow::bail!("时间格式错误:{}", e)
            }
        };
        let time = Local::now() + dur;
        ctx.agent_handler.sleep_until(time).await?;
        Ok(ToolReturn::ignore())
    }
}
