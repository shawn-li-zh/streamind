pub use self::file::build_package;
use tool_macro::package;
#[package("文件操作工具包")]
pub mod file {
    use std::{
        path::{self, PathBuf},
        sync::Arc,
    };

    use kernel::agent::context::AgentContext;

    use anyhow::Ok;
    use chrono::Local;

    use kernel::tool::ToolCallContext;

    pub struct Context {}

    #[context]
    pub fn new_context(agent: Arc<AgentContext>) -> Context {
        Context {}
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
    #[tool(anno = "创建文件", example = r#"create_file(path="/usr/a.txt")"#)]
    pub fn create_file(
        #[arg("文件路径")] path: String,
        #[ctx] ctx: ToolCallContext,
    ) -> anyhow::Result<()> {
        let (path, is_safe) = ctx.get_real_path(&path);
        if !is_safe {
            anyhow::bail!("路径超出安全范围");
        }
        let file = std::fs::File::create_new(path)?;
        Ok(())
    }
    #[tool(
        anno = "创建文件夹",
        example = r#"create_dir(path="/usr/a.txt")"#,
        limit = "30s:10"
    )]
    pub fn create_dir(
        #[arg("文件路径")] path: String,
        #[ctx] ctx: ToolCallContext,
    ) -> anyhow::Result<()> {
        let (path, is_safe) = ctx.get_real_path(&path);
        if !is_safe {
            anyhow::bail!("路径超出安全范围");
        }
        std::fs::create_dir_all(path)?;
        Ok(())
    }
    #[tool(
        anno = "删除文件",
        example = r#"remove_file(path="/usr/a.txt")"#,
        limit = "10s:5"
    )]
    pub fn remove_file(
        #[arg("文件路径")] path: String,
        #[ctx] ctx: ToolCallContext,
    ) -> anyhow::Result<()> {
        let (path, is_safe) = ctx.get_real_path(&path);
        if !is_safe {
            anyhow::bail!("路径超出安全范围");
        }
        if path.is_file() {
            std::fs::remove_file(path)?;
        } else {
            anyhow::bail!("路径不是文件");
        }
        Ok(())
    }
    #[tool(
        anno = "删除文件夹",
        example = r#"remove_dir(path="/usr/a.txt")"#,
        limit = "10s:5"
    )]
    pub fn remove_dir(
        #[arg("文件路径")] path: String,
        #[ctx] ctx: ToolCallContext,
    ) -> anyhow::Result<()> {
        let (path, is_safe) = ctx.get_real_path(&path);
        if !is_safe {
            anyhow::bail!("路径超出安全范围");
        }
        if path.is_dir() {
            std::fs::remove_dir_all(path)?;
        } else {
            anyhow::bail!("路径不是文件夹");
        }
        Ok(())
    }
}
