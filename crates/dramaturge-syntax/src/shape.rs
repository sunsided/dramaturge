//! The serde shape of a dialogue (YAML, JSON and other formats).
//!
//! ```yaml
//! with: mara
//! start: [again, intro]
//! nodes:
//!   intro:
//!     say: "Hello."
//!     set: { met_mara: true }
//!     options:
//!       - say: "Who are you?"
//!         go: names
//!       - say: "Goodbye."
//!         go: end
//! ```
//!
//! `nodes` is a map from node key to node; the order of the file is the
//! order of [`Dialogue::nodes`]. Unknown fields are errors. A node and an
//! option write their outcomes (`set`, `add`, `relation`, `remember`,
//! `then`) next to their other fields. The dialogue, a node and an option
//! can have author `notes` (a list of `M`, left out when empty).
//!
//! A line can say a named text pool: `say: { text: keeper_remark }`
//! ([`Text::Pool`]).
//!
//! # Text pools
//!
//! A [`TextPool`] (for example `texts/keeper_remark.yaml`):
//!
//! ```yaml
//! id: keeper_replies
//! pick: best                 # all, first (the default) or best
//! one_of:
//!   - { speaker: keeper, topic: stove, text: '"Leave the stove alone."' }
//!   - { speaker: keeper, text: '"I keep the lock. That is all."' }
//!   - "Nobody answers."      # a bare text: the default row
//!   - when: stove.hot and seen_by(player)
//!     text: "The ash on the grate is fresh."
//!     learn: [ash_is_fresh]
//!   - id: weather            # an inline pool in place of a text
//!     pick: first
//!     one_of: ["Rain.", "Wind."]
//! ```
//!
//! A candidate has `key`, `when`, `speaker`, `topic`, `mode`, `learn`, and
//! its text: `text` (one variant), `variants` (a list), or `id`, `pick` and
//! `one_of` (an inline pool). A variant is a text, or an inline pool
//! `{ id, pick, one_of }`. In a pool, `{ text: … }` is a candidate with that
//! text, not a pool reference.

use crate::ast::{
    Candidate, Choice, Dialogue, Effect, Fragment, FragmentMode, Hint, Intentions, Line, Node,
    Outcomes, Pick, Tag, Target, Text, TextPool, Value, Variant,
};
use serde::de::{self, MapAccess, Visitor};
use serde::ser::SerializeMap;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::BTreeMap;
use std::fmt;
use std::marker::PhantomData;

const NUMERIC_KEY: &str = "numeric ids were removed in 0.3 (ADR 0001); \
     use a key, or write the number in quotes";

/// A key: a string. A number is an error (ADR 0001).
struct Key(String);

impl<'de> Deserialize<'de> for Key {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct V;
        impl Visitor<'_> for V {
            type Value = Key;
            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("a key string")
            }
            fn visit_str<E: de::Error>(self, v: &str) -> Result<Key, E> {
                Ok(Key(v.to_string()))
            }
            fn visit_string<E: de::Error>(self, v: String) -> Result<Key, E> {
                Ok(Key(v))
            }
            fn visit_u64<E: de::Error>(self, v: u64) -> Result<Key, E> {
                Err(E::custom(format!("numeric id {v}: {NUMERIC_KEY}")))
            }
            fn visit_i64<E: de::Error>(self, v: i64) -> Result<Key, E> {
                Err(E::custom(format!("numeric id {v}: {NUMERIC_KEY}")))
            }
            fn visit_f64<E: de::Error>(self, v: f64) -> Result<Key, E> {
                Err(E::custom(format!("numeric id {v}: {NUMERIC_KEY}")))
            }
        }
        d.deserialize_any(V)
    }
}

fn key<'de, D: Deserializer<'de>>(d: D) -> Result<String, D::Error> {
    Key::deserialize(d).map(|k| k.0)
}

/// An optional key. An explicit null (`id:` with no value) is `None`, as
/// a missing field is.
fn opt_key<'de, D: Deserializer<'de>>(d: D) -> Result<Option<String>, D::Error> {
    Option::<Key>::deserialize(d).map(|k| k.map(|k| k.0))
}

