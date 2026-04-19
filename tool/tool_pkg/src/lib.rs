use std::collections::HashMap;

use tool_core::ToolPackage;

pub mod file;

pub fn tool_pool() -> HashMap<&'static str, ToolPackage> {
    let mut pool: HashMap<&'static str, ToolPackage> = HashMap::new();
    let pkg = file::pkg::build_package();
    pool.insert(pkg.name, pkg);
    pool
}
