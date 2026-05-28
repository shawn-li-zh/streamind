pub use self::event::build_package;
use tool_macro::package;
#[package("历史事件搜索包")]
pub mod event {

    use chrono::{Local, NaiveDateTime};

    use entity_service::event::event_convert;
    use kernel::{
        agent::event::EnvEvent as _,
        tool::{ToolCallContext, core::ToolReturn},
    };

    const FMT: &str = "%Y-%m-%d %H:%M";
    //根据id查询
    #[tool(
        anno = "根据id查询事件",
        example = r#"get_by_id(id=1)"#,
        limit = "30s:10"
    )]
    pub async fn get_by_id(
        #[arg("事件id")] id: i64,
        #[ctx] ctx: ToolCallContext,
    ) -> anyhow::Result<ToolReturn> {
        let identity = ctx.agent_identity;
        let model = entity_service::event::get_by_id(identity.id(), id).await?;
        match model {
            Some(model) => {
                let agent_event = event_convert::from_entity(model);
                let (text, _paths) = agent_event.to_message().to_tuple();
                Ok(ToolReturn::raw(text))
            }
            None => Ok(ToolReturn::raw("事件不存在")),
        }
    }
    #[tool(
        anno = "搜索指定事件发生之前或之后的事件列表",
        example = r#"search_at(id=1,before=true,key='事件关键字',event_type='事件类型',limit=10)"#,
        limit = "30s:10"
    )]
    pub async fn search_at(
        #[arg("事件id")] id: i64,
        #[arg("时间方向")] before: bool,
        #[arg("关键字")] key: Option<String>,
        #[arg("事件类型")] event_type: Option<String>,
        #[arg("数量,最多100")] mut limit: u64,
        #[ctx] ctx: ToolCallContext,
    ) -> anyhow::Result<ToolReturn> {
        if limit > 100 {
            limit = 100;
        }
        let identity = ctx.agent_identity;
        let models =
            entity_service::event::search_at(identity.id(), id, event_type, key, limit, before)
                .await?;
        Ok(to_return(models))
    }

    #[tool(
        anno = "在时间范围内搜索事件,时间格式:%Y-%m-%d %H:%M",
        example = r#"search(start='2022-01-01 00:00',end='2022-01-01 23:59',key='事件关键字',event_type='事件类型',limit=10)"#,
        limit = "30s:10"
    )]
    pub async fn search(
        #[arg("开始时间")] start: String,
        #[arg("结束时间")] end: String,
        #[arg("关键字")] key: Option<String>,
        #[arg("事件类型")] event_type: Option<String>,
        #[arg("数量,最多100")] mut limit: u64,
        #[ctx] ctx: ToolCallContext,
    ) -> anyhow::Result<ToolReturn> {
        if limit > 100 {
            limit = 100;
        }
       
        let identity = ctx.agent_identity;
        let start = NaiveDateTime::parse_from_str(&start, FMT)?;
        let start = start.and_local_timezone(Local).unwrap();
        let end = NaiveDateTime::parse_from_str(&end, FMT)?.and_local_timezone(Local).unwrap();
        let models =
            entity_service::event::time_range(identity.id(), start, end, event_type, key, limit)
                .await?;
        Ok(to_return(models))
    }
    fn to_return(events: Vec<entity::event::Model>) -> ToolReturn {
        if events.is_empty() {
            return ToolReturn::raw("查询结果为空");
        }
        let mut res = vec![];
        res.push(format!("结果总数:{}\n---", events.len()));
        for event in events {
            let agent_event = event_convert::from_entity(event);
            let (text, _paths) = agent_event.to_message().to_tuple();
            res.push(text);
        }
        ToolReturn::raws(res)
    }
}