/// `start` takes one key or a list of keys.
fn keys<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<String>, D::Error> {
    struct V;
    impl<'de> Visitor<'de> for V {
        type Value = Vec<String>;
        fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str("a key or a list of keys")
        }
        fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
            Ok(vec![v.to_string()])
        }
        fn visit_string<E: de::Error>(self, v: String) -> Result<Self::Value, E> {
            Ok(vec![v])
        }
        fn visit_u64<E: de::Error>(self, v: u64) -> Result<Self::Value, E> {
            Err(E::custom(format!("numeric id {v}: {NUMERIC_KEY}")))
        }
        fn visit_i64<E: de::Error>(self, v: i64) -> Result<Self::Value, E> {
            Err(E::custom(format!("numeric id {v}: {NUMERIC_KEY}")))
        }
        fn visit_seq<A: de::SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
            let mut out = Vec::new();
            while let Some(k) = seq.next_element::<Key>()? {
                out.push(k.0);
            }
            Ok(out)
        }
    }
    d.deserialize_any(V)
}

impl Serialize for Target {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(self.as_key())
    }
}

impl<'de> Deserialize<'de> for Target {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        Key::deserialize(d).map(|k| Target::parse(&k.0))
    }
}

impl Serialize for Value {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        match self {
            Value::Bool(b) => s.serialize_bool(*b),
            Value::Int(n) => s.serialize_i64(*n),
            Value::Text(t) => s.serialize_str(t),
        }
    }
}

impl<'de> Deserialize<'de> for Value {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct V;
        impl Visitor<'_> for V {
            type Value = Value;
            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("a boolean, an integer or a string")
            }
            fn visit_bool<E: de::Error>(self, v: bool) -> Result<Value, E> {
                Ok(Value::Bool(v))
            }
            fn visit_i64<E: de::Error>(self, v: i64) -> Result<Value, E> {
                Ok(Value::Int(v))
            }
            fn visit_u64<E: de::Error>(self, v: u64) -> Result<Value, E> {
                i64::try_from(v)
                    .map(Value::Int)
                    .map_err(|_| E::custom("the integer is too large"))
            }
            fn visit_str<E: de::Error>(self, v: &str) -> Result<Value, E> {
                Ok(Value::Text(v.to_string()))
            }
            fn visit_string<E: de::Error>(self, v: String) -> Result<Value, E> {
                Ok(Value::Text(v))
            }
        }
        d.deserialize_any(V)
    }
}

// ---------------------------------------------------------------- dialogue

#[derive(Serialize)]
#[serde(bound = "C: Serialize, X: Serialize, E: Serialize, M: Serialize")]
struct DialogueOut<'a, C, X, E, M> {
    #[serde(skip_serializing_if = "Option::is_none")]
    id: &'a Option<String>,
    with: &'a str,
    start: &'a [String],
    nodes: NodesOut<'a, C, X, E, M>,
    #[serde(skip_serializing_if = "<[_]>::is_empty")]
    notes: &'a [M],
}

struct NodesOut<'a, C, X, E, M>(&'a [Node<C, X, E, M>]);

impl<C: Serialize, X: Serialize, E: Serialize, M: Serialize> Serialize
    for NodesOut<'_, C, X, E, M>
{
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        // A map can not hold a key twice. Refuse an AST with a duplicate
        // node key, which `check` reports as an error too.
        let mut seen = std::collections::HashSet::new();
        if let Some(n) = self.0.iter().find(|n| !seen.insert(n.key.as_str())) {
            return Err(serde::ser::Error::custom(format!(
                "duplicate node key `{}`",
                n.key
            )));
        }
        let mut map = s.serialize_map(Some(self.0.len()))?;
        for n in self.0 {
            map.serialize_entry(
                &n.key,
                &NodeOut {
                    speaker: &n.speaker,
                    when: &n.when,
                    say: &n.say,
                    lines: &n.lines,
                    next: &n.next,
                    set: &n.outcomes.set,
                    add: &n.outcomes.add,
                    relation: &n.outcomes.relation,
                    remember: &n.outcomes.remember,
                    then: &n.outcomes.then,
                    options: &n.options,
                    notes: &n.notes,
                },
            )?;
        }
        map.end()
    }
}

