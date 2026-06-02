use std::collections::HashSet;

pub use self::reflect::build_package;
use futures::future::BoxFuture;
use tool_macro::package;
#[package("条件反射记忆系统管理器")]
pub mod reflect {

    use chrono::{DateTime, Local};
    use entity_service::DB;
    use kernel::tool::{ToolCallContext, core::ToolReturn};
    use regex::Regex;

    use crate::reflect::ReflectInjected;

    #[tool(
        anno = "分页查询领域的基础信息",
        example = r#"page_scopes(page=1,size=10)"#,
        limit = "30s:10"
    )]
    pub async fn page_scopes(
        #[arg("页码")] page: usize,
        #[arg("数量")] size: usize,
        #[arg("关键字匹配")] key: Option<String>,
        #[arg("领域描述，语义检索")] describe: Option<String>,
        #[ctx] ctx: ToolCallContext,
    ) -> anyhow::Result<ToolReturn> {
        let describe = match describe {
            Some(v) => {
                if let Some(injected) = ctx.agent_context.get::<ReflectInjected>() {
                    Some((injected.embedding)(v).await?)
                } else {
                    anyhow::bail!("反射功能未启用")
                }
            }
            None => None,
        };
        let scopes = entity_service::rule::page_scopes(page as _, size as _, key, describe).await?;
        if scopes.is_empty() {
            return Ok(ToolReturn::raw("查询结果为空"));
        }
        let mut res = vec![];
        for scope in scopes {
            let info = format!(
                "ID={},名称={},创建时间={}\n",
                scope.id,
                scope.name,
                scope.create_time.format("%Y-%m-%d %H:%M:%S")
            );
            res.push(info);
        }
        Ok(ToolReturn::raws(res))
    }
    #[tool(
        anno = "分页查询领域内的反射规则",
        example = r#"page_rules(scope_id=1,page=1,size=10)"#,
        limit = "30s:10"
    )]
    pub async fn page_rules(
        #[arg("领域id")] scope_id: i64,
        #[arg("页码")] page: usize,
        #[arg("数量")] size: usize,
        #[arg("关键字")] key: Option<String>,
        #[ctx] ctx: ToolCallContext,
    ) -> anyhow::Result<ToolReturn> {
        let rules =
            entity_service::rule::page_rule_in_scope(scope_id, page as _, size as _, key).await?;
        if rules.is_empty() {
            return Ok(ToolReturn::raw("查询结果为空"));
        }
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
        anno = "创建新领域,不能和现有领域重名",
        example = r#"new_scope(name="编程")"#,
        limit = "30s:10"
    )]
    pub async fn new_scope(
        #[arg("领域名")] name: String,
        #[arg("领域定义")] content: String,
        #[arg("激活阈值,只有描述的语义相似度大于此值时才认为是同一个领域")] active_threshold: f32,
    ) -> anyhow::Result<ToolReturn> {
        let db = DB.get().unwrap();
        let id = entity_service::rule::new_scope(db, name, content, active_threshold).await?;
        Ok(ToolReturn::raw(format!("id={id}")))
    }

    #[tool(
        anno = "创建新名词概念,不能和现有概念重名",
        example = r#"new_concept(name="编程",content="对概念的完整定义,你的第一人称自述")"#,
        limit = "30s:10"
    )]
    pub async fn new_concept(
        #[arg("领域id")] scope_id: i64,
        #[arg("概念名")] name: String,
        #[arg("内容")] content: String,
    ) -> anyhow::Result<ToolReturn> {
        let db = DB.get().unwrap();
        let id = entity_service::concept::create(db, scope_id, name, content).await?;
        Ok(ToolReturn::raw(format!("id={id}")))
    }

    #[tool(
        anno = "分页查询已定义概念",
        example = r#"page_concepts(page=1,size=10)"#,
        limit = "30s:10"
    )]
    pub async fn page_concepts(
        #[arg("页码")] page: usize,
        #[arg("数量")] size: usize,
        #[arg("关键字")] key: Option<String>,
    ) -> anyhow::Result<ToolReturn> {
        let concepts = entity_service::concept::page_concepts(page, size, key).await?;
        if concepts.is_empty() {
            return Ok(ToolReturn::raw("查询结果为空"));
        }
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
        example = r#"save_rule(group_id=1,name="配置文件地址",
        expr="matches("配置|env|环境|config")&similar("编程")>0.6",
        content="文件在xxx路径下,包含xxx,关联事件id有...")"#,
        limit = "30s:10"
    )]
    pub async fn save_rule(
        #[arg("组id")] group_id: i64,
        #[arg("规则名")] name: String,
        #[arg("表达式")] mut expr: String,
        #[arg("反射内容，你的第一人称自述")] content: String,
        #[arg("过期时间,默认一年,格式:'24h 30m 30s'")] expire_dur: Option<String>,
        #[ctx] ctx: ToolCallContext,
    ) -> anyhow::Result<ToolReturn> {
        //从expr中提取similar里面的内容
        {
            let regex = Regex::new(r#"similar\((['"].*['"])\)"#).unwrap();
            let mut concept_names = vec![];
            for cap in regex.captures_iter(&expr) {
                let inner = cap[1].to_string();
                concept_names.push(inner);
            }
            let concepts = entity_service::concept::find_by_names(&concept_names).await?;
            for concept in concepts {
                concept_names.retain(|v| &concept.name != v);
            }
            if !concept_names.is_empty() {
                return Err(anyhow::anyhow!(
                    "concept not found: {}",
                    concept_names.join(", ")
                ));
            }
        }
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
        if group_names.is_empty() {
            anyhow::bail!("请指定规则名")
        }
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
        if group_names.is_empty() {
            anyhow::bail!("请指定规则名")
        }
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
    pub embedding:
        Box<dyn (Fn(String) -> BoxFuture<'static, anyhow::Result<Vec<f32>>>) + Send + Sync>,
    pub unload_groups:
        Box<dyn (Fn(Vec<String>) -> BoxFuture<'static, anyhow::Result<()>>) + Send + Sync>,
    pub load_groups:
        Box<dyn (Fn(Vec<String>) -> BoxFuture<'static, anyhow::Result<()>>) + Send + Sync>,
    pub current_group_names:
        Box<dyn (Fn() -> BoxFuture<'static, anyhow::Result<HashSet<String>>>) + Send + Sync>,
    pub simplify_expr:
        Box<dyn (Fn(String) -> BoxFuture<'static, anyhow::Result<String>>) + Send + Sync>,
}
