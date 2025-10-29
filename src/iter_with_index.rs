use crate::{Children, Node, NodeIndex, SceneGraph};

/// An iterator over the SceneGraph. See [iter] for more information.
///
/// [iter]: SceneGraph::iter
pub struct SceneGraphIterWithIndex<'a, T> {
    sg: &'a SceneGraph<T>,
    stacks: Vec<StackState<'a, T>>,
}

impl<'a, T> SceneGraphIterWithIndex<'a, T> {
    pub(crate) fn new(sg: &'a SceneGraph<T>, root_index: NodeIndex, root_children: Option<&'a Children>) -> Self {
        let mut stacks = Vec::new();
        if let Some(first_child) = root_children.map(|v| v.first) {
            stacks.push(StackState::new(NodeIndex::Branch(first_child), &sg.arena[first_child]));
        };
        SceneGraphIterWithIndex { sg, stacks }
    }
}

impl<'a, T> Iterator for SceneGraphIterWithIndex<'a, T> {
    type Item = (NodeIndex, &'a T);

    fn next(&mut self) -> Option<Self::Item> {
        // if we're out of stack frames, we die here
        let stack_frame = self.stacks.pop()?;

        // if there's a sibling, push it onto the to do list!
        if let Some(next_sibling) = stack_frame.current_child.next_sibling {
            self.stacks
                .push(StackState::new(NodeIndex::Branch(next_sibling), &self.sg.arena[next_sibling]));
        }

        if let Some(first_child) = stack_frame.current_child.children.map(|v| v.first) {
            self.stacks.push(StackState::new(NodeIndex::Branch(first_child), &self.sg.arena[first_child],
            ));
        }

        Some((stack_frame.current_child_index, &stack_frame.current_child.value))
    }
}

#[derive(Debug)]
struct StackState<'a, T> {
    current_child_index: NodeIndex,
    current_child: &'a Node<T>,
}

impl<'a, T> StackState<'a, T> {
    fn new(index: NodeIndex, first_child: &'a Node<T>) -> Self {
        Self {
            current_child_index: index,
            current_child: first_child,
        }
    }
}

#[cfg(test)]
mod tests {
  use crate::{NodeIndex, SceneGraph};

  #[test]
    fn scene_graph_returns_nothing_on_empty_iteration() {
        let scene_graph = SceneGraph::new("Root");

        assert!(scene_graph.iter_from_node_with_index(NodeIndex::Root)
            .expect("Expected iterator to be successfully returned")
            .next().is_none());

      let mut scene_graph = SceneGraph::new("Root");
      let child_idx = scene_graph.attach(NodeIndex::Root, "First Child").unwrap();

      assert!(scene_graph.iter_from_node_with_index(child_idx)
          .expect("Expected iterator to be successfully returned")
          .next().is_none());
    }

    #[test]
    fn normal_iteration() {
        let mut sg = SceneGraph::new("Root");
        let child_1 = sg.attach(NodeIndex::Root, "First Child").unwrap();

        let child_2 = sg.attach(NodeIndex::Root, "Second Child").unwrap();
        let grandchild = sg.attach(child_2, "First Grandchild").unwrap();

        assert_eq!(
            Vec::from_iter(sg.iter_from_node_with_index(NodeIndex::Root)
                .expect("Expected iterator to be successfully returned")
                .map(|(node_idx, value)| (node_idx, value.clone()))),
            vec![(child_1, "First Child"), (child_2, "Second Child"), (grandchild, "First Grandchild")]
        );
    }

    #[test]
    fn stagger_iteration() {
        let mut sg = SceneGraph::new("Root");
        let child = sg.attach(NodeIndex::Root, "First Child").unwrap();
        let child_2 = sg.attach(child, "Second Child").unwrap();

        assert_eq!(
            Vec::from_iter(sg.iter_from_node_with_index(NodeIndex::Root)
                .expect("Expected iterator to be successfully returned")
                .map(|(node_idx, value)| (node_idx, value.clone()))),
            vec![(child, "First Child"), (child_2, "Second Child")]
        );
    }

    #[test]
    fn stagger_iteration_from_branch() {
      let mut sg = SceneGraph::new("Root");
      let root_idx = NodeIndex::Root;
      let child = sg.attach(root_idx, "First Child").unwrap();
      let grandchild_1 = sg.attach(child, "Child 1-1").unwrap();
      let grandchild_2 = sg.attach(child, "Child 1-2").unwrap();

      assert_eq!(
        Vec::from_iter(sg.iter_from_node_with_index(child)
            .expect("Expected iterator to be successfully returned")
            .map(|(node_idx, value)| (node_idx, value.clone()))),
        vec![(grandchild_1, "Child 1-1"), (grandchild_2, "Child 1-2")]
      );
    }

    #[test]
    fn single_iteration() {
        let mut sg = SceneGraph::new("Root");
        let child_idx = sg.attach(NodeIndex::Root, "First Child").unwrap();

        assert_eq!(
            Vec::from_iter(sg.iter_from_node_with_index(NodeIndex::Root)
                .expect("Expected iterator to be successfully returned")
                .map(|(node_idx, value)| (node_idx, value.clone()))),
            vec![(child_idx, "First Child")]
        );
    }
}