impl<C: Serialize, X: Serialize, E: Serialize, M: Serialize> Serialize for Dialogue<C, X, E, M> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        DialogueOut {
            id: &self.id,
            with: &self.with,
            start: &self.start,
            nodes: NodesOut(&self.nodes),
            notes: &self.notes,
        }
        .serialize(s)
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(
    bound = "C: Deserialize<'de>, X: Deserialize<'de>, E: Deserialize<'de>, M: Deserialize<'de>"
)]
struct DialogueIn<C, X, E, M> {
    #[serde(default, deserialize_with = "opt_key")]
    id: Option<String>,
    #[serde(deserialize_with = "key")]
    with: String,
    #[serde(deserialize_with = "keys")]
    start: Vec<String>,
    nodes: NodesIn<C, X, E, M>,
    #[serde(default)]
    notes: Vec<M>,
}

struct NodesIn<C, X, E, M>(Vec<Node<C, X, E, M>>);

impl<'de, C: Deserialize<'de>, X: Deserialize<'de>, E: Deserialize<'de>, M: Deserialize<'de>>
    Deserialize<'de> for NodesIn<C, X, E, M>
{
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct V<C, X, E, M>(PhantomData<(C, X, E, M)>);
        impl<
            'de,
            C: Deserialize<'de>,
            X: Deserialize<'de>,
            E: Deserialize<'de>,
            M: Deserialize<'de>,
        > Visitor<'de> for V<C, X, E, M>
        {
            type Value = NodesIn<C, X, E, M>;
            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("a map of node keys to nodes")
            }
            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                let mut out = Vec::new();
                while let Some((key, n)) = map.next_entry::<Key, NodeIn<C, X, E, M>>()? {
                    out.push(Node {
                        key: key.0,
                        speaker: n.speaker,
                        when: n.when,
                        say: n.say,
                        lines: n.lines,
                        next: n.next,
                        outcomes: Outcomes {
                            set: n.set,
                            add: n.add,
                            relation: n.relation,
                            remember: n.remember,
                            then: n.then,
                        },
                        options: n.options,
                        notes: n.notes,
                    });
                }
                Ok(NodesIn(out))
            }
        }
        d.deserialize_map(V(PhantomData))
    }
}

impl<'de, C: Deserialize<'de>, X: Deserialize<'de>, E: Deserialize<'de>, M: Deserialize<'de>>
    Deserialize<'de> for Dialogue<C, X, E, M>
{
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let raw = DialogueIn::<C, X, E, M>::deserialize(d)?;
        Ok(Dialogue {
            id: raw.id,
            with: raw.with,
            start: raw.start,
            nodes: raw.nodes.0,
            notes: raw.notes,
        })
    }
}

// -------------------------------------------------------------------- node

#[derive(Serialize)]
#[serde(bound = "C: Serialize, X: Serialize, E: Serialize, M: Serialize")]
struct NodeOut<'a, C, X, E, M> {
    #[serde(skip_serializing_if = "Option::is_none")]
    speaker: &'a Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    when: &'a Option<C>,
    say: &'a X,
    #[serde(skip_serializing_if = "<[_]>::is_empty")]
    lines: &'a [Line<X>],
    #[serde(skip_serializing_if = "Option::is_none")]
    next: &'a Option<Target>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    set: &'a BTreeMap<String, Value>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    add: &'a BTreeMap<String, i64>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    relation: &'a BTreeMap<String, i32>,
    #[serde(skip_serializing_if = "<[_]>::is_empty")]
    remember: &'a [String],
    #[serde(skip_serializing_if = "<[_]>::is_empty")]
    then: &'a [E],
    #[serde(skip_serializing_if = "<[_]>::is_empty")]
    options: &'a [Choice<C, X, E, M>],
    #[serde(skip_serializing_if = "<[_]>::is_empty")]
    notes: &'a [M],
}

