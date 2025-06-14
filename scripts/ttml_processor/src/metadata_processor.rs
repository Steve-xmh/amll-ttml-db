//! metadata_processor.rs
//!
//! 该模块定义了 `MetadataStore`，一个用于统一处理、规范化和存储从
//! 各种歌词源文件中解析出的元数据的核心组件。
//!
//! 主要职责包括：
//! 1. 将不同别名或格式的元数据键（如 "title", "musicName"）映射到
//!    一个统一的规范化键（`CanonicalMetadataKey`）。
//! 2. 存储和管理元数据值，支持单个键对应多个值的情况（例如，多个艺术家）。
//! 3. 提供数据清理功能，如去除重复值和首尾空格。
//! 4. 作为一个中间层，为后续的验证和文件生成步骤提供干净、一致的元数据源。

// 引入所需的类型，包括规范化元数据键及其解析错误。
use crate::types::{CanonicalMetadataKey, ParseCanonicalMetadataKeyError};
use std::collections::HashMap;

/// `MetadataStore` 是一个用于存储和管理规范化后元数据的容器。
///
/// 它内部使用一个 `HashMap` 来存储数据，其中键是 `CanonicalMetadataKey` 枚举，
/// 确保了元数据键的类型安全和一致性；值是一个 `Vec<String>`，
/// 用以支持像“艺术家”这样可能拥有多个值的元数据项。
#[derive(Debug, Clone, Default)]
pub struct MetadataStore {
    /// 内部数据存储结构。
    /// Key: `CanonicalMetadataKey` - 经过规范化处理的元数据键。
    /// Value: `Vec<String>` - 对应此键的所有值的列表。
    data: HashMap<CanonicalMetadataKey, Vec<String>>,
}

impl MetadataStore {
    /// 创建一个新的、空的 `MetadataStore` 实例。
    ///
    /// # 返回
    ///
    /// 返回一个初始化的 `MetadataStore`。
    pub fn new() -> Self {
        Self::default()
    }

    /// 向存储中添加一个元数据键值对。
    ///
    /// 该方法是 `MetadataStore` 的核心功能之一。它执行以下操作：
    /// 1. 对传入的 `value` 字符串进行修剪（trim），去除首尾的空白字符。
    /// 2. 如果修剪后的值为空字符串，则忽略该值，不进行添加。
    /// 3. 尝试将传入的 `key_str` 字符串解析为一个标准的 `CanonicalMetadataKey`。
    /// 4. 如果解析成功（例如，"title" 被解析为 `CanonicalMetadataKey::Title`），则使用标准键。
    /// 5. 如果解析失败，则将原始的 `key_str`（转换为小写）作为一个自定义键 `CanonicalMetadataKey::Custom(...)` 来存储。
    /// 6. 将处理后的值添加到对应键的值列表中。如果键不存在，则会先创建一个新的空列表。
    ///
    /// # 参数
    ///
    /// * `key_str` - 原始的元数据键字符串 (例如, "title", "ncmMusicId", 或一个非标准的 "my-custom-tag")。
    /// * `value` - 与键关联的元数据值。
    ///
    /// # 返回
    ///
    /// * `Ok(())` - 如果值被成功添加或因其为空而被安全忽略。
    ///
    /// # 注意
    ///
    /// 此函数目前返回 `Result` 主要是为了保持 API 的扩展性，但在当前实现中，
    /// 由于所有解析失败的情况都被优雅地处理为自定义键，因此它实际上总会返回 `Ok(())`。
    pub fn add(
        &mut self,
        key_str: &str,
        value: String,
    ) -> Result<(), ParseCanonicalMetadataKeyError> {
        // 1. 修剪值的首尾空白字符
        let trimmed_value = value.trim();

        // 2. 如果值为空，则直接返回
        if trimmed_value.is_empty() {
            return Ok(());
        }

        // 3. 尝试将字符串键解析为规范化键。
        //    `unwrap_or_else` 保证了即使解析失败，程序也能继续执行，
        //    此时会将原始键的小写形式作为自定义键处理。
        let canonical_key = key_str
            .parse::<CanonicalMetadataKey>()
            .unwrap_or_else(|_| CanonicalMetadataKey::Custom(key_str.to_lowercase()));

        // 4. 获取键对应的值列表（如果不存在则创建），并将修剪后的值添加进去。
        self.data
            .entry(canonical_key)
            .or_default()
            .push(trimmed_value.to_string());

        Ok(())
    }

