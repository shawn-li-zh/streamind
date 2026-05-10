use std::collections::HashSet;
use std::fs::{File, OpenOptions};
use std::io::{self, BufReader, Read, Seek, SeekFrom};
use std::path::PathBuf;
use std::result;

use rand::rngs::StdRng;
use rand::{Rng, SeedableRng, thread_rng};
use serde::de;
use unicode_reader::CodePoints;

use crate::editor::collection::WindowedSortedCollection;

pub fn read_n_chars(file: &mut File, start_pos: u64, n: usize) -> io::Result<Vec<char>> {
    // 1. 定位
    file.seek(SeekFrom::Start(start_pos))?;

    // 2. 使用 BufReader 包装以提高效率
    let reader = std::io::BufReader::new(file);
    let mut res = vec![];

    for ele in CodePoints::from(reader).take(n) {
        match ele {
            Ok(c) => {
                res.push(c);
            }
            Err(e) => match e.kind() {
                io::ErrorKind::UnexpectedEof => {
                    break;
                }
                _ => {
                    return Err(e);
                }
            },
        }
    }
    Ok(res)
}

/// 在指定位置附近寻找有效的 UTF-8 字符边界
/// 返回校准后的字节偏移量 (u64)
pub fn find_boundary_near(mut file: &File, pos: u64, is_forward: bool) -> io::Result<u64> {
    let file_len = file.metadata()?.len();
    if file_len == 0 {
        return Ok(0);
    }
    if pos >= file_len {
        return Ok(file_len);
    }

    if is_forward {
        // --- 向前寻找 (Forward) ---
        // 我们从 pos 开始读 4 字节（UTF-8 最大长度）
        let read_len = std::cmp::min(4, file_len - pos) as usize;
        let mut buf = vec![0u8; read_len];
        file.seek(SeekFrom::Start(pos))?;
        file.read_exact(&mut buf)?;

        for i in 0..read_len {
            // 只要不是 10xxxxxx，就是起始位置
            if (buf[i] & 0xC0) != 0x80 {
                return Ok(pos + i as u64);
            }
        }
    } else {
        // --- 向后寻找 (Backward) ---
        // 我们需要检查 pos 及其之前的 3 个字节 (总共最多 4 字节)
        let start_offset = if pos >= 3 { pos - 3 } else { 0 };
        let read_len = (pos - start_offset + 1) as usize;

        let mut buf = vec![0u8; read_len];
        file.seek(SeekFrom::Start(start_offset))?;
        file.read_exact(&mut buf)?;

        // 从缓冲区末尾（即原始 pos 处）开始逆路寻找
        for i in (0..read_len).rev() {
            if (buf[i] & 0xC0) != 0x80 {
                return Ok(start_offset + i as u64);
            }
        }
    }

    Err(io::Error::new(
        io::ErrorKind::InvalidData,
        "在 4 字节范围内未找到有效 UTF-8 边界，文件可能损坏或非 UTF-8",
    ))
}

/// 主入口：字符位置转字节位置
pub fn char_pos_to_byte_pos(
    file: &mut File,
    base_byte_pos: u64,
    base_char_pos: u64,
    target_char_pos: u64,
) -> io::Result<u64> {
    // 边界检查：确保基础位置在文件范围内
    let file_len = file.metadata()?.len();
    if base_byte_pos > file_len {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "基础字节位置超出文件大小",
        ));
    }

    if target_char_pos == base_char_pos {
        return Ok(base_byte_pos);
    }

    if target_char_pos > base_char_pos {
        let count = target_char_pos - base_char_pos;
        search_forward(file, base_byte_pos, count, file_len)
    } else {
        let count = base_char_pos - target_char_pos;
        search_backward(file, base_byte_pos, count)
    }
}

/// 向前搜索：利用 BufReader 批量读取
fn search_forward(file: &File, start_byte: u64, count: u64, file_len: u64) -> io::Result<u64> {
    let mut current_byte = start_byte;
    let mut f = file;
    f.seek(SeekFrom::Start(start_byte))?;

    let mut reader = BufReader::new(f);

    for _ in 0..count {
        if current_byte >= file_len {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "向前搜索超出文件末尾",
            ));
        }

        let mut b = [0u8; 1];
        reader.read_exact(&mut b)?;

        // 获取 UTF-8 字符长度
        let len = get_utf8_len(b[0])?;

        // 边界预检：防止最后一个字符被截断
        if current_byte + len as u64 > file_len {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "文件末尾存在不完整的 UTF-8 字符",
            ));
        }

        if len > 1 {
            // 在缓冲区内移动，不产生系统调用
            reader.seek_relative(len as i64 - 1)?;
        }
        current_byte += len as u64;
    }
    Ok(current_byte)
}

