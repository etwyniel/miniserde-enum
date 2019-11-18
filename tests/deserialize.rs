use miniserde::{json, Deserialize};
use miniserde_enum::Deserialize_enum;

#[test]
fn test_external() {
    #[derive(Deserialize_enum, Debug, PartialEq)]
    enum External {
        A(i32),
        #[serde(rename = "renamedB")]
        B(i32, String),
        C {
            x: i32,
        },
        D,
    }
    use External::*;
    let example = r#"[{"A":21},{"renamedB":[42,"everything"]},{"C":{"x":2}},"D"]"#;
    let actual: Vec<External> = json::from_str(example).unwrap();
    let expected = [A(21), B(42, "everything".to_string()), C { x: 2 }, D];
    assert_eq!(actual, expected);
}

#[test]
fn test_adjacent() {
    #[derive(Deserialize_enum, Debug, PartialEq)]
    #[serde(tag = "type", content = "content")]
    enum Adjacent {
        A(i32),
        #[serde(rename = "renamedB")]
        B(i32, String),
        C {
            x: i32,
        },
        D,
    }
    use Adjacent::*;
    let example = r#"[{"type":"A","content":21},{"type":"renamedB","content":[42,"everything"]},{"type":"C","content":{"x":2}},{"type":"D"}]"#;
    let actual: Vec<Adjacent> = json::from_str(example).unwrap();
    let expected = [A(21), B(42, "everything".to_string()), C { x: 2 }, D];
    assert_eq!(actual, expected);
}

#[test]
fn test_internal() {
    #[derive(Deserialize_enum, Debug, PartialEq)]
    #[serde(tag = "type")]
    enum Internal {
        #[serde(rename = "renamedB")]
        B,
        C {
            x: i32,
        },
        D,
    }
    use Internal::*;
    let example = r#"[{"type":"renamedB"},{"type":"C","x":2},{"type":"D"}]"#;
    let actual: Vec<Internal> = json::from_str(example).unwrap();
    let expected = [B, C { x: 2 }, D];
    assert_eq!(actual, expected);
}
