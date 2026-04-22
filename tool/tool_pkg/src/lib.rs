use std::collections::HashMap;

use streamind_core::tool::ToolPackagePool;

pub mod file;
pub mod tool;
pub mod read;

pub fn tool_pool() -> ToolPackagePool {
    let mut pool: ToolPackagePool = HashMap::new();
    //
    let pkg = tool::build_package();
    pool.insert(pkg.name, pkg);
    //
     let pkg = file::build_package();
    pool.insert(pkg.name, pkg);
    pool
}
