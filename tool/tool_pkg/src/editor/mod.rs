use std::{
    collections::HashMap,
    fs::File,
    io::{self, BufReader},
    sync::LazyLock,
};

mod file_kit;
mod read;

pub mod collection;

use tool_macro::package;

static MODES: LazyLock<HashMap<String, Vec<String>>> =
    LazyLock::new(|| load_mode_delimiters(r"D:\project\rust\streamind\reader\modes"));
fn load_mode_delimiters(dir: &str) -> HashMap<String, Vec<String>> {
    let mut map = HashMap::new();
    let paths = file_kit::read_files(dir, "json").unwrap();
    //
    for (mode, values) in paths.into_iter().map(|p| {
        let name = p
            .file_name()
            .unwrap()
            .to_ascii_lowercase()
            .into_string()
            .unwrap();
        let name = name.trim_end_matches(".json").to_string();
        let file = File::open(p).unwrap();

        let reader = BufReader::new(file);
        let value: Vec<serde_json::Value> =
            serde_json::from_reader(reader).expect("解析json文件失败");
        (name, value)
    }) {
        let mut delimiters: Vec<String> = vec![];
        fn recursion(delimiters: &mut Vec<String>, values: Vec<serde_json::Value>) {
            for value in values {
                if value.is_string() {
                    delimiters.push(value.to_string());
                    return;
                }
                if value.is_array() {
                    let values: Vec<serde_json::Value> = serde_json::from_value(value).unwrap();
                    recursion(delimiters, values);
                }
            }
        }
        recursion(&mut delimiters, values);
        //倒序
        delimiters.sort_by(|a, b| b.cmp(a));
        map.insert(mode, delimiters);
    }
    map
}
pub use self::editor::build_package;
#[package("文件编辑器")]
mod editor {
    use std::{
        collections::HashSet,
        fs::{File, OpenOptions},
        io::{BufReader, BufWriter},
        os::windows::fs::{MetadataExt, OpenOptionsExt},
        path::PathBuf,
    };

    use kernel::{
        agent::AgentContext,
        tool::{ToolCallContext, core::ToolReturn},
    };
    use ropey::Rope;

    use crate::editor::{MODES, read};
    pub struct Context {
        pub path: Option<PathBuf>,
        pub content: Option<Rope>,
        pub is_modified: bool,
        pub is_safe: bool,
    }
    #[context]
    pub fn new_context(ctx: AgentContext) -> Context {
        Context {
            path: None,
            content: None,
            is_modified: false,
            is_safe: false,
        }
    }

    #[tool(anno = "加载文件到编辑器,期间文件无法被其他进程修改,加载新文件时，旧文件会自动释放")]
    pub async fn load_file(
        #[arg("文件地址")] path: String,
        #[ctx] ctx: ToolCallContext,
    ) -> anyhow::Result<ToolReturn> {
        let (path, is_safe) = ctx.get_real_path(path.as_str());
        let mut ctx = ctx.context_data.write().await;
        let ctx = ctx.downcast_mut::<Context>().unwrap();
        let file = OpenOptions::new().read(true).write(true).open(&path)?;
        let content = Rope::from_reader(BufReader::new(file))?;
        //
        ctx.path = Some(path);
        ctx.content = Some(content);
        if !is_safe {
            return Ok(ToolReturn::raw("加载成功,文件位于工作区之外,只允许阅读"));
        }
        Ok(ToolReturn::raw("加载成功"))
    }
    #[tool(anno = "手动释放编辑器中的文件,未保存的内容将会丢失")]
    pub async fn release(#[ctx] ctx: ToolCallContext) -> anyhow::Result<()> {
        let mut ctx = ctx.context_data.write().await;
        let ctx = ctx.downcast_mut::<Context>().unwrap();
        ctx.path = None;
        ctx.content = None;
        Ok(())
    }

