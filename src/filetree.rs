use std::collections::HashSet;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
struct FileNode<'a> {
    group: Option<&'a str>,
    parent_node_idx: usize,
    // points to a path in FileTree::paths
    path_idx: usize,
    // index into FileTree::nodes
    children: Option<Vec<usize>>,
}

#[derive(Default, Debug)]
struct FileTree<'a> {
    paths: Vec<Option<PathBuf>>,
    nodes: Vec<Option<FileNode<'a>>>,
    groups: HashSet<String>,
}

struct FileTreeIterator<'a> {
    tree: &'a FileTree<'a>,
    stack: Option<Vec<usize>>,
}

impl<'a> Iterator for FileTreeIterator<'a> {
    type Item = (usize, &'a Option<FileNode<'a>>);

    fn next(&mut self) -> Option<Self::Item> {
        let stack = self.stack.as_mut()?;
        let node_idx = stack.pop()?;
        let node = &self.tree.nodes[node_idx];

        if let Some(node) = node {
            if let Some(ref children) = node.children {
                stack.extend_from_slice(children);
            }
        }

        Some((node_idx, node))
    }
}

impl<'a> FileTree<'a> {
    pub fn new(root: &Path) -> Self {
        let mut tree: Self = Default::default();
        tree.insert(None, root);
        tree
    }

