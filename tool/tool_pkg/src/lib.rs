use std::collections::HashMap;

use kernel::tool::ToolPackagePool;

pub mod browser;
pub mod editor;
pub mod event;
pub mod file;
pub mod http;
pub mod render;
pub mod timer;
pub mod tool;

pub fn tool_pool() -> ToolPackagePool {
    let mut pool: ToolPackagePool = HashMap::new();
    //工具管理器
    let pkg = tool::build_package();
    pool.insert(pkg.name, pkg);
    //组件管理器
    let pkg = render::build_package();
    pool.insert(pkg.name, pkg);
    //历史事件搜索
    let pkg = event::build_package();
    pool.insert(pkg.name, pkg);
    //文件管理器
    let pkg = file::build_package();
    pool.insert(pkg.name, pkg);
    //编辑器
    let pkg = editor::build_package();
    pool.insert(pkg.name, pkg);
    //http
    let pkg = http::build_package();
    pool.insert(pkg.name, pkg);
    //浏览器
    let pkg = browser::build_package();
    pool.insert(pkg.name, pkg);
    //定时器
    let pkg = timer::build_package();
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