    #[tool(anno = "读取内容,根据关注度随机在文本中添加位置掩码，提高阅读效率")]
    pub async fn read(
        #[arg("起始字符位置")] pos: usize,
        #[arg("读取数量")] len: usize,
        #[arg("关注程度 取值范围(0,1],数值越低，读取的内容越稀疏缺失")] focus_level: f32,
        #[arg("关注词,英文逗号分隔,会放大相关词被保留下来的概率")] focus_wrods: Option<String>,
        #[arg("阅读模式:中文=zh,英文=en,代码=code,默认为zh")] mode: Option<String>,
        #[ctx] ctx: ToolCallContext,
    ) -> anyhow::Result<ToolReturn> {
        let ctx = ctx.context_data.read().await;
        let content = {
            let ctx = ctx.downcast_ref::<Context>().unwrap();
            if ctx.content.is_none() {
                return Err(anyhow::anyhow!("请先加载文件"));
            }
            ctx.content.as_ref().unwrap()
        };
        let slice = content.get_slice(pos..pos + len).unwrap();
        let content = slice.chars().collect();
        //
        let mut focus_word_set = HashSet::new();
        let mut focus_word_char_set = vec![];
        let focus_wrods = focus_wrods.as_ref().map(|v| v.as_str()).unwrap_or_default();
        for ele in focus_wrods.split(",") {
            focus_word_set.insert(ele.trim());
            focus_word_char_set.push(ele);
        }
        let common = MODES.get("common").unwrap();
        let delimiters: Vec<&str> =
            match MODES.get(mode.as_ref().map(|v| v.as_str()).unwrap_or("zh")) {
                Some(d) => {
                    let mut delimiters = Vec::with_capacity(common.len() + d.len());
                    for ele in common {
                        if focus_word_set.contains(ele.as_str()) {
                            continue;
                        }
                        delimiters.push(ele.as_str());
                    }
                    for ele in d {
                        if focus_word_set.contains(ele.as_str()) {
                            continue;
                        }
                        delimiters.push(ele.as_str());
                    }
                    delimiters
                }
                None => {
                    return Err(anyhow::anyhow!("不支持的阅读模式"));
                }
            };
        let text = read::skim_read(
            content,
            100,
            focus_level,
            &focus_word_char_set,
            &delimiters,
            25,
            ('<', '>'),
        )?;
        Ok(ToolReturn::raw(text))
    }
    #[tool(anno = "在指定位置处写入内容")]
    pub async fn write(
        #[arg("起始字符位置")] pos: usize,
        #[arg("写入内容")] text: String,
        #[ctx] ctx: ToolCallContext,
    ) -> anyhow::Result<()> {
        if text.is_empty() {
            return Ok(());
        }
        let mut ctx = ctx.context_data.write().await;
        let ctx = ctx.downcast_mut::<Context>().unwrap();
        if ctx.content.is_none() {
            return Err(anyhow::anyhow!("请先加载文件"));
        }
        if !ctx.is_safe {
            return Err(anyhow::anyhow!("无编辑权限"));
        }
        let content = ctx.content.as_mut().unwrap();
        content.insert(pos, &text);
        ctx.is_modified = true;
        Ok(())
    }

    #[tool(anno = "删除指定位置处的内容")]
    pub async fn delete(
        #[arg("起始字符位置")] pos: usize,
        #[arg("删除数量")] len: usize,
        #[ctx] ctx: ToolCallContext,
    ) -> anyhow::Result<()> {
        let mut ctx = ctx.context_data.write().await;
        let ctx = ctx.downcast_mut::<Context>().unwrap();
        if ctx.content.is_none() {
            return Err(anyhow::anyhow!("请先加载文件"));
        }
        if !ctx.is_safe {
            return Err(anyhow::anyhow!("无编辑权限"));
        }
        let content = ctx.content.as_mut().unwrap();
        content.remove(pos..pos + len);
        ctx.is_modified = true;
        Ok(())
    }

    #[tool(anno = "文件修改后,保存修改后的内容到文件")]
    pub async fn save(
        #[arg("另外保存到地址,为空则保存到原地址")] path: Option<String>,
        #[ctx] ctx: ToolCallContext,
    ) -> anyhow::Result<()> {
        let mut ctx = ctx.context_data.write().await;
        let ctx = ctx.downcast_mut::<Context>().unwrap();
        if !ctx.is_modified {
            return Ok(());
        }
        if ctx.content.is_none() {
            return Err(anyhow::anyhow!("请先加载文件"));
        }
        let content = ctx.content.as_mut().unwrap();
        //
        let path = path
            .map(|v| PathBuf::from(&v))
            .unwrap_or(ctx.path.clone().unwrap());
        if let Some(dir) = path.parent() {
            if !dir.exists() {
                std::fs::create_dir_all(dir)?;
            }
        };
        let file = OpenOptions::new().create(true).write(true).open(&path)?;
        content.write_to(BufWriter::new(file))?;
        Ok(())
    }
}

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use std::collections::HashSet;
//     use std::fs::File;
//     use std::io::Write;
//     use std::sync::LazyLock;

