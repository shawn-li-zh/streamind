pub fn new_file(path: &str) -> Result<String, anyhow::Error> {
    std::fs::File::create_new(path)?;
    Ok("文件创建成功".to_string())
}