use std::collections::VecDeque;

use bevy::utils::{HashMap, HashSet};

use super::loader::GltfError;

pub(crate) struct GltfTreeIterator<'a> {
    nodes: Vec<gltf::Node<'a>>,
}

impl<'a> GltfTreeIterator<'a> {
    #[allow(clippy::result_large_err)]
    pub(crate) fn try_new(gltf: &'a gltf::Gltf) -> Result<Self, GltfError> {
        let nodes = gltf.nodes().collect::<Vec<_>>();

        let mut empty_children = VecDeque::new();
        let mut parents = vec![None; nodes.len()];
        let mut unprocessed_nodes = nodes
            .into_iter()
            .enumerate()
            .map(|(i, node)| {
                let children = node
                    .children()
                    .map(|child| child.index())
                    .collect::<HashSet<_>>();
                for &child in &children {
                    let parent = parents.get_mut(child).unwrap();
                    *parent = Some(i);
                }
                if children.is_empty() {
                    empty_children.push_back(i);
                }
                (i, (node, children))
            })
            .collect::<HashMap<_, _>>();

        let mut nodes = Vec::new();
        while let Some(index) = empty_children.pop_front() {
            let (node, children) = unprocessed_nodes.remove(&index).unwrap();
            assert!(children.is_empty());
            nodes.push(node);

            if let Some(parent_index) = parents[index] {
                let (_, parent_children) = unprocessed_nodes.get_mut(&parent_index).unwrap();

                assert!(parent_children.remove(&index));
                if parent_children.is_empty() {
                    empty_children.push_back(parent_index);
                }
            }
        }

        if !unprocessed_nodes.is_empty() {
            return Err(GltfError::CircularChildren(format!(
                "{:?}",
                unprocessed_nodes
                    .iter()
                    .map(|(k, _v)| *k)
                    .collect::<Vec<_>>(),
            )));
        }

        nodes.reverse();
        Ok(Self {
            nodes: nodes.into_iter().collect(),
        })
    }
}

impl<'a> Iterator for GltfTreeIterator<'a> {
    type Item = gltf::Node<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        self.nodes.pop()
    }
}

impl<'a> ExactSizeIterator for GltfTreeIterator<'a> {
    fn len(&self) -> usize {
        self.nodes.len()
    }
}
