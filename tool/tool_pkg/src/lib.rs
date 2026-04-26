use std::collections::HashMap;

use streamind_core::tool::ToolPackagePool;

pub mod file;
pub mod http;
pub mod editor;
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
    pool
}