/// A node as written, without its key. The fields of [`Outcomes`] are
/// spelled out instead of `#[serde(flatten)]`, so an unknown field is an
/// error.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(
    bound = "C: Deserialize<'de>, X: Deserialize<'de>, E: Deserialize<'de>, M: Deserialize<'de>"
)]
struct NodeIn<C, X, E, M> {
    #[serde(default, deserialize_with = "opt_key")]
    speaker: Option<String>,
    #[serde(default)]
    when: Option<C>,
    say: X,
    #[serde(default)]
    lines: Vec<Line<X>>,
    #[serde(default)]
    next: Option<Target>,
    #[serde(default)]
    set: BTreeMap<String, Value>,
    #[serde(default)]
    add: BTreeMap<String, i64>,
    #[serde(default)]
    relation: BTreeMap<String, i32>,
    #[serde(default)]
    remember: Vec<String>,
    #[serde(default)]
    then: Vec<E>,
    #[serde(default)]
    options: Vec<Choice<C, X, E, M>>,
    #[serde(default)]
    notes: Vec<M>,
}

#[derive(Serialize)]
#[serde(bound = "X: Serialize")]
struct LineOut<'a, X> {
    speaker: &'a str,
    say: &'a X,
}

impl<X: Serialize> Serialize for Line<X> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        LineOut {
            speaker: &self.speaker,
            say: &self.say,
        }
        .serialize(s)
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(bound = "X: Deserialize<'de>")]
struct LineIn<X> {
    #[serde(deserialize_with = "key")]
    speaker: String,
    say: X,
}

impl<'de, X: Deserialize<'de>> Deserialize<'de> for Line<X> {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let raw = LineIn::<X>::deserialize(d)?;
        Ok(Line {
            speaker: raw.speaker,
            say: raw.say,
        })
    }
}

// ------------------------------------------------------------------ option

fn is_end(t: &Target) -> bool {
    *t == Target::End
}

#[derive(Serialize)]
#[serde(bound = "C: Serialize, X: Serialize, E: Serialize, M: Serialize")]
struct ChoiceOut<'a, C, X, E, M> {
    #[serde(skip_serializing_if = "Option::is_none")]
    id: &'a Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    say: &'a Option<X>,
    #[serde(skip_serializing_if = "Option::is_none")]
    when: &'a Option<C>,
    #[serde(skip_serializing_if = "is_end")]
    go: &'a Target,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    set: &'a BTreeMap<String, Value>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    add: &'a BTreeMap<String, i64>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    relation: &'a BTreeMap<String, i32>,
    #[serde(skip_serializing_if = "<[_]>::is_empty")]
    remember: &'a [String],
    #[serde(skip_serializing_if = "<[_]>::is_empty")]
    then: &'a [E],
    #[serde(skip_serializing_if = "<[_]>::is_empty")]
    tags: &'a [Tag],
    #[serde(skip_serializing_if = "Intentions::is_empty")]
    intentions: &'a Intentions,
    #[serde(skip_serializing_if = "Option::is_none")]
    hint: &'a Option<Hint>,
    #[serde(skip_serializing_if = "<[_]>::is_empty")]
    notes: &'a [M],
}

impl<C: Serialize, X: Serialize, E: Serialize, M: Serialize> Serialize for Choice<C, X, E, M> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        ChoiceOut {
            id: &self.id,
            say: &self.say,
            when: &self.when,
            go: &self.go,
            set: &self.outcomes.set,
            add: &self.outcomes.add,
            relation: &self.outcomes.relation,
            remember: &self.outcomes.remember,
            then: &self.outcomes.then,
            tags: &self.tags,
            intentions: &self.intentions,
            hint: &self.hint,
            notes: &self.notes,
        }
        .serialize(s)
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(
    bound = "C: Deserialize<'de>, X: Deserialize<'de>, E: Deserialize<'de>, M: Deserialize<'de>"
)]
struct ChoiceIn<C, X, E, M> {
    #[serde(default, deserialize_with = "opt_key")]
    id: Option<String>,
    #[serde(default)]
    say: Option<X>,
    #[serde(default)]
    when: Option<C>,
    #[serde(default)]
    go: Target,
    #[serde(default)]
    set: BTreeMap<String, Value>,
    #[serde(default)]
    add: BTreeMap<String, i64>,
    #[serde(default)]
    relation: BTreeMap<String, i32>,
    #[serde(default)]
    remember: Vec<String>,
    #[serde(default)]
    then: Vec<E>,
    #[serde(default)]
    tags: Vec<Tag>,
    #[serde(default)]
    intentions: Intentions,
    #[serde(default)]
    hint: Option<Hint>,
    #[serde(default)]
    notes: Vec<M>,
}

