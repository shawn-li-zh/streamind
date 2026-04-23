use std::collections::HashSet;
use std::fs::File;
use std::io::{self, BufReader, Read, Seek, SeekFrom};
use std::path::PathBuf;
use std::result;

use rand::rngs::StdRng;
use rand::{Rng, SeedableRng, thread_rng};
use serde::de;
use unicode_reader::CodePoints;

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

fn digits_usize_fast(n: usize) -> usize {
    if n < 10 {
        1
    } else if n < 100 {
        2
    } else if n < 1000 {
        3
    } else if n < 10000 {
        4
    } else if n < 100000 {
        5
    } else if n < 1000000 {
        6
    } else if n < 10000000 {
        7
    } else if n < 100000000 {
        8
    } else if n < 1000000000 {
        9
    } else {
        10
    }
}

/// byte_pos的位置必须刚好等于全文第char_pos个字符的位置

pub fn skim_read(
    file: &mut File,
    byte_pos: u64,
    len: usize,
    win_size: usize,
    focus_level: f32,
    focus_words: &[&str],
    delimiters: &[&str],
    big_word_base: usize,
    (mask_s, mask_e): (char, char),
) -> anyhow::Result<String> {
    // 1. 基本校验
    if focus_level <= 0.0 {
        return Err(anyhow::anyhow!("关注度必须 > 0"));
    }
    if delimiters.is_empty() {
        return Err(anyhow::anyhow!("分隔符不能为空"));
    }

    // 2. 位置校准与读取
    if byte_pos != 0 {
        let pos = find_boundary_near(file, byte_pos, true)?;
        if pos != byte_pos {
            return Err(anyhow::anyhow!("读取位置异常,非有效utf8字符"));
        }
    }
    let content: Vec<char> = read_n_chars(file, byte_pos, len)?;
    if content.is_empty() {
        return Ok("EOF".to_string());
    }
    if focus_level >= 1.0 {
        return Ok(content.into_iter().collect());
    }
    // 3. 预处理分隔符为字符数组以加快匹配速度
    let delim_chars: Vec<Vec<char>> = delimiters.iter().map(|s| s.chars().collect()).collect();
    let focus_chars = focus_words.iter().map(|s| s.chars().collect()).collect();
    //分词+加权
    let mut words: Vec<(String, usize, usize, f32)> =
        split_word(&content, focus_chars, delim_chars, big_word_base);

    //topk词组
    let words = {
        let n = words.len();
        // 计算保底需要的数量
        let min_count = (n as f32 * focus_level).ceil() as usize;
        // 2. 根据分数从高到低排序
        // 我们使用 unstable_sort 因为它比 stable_sort 快且我们不需要保留相等分数元素的原始顺序
        words.sort_unstable_by(|a, b| b.3.partial_cmp(&a.3).unwrap_or(std::cmp::Ordering::Equal));

        // 3. 取前 min_count 个最高分的数据
        let mut words: Vec<(String, usize)> = words
            .into_iter()
            .take(min_count)
            .map(|(w, end, _, wg)| (w, end))
            .collect();
        words.sort_unstable_by(|a, b| a.1.cmp(&b.1));
        words
    };

    //组合结果
    let mut last_pos = 0;
    let mut result = String::new();

    for (w, end) in words {
        result.push_str(&w);
        if end < last_pos + win_size {
            //在窗口内
            result.push_str(" ");
        } else {
            let mask = format!("{mask_s}{end}{mask_e}\n");
            result.push_str(&mask);
            last_pos = end;
        }
    }
    // 5. 膨胀检查
    // 如果 result 的字符数比原文还要多，说明掩码插入过频，直接返回原文
    if result.chars().count() > content.len() {
        return Ok(content.into_iter().collect());
    }

    Ok(result)
}

