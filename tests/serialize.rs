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
        A(i32),
        #[serde(rename = "renamedB")]
        B(i32, String),
        C {
            x: i32,
        },
        D,
    }
    use External::*;
    let example = [A(21), B(42, "everything".to_string()), C { x: 2 }, D];
    let actual = json::to_string(&example[..]);
    let expected = r#"[{"A":21},{"renamedB":[42,"everything"]},{"C":{"x":2}},"D"]"#;
    assert_eq!(actual, expected);
}

#[test]
fn test_untagged() {
    #[serde(untagged)]
    #[derive(Serialize_enum)]
    enum Untagged {
        A(i32),
        #[serde(rename = "renamedB")]
        B(i32, String),
        C {
            x: i32,
        },
        D,
    }
    use Untagged::*;
    let example = [A(21), B(42, "everything".to_string()), C { x: 2 }, D];
    let actual = json::to_string(&example[..]);
    let expected = r#"[21,[42,"everything"],{"x":2},"D"]"#;
    assert_eq!(actual, expected);
}

#[test]
fn generic_named() {
    #[derive(Serialize_enum)]
    enum Gen<T: Serialize> {
        A{t: T},
        B,
    }
    use Gen::*;
    let example = [A{t: "abc"}, B];
    let actual = json::to_string(&example[..]);
    let expected = r#"[{"A":{"t":"abc"}},"B"]"#;
    assert_eq!(actual, expected);
}

#[test]
fn generic_unnamed() {
    #[derive(Serialize_enum)]
    enum Gen<T: Serialize> {
        A(T),
        B,
    }
    use Gen::*;
    let example = [A("abc"), B];
    let actual = json::to_string(&example[..]);
    let expected = r#"[{"A":"abc"},"B"]"#;
    assert_eq!(actual, expected);
}