impl<'de, C: Deserialize<'de>, X: Deserialize<'de>, E: Deserialize<'de>, M: Deserialize<'de>>
    Deserialize<'de> for Choice<C, X, E, M>
{
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let o = ChoiceIn::<C, X, E, M>::deserialize(d)?;
        Ok(Choice {
            id: o.id,
            say: o.say,
            when: o.when,
            go: o.go,
            outcomes: Outcomes {
                set: o.set,
                add: o.add,
                relation: o.relation,
                remember: o.remember,
                then: o.then,
            },
            tags: o.tags,
            intentions: o.intentions,
            hint: o.hint,
            notes: o.notes,
        })
    }
}

// -------------------------------------------------------- tags and hints

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct TagRepr {
    axis: String,
    key: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    prefix: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    requires_trait: Option<bool>,
}

impl Serialize for Tag {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        TagRepr {
            axis: self.axis.clone(),
            key: self.key.clone(),
            prefix: self.prefix.clone(),
            requires_trait: self.requires_trait,
        }
        .serialize(s)
    }
}

impl<'de> Deserialize<'de> for Tag {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let t = TagRepr::deserialize(d)?;
        Ok(Tag {
            axis: t.axis,
            key: t.key,
            prefix: t.prefix,
            requires_trait: t.requires_trait,
        })
    }
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct IntentionsRepr {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    explicit: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    implicit: Vec<String>,
}

impl Serialize for Intentions {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        IntentionsRepr {
            explicit: self.explicit.clone(),
            implicit: self.implicit.clone(),
        }
        .serialize(s)
    }
}

impl<'de> Deserialize<'de> for Intentions {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let i = IntentionsRepr::deserialize(d)?;
        Ok(Intentions {
            explicit: i.explicit,
            implicit: i.implicit,
        })
    }
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum HintRepr {
    Continue,
    Leave,
    GoTo(String),
    End,
    Return,
    Custom(String),
}

impl Serialize for Hint {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        match self {
            Hint::Continue => HintRepr::Continue,
            Hint::Leave => HintRepr::Leave,
            Hint::GoTo(t) => HintRepr::GoTo(t.clone()),
            Hint::End => HintRepr::End,
            Hint::Return => HintRepr::Return,
            Hint::Custom(l) => HintRepr::Custom(l.clone()),
        }
        .serialize(s)
    }
}

impl<'de> Deserialize<'de> for Hint {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        Ok(match HintRepr::deserialize(d)? {
            HintRepr::Continue => Hint::Continue,
            HintRepr::Leave => Hint::Leave,
            HintRepr::GoTo(t) => Hint::GoTo(t),
            HintRepr::End => Hint::End,
            HintRepr::Return => Hint::Return,
            HintRepr::Custom(l) => Hint::Custom(l),
        })
    }
}

// -------------------------------------------------------------------- text

#[derive(Serialize, Deserialize)]
#[serde(untagged)]
enum TextRepr {
    Plain(String),
    Fragments(Vec<FragmentRepr>),
    Pool(PoolRef),
}

/// `{ text: <pool id> }`: a named text pool.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct PoolRef {
    text: String,
}

