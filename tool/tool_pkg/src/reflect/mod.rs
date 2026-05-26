use std::collections::HashSet;

pub use self::reflect::build_package;
use futures::future::BoxFuture;
use tool_macro::package;
#[package("条件反射系统,规则管理器")]
pub mod reflect {

    use chrono::{DateTime, Local};
    use entity_service::DB;
    use kernel::tool::{ToolCallContext, core::ToolReturn};

    use crate::reflect::ReflectInjected;

    #[tool(
        anno = "分页查询反射组的基础信息",
        example = r#"page_groups(page=1,size=10)"#,
        limit = "30s:10"
    )]
    pub async fn page_groups(
        #[arg("页码")] page: usize,
        #[arg("数量")] size: usize,
        #[arg("关键字")] key: Option<String>,
        #[ctx] ctx: ToolCallContext,
    ) -> anyhow::Result<ToolReturn> {
        let groups = entity_service::rule::page_groups(page as _, size as _, key).await?;
        let mut res = vec![];
        for group in groups {
            let info = format!(
                "ID={},名称={},创建时间={}\n",
                group.id,
                group.name,
                group.create_time.format("%Y-%m-%d %H:%M:%S")
            );
            res.push(info);
        }
        Ok(ToolReturn::raws(res))
    }
    #[tool(
        anno = "分页查询组内的反射规则",
        example = r#"page_rules(group_id=1,page=1,size=10)"#,
        limit = "30s:10"
    )]
    pub async fn page_rules(
        #[arg("组id")] group_id: i64,
        #[arg("页码")] page: usize,
        #[arg("数量")] size: usize,
        #[arg("关键字")] key: Option<String>,
        #[ctx] ctx: ToolCallContext,
    ) -> anyhow::Result<ToolReturn> {
        let rules =
            entity_service::rule::page_rule_in_group(group_id, page as _, size as _, key).await?;
        let mut res = vec![];
        for rule in rules {
            let info = format!(
                "ID={},名称={},创建时间={}\n表达式={}\n反射内容={}\n",
                rule.id,
                rule.name,
                rule.create_time.format("%Y-%m-%d %H:%M:%S"),
                rule.expr,
                rule.content
            );
            res.push(info);
        }
        Ok(ToolReturn::raws(res))
    }
    //新建分组
    #[tool(
        anno = "创建新分组,不能和现有分组重名",
        example = r#"new_group(name="编程")"#,
        limit = "30s:10"
    )]
    pub async fn new_group(#[arg("组名")] name: String) -> anyhow::Result<ToolReturn> {
        let db = DB.get().unwrap();
        let id = entity_service::rule::new_group(db, name).await?;
        Ok(ToolReturn::raw(format!("id={id}")))
    }

    #[tool(
        anno = "创建新概念,不能和现有概念重名",
        example = r#"new_concept(name="编程",content="对概念的完整定义")"#,
        limit = "30s:10"
    )]
    pub async fn new_concept(
        #[arg("概念名")] name: String,
        #[arg("内容")] content: String,
    ) -> anyhow::Result<ToolReturn> {
        let db = DB.get().unwrap();
        let id = entity_service::concept::new_concept(db, name, content).await?;
        Ok(ToolReturn::raw(format!("id={id}")))
    }

    #[tool(
        anno = "创建新概念,不能和现有概念重名",
        example = r#"new_concept(name="编程",content="对概念的完整定义")"#,
        limit = "30s:10"
    )]
    pub async fn page_concepts(
        #[arg("页码")] page: usize,
        #[arg("数量")] size: usize,
        #[arg("关键字")] key: Option<String>,
    ) -> anyhow::Result<ToolReturn> {
        let concepts = entity_service::concept::page_concepts(page, size, key).await?;
        let mut res = vec![];
        for ele in concepts {
            let info = format!(
                "ID={},名称={}\n定义内容={}\n",
                ele.id, ele.name, ele.content,
            );
            res.push(info);
        }
        Ok(ToolReturn::raws(res))
    }

    #[tool(
        anno = "创建新的规则,如有重复则自动覆盖",
        example = r#"new_rule(group_id=1,name="配置文件地址",
        expr="matches("配置|env|环境|config")&similar("编程")>0.6",
        content="文件在xxx路径下,包含xxx,关联事件id有...")"#,
        limit = "30s:10"
    )]
    pub async fn save_rule(
        #[arg("组id")] group_id: i64,
        #[arg("规则名")] name: String,
        #[arg("表达式")] mut expr: String,
        #[arg("反射内容")] content: String,
        #[arg("过期时间,默认一年,格式:'24h 30m 30s'")] expire_dur: Option<String>,
        #[ctx] ctx: ToolCallContext,
    ) -> anyhow::Result<ToolReturn> {
        if let Some(injected) = ctx.agent_context.get::<ReflectInjected>() {
            expr = (injected.simplify_expr)(expr.clone()).await?;
        } else {
            anyhow::bail!("反射功能未启用")
        }

        let db = DB.get().unwrap();
        let dur = humantime::parse_duration(&expire_dur.unwrap_or("1year".to_string()))?;
        let expire_time = Local::now() + dur;
        let id =
            entity_service::rule::save_rule(db, group_id, name, expr, content, expire_time).await?;
        Ok(ToolReturn::raw(format!("id={id}")))
    }
    #[tool(
        anno = "加载规则组,有则覆盖",
        example = r#"load_groups(group_names=['编程','生活','人际关系','股票','法律'])"#,
        limit = "30s:10"
    )]
    pub async fn load_groups(
        #[arg("规则名")] group_names: Vec<String>,
        #[ctx] ctx: ToolCallContext,
    ) -> anyhow::Result<ToolReturn> {
        if let Some(injected) = ctx.agent_context.get::<ReflectInjected>() {
            (injected.load_groups)(group_names).await?;
        } else {
            anyhow::bail!("反射功能未启用")
        }
        Ok(ToolReturn::raw(format!("重载成功")))
    }
    #[tool(
        anno = "移除当前加载的知识组",
        example = r#"unload_groups(group_names=['编程','生活','人际关系','股票','法律'])"#,
        limit = "30s:10"
    )]
    pub async fn unload_groups(
        #[arg("规则名")] group_names: Vec<String>,
        #[ctx] ctx: ToolCallContext,
    ) -> anyhow::Result<ToolReturn> {
        if let Some(injected) = ctx.agent_context.get::<ReflectInjected>() {
            (injected.unload_groups)(group_names).await?;
        } else {
            anyhow::bail!("反射功能未启用")
        }
        Ok(ToolReturn::raw(format!("重载成功")))
    }
    #[tool(
        anno = "查看当前系统里加载了的反射组",
        example = r#"current_groups()"#,
        limit = "30s:10"
    )]
    pub async fn current_groups(#[ctx] ctx: ToolCallContext) -> anyhow::Result<ToolReturn> {
        if let Some(injected) = ctx.agent_context.get::<ReflectInjected>() {
            let names = (injected.current_group_names)().await?;
            let names = names.into_iter().collect::<Vec<_>>();
            Ok(ToolReturn::raw(names.join("\n")))
        } else {
            anyhow::bail!("反射功能未启用")
        }
    }
}

pub struct ReflectInjected {
    pub unload_groups:
        Box<dyn (Fn(Vec<String>) -> BoxFuture<'static, anyhow::Result<()>>) + Send + Sync>,
    pub load_groups:
        Box<dyn (Fn(Vec<String>) -> BoxFuture<'static, anyhow::Result<()>>) + Send + Sync>,
    pub current_group_names:
        Box<dyn (Fn() -> BoxFuture<'static, anyhow::Result<HashSet<String>>>) + Send + Sync>,
    pub simplify_expr:
        Box<dyn (Fn(String) -> BoxFuture<'static, anyhow::Result<String>>) + Send + Sync>,
}