fn split_word(
    content: &Vec<char>,
    focus_chars: Vec<Vec<char>>,
    delim_chars: Vec<Vec<char>>,
    big_word_base: usize,
) -> Vec<((String, usize, usize, f32))> {
    let mut last_word_end = 0;
    let mut cursor = 0;
    let mut words: Vec<((String, usize, usize, f32))> = vec![];
    let mut rng = thread_rng();
    // 4. 扫描内容
    while cursor < content.len() {
        let mut matched_delim_len = 0;

        // 检查当前位置是否匹配任何一个分隔符
        for delim in &delim_chars {
            if content[cursor..].starts_with(delim) {
                matched_delim_len = delim.len();
                break;
            }
        }
        // 如果找到了分隔符，或者到了数组末尾，就处理之前的“词”
        if cursor == content.len() - 1 {
            matched_delim_len = 1;
        } else if matched_delim_len == 0 {
            //没匹配
            cursor += 1;
            continue;
        }

        // 确定当前词的终点
        let word_end = if matched_delim_len > 0 {
            cursor
        } else {
            content.len()
        };
        let word_slice = &content[last_word_end..word_end];
        // 更新指针
        cursor += matched_delim_len;
        //前面的词大于掩码
        if digits_usize_fast(word_end) + 2 < word_slice.len() {
            let word = String::from_iter(word_slice);
            // 更新下一个词的起始点
            last_word_end = cursor;
            let pre_word = word.trim();
            if pre_word.is_empty() {
                continue;
            }
            let pre_word = pre_word.to_string();
            let word_len = word_slice.len() - (word.len() - pre_word.len());
            let mut weight = 0.0;
            {
                //放大长词权重

                if word_len < big_word_base {
                    weight += (word_len as f32) / big_word_base as f32;
                } else {
                    weight += 0.3;
                }
                //放大命中字符
                if !focus_chars.is_empty() {
                    let mut total = 0.0;
                    let mut times = 0;
                    for (hit, len) in count_unique_character_hits(word_slice, &focus_chars) {
                        if hit > 0 {
                            total += hit as f32 / len as f32;
                            times += 1;
                        }
                    }
                    if times != 0 {
                        let avg = total / times as f32;
                        weight = avg;
                    }
                }
            }
            let r: f32 = rng.r#gen(); // 生成 0.0 到 1.0 之间的随机数
            words.push(((pre_word, word_end, word_len, weight + r)));
        }
    }
    words
}

pub fn count_unique_character_hits(
    long_str: &[char],
    short_strs: &Vec<Vec<char>>,
) -> Vec<(usize, usize)> {
    // 定义阈值：如果 long_str 长度超过 64，则构建索引
    const THRESHOLD: usize = 64;

    if long_str.len() <= THRESHOLD {
        // --- 策略 A: 线性扫描 (利用 L1 缓存) ---
        // 这种情况下，遍历数组比计算哈希值更快
        short_strs
            .iter()
            .map(|short| {
                let hit_count = short.iter().filter(|&&c| long_str.contains(&c)).count();
                (hit_count, short.len())
            })
            .collect()
    } else {
        // --- 策略 B: 哈希集合 (O(1) 查找) ---
        // 虽然构建 HashSet 有开销，但被大量的 O(1) 查找抵消了
        let set: HashSet<char> = long_str.iter().cloned().collect();
        short_strs
            .iter()
            .map(|short| {
                let hit_count = short.iter().filter(|&&c| set.contains(&c)).count();
                (hit_count, short.len())
            })
            .collect()
    }
}

/// 基于权重随机保留数据
/// - input: Vec<(数据, 权重)>
/// - rate: 保留比例 (0.0 ~ 1.0)
fn weighted_retain<T>(input: Vec<(T, f32)>, rate: f32) -> Vec<T> {
    if input.is_empty() {
        return vec![];
    }
    let n = input.len();
    // 计算保底需要的数量
    let min_count = (n as f32 * rate).ceil() as usize;
    let mut rng = thread_rng();
    // 1. 为每个元素生成一个加权随机分数
    let mut scored_data: Vec<(T, f32)> = input
        .into_iter()
        .map(|(data, prob)| {
            let r: f32 = rng.r#gen(); // 生成 0.0 到 1.0 之间的随机数
            let score = (r + prob) / 2.0;
            (data, score)
        })
        .collect();

    // 2. 根据分数从高到低排序
    // 我们使用 unstable_sort 因为它比 stable_sort 快且我们不需要保留相等分数元素的原始顺序
    scored_data.sort_unstable_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    // 3. 取前 min_count 个最高分的数据
    scored_data
        .into_iter()
        .take(min_count)
        .map(|(data, _score)| data)
        .collect()
}

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
#[cfg(test)]
#[test]
fn t() {
    let s = weighted_retain(
        vec![
            (1, 0.1),
            (2, 0.1),
            (3, 0.1),
            (4, 0.1),
            (5, 0.5),
            (6, 0.1),
            (7, 0.1),
        ],
        0.2,
    );
    println!("{:?}", s);
}