    fn iter(&'a self) -> FileTreeIterator<'a> {
        FileTreeIterator {
            tree: self,
            stack: match self.nodes.is_empty() {
                true => None,
                false => Some(vec![0]),
            },
        }
    }

    /// Returns true if the path is starts with the path in the root node of the file tree
    pub fn path_is_in_root(&self, value: &Path) -> bool {
        let root_node = self
            .nodes
            .first()
            .unwrap()
            .as_ref()
            .expect("root node should never be marked as None");

        let root_path = self
            .paths
            .get(root_node.path_idx)
            .unwrap()
            .as_ref()
            .expect("root should never be None");

        value.starts_with(root_path)
    }

    pub fn contains_path(&self, value: &Path) -> bool {
        self.find_node_idx(value).is_some()
    }

    pub fn contains_group(&self, groupname: &str) -> bool {
        self.groups.contains(groupname)
    }

    fn find_node_idx(&self, value: &Path) -> Option<usize> {
        for (idx, node) in self.iter() {
            let node = match node {
                Some(node) => node,
                None => continue,
            };

            if let Some(ref path_node) = self.paths[node.path_idx] {
                if path_node == value {
                    return Some(idx);
                }
            }
        }

        None
    }

    /// inserts a path from a group with no regard for intermediate values (each component in the path)
    /// Returns true if the insertion was successful
    fn insert_path(&mut self, group: Option<&'a str>, value: &Path) -> bool {
        if self.contains_path(value) {
            return false;
        }

        if let Some(group) = group {
            self.groups.insert(group.into());
        }

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

            return true;
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

                true
            }

            None => false,
        }
    }

    /// Inserts a path from a group along with all of the intermediate paths (each component in the path)
    /// An intermediate paths group is always `None`
    pub fn insert(&mut self, group: Option<&'a str>, value: &Path) -> bool {
        if self.paths.is_empty() {
            return self.insert_path(group, value);
        }

        let mut curr_path = self.paths[0].clone().expect("root should never be none");
        let Ok(value_components) = value.strip_prefix(&curr_path) else {
            return false;
        };

        let Some(last_component) = value.file_name() else {
            return false;
        };

        let mut last_insert_ret = false;
        for component in value_components {
            curr_path.push(component);
            last_insert_ret = self.insert_path(
                if component == last_component {
                    group
                } else {
                    None
                },
                &curr_path,
            );
        }

        last_insert_ret
    }

    fn remove_idx(&mut self, idx: usize) -> Option<PathBuf> {
        let node = self.nodes.get(idx)?.clone()?;

        if let Some(ref mut parent_node) = self.nodes[node.parent_node_idx] {
            if let Some(ref children) = parent_node.children {
                let children: Vec<_> = children.iter().filter(|v| **v != idx).copied().collect();

                parent_node.children = if children.is_empty() {
                    None
                } else {
                    Some(children)
                };
            }
        }

        self.nodes[idx] = None;
        let discarded_path = self.paths[node.path_idx].clone()?;
        self.paths[node.path_idx] = None;

        if let Some(group) = node.group {
            let has_group = self.nodes.iter().any(|n| {
                let Some(n) = n else {
                    return false;
                };

                let Some(ngroup) = n.group else {
                    return false;
                };

                ngroup == group
            });

            if !has_group {
                self.groups.remove(group);
            }
        }

        Some(discarded_path)
    }

    // todo: find out way to remove lingering None group node
    pub fn remove(&mut self, idx: usize) -> Option<PathBuf> {
        let mut curr_idx = idx;
        while let Some(node) = self.nodes.get(curr_idx)? {
            let parent_idx = node.parent_node_idx;

            let parent = self.nodes[parent_idx].clone().unwrap();
            if parent.group.is_some() {
                break;
            }

            let children = parent.children.expect("parent should always have children");

            if children.len() == 1 {
                self.remove_idx(node.parent_node_idx);
            }

            curr_idx = parent_idx;
        }

        self.remove_idx(idx)
    }

    pub fn remove_path(&mut self, value: &Path) -> Option<PathBuf> {
        let value_idx = self.find_node_idx(value)?;
        self.remove(value_idx)
    }

    // note: instead of PathBuf consider using T or just plain dotfiles::Dotfile
    pub fn get(&self, group: &str) -> Option<HashSet<PathBuf>> {
        let mut group_paths = HashSet::new();

        for (_, node) in self.iter() {
            let node = match node {
                Some(node) => node,
                None => unreachable!("there should not be any valid node that is none"),
            };

            if node.group == Some(group) {
                let node_path = self.paths[node.path_idx].as_ref()?;
                group_paths.insert(node_path.clone());
            }
        }

        Some(group_paths)
    }

    pub fn canonicalize(&mut self) {
        todo!()
    }

    pub fn is_empty(&self) -> bool {
        if self.paths.is_empty() && self.nodes.is_empty() {
            return true;
        }

        let has_items = self.paths.iter().any(|path| path.is_some())
            || self.nodes.iter().any(|node| node.is_some());

        !has_items
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ftree_initialized_with_root_at_idx_0() {
        let ft = FileTree::new(Path::new("/"));
        assert_eq!(ft.paths.len(), 1);
        assert_eq!(ft.nodes.len(), 1);
        assert_eq!(ft.groups.len(), 0);

        let root_node = ft.nodes.first().unwrap().as_ref().unwrap();
        assert_eq!(root_node.path_idx, 0);

        let root_path = ft.paths.get(root_node.path_idx).unwrap().as_ref().unwrap();
        assert_eq!(root_path, Path::new("/"));
    }

    #[test]
    fn inserting_items() {
        let mut ft = FileTree::new(Path::new("/"));
        ft.insert(Some("test"), Path::new("/usr"));
        ft.insert(Some("test"), Path::new("/usr/bin"));

        assert!(ft.contains_group("test"));
        assert!(ft.paths.len() == 3 && ft.nodes.len() == 3);

        assert!(!ft.insert(Some("test"), Path::new("/usr/bin")));
        assert!(ft.insert(Some("test"), Path::new("/usr/bin/tuckr")));
    }

    #[test]
    fn removing_items() {
        let mut ft = FileTree::new(Path::new("/"));
        ft.insert(Some("test"), Path::new("/usr"));
        ft.insert(Some("test2"), Path::new("/usr/bin/ls"));

        ft.remove_path(Path::new("/usr/bin/ls"));

        assert!(!ft.contains_group("test2"));
        assert!(ft.nodes.iter().any(|p| p.is_none()));
    }

    #[test]
    fn iterator_only_sees_valid_nodes() {
        let mut ft = FileTree::new(Path::new("/"));
        ft.insert(Some("test"), Path::new("/usr"));
        ft.insert(Some("test"), Path::new("/usr/bin"));
        ft.remove_path(Path::new("/usr/bin"));

        assert!(!ft.iter().any(|(_, v)| v.is_none()));

        let mut ft = FileTree::new(Path::new("/"));
        ft.insert(Some("test"), Path::new("/usr"));
        ft.insert(Some("test"), Path::new("/usr/bin/ls"));

        // todo: extract into its own test
        // what it does: ensures that intermediary nodes are always inserted with group `None`
        for node in ft.iter() {
            let node = node.1.as_ref().unwrap();
            println!(
                "
                    group: {:?}
                    path: {:?}
                ",
                node.group, ft.paths[node.path_idx]
            );

            let Some(ref children) = node.children else {
                continue;
            };

            for child in children {
                println!(
                    "
                    child: {:?}
                    ",
                    ft.paths[ft.nodes[*child].as_ref().unwrap().path_idx]
                );
            }
        }
    }

    #[test]
    fn insert_rejects_paths_outside_of_root() {
        let mut ft = FileTree::new(Path::new("/home/tuckr"));
        assert_eq!(ft.insert(Some("test"), Path::new("/home/tuckr/test")), true);

        assert_eq!(ft.insert(Some("test"), Path::new("/usr/bin")), false);
        assert!(ft.remove_path(Path::new("/usr/bin")).is_none());
    }
}