#[derive(Serialize, Deserialize)]
#[serde(untagged)]
enum FragmentRepr {
    Plain(String),
    Full(FragmentSpec),
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct FragmentSpec {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    when: Option<String>,
    #[serde(default, skip_serializing_if = "is_always")]
    mode: ModeRepr,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    text: Option<TextRepr>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    variants: Vec<TextRepr>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    replaces: bool,
}

#[derive(Serialize, Deserialize, Default, Clone, Copy, PartialEq)]
#[serde(rename_all = "snake_case")]
enum ModeRepr {
    #[default]
    Always,
    FirstTime,
    Once,
    Cycle,
    Shuffle,
    Sticky,
}

fn is_always(m: &ModeRepr) -> bool {
    *m == ModeRepr::Always
}

impl From<&Text> for TextRepr {
    fn from(t: &Text) -> Self {
        match t {
            Text::Plain(s) => TextRepr::Plain(s.clone()),
            Text::Fragments(fs) => TextRepr::Fragments(fs.iter().map(FragmentRepr::from).collect()),
            Text::Pool(id) => TextRepr::Pool(PoolRef { text: id.clone() }),
        }
    }
}

impl From<&Fragment> for FragmentRepr {
    fn from(f: &Fragment) -> Self {
        let plain = f.key.is_none()
            && f.when.is_none()
            && f.mode == FragmentMode::Always
            && !f.replaces
            && matches!(f.variants.as_slice(), [Text::Plain(_)]);
        if plain && let [Text::Plain(s)] = f.variants.as_slice() {
            return FragmentRepr::Plain(s.clone());
        }
        let single = matches!(
            f.mode,
            FragmentMode::Always | FragmentMode::FirstTime | FragmentMode::Once
        ) && f.variants.len() == 1;
        let (text, variants) = if single {
            (Some(TextRepr::from(&f.variants[0])), Vec::new())
        } else {
            (None, f.variants.iter().map(TextRepr::from).collect())
        };
        FragmentRepr::Full(FragmentSpec {
            key: f.key.clone(),
            when: f.when.clone(),
            mode: match f.mode {
                FragmentMode::Always => ModeRepr::Always,
                FragmentMode::FirstTime => ModeRepr::FirstTime,
                FragmentMode::Once => ModeRepr::Once,
                FragmentMode::Cycle => ModeRepr::Cycle,
                FragmentMode::Shuffle => ModeRepr::Shuffle,
                FragmentMode::Sticky => ModeRepr::Sticky,
            },
            text,
            variants,
            replaces: f.replaces,
        })
    }
}

impl From<TextRepr> for Text {
    fn from(t: TextRepr) -> Self {
        match t {
            TextRepr::Plain(s) => Text::Plain(s),
            TextRepr::Fragments(fs) => {
                Text::Fragments(fs.into_iter().map(Fragment::from).collect())
            }
            TextRepr::Pool(r) => Text::Pool(r.text),
        }
    }
}

impl From<FragmentRepr> for Fragment {
    fn from(f: FragmentRepr) -> Self {
        match f {
            FragmentRepr::Plain(s) => Fragment::always(s),
            FragmentRepr::Full(spec) => {
                let mut variants: Vec<Text> = spec.text.into_iter().map(Text::from).collect();
                variants.extend(spec.variants.into_iter().map(Text::from));
                Fragment {
                    key: spec.key,
                    when: spec.when,
                    mode: match spec.mode {
                        ModeRepr::Always => FragmentMode::Always,
                        ModeRepr::FirstTime => FragmentMode::FirstTime,
                        ModeRepr::Once => FragmentMode::Once,
                        ModeRepr::Cycle => FragmentMode::Cycle,
                        ModeRepr::Shuffle => FragmentMode::Shuffle,
                        ModeRepr::Sticky => FragmentMode::Sticky,
                    },
                    variants,
                    replaces: spec.replaces,
                }
            }
        }
    }
}

impl Serialize for Text {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        TextRepr::from(self).serialize(s)
    }
}

impl<'de> Deserialize<'de> for Text {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        TextRepr::deserialize(d).map(Text::from)
    }
}

// -------------------------------------------------------------- text pools

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq)]
#[serde(rename_all = "snake_case")]
enum PickRepr {
    All,
    First,
    Best,
}

impl From<Pick> for PickRepr {
    fn from(p: Pick) -> Self {
        match p {
            Pick::All => PickRepr::All,
            Pick::First => PickRepr::First,
            Pick::Best => PickRepr::Best,
        }
    }
}

impl From<PickRepr> for Pick {
    fn from(p: PickRepr) -> Self {
        match p {
            PickRepr::All => Pick::All,
            PickRepr::First => Pick::First,
            PickRepr::Best => Pick::Best,
        }
    }
}

