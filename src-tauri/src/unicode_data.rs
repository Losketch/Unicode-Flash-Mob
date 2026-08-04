use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

fn clean_range_name(value: &str) -> String {
    value
        .replace("First", "")
        .replace("Last", "")
        .replace(['<', '>'], "")
        .trim()
        .trim_end_matches(',')
        .trim()
        .to_string()
}

#[derive(Debug, Clone)]
pub struct UnicodeInfo {
    pub name: String,
    pub category: String,
    pub block: Option<String>,
}

pub struct UnicodeDataManager {
    pub data: HashMap<u32, UnicodeInfo>,
    pub block_ranges: Vec<(u32, u32, String)>,
}

impl UnicodeDataManager {
    pub fn empty() -> Self {
        Self {
            data: HashMap::new(),
            block_ranges: Vec::new(),
        }
    }

    pub fn load(unicode_data_path: &Path, blocks_path: &Path) -> Result<Self> {
        let mut data = HashMap::new();
        let mut block_ranges = Vec::new();

        // UnicodeData encodes large ranges as paired First/Last records.
        if unicode_data_path.exists() {
            let content = std::fs::read_to_string(unicode_data_path).with_context(|| {
                format!(
                    "Failed to read UnicodeData.txt: {}",
                    unicode_data_path.display()
                )
            })?;

            let mut range_start: Option<u32> = None;
            let mut range_desc: String = String::new();

            for line in content.lines() {
                let line = line.trim();
                if line.is_empty() || line.starts_with('#') {
                    continue;
                }

                let parts: Vec<&str> = line.split(';').collect();
                if parts.len() < 2 {
                    continue;
                }

                let cp = match u32::from_str_radix(parts[0], 16) {
                    Ok(cp) => cp,
                    Err(_) => continue,
                };

                let raw_desc = parts[1].trim();
                let category = if parts.len() > 2 {
                    parts[2].to_string()
                } else {
                    String::new()
                };

                if raw_desc.contains("First") {
                    range_start = Some(cp);
                    range_desc = clean_range_name(raw_desc);
                } else if raw_desc.contains("Last") {
                    if let Some(start) = range_start.take() {
                        let desc = clean_range_name(raw_desc);
                        let range_name = if !range_desc.is_empty() {
                            range_desc.clone()
                        } else {
                            desc
                        };

                        for cp_in_range in start..=cp {
                            let name_with_cp = format!("{}-{:04X}", range_name, cp_in_range);
                            data.insert(
                                cp_in_range,
                                UnicodeInfo {
                                    name: name_with_cp,
                                    category: category.clone(),
                                    block: None,
                                },
                            );
                        }
                    }
                    range_desc.clear();
                } else {
                    let desc = raw_desc.replace('-', " ").trim().to_string();
                    data.insert(
                        cp,
                        UnicodeInfo {
                            name: desc,
                            category,
                            block: None,
                        },
                    );
                }
            }
        }

        if blocks_path.exists() {
            let content = std::fs::read_to_string(blocks_path).with_context(|| {
                format!(
                    "Failed to read UnicodeBlocks.txt: {}",
                    blocks_path.display()
                )
            })?;

            for line in content.lines() {
                let line = line.trim();
                if line.is_empty() || line.starts_with('#') {
                    continue;
                }

                // Format: 0000..007F; Basic Latin
                let parts: Vec<&str> = line.split(';').collect();
                if parts.len() < 2 {
                    continue;
                }

                let range_str = parts[0].trim();
                let block_name = parts[1].trim();

                if let Some((start_str, end_str)) = range_str.split_once("..") {
                    if let (Ok(start), Ok(end)) = (
                        u32::from_str_radix(start_str, 16),
                        u32::from_str_radix(end_str, 16),
                    ) {
                        block_ranges.push((start, end, block_name.to_string()));

                        for cp in start..=end {
                            if let Some(info) = data.get_mut(&cp) {
                                info.block = Some(block_name.to_string());
                            }
                        }
                    }
                }
            }
        }

        Ok(Self { data, block_ranges })
    }

    pub fn get_info(&self, cp: u32) -> Option<&UnicodeInfo> {
        self.data.get(&cp)
    }

    pub fn get_description(&self, cp: u32) -> String {
        let block = self
            .get_block(cp)
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| "No_Block".to_string());
        let name = self
            .data
            .get(&cp)
            .map(|info| info.name.trim())
            .filter(|value| !value.is_empty())
            .unwrap_or("UNDEFINED_CHARACTERS");

        format!("{}\n{}", block, name)
    }

    pub fn get_name(&self, cp: u32) -> String {
        self.data
            .get(&cp)
            .map(|i| i.name.clone())
            .unwrap_or_default()
    }

    pub fn get_category(&self, cp: u32) -> String {
        self.data
            .get(&cp)
            .map(|i| i.category.clone())
            .unwrap_or_default()
    }

    pub fn is_combining_mark(&self, cp: u32) -> bool {
        if let Some(info) = self.data.get(&cp) {
            let cat = &info.category;
            cat == "Mn" || cat == "Mc" || cat == "Me"
        } else {
            matches!(
                cp,
                0x0300..=0x036F
                    | 0x1AB0..=0x1AFF
                    | 0x1DC0..=0x1DFF
                    | 0x20D0..=0x20FF
                    | 0xFE20..=0xFE2F
            )
        }
    }

    pub fn get_block(&self, cp: u32) -> Option<String> {
        if let Some(info) = self.data.get(&cp) {
            if info.block.is_some() {
                return info.block.clone();
            }
        }

        for (start, end, name) in &self.block_ranges {
            if cp >= *start && cp <= *end {
                return Some(name.clone());
            }
        }

        None
    }
}

pub fn find_unicode_data_files() -> (PathBuf, PathBuf) {
    let data_dir = crate::json_config::asset_path("data");
    let data_path = data_dir.join("UnicodeData.txt");
    let blocks_path = data_dir.join("UnicodeBlocks.txt");
    (data_path, blocks_path)
}

#[cfg(test)]
mod tests {
    use super::{UnicodeDataManager, UnicodeInfo};
    use std::collections::HashMap;

    #[test]
    fn description_uses_block_and_undefined_placeholder_when_data_is_missing() {
        let manager = UnicodeDataManager {
            data: HashMap::new(),
            block_ranges: vec![(0x0000, 0x007F, "Basic Latin".to_string())],
        };

        assert_eq!(
            manager.get_description(0x0037),
            "Basic Latin\nUNDEFINED_CHARACTERS"
        );
    }

    #[test]
    fn description_uses_no_block_and_character_name_when_block_is_missing() {
        let mut data = HashMap::new();
        data.insert(
            0xFFFD,
            UnicodeInfo {
                name: "REPLACEMENT CHARACTER".to_string(),
                category: "So".to_string(),
                block: None,
            },
        );
        let manager = UnicodeDataManager {
            data,
            block_ranges: Vec::new(),
        };

        assert_eq!(
            manager.get_description(0xFFFD),
            "No_Block\nREPLACEMENT CHARACTER"
        );
    }

    #[test]
    fn description_uses_both_placeholders_when_data_and_block_are_missing() {
        let manager = UnicodeDataManager::empty();

        assert_eq!(
            manager.get_description(0x0378),
            "No_Block\nUNDEFINED_CHARACTERS"
        );
    }
}
