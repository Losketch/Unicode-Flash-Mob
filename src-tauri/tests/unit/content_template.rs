use super::*;

#[test]
fn resolves_builtin_and_custom_templates() {
    let definitions = BTreeMap::from([(
        "label".to_string(),
        ContentTemplate::Text {
            value: "{char} — {description}".to_string(),
        },
    )]);
    let resolver = ContentTemplateResolver::new(definitions);
    let context = TemplateContext {
        character: "A",
        glyph_expression: "U+0041",
        description: "LATIN CAPITAL LETTER A",
    };
    assert_eq!(
        resolver.resolve("[{label}]", &context).unwrap(),
        "[A — LATIN CAPITAL LETTER A]"
    );
}

#[test]
fn rejects_recursive_templates() {
    let definitions = BTreeMap::from([(
        "loop".to_string(),
        ContentTemplate::Text {
            value: "{loop}".to_string(),
        },
    )]);
    let resolver = ContentTemplateResolver::new(definitions);
    let context = TemplateContext {
        character: "A",
        glyph_expression: "U+0041",
        description: "LATIN CAPITAL LETTER A",
    };
    assert!(resolver.resolve("{loop}", &context).is_err());
}
