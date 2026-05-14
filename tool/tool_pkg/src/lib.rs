use std::collections::HashMap;

use kernel::tool::ToolPackagePool;

pub mod editor;
pub mod file;
pub mod http;
pub mod tool;

pub fn tool_pool() -> ToolPackagePool {
    let mut pool: ToolPackagePool = HashMap::new();
    //工具管理器
    let pkg = tool::build_package();
    pool.insert(pkg.name, pkg);
    //文件管理器
    let pkg = file::build_package();
    pool.insert(pkg.name, pkg);
    //阅读器
    let pkg = editor::build_package();
    pool.insert(pkg.name, pkg);
    //http
    let pkg = http::build_package();
    pool.insert(pkg.name, pkg);
    //
    for (key, pkg) in &pool {
        println!("{}:{}", pkg.name, pkg.annotation);
        for (key, tool) in pkg.tools() {
            println!("{}\n\n", tool.to_doc());
        }
        println!("---")
    }
    pool
}
