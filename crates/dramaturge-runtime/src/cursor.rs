//! A dialogue cursor: the current node and a return stack.
//!
//! [`DialogCursor`] follows [`DialogTarget`]s. A [`DialogTarget::Node`]
//! pushes the current node on the return stack; a [`DialogTarget::Return`]
//! pops it. Hosts that keep their own talk state can store the cursor (see
//! `DialogCursorDto` in `dramaturge-serde`).
//!
//! ```rust
//! use dramaturge_model::{DialogTarget, DialogueNodeId};
//! use dramaturge_runtime::DialogCursor;
//!
//! let hub = DialogueNodeId::new(1);
//! let mut cursor = DialogCursor::new(hub);
//!
//! // Go to a topic, then back to the hub.
//! assert_eq!(cursor.follow(&DialogTarget::node(2)), Some(DialogueNodeId::new(2)));
//! assert_eq!(cursor.follow(&DialogTarget::Return), Some(hub));
//!
//! // `Return` with an empty stack ends the talk.
//! assert_eq!(cursor.follow(&DialogTarget::Return), None);
//! ```

use dramaturge_model::{DialogTarget, DialogueNodeId};

/// The current node of a talk and the nodes to return to.
///
/// The fields are public for reading and writing (for example, to clear
/// the return stack). Create a cursor with [`DialogCursor::new`].
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct DialogCursor {
    /// The node the talk is at.
    pub current: DialogueNodeId,
    /// The return stack: earlier nodes, the most recent last.
    pub back: Vec<DialogueNodeId>,
}

impl DialogCursor {
    /// Creates a cursor at `start` with an empty return stack.
    pub fn new(start: DialogueNodeId) -> Self {
        Self {
            current: start,
            back: Vec::new(),
        }
    }

    /// Follows `target` and returns the node the talk is at next, or `None`
    /// if the cursor does not move to a node.
    ///
    /// - [`DialogTarget::Node`]: pushes the current node on the return
    ///   stack and moves to the target node.
    /// - [`DialogTarget::Return`]: pops the most recent node from the
    ///   return stack and moves to it. With an empty stack, returns `None`
    ///   (the talk ends) and the cursor does not change.
    /// - [`DialogTarget::End`]: returns `None` (the talk ends). The cursor
    ///   does not change.
    /// - [`DialogTarget::Custom`] and targets added later: returns `None`
    ///   and the cursor does not change. The host handles them, and can
    ///   set [`current`](Self::current) and [`back`](Self::back) itself.
    ///
    /// Every `Node` target pushes, also a target back to an earlier node,
    /// so the stack grows in a talk that loops through a hub node. Clear
    /// [`back`](Self::back) where the talk has a natural restart point.
    pub fn follow(&mut self, target: &DialogTarget) -> Option<DialogueNodeId> {
        match target {
            DialogTarget::Node(next) => {
                self.back.push(self.current);
                self.current = *next;
                Some(*next)
            }
            DialogTarget::Return => {
                let previous = self.back.pop()?;
                self.current = previous;
                Some(previous)
            }
            _ => None,
        }
    }

    /// Returns the node a [`DialogTarget::Return`] would move to, if any.
    pub fn peek_back(&self) -> Option<DialogueNodeId> {
        self.back.last().copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert2::check;

    fn id(n: u64) -> DialogueNodeId {
        DialogueNodeId::new(n)
    }

    #[test]
    fn node_target_pushes() {
        let mut cursor = DialogCursor::new(id(1));
        check!(cursor.follow(&DialogTarget::node(2)) == Some(id(2)));
        check!(cursor.follow(&DialogTarget::node(3)) == Some(id(3)));
        check!(cursor.current == id(3));
        check!(cursor.back == vec![id(1), id(2)]);
        check!(cursor.peek_back() == Some(id(2)));
    }

    #[test]
    fn return_target_pops() {
        let mut cursor = DialogCursor::new(id(1));
        cursor.follow(&DialogTarget::node(2));
        cursor.follow(&DialogTarget::node(3));
        check!(cursor.follow(&DialogTarget::Return) == Some(id(2)));
        check!(cursor.current == id(2));
        check!(cursor.follow(&DialogTarget::Return) == Some(id(1)));
        check!(cursor.current == id(1));
        check!(cursor.back.is_empty());
    }

    #[test]
    fn return_on_empty_stack_ends_the_talk() {
        let mut cursor = DialogCursor::new(id(1));
        check!(cursor.peek_back().is_none());
        check!(cursor.follow(&DialogTarget::Return).is_none());
        check!(cursor == DialogCursor::new(id(1)));
    }

    #[test]
    fn end_and_custom_do_not_move() {
        let mut cursor = DialogCursor::new(id(1));
        cursor.follow(&DialogTarget::node(2));
        let before = cursor.clone();
        check!(cursor.follow(&DialogTarget::End).is_none());
        check!(cursor.follow(&DialogTarget::custom("shop")).is_none());
        check!(cursor == before);
    }
}
