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
