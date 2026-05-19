pub use self::tool::build_package;
use tool_macro::package;
#[package("tool管理器")]
pub mod tool {
    use kernel::tool::{ToolCallContext, core::ToolReturn};

    #[tool(
        anno = "列出指定区间内的工具包的基础信息",
        example = r#"list_packages(start=0,num=100)"#,
        limit = "30s:10"
    )]
    pub fn list_packages(
        #[arg("起始位置")] start: usize,
        #[arg("数量")] num: usize,
        #[arg("关键字")] key: Option<String>,
        #[ctx] ctx: ToolCallContext,
    ) -> anyhow::Result<ToolReturn> {
        let agent_context = ctx.agent.context();
        let tool_engine_context = &agent_context.tool_engine_context;
        let tool_packages = &tool_engine_context.tool_packages;
        let total = tool_packages.len();
        if start >= total {
            return Err(anyhow::anyhow!("工具包数量为:{}", total));
        }
        let mut end = start + num;
        if end > total {
            end = total;
        }
        let tool_packages: Vec<&str> = tool_packages.iter().map(|v| *v).collect();
        let mut res = vec![];
        for pkg_name in &tool_packages[start..end] {
            let pkg = tool_engine_context.get_package(*pkg_name).unwrap();
            if let Some(key) = &key {
                if !(pkg.name.contains(key) || pkg.annotation.contains(key)) {
                    continue;
                }
            }
            let info = format!(
                "{}({}),tool_size={}",
                pkg.name,
                pkg.annotation,
                pkg.tools().len()
            );
            res.push(info);
        }
        Ok(ToolReturn::raws(res))
    }
    #[tool(anno = "列出包内工具的基本信息")]
    pub fn list_tools(
        #[arg("工具包name")] pkg_name: String,
        #[arg("tool关键词")] tool_key: Option<String>,
        #[ctx] ctx: ToolCallContext,
    ) -> anyhow::Result<ToolReturn> {
        let agent_context = ctx.agent.context();
        let tool_engine_context = &agent_context.tool_engine_context;
        let pkg = match tool_engine_context.get_package(&pkg_name) {
            Some(e) => e,
            None => return Err(anyhow::anyhow!("工具包不存在:{}", pkg_name)),
        };
        let mut res = vec![];
        for (tool_name, tool) in pkg.tools() {
            if let Some(tool_key) = &tool_key {
                if !(tool_name.contains(tool_key) || tool.annotation.contains(tool_key)) {
                    continue;
                }
            }
            let lines: Vec<String> = tool
                .annotation
                .split("\n")
                .map(|s| format!("//{}", s.trim()))
                .collect();
            res.push(format!("{}\n{}", lines.join("\n"), tool.ts_singature()));
        }
        Ok(ToolReturn::raws(res))
    }
    #[tool(anno = "获取工具的完整文档,ts代码格式")]
    pub fn get_tool_info(
        #[arg("工具包name")] pkg_name: String,
        #[arg("工具name")] tool_name: String,
        #[ctx] ctx: ToolCallContext,
    ) -> anyhow::Result<ToolReturn> {
        let agent_context = ctx.agent.context();
        let tool_engine_context = &agent_context.tool_engine_context;
        let pkg = match tool_engine_context.get_package(&pkg_name) {
            Some(e) => e,
            None => return Err(anyhow::anyhow!("工具包不存在:{}", pkg_name)),
        };
        let tool = match pkg.get_tool(&tool_name) {
            Some(e) => e,
            None => {
                return Err(anyhow::anyhow!("工具不存在:{}", tool_name));
            }
        };
        Ok(ToolReturn::raw(tool.to_doc()))
    }
    #[tool(anno = "获取工具包内的类型信息,ts代码格式")]
    pub fn get_type_info(
        #[arg("工具包name")] pkg_name: String,
        #[arg("类型name")] type_name: String,
        #[ctx] ctx: ToolCallContext,
    ) -> anyhow::Result<ToolReturn> {
        let agent_context = ctx.agent.context();
        let tool_engine_context = &agent_context.tool_engine_context;
        let pkg = match tool_engine_context.get_package(&pkg_name) {
            Some(e) => e,
            None => return Err(anyhow::anyhow!("工具包不存在:{}", pkg_name)),
        };
        let r#type = match pkg.get_type(&type_name) {
            Some(e) => e,
            None => {
                return Err(anyhow::anyhow!("类型不存在:{}", type_name));
            }
        };
        Ok(ToolReturn::raw(r#type.to_doc()))
    }
}
