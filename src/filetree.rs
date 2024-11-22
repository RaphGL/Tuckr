use std::collections::HashSet;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
struct FileNode<'a> {
    group: &'a str,
    parent_node_idx: usize,
    // points to a path in FileTree::paths
    path_idx: usize,
    children: Option<Vec<usize>>,
}

#[derive(Default, Debug)]
struct FileTree<'a> {
    paths: Vec<Option<PathBuf>>,
    nodes: Vec<Option<FileNode<'a>>>,
    groups: HashSet<String>,
}

// todo: change to tree to store generic values
impl<'a> FileTree<'a> {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn contains_file(&self, value: &Path) -> bool {
        self.find_node_idx(value).is_some()
    }

    pub fn contains_group(&self, groupname: &str) -> bool {
        self.groups.contains(groupname)
    }

    fn find_node_idx(&self, value: &Path) -> Option<usize> {
        let mut children_stack = vec![0];

        while !children_stack.is_empty() {
            let Some(curr_node_idx) = children_stack.pop() else {
                unreachable!(
                    "loop stops when stack is empty, therefore this should never be reachable"
                );
            };

            let curr_node = match self.nodes.get(curr_node_idx)? {
                Some(ref node) => node,
                None => continue,
            };

            if let Some(ref path_node) = self.paths[curr_node.path_idx] {
                if path_node == value {
                    return Some(curr_node_idx);
                }
            }

            if let Some(ref children) = curr_node.children {
                children_stack.extend_from_slice(children);
            }
        }

        None
    }

    pub fn insert(&mut self, value: &Path, group: &'a str) {
        self.groups.insert(group.into());
        self.paths.push(Some(value.into()));
        let path_idx = self.paths.len() - 1;

        let value_parent = {
            let mut parent = value.to_path_buf();
            parent.pop();
            parent
        };

        if self.nodes.is_empty() {
            self.nodes.push(Some(FileNode {
                group,
                parent_node_idx: 0,
                path_idx,
                children: None,
            }));

            return;
        }

        match self.find_node_idx(&value_parent) {
            Some(parent) => {
                let new_node_idx = self.nodes.len();

                if let Some(ref mut node) = self.nodes[parent] {
                    match node.children {
                        Some(ref mut children) => children.push(new_node_idx),
                        None => node.children = Some(vec![new_node_idx]),
                    }
                }

                self.nodes.push(Some(FileNode {
                    group,
                    parent_node_idx: parent,
                    path_idx,
                    children: None,
                }));
            }

            None => unreachable!(),
        }
    }

    pub fn remove(&mut self, value: &Path) {
        let Some(value_idx) = self.find_node_idx(value) else {
            return;
        };

        let node = self.nodes.get(value_idx).unwrap().clone().unwrap();

        if let Some(ref mut parent_node) = self.nodes[node.parent_node_idx] {
            if let Some(ref children) = parent_node.children {
                parent_node.children = Some(
                    children
                        .iter()
                        .filter(|v| **v != value_idx)
                        .map(|v| *v)
                        .collect(),
                );
            }
        }

        self.nodes[value_idx] = None;
        self.paths[node.path_idx] = None;

        if !self.contains_group(node.group) {
            self.groups.remove(node.group);
        }
    }

    // note: instead of PathBuf should use T or just plain dotfiles::Dotfile
    pub fn get(&self, group: &str) -> HashSet<PathBuf> {
        todo!()
    }

    pub fn canonicalize(&mut self) {}

    pub fn is_empty(&self) -> bool {
        if self.paths.is_empty() && self.nodes.is_empty() {
            return true;
        }

        let has_items = self.paths.iter().any(|path| path.is_some())
            || self.nodes.iter().any(|node| node.is_some());

        !has_items
    }
}

// fn main() {
//     let mut ft = FileTree::new();

//     ft.insert(Path::new("test"), "test");
//     ft.insert(Path::new("test/file"), "test2");
//     ft.insert(Path::new("test/file2"), "test2");
//     ft.insert(Path::new("test/file2/file3"), "test1");

//     ft.remove(Path::new("test/file"));

//     println!(
//         "contains test/file2/file3: {}",
//         ft.contains_file(Path::new("test/file2"))
//     );
// }
