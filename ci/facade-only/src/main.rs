//! The derives must compile with `dramaturge` as the only dependency.

use dramaturge::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, FactValue, ConditionBuilder)]
enum Fact {
    #[fact(bool)]
    MetMara,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, DialogTag)]
enum Tag {
    #[tag(axis = "moral", key = "good")]
    Good,
}

fn main() {
    let _ = Fact::MetMara.is_true(FactScope::Player);
    assert_eq!(Tag::Good.axis(), "moral");
    assert_eq!(Tag::Good.key(), "good");
}
