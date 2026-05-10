pub fn extract_tags<'a>(text: &'a str, tags: &[&str]) -> Vec<(&'a str, &'a str)> {
    // 构造如 (name|age|city) 的分支
    let tags_pattern = tags.join("|");
    // 构造正则：要求开始和结束必须是分支中的某一个
    // 注意：由于没有后向引用，这里依然需要在逻辑中二次确认开始/结束标签是否成对
    let pattern = format!(r#"(?s)<({0})>(.*?)</({0})>"#, tags_pattern);
    let re = regex::Regex::new(&pattern).unwrap();

    re.captures_iter::<'_, 'a>(text)
        .filter_map(|cap: regex::Captures<'a>| {
            if &cap[1] == &cap[3] {
                let name = cap.get(1).unwrap().as_str();
                let c = cap.get(2).unwrap().as_str();
                Some((name, c))
            } else {
                None // 忽略 <name>...</age> 这种不匹配的情况
            }
        })
        .collect()
}
