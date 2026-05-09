pub use self::tool::build_package;
use tool_macro::package;
#[package("tool管理器")]
pub mod tool {
    use streamind_core::tool::ToolCallContext;

    use anyhow::Ok;

    #[tool(
        anno = "列出指定区间内的工具包的基础信息",
        example = r#"list_packages(start=0,num=100)"#,
        limit = "30s:10"
    )]
    pub fn find_packages(
        #[arg("关键字")] key: String,
        #[ctx] ctx: ToolCallContext,
    ) -> anyhow::Result<Vec<String>> {
        let agent_context = ctx.agent_context.clone();
        let tool_engine_context = &agent_context.tool_engine_context;
        let tool_packages = &tool_engine_context.tool_packages;
        let mut res = vec![];
        for pkg_name in tool_packages.iter() {
            let pkg = tool_engine_context.get_package(*pkg_name).unwrap();
            if !(pkg.name.contains(&key) || pkg.annotation.contains(&key)) {
                continue;
            }
            let info = format!(
                "{}({}),tool_size={}",
                pkg.name,
                pkg.annotation,
                pkg.tools().len()
            );
            res.push(info);
        }
        Ok(res)
    }
    #[tool(
        anno = "列出指定区间内的工具包的基础信息",
        example = r#"list_packages(start=0,num=100)"#,
        limit = "30s:10"
    )]
    pub fn list_packages(
        #[arg("起始位置")] start: usize,
        #[arg("数量")] num: usize,
        #[ctx] ctx: ToolCallContext,
    ) -> anyhow::Result<Vec<String>> {
        let agent_context = ctx.agent_context.clone();
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
            let info = format!(
                "{}({}),tool_size={}",
                pkg.name,
                pkg.annotation,
                pkg.tools().len()
            );
            res.push(info);
        }
        Ok(res)
    }
    #[tool(anno = "列出包内工具的基本信息")]
    pub fn list_tool(
        #[arg("工具包name")] pkg_name: String,
        #[arg("tool关键词")] tool_key: Option<String>,
        #[ctx] ctx: ToolCallContext,
    ) -> anyhow::Result<Vec<String>> {
        let agent_context = ctx.agent_context.clone();
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
            res.push(format!("//{}\n{}", &tool.annotation, tool.ts_singature()));
        }
        Ok(res)
    }
    #[tool(anno = "获取工具的完整文档,ts代码格式")]
    pub fn get_tool_doc(
        #[arg("工具包name")] pkg_name: String,
        #[arg("工具name")] tool_name: String,
        #[ctx] ctx: ToolCallContext,
    ) -> anyhow::Result<String> {
        let agent_context = ctx.agent_context.clone();
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
        Ok(tool.to_doc())
    }
}
