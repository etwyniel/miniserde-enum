use miniserde::{json, Serialize};
use miniserde_enum::Serialize_enum;

#[test]
fn test_internal() {
    #[serde(tag = "type")]
    #[derive(Serialize_enum)]
    enum Internal {
        A,
        #[serde(rename = "renamedB")]
        B,
        C {
            x: i32,
        },
    }
    use Internal::*;
    let example = [A, B, C { x: 2 }];
    let actual = json::to_string(&example[..]);
    let expected = r#"[{"type":"A"},{"type":"renamedB"},{"type":"C","x":2}]"#;
    assert_eq!(actual, expected);
}

#[test]
fn test_external() {
    #[derive(Serialize_enum)]
    enum External {
        A,
        #[serde(rename = "renamedB")]
        B(i32, String),
        C {
            x: i32,
        },
    }
    use External::*;
    let example = [A, B(42, "everything".to_string()), C { x: 2 }];
    let actual = json::to_string(&example[..]);
    let expected = r#"["A",{"renamedB":[42,"everything"]},{"C":{"x":2}}]"#;
    assert_eq!(actual, expected);
}

#[test]
fn test_untagged() {
    #[serde(untagged)]
    #[derive(Serialize_enum)]
    enum Untagged {
        A,
        #[serde(rename = "renamedB")]
        B(i32, String),
        C {
            x: i32,
        },
    }
    use Untagged::*;
    let example = [A, B(42, "everything".to_string()), C { x: 2 }];
    let actual = json::to_string(&example[..]);
    let expected = r#"["A",[42,"everything"],{"x":2}]"#;
    assert_eq!(actual, expected);
}
