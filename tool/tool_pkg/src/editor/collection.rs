use std::{fs::OpenOptions, ops::Sub, path::PathBuf, time::SystemTime};

pub struct WindowedSortedCollection<B, D> {
    items: Vec<(B, D)>,
    window_size: B,
}

impl<B, D> WindowedSortedCollection<B, D>
where
    B: PartialOrd + Copy + Sub<Output = B>,
{
    pub fn new(window_size: B) -> Self {
        Self {
            items: Vec::new(),
            window_size,
        }
    }
    pub fn clear(&mut self) {
        self.items.clear();
    }
    pub fn insert(&mut self, basis: B, data: D) -> bool {
        if let Some(((b, d), pos)) = self.find_nearest_with_pos(basis) {
            let diff = if *b > basis { *b - basis } else { basis - *b };
            //在窗口内就没必要插入
            if diff < self.window_size {
                return false;
            }
            self.items.insert(pos, (basis, data));
            return true;
        }
        self.items.insert(0, (basis, data));
        true
    }

    /// 快速查找距离给定基数最近的数据
    /// 返回 Option<&(基数, 数据)>
    pub fn find_nearest(&self, target_basis: B) -> Option<&(B, D)> {
        self.find_nearest_with_pos(target_basis).map(|(d, _)| d)
    }

    fn find_nearest_with_pos(&self, target_basis: B) -> Option<(&(B, D), usize)> {
        if self.items.is_empty() {
            return None;
        }

        // 1. 二分查找位置
        let pos = self
            .items
            .binary_search_by(|(b, _)| b.partial_cmp(&target_basis).unwrap())
            .unwrap_or_else(|e| e);

        // 2. 边界处理：如果 pos 指向末尾，则最后一个最接近
        if pos >= self.items.len() {
            return self.items.last().map(|t| (t, pos));
        }
        // 3. 边界处理：如果 pos 指向开头，则第一个最接近
        if pos == 0 {
            return self.items.first().map(|t| (t, pos));
        }

        // 4. 比较前后两个邻居的距离
        let left = &self.items[pos - 1];
        let right = &self.items[pos];

        let dist_left = target_basis - left.0;
        let dist_right = right.0 - target_basis;

        if dist_left <= dist_right {
            Some((left, pos))
        } else {
            Some((right, pos))
        }
    }

    pub fn items(&self) -> &[(B, D)] {
        &self.items
    }
}


