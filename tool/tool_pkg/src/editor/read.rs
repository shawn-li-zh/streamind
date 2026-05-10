use std::collections::HashSet;

use rand::{Rng, thread_rng};

pub fn skim_read(
    content: Vec<char>,
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
    if content.is_empty() {
        return Ok("EOF".to_string());
    }
    if focus_level >= 1.0 {
        return Ok(content.into_iter().collect());
    }
    // 3. 预处理分隔符为字符数组以加快匹配速度
    let delim_chars: Vec<Vec<char>> = delimiters.iter().map(|s| s.chars().collect()).collect();
    let focus_chars: Vec<Vec<char>> = focus_words.iter().map(|s| s.chars().collect()).collect();
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
