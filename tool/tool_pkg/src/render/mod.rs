pub use self::render::build_package;
use tool_macro::package;
#[package("动态渲染组件管理器")]
pub mod render {
    use kernel::tool::{ToolCallContext, core::ToolReturn};

    #[tool(
        anno = "列出指定区间内的组件信息",
        example = r#"page_components(page=1,size=10,key="图片")"#,
        limit = "30s:10"
    )]
    pub async fn page_components(
        #[arg("页码")] page: u64,
        #[arg("每页数量")] size: u64,
        #[arg("关键字")] key: Option<String>,
        #[ctx] ctx: ToolCallContext,
    ) -> anyhow::Result<ToolReturn> {
        let models = entity_service::component::page(page, size, key).await?;
        let mut res = vec![];
        res.push(format!("结果总数:{}\n---",models.len()));
        for comp in models {
            let info = format!(
                "ID:{},组件名:{},组件描述:{}\n文档:\n{}",
                comp.id, comp.name, comp.annotation, comp.doc
            );
            res.push(info);
        }
        Ok(ToolReturn::raws(res))
    }
}
