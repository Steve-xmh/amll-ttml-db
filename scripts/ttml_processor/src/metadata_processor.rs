use crate::types::{CanonicalMetadataKey, ParseCanonicalMetadataKeyError};
use std::collections::HashMap;

/// 一个用于存储和管理规范化后元数据的容器。
#[derive(Debug, Clone, Default)]
pub struct MetadataStore {
    data: HashMap<CanonicalMetadataKey, Vec<String>>,
}

impl MetadataStore {
    /// 创建一个空的 `MetadataStore` 实例。
    pub fn new() -> Self {
        Self {
            data: HashMap::new(),
        }
    }

    /// 添加一个键值对到存储中
    ///
    /// 先尝试将 `key_str` 解析为标准键。
    /// 如果失败，将 `key_str` 作为自定义键来存储。
    /// 值的前后空白会被移除，如果处理后值为空字符串，则不会被添加。
    ///
    /// # 参数
    ///
    /// * `key_str` - 原始的元数据键字符串 (例如, "title", "ncmMusicId")
    /// * `value` - 元数据的值
    ///
    /// # 返回
    ///
    /// * `Ok(())` - 如果添加成功
    pub fn add(
        &mut self,
        key_str: &str,
        value: String,
    ) -> Result<(), ParseCanonicalMetadataKeyError> {
        let trimmed_value = value.trim();
        if trimmed_value.is_empty() {
            return Ok(()); // 不添加空值
        }

        // 尝试解析为标准键，如果失败则作为自定义键处理
        let canonical_key = key_str
            .parse::<CanonicalMetadataKey>()
            .unwrap_or_else(|_| CanonicalMetadataKey::Custom(key_str.to_lowercase()));

        self.data
            .entry(canonical_key)
            .or_default()
            .push(trimmed_value.to_string());

        Ok(())
    }

    /// 获取指定键的第一个值。
    pub fn get_single_value(&self, key: &CanonicalMetadataKey) -> Option<&String> {
        self.data.get(key).and_then(|values| values.first())
    }

    /// 获取指定键的所有值。
    pub fn get_multiple_values(&self, key: &CanonicalMetadataKey) -> Option<&Vec<String>> {
        self.data.get(key)
    }

    /// 清理并去重存储中的所有值。
    pub fn deduplicate_values(&mut self) {
        let mut keys_to_remove: Vec<CanonicalMetadataKey> = Vec::new();
        for (key, values) in self.data.iter_mut() {
            // 确保所有值都已 trim 并移除空值
            values.iter_mut().for_each(|v| *v = v.trim().to_string());
            values.retain(|v| !v.is_empty());

            if values.is_empty() {
                keys_to_remove.push(key.clone());
                continue;
            }
            // 排序并去重
            values.sort_unstable();
            values.dedup();
        }
        for key in keys_to_remove {
            self.data.remove(&key);
        }
    }

    /// 从一个原始的元数据 `HashMap` 加载数据到 `MetadataStore`。
    pub fn load_from_raw(&mut self, raw_metadata: &HashMap<String, Vec<String>>) {
        for (key, values) in raw_metadata {
            for value in values {
                let _ = self.add(key, value.clone());
            }
        }
    }

    pub fn to_serializable_map(&self) -> HashMap<String, Vec<String>> {
        self.data
            .iter()
            .filter(|(key, _values)| key.is_public())
            .map(|(key, values)| (key.to_string(), values.clone()))
            .collect()
    }

    // /// 清空存储中的所有元数据。
    // pub fn clear(&mut self) {
    //     self.data.clear();
    // }
}
