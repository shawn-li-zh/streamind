pub use self::file::build_package;
use tool_macro::package;
#[package("文件操作工具包")]
pub mod file {
    use streamind_core::agent::AgentContext;

    use anyhow::Ok;
    use chrono::Local;

    use streamind_core::tool::{ToolCallContext, ToolCallReturn};

    pub struct Context {
        pub root_path: String,
    }

    #[context]
    pub fn new_context(agent: &AgentContext) -> Context {
        Context {
            root_path: agent.info.workspace.to_string(),
        }
    }
    #[class("文件对象")]
    pub struct File {
        #[field(anno = "文件名")]
        pub name: String,
        #[field(anno="文件类型",enums=["txt","png"])]
        pub ty: String,
        pub is_directory: bool,
        pub is_symlink: bool,
        pub creation_time: chrono::DateTime<Local>,
        pub last_access_time: chrono::DateTime<Local>,
        pub last_write_time: chrono::DateTime<Local>,
        pub change_time: Option<chrono::DateTime<Local>>,
        pub file_size: u64,
    }
    #[tool(
        anno = "创建文件",
        example = r#"create_file("/usr/a.txt")"#,
        active = "正在创建{path}文件"
    )]
    pub async fn create_file(
        #[arg("文件路径")] path: String
    ) -> anyhow::Result<ToolCallReturn> {
        let file = std::fs::File::create_new(path)?;
        Ok(().into())
    }
    #[tool(
        anno = "创建文件夹",
        example = r#"create_dir("/usr/a.txt")"#,
        active = format!("正在创建{path}文件夹"),
        limit = "30s:10"
    )]
    pub fn create_dir(
        #[arg("文件路径")] path: String,
        #[ctx] ctx: ToolCallContext,
    ) -> anyhow::Result<()> {
        std::fs::create_dir_all(path)?;
        Ok(())
    }
}