/// 向后搜索：手动分块读取并内存扫描
fn search_backward(file: &File, start_byte: u64, count: u64) -> io::Result<u64> {
    let mut current_byte = start_byte;
    let mut chars_found = 0;

    const CHUNK_SIZE: usize = 1024;
    let mut buffer = [0u8; CHUNK_SIZE];
    let mut f = file;

    while chars_found < count {
        if current_byte == 0 {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "向后搜索超出文件开头",
            ));
        }

        // 确定本次读取范围
        let read_start = current_byte.saturating_sub(CHUNK_SIZE as u64);
        let bytes_to_read = (current_byte - read_start) as usize;

        f.seek(SeekFrom::Start(read_start))?;
        f.read_exact(&mut buffer[..bytes_to_read])?;

        // 内存扫描：从后往前找非续接字节
        for i in (0..bytes_to_read).rev() {
            if (buffer[i] & 0xC0) != 0x80 {
                chars_found += 1;
                current_byte = read_start + i as u64;

                if chars_found == count {
                    return Ok(current_byte);
                }
            }
        }
        // 如果本块读完还没找够，继续循环（此时 current_byte 已更新为 read_start）
        current_byte = read_start;
    }
    Ok(current_byte)
}

/// 辅助函数：根据首字节判断 UTF-8 长度
fn get_utf8_len(first_byte: u8) -> io::Result<usize> {
    match first_byte {
        0..=0x7F => Ok(1),
        0xC0..=0xDF => Ok(2),
        0xE0..=0xEF => Ok(3),
        0xF0..=0xF7 => Ok(4),
        _ => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "无效的 UTF-8 起始字节",
        )),
    }
}

/// byte_pos的位置必须刚好等于全文第char_pos个字符的位置
// pub fn skim_read(
//     file: &mut File,
//     byte_pos: u64,
//     len: usize,
//     win_size: usize,
//     focus_level: f32,
//     focus_words: &[&str],
//     delimiters: &[&str],
//     big_word_base: usize,
//     (mask_s, mask_e): (char, char),
// ) -> anyhow::Result<String> {
//     // 1. 基本校验
//     if focus_level <= 0.0 {
//         return Err(anyhow::anyhow!("关注度必须 > 0"));
//     }
//     if delimiters.is_empty() {
//         return Err(anyhow::anyhow!("分隔符不能为空"));
//     }

//     let content: Vec<char> = read_n_chars(file, byte_pos, len)?;
//     if content.is_empty() {
//         return Ok("EOF".to_string());
//     }
//     if focus_level >= 1.0 {
//         return Ok(content.into_iter().collect());
//     }
//     // 3. 预处理分隔符为字符数组以加快匹配速度
//     let delim_chars: Vec<Vec<char>> = delimiters.iter().map(|s| s.chars().collect()).collect();
//     let focus_chars: Vec<Vec<char>> = focus_words.iter().map(|s| s.chars().collect()).collect();
//     //分词+加权
//     let mut words: Vec<(String, usize, usize, f32)> =
//         split_word(&content, focus_chars, delim_chars, big_word_base);

//     //topk词组
//     let words = {
//         let n = words.len();
//         // 计算保底需要的数量
//         let min_count = (n as f32 * focus_level).ceil() as usize;
//         // 2. 根据分数从高到低排序
//         // 我们使用 unstable_sort 因为它比 stable_sort 快且我们不需要保留相等分数元素的原始顺序
//         words.sort_unstable_by(|a, b| b.3.partial_cmp(&a.3).unwrap_or(std::cmp::Ordering::Equal));

//         // 3. 取前 min_count 个最高分的数据
//         let mut words: Vec<(String, usize)> = words
//             .into_iter()
//             .take(min_count)
//             .map(|(w, end, _, wg)| (w, end))
//             .collect();
//         words.sort_unstable_by(|a, b| a.1.cmp(&b.1));
//         words
//     };

//     //组合结果
//     let mut last_pos = 0;
//     let mut result = String::new();

//     for (w, end) in words {
//         result.push_str(&w);
//         if end < last_pos + win_size {
//             //在窗口内
//             result.push_str(" ");
//         } else {
//             let mask = format!("{mask_s}{end}{mask_e}\n");
//             result.push_str(&mask);
//             last_pos = end;
//         }
//     }
//     // 5. 膨胀检查
//     // 如果 result 的字符数比原文还要多，说明掩码插入过频，直接返回原文
//     if result.chars().count() > content.len() {
//         return Ok(content.into_iter().collect());
//     }

//     Ok(result)
// }


pub fn read_files(dir: &str, ext: &str) -> anyhow::Result<Vec<PathBuf>> {
    let mut paths = vec![];
    // 2. 读取文件夹下的所有项
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let file_path: std::path::PathBuf = entry.path();

        // 3. 过滤出文件，并且扩展名为 "json"
        if file_path.is_file() && file_path.extension().and_then(|s| s.to_str()) == Some(ext) {
            paths.push(file_path);
        }
    }
    Ok(paths)
}

//文件字符位置索引
pub struct FileCharIndex {
    last_write_time: u64,
    file_size: u64,
    //字符位置到字节位置的映射
    pos_map: WindowedSortedCollection<u64, u64>,
}
impl FileCharIndex {
    pub fn save_in_path(&self, path: &PathBuf) -> anyhow::Result<()> {
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(path)?;

        Ok(())
    }
}

// #[cfg(test)]
// #[test]
// fn t() {
//     let s = weighted_retain(
//         vec![
//             (1, 0.1),
//             (2, 0.1),
//             (3, 0.1),
//             (4, 0.1),
//             (5, 0.5),
//             (6, 0.1),
//             (7, 0.1),
//         ],
//         0.2,
//     );
//     println!("{:?}", s);
// }
