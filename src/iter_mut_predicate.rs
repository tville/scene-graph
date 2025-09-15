use thunderdome::Index;

use crate::{Node, NodeIndex, SceneGraph};

/// A mutable iterator over the children of a node in a [SceneGraph],
/// that skips branches/subtrees where the predicate is not fulfilled.
/// See [SceneGraph::iter_mut_predicate] for more information.
pub struct SceneGraphIterMutPredicate<'a, T> {
    sg: &'a mut SceneGraph<T>,
    predicate: fn(&T) -> bool,
    stacks: Vec<StackState>,
}

impl<'a, T> SceneGraphIterMutPredicate<'a, T> {
    pub(crate) fn new(sg: &'a mut SceneGraph<T>, root_node_idx: NodeIndex, predicate: fn(&T) -> bool) -> Self {
        let mut stacks = Vec::new();

        let first_child = match root_node_idx {
            NodeIndex::Root => sg.root_children.map(|v| v.first),
            NodeIndex::Branch(idx) => sg.arena.get(idx).and_then(|v| v.children.map(|v| v.first)),
        };

        if let Some(first_child) = first_child {
            stacks.push(StackState::new(root_node_idx, first_child));
        };
        SceneGraphIterMutPredicate {
            sg,
            predicate,
            stacks
        }
    }
}

impl<'a, T> Iterator for SceneGraphIterMutPredicate<'a, T> {
    type Item = (&'a mut T, &'a mut T);

    fn next(&mut self) -> Option<Self::Item> {
        while !self.stacks.is_empty() {
            let stack_frame = self.stacks.pop()?;

            let (parent, current_child) = match stack_frame.parent {
                NodeIndex::Root => {
                    let parent = &mut self.sg.root;
                    if !(self.predicate)(&parent) {
                        // The root does not fulfill the predicate, the whole graph will be skipped
                        continue;
                    }
                    let child = self.sg.arena.get_mut(stack_frame.current_child).unwrap();
                    (parent, child)
                }
                NodeIndex::Branch(idx) => {
                    let (parent, current_child) = self.sg.arena.get2_mut(idx, stack_frame.current_child);

                    (&mut parent.unwrap().value, current_child.unwrap())
                }
            };

            // if there's a sibling, push it onto the to do list!
            if let Some(next_sibling) = current_child.next_sibling {
                self.stacks.push(StackState::new(stack_frame.parent, next_sibling));
            }

            if !(self.predicate)(&current_child.value) {
                // This child and it's children should be skipped.
                // Continue with the next candidate on the stack.
                continue;
            }

            // safety:  this is a lifetime extension, which i know is valid because get2_mut
            // panics when we pass in two of the same things, and this iterator requires `&mut SG`
            // to call `next`.
            let (parent, current_child): (&mut T, &mut Node<T>) =
                unsafe { (&mut *(parent as *mut _), &mut *(current_child as *mut _)) };

            if let Some(first_child) = current_child.children.map(|v| v.first) {
                self.stacks.push(StackState::new(
                    NodeIndex::Branch(stack_frame.current_child),
                    first_child,
                ));
            }

            return Some((parent, &mut current_child.value));
        }
        return None;
    }
}

#[derive(Debug)]
struct StackState {
    parent: NodeIndex,
    current_child: Index,
}

impl StackState {
    fn new(parent: NodeIndex, first_child: Index) -> Self {
        Self {
            parent,
            current_child: first_child,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scene_graph_returns_nothing_on_empty_iteration() {
        let mut scene_graph = SceneGraph::new("Root");

        assert!(scene_graph.iter_mut_predicate(|node| {true}).next().is_none());
    }

    #[test]
    fn normal_iteration() {
        let mut sg = SceneGraph::new("Root");
        let root_idx = NodeIndex::Root;
        sg.attach(root_idx, "First Child").unwrap();

        let second_child = sg.attach(root_idx, "Second Child").unwrap();
        sg.attach(second_child, "First Grandchild").unwrap();

        assert_eq!(
            Vec::from_iter(sg
                .iter_mut_predicate(|node| {true})
                .map(|(_parent, value)| &*value)
                .copied()),
            vec!["First Child", "Second Child", "First Grandchild"]
        );
    }

    #[test]
    fn stagger_iteration() {
        let mut sg = SceneGraph::new("Root");
        let root_idx = NodeIndex::Root;
        let child = sg.attach(root_idx, "First Child").unwrap();
        sg.attach(child, "Second Child").unwrap();

        assert_eq!(
            Vec::from_iter(sg
                .iter_mut_predicate(|node| {true})
                .map(|(_parent, value)| &*value)
                .copied()),
            vec!["First Child", "Second Child"]
        );
    }

    #[test]
    fn single_iteration() {
        let mut sg = SceneGraph::new("Root");
        let root_idx = NodeIndex::Root;
        sg.attach(root_idx, "First Child").unwrap();

        assert_eq!(
            Vec::from_iter(sg
                .iter_mut_predicate(|node| {true})
                .map(|(_parent, value)| &*value).copied()),
            vec!["First Child",]
        );
    }

    #[test]
    fn visits_none_when_root_does_not_match() {
        let mut sg = SceneGraph::new(ConditionalNode::new("Root",false));
        let root_idx = NodeIndex::Root;
        let c1 = sg.attach(root_idx, ConditionalNode::new("Child 1", true)).unwrap();

        assert_eq!(0, sg.iter_mut_predicate(|node| {node.condition}).count());
    }

    #[test]
    fn visits_only_matching_nodes() {
        let mut sg = SceneGraph::new(ConditionalNode::new("Root",true));
        let root_idx = NodeIndex::Root;
        let c1 = sg.attach(root_idx, ConditionalNode::new("Child 1", true)).unwrap();
        let c2 = sg.attach(root_idx, ConditionalNode::new("Child 2", false)).unwrap();
        let c3 = sg.attach(root_idx, ConditionalNode::new("Child 3", true)).unwrap();
        let c1c = sg.attach(c1, ConditionalNode::new("Child of child 1", true)).unwrap();
        // Should be skipped due to c2 being set to false
        let c2c = sg.attach(c2, ConditionalNode::new("Child of child 2", true)).unwrap();

        assert_eq!(
            Vec::from_iter(sg
                .iter_mut_predicate(|node| {node.condition})
                .map(|(_parent, value)| &value.name).cloned()),
            vec!["Child 1", "Child of child 1", "Child 3"]
        );
    }

    #[derive(PartialEq, Clone)]
    struct ConditionalNode {
        pub name: &'static str,
        pub condition: bool,
    }

    impl ConditionalNode {
        pub fn new(name: &'static str, condition: bool) -> Self {
            ConditionalNode { name, condition }
        }
    }
}