fn mode_repr(m: FragmentMode) -> ModeRepr {
    match m {
        FragmentMode::Always => ModeRepr::Always,
        FragmentMode::FirstTime => ModeRepr::FirstTime,
        FragmentMode::Once => ModeRepr::Once,
        FragmentMode::Cycle => ModeRepr::Cycle,
        FragmentMode::Shuffle => ModeRepr::Shuffle,
        FragmentMode::Sticky => ModeRepr::Sticky,
    }
}

fn mode_ast(m: ModeRepr) -> FragmentMode {
    match m {
        ModeRepr::Always => FragmentMode::Always,
        ModeRepr::FirstTime => FragmentMode::FirstTime,
        ModeRepr::Once => FragmentMode::Once,
        ModeRepr::Cycle => FragmentMode::Cycle,
        ModeRepr::Shuffle => FragmentMode::Shuffle,
        ModeRepr::Sticky => FragmentMode::Sticky,
    }
}

#[derive(Serialize)]
#[serde(bound = "C: Serialize, X: Serialize")]
struct PoolOut<'a, C, X> {
    #[serde(skip_serializing_if = "Option::is_none")]
    id: &'a Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pick: Option<PickRepr>,
    one_of: &'a [Candidate<C, X>],
}

impl<C: Serialize, X: Serialize> Serialize for TextPool<C, X> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        PoolOut {
            id: &self.id,
            pick: self.pick.map(PickRepr::from),
            one_of: &self.one_of,
        }
        .serialize(s)
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(bound = "C: Deserialize<'de>, X: Deserialize<'de>")]
struct PoolIn<C, X> {
    #[serde(default, deserialize_with = "opt_key")]
    id: Option<String>,
    #[serde(default)]
    pick: Option<PickRepr>,
    one_of: Vec<Candidate<C, X>>,
}

impl<C, X> From<PoolIn<C, X>> for TextPool<C, X> {
    fn from(p: PoolIn<C, X>) -> Self {
        TextPool {
            id: p.id,
            pick: p.pick.map(Pick::from),
            one_of: p.one_of,
        }
    }
}

impl<'de, C: Deserialize<'de>, X: Deserialize<'de>> Deserialize<'de> for TextPool<C, X> {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        PoolIn::<C, X>::deserialize(d).map(TextPool::from)
    }
}

impl<C: Serialize, X: Serialize> Serialize for Variant<C, X> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        match self {
            Variant::Text(x) => x.serialize(s),
            Variant::Pool(p) => p.serialize(s),
        }
    }
}

/// A pool first: it needs `one_of`, so a host text that is a map does not
/// read as a pool.
#[derive(Deserialize)]
#[serde(untagged)]
#[serde(bound = "C: Deserialize<'de>, X: Deserialize<'de>")]
enum VariantIn<C, X> {
    Pool(PoolIn<C, X>),
    Text(X),
}

impl<'de, C: Deserialize<'de>, X: Deserialize<'de>> Deserialize<'de> for Variant<C, X> {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        Ok(match VariantIn::<C, X>::deserialize(d)? {
            VariantIn::Pool(p) => Variant::Pool(p.into()),
            VariantIn::Text(x) => Variant::Text(x),
        })
    }
}

#[derive(Serialize)]
#[serde(bound = "C: Serialize, X: Serialize")]
struct CandidateOut<'a, C, X> {
    #[serde(skip_serializing_if = "Option::is_none")]
    key: &'a Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    speaker: &'a Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    topic: &'a Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    when: &'a Option<C>,
    #[serde(skip_serializing_if = "is_always")]
    mode: ModeRepr,
    #[serde(skip_serializing_if = "Option::is_none")]
    text: Option<&'a Variant<C, X>>,
    #[serde(skip_serializing_if = "<[_]>::is_empty")]
    variants: &'a [Variant<C, X>],
    #[serde(skip_serializing_if = "<[_]>::is_empty")]
    learn: &'a [String],
}