    /// 获取指定规范化键对应的第一个值。
    ///
    /// # 参数
    ///
    /// * `key` - 一个对 `CanonicalMetadataKey` 的引用。
    ///
    /// # 返回
    ///
    /// * `Some(&String)` - 如果键存在且其值列表不为空，则返回第一个值的引用。
    /// * `None` - 如果键不存在或其值列表为空。
    pub fn get_single_value(&self, key: &CanonicalMetadataKey) -> Option<&String> {
        self.data.get(key).and_then(|values| values.first())
    }

    /// 获取指定规范化键对应的所有值的列表。
    ///
    /// 对于可能存在多个值的元数据项（如 `Artist`, `Songwriter`），此方法返回包含所有值的向量的引用。
    ///
    /// # 参数
    ///
    /// * `key` - 一个对 `CanonicalMetadataKey` 的引用。
    ///
    /// # 返回
    ///
    /// * `Some(&Vec<String>)` - 如果键存在，返回其完整值列表的引用。
    /// * `None` - 如果键不存在。
    pub fn get_multiple_values(&self, key: &CanonicalMetadataKey) -> Option<&Vec<String>> {
        self.data.get(key)
    }

    /// 对存储中的所有元数据值进行清理和去重。
    ///
    /// 它执行以下步骤：
    /// 1. 遍历所有键值对。
    /// 2. 对每个值列表中的所有字符串再次进行 trim 和空值移除，以防数据源不一致。
    /// 3. 如果清理后一个键的值列表变为空，则将该键标记为待删除。
    /// 4. 对非空的值列表进行不稳定排序（`sort_unstable`），然后使用 `dedup` 移除相邻的重复项。
    /// 5. 最后，移除所有被标记为待删除的键。
    pub fn deduplicate_values(&mut self) {
        let mut keys_to_remove: Vec<CanonicalMetadataKey> = Vec::new();

        for (key, values) in self.data.iter_mut() {
            // 确保所有值都已 trim 并移除空值
            values.iter_mut().for_each(|v| *v = v.trim().to_string());
            values.retain(|v| !v.is_empty());

            // 如果值列表变为空，则记录该键以便稍后移除
            if values.is_empty() {
                keys_to_remove.push(key.clone());
                continue;
            }

            // 排序并去重，这是高效去重的标准做法
            values.sort_unstable();
            values.dedup();
        }

        // 移除所有值已变为空的条目
        for key in keys_to_remove {
            self.data.remove(&key);
        }
    }

    /// 从一个原始的、未规范化的元数据 `HashMap` 中加载数据。
    ///
    /// 这个方法通常在解析完源文件后调用，用于将解析器产出的原始元数据
    /// (`HashMap<String, Vec<String>>`) 填入 `MetadataStore`，
    /// 在这个过程中会通过调用 `add` 方法来完成键的规范化和值的清理。
    ///
    /// # 参数
    ///
    /// * `raw_metadata` - 一个包含原始键值对的 HashMap 的引用。
    pub fn load_from_raw(&mut self, raw_metadata: &HashMap<String, Vec<String>>) {
        for (key, values) in raw_metadata {
            for value in values {
                // 调用 self.add 来处理每一个键值对，实现规范化
                // `let _ = ...` 用于表示我们不关心 add 方法的返回值
                let _ = self.add(key, value.clone());
            }
        }
    }

    /// 将存储的元数据转换为一个可序列化（例如，转换为 JSON）的 `HashMap`。
    ///
    /// 此方法只包含被认为是“公共”的元数据项，用于输出。
    /// “公共”的定义由 `CanonicalMetadataKey::is_public()` 方法决定。
    /// 这可以防止内部使用的的元数据被意外暴露。
    ///
    /// # 返回
    ///
    /// 返回一个新的 `HashMap<String, Vec<String>>`，其中键是元数据键的字符串表示。
    pub fn to_serializable_map(&self) -> HashMap<String, Vec<String>> {
        self.data
            .iter()
            // 筛选出公共元数据
            .filter(|(key, _values)| key.is_public())
            // 将 CanonicalMetadataKey 转换为 String，并克隆值
            .map(|(key, values)| (key.to_string(), values.clone()))
            // 收集成一个新的 HashMap
            .collect()
    }

    // /// (目前不使用) 清空存储中的所有元数据。
    // pub fn clear(&mut self) {
    //     self.data.clear();
    // }
}
