# miniserde-enum

This crate exposes derive macros for miniserde's `Serialize` and `Deserialize`
traits on enums.

The goal of this crate is to provide enum support like that of Serde for
miniserde (see [Serde's list of enum representations](https://serde.rs/enum-representations.html)).

# Examples

## Deserializing an externally tagged enum
```rust
use miniserde::{Deserialize, json};
use miniserde_enum::Deserialize_enum;

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
```

## Serializing an internally tagged enum
```rust
use miniserde::{json, Serialize};
use miniserde_enum::Serialize_enum;

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
```

More examples can be found in the [tests](https://github.com/etwyniel/miniserde-enum/tree/master/tests)
directory.

# TODO

- Serialization:
  - ~~Externally tagged enums~~
  - ~~Internally tagged enums~~
  - ~~Untagged enums~~
  - Adjacently tagged enums
- Deserialization
  - ~~Externally tagged enums~~
  - Internally tagged enums
  - Untagged enums
  - Adjacently tagged enums