impl<C: Serialize, X: Serialize> Serialize for Candidate<C, X> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        // Always the map form: a bare host text that is a map (such as
        // `Text::Pool`, `{ text: id }`) would read back as a candidate map.
        let (text, variants) = match self.variants.as_slice() {
            [one] => (Some(one), &[][..]),
            many => (None, many),
        };
        CandidateOut {
            key: &self.key,
            speaker: &self.speaker,
            topic: &self.topic,
            when: &self.when,
            mode: mode_repr(self.mode),
            text,
            variants,
            learn: &self.learn,
        }
        .serialize(s)
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(bound = "C: Deserialize<'de>, X: Deserialize<'de>")]
struct CandidateSpec<C, X> {
    #[serde(default, deserialize_with = "opt_key")]
    key: Option<String>,
    #[serde(default, deserialize_with = "opt_key")]
    speaker: Option<String>,
    #[serde(default, deserialize_with = "opt_key")]
    topic: Option<String>,
    #[serde(default)]
    when: Option<C>,
    #[serde(default)]
    mode: ModeRepr,
    #[serde(default)]
    text: Option<Variant<C, X>>,
    #[serde(default)]
    variants: Vec<Variant<C, X>>,
    #[serde(default)]
    learn: Vec<String>,
    // An inline pool in place of `text`.
    #[serde(default, deserialize_with = "opt_key")]
    id: Option<String>,
    #[serde(default)]
    pick: Option<PickRepr>,
    #[serde(default)]
    one_of: Option<Vec<Candidate<C, X>>>,
}

/// A candidate map first, then a bare text.
#[derive(Deserialize)]
#[serde(untagged)]
#[serde(bound = "C: Deserialize<'de>, X: Deserialize<'de>")]
enum CandidateIn<C, X> {
    Spec(Box<CandidateSpec<C, X>>),
    Text(X),
}

impl<'de, C: Deserialize<'de>, X: Deserialize<'de>> Deserialize<'de> for Candidate<C, X> {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let spec = match CandidateIn::<C, X>::deserialize(d)? {
            CandidateIn::Text(x) => return Ok(Candidate::text(x)),
            CandidateIn::Spec(spec) => *spec,
        };
        let mut variants: Vec<Variant<C, X>> = spec.text.into_iter().collect();
        variants.extend(spec.variants);
        match spec.one_of {
            Some(one_of) => variants.push(Variant::Pool(TextPool {
                id: spec.id,
                pick: spec.pick.map(Pick::from),
                one_of,
            })),
            None if spec.id.is_some() || spec.pick.is_some() => {
                return Err(de::Error::custom(
                    "`id` and `pick` on a candidate need `one_of` (an inline pool)",
                ));
            }
            None => {}
        }
        if variants.is_empty() {
            return Err(de::Error::custom(
                "a candidate needs `text`, `variants` or `one_of`",
            ));
        }
        let single = matches!(
            spec.mode,
            ModeRepr::Always | ModeRepr::FirstTime | ModeRepr::Once
        );
        if single && variants.len() > 1 {
            return Err(de::Error::custom(
                "this mode shows one text; use `variants` only with `cycle`, `shuffle` or `sticky`",
            ));
        }
        Ok(Candidate {
            key: spec.key,
            when: spec.when,
            speaker: spec.speaker,
            topic: spec.topic,
            mode: mode_ast(spec.mode),
            variants,
            learn: spec.learn,
        })
    }
}

// ------------------------------------------------------------------ effect

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
enum EffectRepr {
    Relation {
        from: String,
        to: String,
        axis: String,
        delta: i32,
    },
    Custom {
        key: String,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        args: Vec<Value>,
    },
}

impl Serialize for Effect {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        match self {
            Effect::Relation {
                from,
                to,
                axis,
                delta,
            } => EffectRepr::Relation {
                from: from.clone(),
                to: to.clone(),
                axis: axis.clone(),
                delta: *delta,
            },
            Effect::Custom { key, args } => EffectRepr::Custom {
                key: key.clone(),
                args: args.clone(),
            },
        }
        .serialize(s)
    }
}

impl<'de> Deserialize<'de> for Effect {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        Ok(match EffectRepr::deserialize(d)? {
            EffectRepr::Relation {
                from,
                to,
                axis,
                delta,
            } => Effect::Relation {
                from,
                to,
                axis,
                delta,
            },
            EffectRepr::Custom { key, args } => Effect::Custom { key, args },
        })
    }
}
