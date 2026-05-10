pub use self::file::build_package;
use tool_macro::package;
#[package("文件操作工具包")]
pub mod file {
    use std::path::PathBuf;

    use kernel::agent::AgentInfo;

    use anyhow::Ok;
    use chrono::Local;

    use kernel::tool::ToolCallContext;

    pub struct Context {
        pub root_path: PathBuf,
    }

    #[context]
    pub fn new_context(agent: &AgentInfo) -> Context {
        Context {
            root_path: agent.workspace.clone(),
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
    #[tool(anno = "创建文件", example = r#"create_file("/usr/a.txt")"#)]
    pub fn create_file(#[arg("文件路径")] path: String) -> anyhow::Result<()> {
        let file = std::fs::File::create_new(path)?;
        Ok(())
    }
    #[tool(
        anno = "创建文件夹",
        example = r#"create_dir("/usr/a.txt")"#,
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