//     fn create_test_file(content: &str) -> (File, std::path::PathBuf) {
//         let path = std::path::Path::new(r"D:\project\rust\streamind/test/temp.txt");

//         let mut temp_file = if !path.exists() {
//             let p = path.parent().unwrap();
//             if !p.exists() {
//                 std::fs::create_dir_all(path.parent().unwrap()).unwrap();
//             }
//             File::create_new(path).unwrap()
//         } else {
//             File::open(path).unwrap()
//         };
//         temp_file.write_all(content.as_bytes()).unwrap();
//         (temp_file, path.to_path_buf())
//     }
//     static DELIMITERS: LazyLock<Vec<&str>> = LazyLock::new(|| {
//         let mut arr = vec![
//             ",", "，", "。", ":", "：", "<", ">", "你", "会", "可以", "必须", "我", "他", "它",
//             "她", "能", "了", "呢", "啊", "哈", "需要", "一个", "并", "且", "有", "没有", "在",
//             "其实", "某些", "一些", "一点", "/", ".", "(", ")", "\"", "'", "_", "#", "\n", "\t",
//             "=", "?", "!", "？", "！", "+", "是", "的", " ", "和", ";", "非常", "优秀", "极其",
//             "、",
//         ];
//         //正序
//         arr.sort_by(|a, b| b.len().cmp(&a.len()));
//         arr
//     });

//     // #[tokio::test]
//     // async fn test_skim_read_basic() -> anyhow::Result<()> {
//     //     let content = "Rust是一门极其优秀的编程语言。它非常注重内存安全、并发性能和开发效率。";
//     //     let (mut file, _path) = create_test_file(content).await;

//     //     // 1. 测试全量读取 (focus_level = 1.0)
//     //     let res1 = skim_read(&mut file, 0, 1000, 1.0, &DELIMITERS).await?;
//     //     println!("Full Read: {}", res1);
//     //     let res2 = skim_read(&mut file, 0, 1000, 0.5, &DELIMITERS).await?;
//     //     println!("Mid Focus: {}", res2);

//     //     let res2 = skim_read(&mut file, 0, 1000, 0.3, &DELIMITERS).await?;
//     //     println!("Mid Focus: {}", res2);
//     //     // 2. 测试低聚焦模式 (focus_level = 0.1)
//     //     // 阈值会很高，短句应该变成掩码 [pos]
//     //     let res2 = skim_read(&mut file, 0, 1000, 0.1, &DELIMITERS).await?;
//     //     println!("Low Focus: {}", res2);

//     //     Ok(())
//     // }

//     #[tokio::test]
//     async fn test_real_file() -> anyhow::Result<()> {
//         let mut file =
//             File::open(r"D:\project\rust\streamind\prompts\common\tool_call_rule.md").unwrap();
//         let foucs_chars = vec!["李修顺", "异步执行"];
//         for i in 8..10 {
//             let level = i as f32 / 10.0;
//             let res = file_kit::skim_read(
//                 &mut file,
//                 0,
//                 500,
//                 100,
//                 level,
//                 &foucs_chars,
//                 &DELIMITERS,
//                 10,
//                 ('|', '|'),
//             )?;
//             println!("Read:============{i}=============== {},\n {}", level, &res);
//             println!("================={} \n\n\n", res.trim().chars().count())
//         }
//         let level = 0.5;
//         let pos = file_kit::char_pos_to_byte_pos(&mut file, 0, 0, 96)?;
//         let res = file_kit::skim_read(
//             &mut file,
//             pos,
//             150,
//             100,
//             level,
//             &foucs_chars,
//             &DELIMITERS,
//             25,
//             ('|', '|'),
//         )?;
//         println!("Read:===================== {},\n {}", level, &res);
//         println!("================={} \n\n\n", res.trim().chars().count());
//         Ok(())
//     }
// }
