use miniserde::{json, Deserialize};
use miniserde_enum::Deserialize_enum;

#[test]
fn test_external() {
    #[derive(Deserialize_enum, Debug, PartialEq)]
    enum External {
        // A(i32),
        // #[serde(rename = "renamedB")]
        // B(i32, String),
        C {
            x: i32,
        },
        // D,
    }
    use External::*;
    // let example = r#"[{"A":21},{"renamedB":[42,"everything"]},{"C":{"x":2}},"D"]"#;
    let example = r#"[{"C":{"x":2}}]"#;
    let actual: Vec<External> = json::from_str(example).unwrap();
    // let expected = [A(21), B(42, "everything".to_string()), C { x: 2 }, D];
    let expected = vec![C { x: 2 }];
    assert_eq!(actual, expected);
}
