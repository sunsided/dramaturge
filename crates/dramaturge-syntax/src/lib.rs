//! One dialogue AST for the `dramaturge` toolkit.
//!
//! The YAML loader of `dramaturge-serde`, the `dialogue!` macro of
//! `dramaturge-macros` and host macros (loremaster's `world!`) all read a
//! dialogue into the same [`Dialogue`]. [`lower()`] turns it into a model
//! [`DialogueGraph`](dramaturge_model::DialogueGraph).
//!
//! ```text
//!  YAML / JSON (serde)  ─┐
//!  dialogue! (syn)      ─┼─▶ Dialogue<C, X, E> ─ check ─ lower ─▶ DialogueGraph<H, T>
//!  world! dialogue (syn)─┘                                ▲
//!                                          Lower hooks of the host
//! ```
//!
//! # The shape
//!
//! ```yaml
//! with: mara                 # the character that the player talks to
//! start: [again, intro]      # entry nodes: the first whose `when` holds
//! nodes:
//!   intro:
//!     say: "Hello, traveller."
//!     set: { met_mara: true }          # outcomes on enter
//!     options:
//!       - say: "Who are you?"
//!         go: names                    # a node key, `end` or `back`
//!         relation: { trust: 1 }       # outcomes when chosen
//!       - say: "Goodbye."              # without `go`: the talk ends
//!   again:
//!     when: met_mara
//!     say: "You again."
//!     options:
//!       - { say: "Goodbye.", go: end }
//!   names:
//!     say: "They call me Mara."
//!     lines:                           # more lines, each with a speaker
//!       - { speaker: player, say: "Mara. I will remember." }
//!     next: end                        # go on without a choice
//! ```
//!
//! Node and option outcomes: `set` (facts), `add` (integer facts),
//! `relation` (player to the node's speaker), `remember` (memories of the
//! node's speaker) and `then` (host effects). Options also have `id`,
//! `tags`, `intentions` and `hint`. The dialogue, nodes and options can
//! have author `notes`; the lowering ignores them.
//!
//! A line can say a named text pool: `say: { text: keeper_remark }`.
//!
//! # Text pools
//!
//! A [`TextPool`] (`id`, `pick`, `one_of`) is the AST of a text pool for
//! the text selection core. [`lower_pool`] makes a
//! `dramaturge_model::Pool` through the same [`Lower`] hooks. See
//! [`shape`](crate#serde) for the YAML form.
//!
//! # Payloads and hooks
//!
//! The AST is generic over the host's condition (`C`), text (`X`),
//! effect (`E`) and note (`M`, default `String`) payloads. The parsers read them as they are; the
//! [`Lower`] hooks of the host turn them into model values. The defaults
//! are for plain dramaturge: `C = String` in the
//! [default condition language](condition), `X = `[`Text`] and
//! `E = `[`Effect`], lowered by [`Plain`].
//!
//! ```rust
//! use dramaturge_syntax::{Choice, Dialogue, Node, Target, lower_plain};
//!
//! let dialogue: Dialogue = Dialogue::new("mara")
//!     .with_start("intro")
//!     .with_node(
//!         Node::new("intro", "Hello.".into())
//!             .with_option(Choice::new("Goodbye.".into(), Target::End)),
//!     );
//! let loaded = lower_plain(&dialogue).unwrap();
//! assert_eq!(loaded.graph.nodes.len(), 1);
//! assert_eq!(loaded.symbols.node_key(loaded.graph.entry_nodes[0]), Some("intro"));
//! ```
//!
//! # Features
//!
//! - <a id="serde"></a>`serde`: `Serialize` and `Deserialize` for the AST,
//!   in the shape above (see [`shape`] for the details).
//! - <a id="tokens"></a>`syn`: the token form, for `dialogue!` and for
//!   `dialogue` blocks in a host macro (see [`tokens`]).
//!
//! The YAML loader uses `serde` only, so it does not compile `syn`.

#![cfg_attr(docsrs, feature(doc_cfg))]
#![warn(missing_docs)]

pub mod ast;
pub mod check;
pub mod condition;
pub mod lower;
#[cfg(feature = "serde")]
#[cfg_attr(docsrs, doc(cfg(feature = "serde")))]
pub mod shape;
#[cfg(feature = "syn")]
#[cfg_attr(docsrs, doc(cfg(feature = "syn")))]
pub mod tokens;

pub use ast::{
    Candidate, Choice, Dialogue, Effect, Fragment, FragmentMode, Hint, Intentions, Line, Node,
    Outcomes, Pick, Tag, Target, Text, TextPool, Value, Variant,
};
pub use check::{Issue, IssueKind, OutcomePlace, Place, Severity, check};
pub use condition::ParseError;
pub use lower::{
    Cx, Loaded, Lower, LowerError, LowerResult, Lowered, Plain, PoolResult, lower, lower_plain,
    lower_pool, lower_pool_plain,
};
#[cfg(feature = "syn")]
pub use tokens::{ParseHost, Parsed, Spanned, parse_dialogue};
