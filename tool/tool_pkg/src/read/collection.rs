use std::ops::Sub;

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
    pub fn clear(&mut self){
        self.items.clear();
    }
    pub fn insert(&mut self, basis: B, data: D) -> bool {
        let pos = self
            .items
            .binary_search_by(|(existing_basis, _)| existing_basis.partial_cmp(&basis).unwrap())
            .unwrap_or_else(|e| e);

        if let Some((next_basis, _)) = self.items.get(pos) {
            if *next_basis - basis < self.window_size {
                return false;
            }
        }
        if pos > 0 {
            if let Some((prev_basis, _)) = self.items.get(pos - 1) {
                if basis - *prev_basis < self.window_size {
                    return false;
                }
            }
        }

        self.items.insert(pos, (basis, data));
        true
    }

    /// 【新函数】快速查找距离给定基数最近的数据
    /// 返回 Option<&(基数, 数据)>
    pub fn find_nearest(&self, target_basis: B) -> Option<&(B, D)> {
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
            return self.items.last();
        }
        // 3. 边界处理：如果 pos 指向开头，则第一个最接近
        if pos == 0 {
            return self.items.first();
        }

        // 4. 比较前后两个邻居的距离
        let left = &self.items[pos - 1];
        let right = &self.items[pos];

        let dist_left = target_basis - left.0;
        let dist_right = right.0 - target_basis;

        if dist_left <= dist_right {
            Some(left)
        } else {
            Some(right)
        }
    }

    /// 【新函数】查找“不大于”目标值的最近数据 (常用于 Seek 锚点)
    /// 比如查找 55，集合有 [10, 50, 100]，则返回 50
    pub fn find_nearest_le(&self, target_basis: B) -> Option<&(B, D)> {
        let pos = self
            .items
            .binary_search_by(|(b, _)| b.partial_cmp(&target_basis).unwrap())
            .unwrap_or_else(|e| e);

        // 如果刚好匹配到，或者是 Err(pos)，pos 之前的那个就是最大的 LE (Less or Equal)
        if pos < self.items.len() && self.items[pos].0 == target_basis {
            Some(&self.items[pos])
        } else if pos > 0 {
            Some(&self.items[pos - 1])
        } else {
            None
        }
    }

    pub fn items(&self) -> &[(B, D)] {
        &self.items
    }
}
