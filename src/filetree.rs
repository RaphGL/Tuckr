use std::collections::HashSet;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
struct FileNode {
    group: Option<String>,
    parent_node_idx: usize,
    // points to a dotfile in FileTree::dotfiles
    dotfile_idx: usize,
    // index into FileTree::nodes
    children: Option<Vec<usize>>,
}

#[derive(Default, Debug, Clone)]
pub struct FileTree {
    dotfiles: Vec<Option<PathBuf>>,
    nodes: Vec<Option<FileNode>>,
    groups: HashSet<String>,
}

// the struct exposed to the user through the iterator
pub struct File<'tree> {
    pub group: Option<String>,
    pub path: &'tree Path,
}

struct InternalFileTreeIterator<'a> {
    tree: &'a FileTree,
    stack: Option<Vec<usize>>,
}

impl<'a> Iterator for InternalFileTreeIterator<'a> {
    type Item = (usize, &'a Option<FileNode>);

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

pub struct FileTreeIterator<'tree, 'iter> {
    tree: &'tree FileTree,
    internal_iter: InternalFileTreeIterator<'iter>,
}

impl<'tree, 'iter> Iterator for FileTreeIterator<'tree, 'iter> {
    type Item = File<'tree>;

    // TODO: change so that we only use &Path and return a FileNode of sorts that contains the path and the group the path belongs to
    // this is so we can save on doing computations every time we want to insert a Dotfile
    fn next(&mut self) -> Option<Self::Item> {
        let Some((node_idx, curr_node)) = self.internal_iter.next() else {
            return None;
        };

        let curr_node = curr_node.as_ref().unwrap();

        Some(File {
            group: curr_node.group.clone(),
            path: self.tree.dotfiles[node_idx].as_ref()?,
        })
    }
}

impl<'a> FileTree {
    pub fn new<P: AsRef<Path>>(root: P) -> Self {
        let mut tree: Self = Default::default();
        tree.insert(None, root.as_ref());
        tree
    }

    // returns an iterator that returns a tuple `(node_idx, node)`
    fn internal_iter(&'a self) -> InternalFileTreeIterator<'a> {
        InternalFileTreeIterator {
            tree: self,
            stack: match self.nodes.is_empty() {
                true => None,
                false => Some(vec![0]),
            },
        }
    }

    pub fn iter(&self) -> FileTreeIterator<'_, '_> {
        let mut ft_iter = FileTreeIterator {
            tree: self,
            internal_iter: self.internal_iter(),
        };
        // since root is always guaranteed to be the first node
        // we can just skip it, on iteration since we're pretending that we're iterating a diretory contents
        ft_iter.internal_iter.next();
        ft_iter
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
            .dotfiles
            .get(root_node.dotfile_idx)
            .unwrap()
            .as_ref()
            .expect("root should never be None");

        value.starts_with(&root_path)
    }

    pub fn contains_path(&self, value: &Path) -> bool {
        self.find_node_idx(value).is_some()
    }

    pub fn contains_group(&self, groupname: &str) -> bool {
        self.groups.contains(groupname)
    }

    fn find_node_idx(&self, value: &Path) -> Option<usize> {
        for (idx, node) in self.internal_iter() {
            let node = match node {
                Some(node) => node,
                None => continue,
            };

            if let Some(ref dotfile_node) = self.dotfiles[node.dotfile_idx] {
                if dotfile_node == value {
                    return Some(idx);
                }
            }
        }

        None
    }

    /// inserts a path from a group with no regard for intermediate values (each component in the path)
    /// Returns true if the insertion was successful
    fn insert_path(&mut self, group: Option<String>, value: &Path) -> bool {
        if self.contains_path(value) {
            return false;
        }

        if let Some(ref group) = group {
            self.groups.insert(group.into());
        }

        self.dotfiles.push(Some(value.into()));
        let path_idx = self.dotfiles.len() - 1;

        let value_parent = {
            let mut parent = value.to_path_buf();
            parent.pop();
            parent
        };

        if self.nodes.is_empty() {
            self.nodes.push(Some(FileNode {
                group,
                parent_node_idx: 0,
                dotfile_idx: path_idx,
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
                    dotfile_idx: path_idx,
                    children: None,
                }));

                true
            }

            None => false,
        }
    }

    /// Inserts a path from a group along with all of the intermediate paths (each component in the path)
    /// An intermediate paths group is always `None`
    pub fn insert(&mut self, group: Option<String>, value: &Path) -> bool {
        if self.dotfiles.is_empty() {
            return self.insert_path(group, value);
        }

        let mut curr_dotfile = self.dotfiles[0].clone().expect("root should never be none");
        let Ok(value_components) = value.strip_prefix(&curr_dotfile) else {
            return false;
        };

        let Some(last_component) = value.file_name() else {
            return false;
        };

        let mut last_insert_ret = false;
        for component in value_components {
            curr_dotfile.push(component);
            last_insert_ret = self.insert_path(
                if component == last_component {
                    group.clone()
                } else {
                    None
                },
                &curr_dotfile,
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
        let discarded_path = self.dotfiles[node.dotfile_idx].clone();
        self.dotfiles[node.dotfile_idx] = None;

        if let Some(group) = node.group {
            let has_group = self.nodes.iter().any(|node| {
                let Some(node) = node else {
                    return false;
                };

                let Some(ref ngroup) = node.group else {
                    return false;
                };

                *ngroup == group
            });

            if !has_group {
                self.groups.remove(&group);
            }
        }

        discarded_path
    }

    // TODO: find out way to remove lingering None group node
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

        for (_, node) in self.internal_iter() {
            let node = match node {
                Some(node) => node,
                None => unreachable!("there should not be any valid node that is none"),
            };

            if node.group.as_deref() == Some(group) {
                let node_path = self.dotfiles[node.dotfile_idx].as_ref()?;
                group_paths.insert(node_path.clone());
            }
        }

        Some(group_paths)
    }

    pub fn get_groups(&self) -> &HashSet<String> {
        &self.groups
    }

    pub fn canonicalize(&mut self) {
        // TODO: implement iter_mut so we can change the node group to None
        for (node_idx, node) in self.internal_iter() {
            let node = node.as_ref().unwrap();
            let Some(ref children) = node.children else {
                continue;
            };

            let first_child = self.nodes[children[0]].as_ref().unwrap();
            let mut share_parent = false;
            for child in &children[1..] {
                let child = self.nodes[*child].as_ref().unwrap();
                if child.group != first_child.group {
                    share_parent = true;
                    break;
                }
            }

            if share_parent {
                // let mut node = self.nodes[node_idx].as_mut().unwrap();
                // node.group = None;
            }
        }

        // TODO: convert canonicalization logic to use file tree
        //
        // fn remove_empty_groups(group_type: HashCache) -> HashCache {
        //     group_type
        //         .iter()
        //         .filter(|(_, v)| !v.is_empty())
        //         .map(|(k, v)| (k.to_owned(), v.to_owned()))
        //         .collect()
        // }

        // // removes entries for paths that are subpaths of another entry (canonicalization).
        // // this procedure makes so that symlinks are shallow.
        // //
        // // shallow symlinking: only symlinking files/directories that don't exist already
        // fn canonicalize_groups(groups: &mut HashCache) {
        //     for files in groups.values_mut() {
        //         let files_copy = files.clone();

        //         for file in &files_copy {
        //             for file2 in &files_copy {
        //                 if file2.path != file.path && file2.path.starts_with(&file.path) {
        //                     files.remove(file2);
        //                 }
        //             }
        //         }
        //     }
        // }

        // // canonicalizes not_symlinked based on symlinked cache
        // //
        // // this is necessary because if a directory is canonicalized and symlinked,
        // // files inside it won't be symlinked and thus marked as `not_symlinked` wrongly.
        // for (group, files) in &symlinked {
        //     let Some(unsymlinked_group) = not_symlinked.get_mut(group) else {
        //         continue;
        //     };

        //     let unsymlinked_group_copy = unsymlinked_group.clone();

        //     for file1 in files {
        //         for file2 in unsymlinked_group_copy.iter() {
        //             if file2.path.starts_with(&file1.path) {
        //                 unsymlinked_group.remove(file2);
        //             }
        //         }
        //     }
        // }

        // canonicalize_groups(&mut symlinked);
        // canonicalize_groups(&mut not_symlinked);
        // canonicalize_groups(&mut not_owned);

        // self.symlinked = remove_empty_groups(symlinked);
        // self.not_symlinked = remove_empty_groups(not_symlinked);
        // self.not_owned = remove_empty_groups(not_owned);
    }

    pub fn is_empty(&self) -> bool {
        if self.dotfiles.is_empty() && self.nodes.is_empty() {
            return true;
        }

        let has_items = self.dotfiles.iter().any(|path| path.is_some())
            || self.nodes.iter().any(|node| node.is_some());

        !has_items
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dotfiles;

    fn get_dotfiles_configs() -> PathBuf {
        let mut dotfiles_path = dotfiles::get_dotfiles_path(None).unwrap();
        dotfiles_path.push("Configs");
        dotfiles_path
    }

    #[test]
    fn ftree_initialized_with_root_at_idx_0() {
        let dotfiles_root = get_dotfiles_configs();
        let ft = FileTree::new(&dotfiles_root);
        assert_eq!(ft.dotfiles.len(), 1);
        assert_eq!(ft.nodes.len(), 1);
        assert_eq!(ft.groups.len(), 0);

        let root_node = ft.nodes.first().unwrap().as_ref().unwrap();
        assert_eq!(root_node.dotfile_idx, 0);

        let root_path = ft
            .dotfiles
            .get(root_node.dotfile_idx)
            .unwrap()
            .as_ref()
            .unwrap();
        assert_eq!(root_path, &dotfiles_root);
    }

    #[test]
    fn inserting_items() {
        let root = get_dotfiles_configs();
        let nvim = {
            let mut nvim = root.clone();
            nvim = nvim.join("nvim");
            nvim
        };

        let nvim_config = {
            let mut nvim_config = nvim.clone();
            nvim_config = nvim_config.join(".config");
            nvim_config
        };

        let nvim_config_nvim = {
            let mut nvim_config_nvim = nvim_config.clone();
            nvim_config_nvim = nvim_config_nvim.join("neovim");
            nvim_config_nvim
        };

        let mut ft = FileTree::new(&root);

        ft.insert(Some("test".into()), &nvim);
        ft.insert(Some("test".into()), &nvim_config);

        assert!(ft.contains_group("test"));
        assert!(ft.dotfiles.len() == 3 && ft.nodes.len() == 3);

        assert!(!ft.insert(Some("test".into()), &nvim));
        assert!(ft.insert(Some("test".into()), &nvim_config_nvim));
    }

    #[test]
    fn removing_items() {
        let root = get_dotfiles_configs();
        let mut ft = FileTree::new(&root);

        let test = {
            let mut test = root.clone();
            test = test.join("usr");
            test
        };

        let test_child = {
            let mut test_child = test.clone();
            test_child = test_child.join("bin").join("ls");
            test_child
        };

        ft.insert(Some("test".into()), &test);
        ft.insert(Some("test2".into()), &test_child);

        ft.remove_path(&test_child);

        assert!(!ft.contains_group("test2"));
        assert!(ft.nodes.iter().any(|p| p.is_none()));
    }

    // TODO: finish fixing unit tests
    //
    // #[test]
    // fn iterator_only_sees_valid_nodes() {
    //     let mut ft = FileTree::new(Path::new("/"));
    //     ft.insert(Some("test".into()), Path::new("/usr"));
    //     ft.insert(Some("test".into()), Path::new("/usr/bin"));
    //     ft.remove_path(Path::new("/usr/bin"));

    //     assert!(!ft.internal_iter().any(|(_, v)| v.is_none()));

    //     let mut ft = FileTree::new(Path::new("/"));
    //     ft.insert(Some("test".into()), Path::new("/usr"));
    //     ft.insert(Some("test".into()), Path::new("/usr/bin/ls"));

    //     // todo: extract into its own test
    //     // what it does: ensures that intermediary nodes are always inserted with group `None`
    //     for node in ft.internal_iter() {
    //         let node = node.1.as_ref().unwrap();
    //         println!(
    //             "
    //                  group: {:?}
    //                  path: {:?}
    //              ",
    //             node.group, ft.dotfiles[node.dotfile_idx]
    //         );

    //         let Some(ref children) = node.children else {
    //             continue;
    //         };

    //         for child in children {
    //             println!(
    //                 "
    //                  child: {:?}
    //                  ",
    //                 ft.dotfiles[ft.nodes[*child].as_ref().unwrap().dotfile_idx]
    //             );
    //         }
    //     }
    // }

    #[test]
    fn insert_rejects_paths_outside_of_root() {
        let root = {
            let mut root = get_dotfiles_configs();
            root = root.join("nvim");
            root
        };

        let in_root = {
            let mut in_root = root.clone();
            in_root = in_root.join("test");
            in_root
        };

        let out_of_root = {
            let mut out_of_root = get_dotfiles_configs();
            out_of_root = out_of_root.join("something");
            out_of_root
        };

        let nonexistent = {
            let mut nonexistent = get_dotfiles_configs();
            nonexistent = nonexistent.join("nonexistent");
            nonexistent
        };

        let mut ft = FileTree::new(&root);
        assert_eq!(ft.insert(Some("test".into()), &in_root), true);

        assert_eq!(ft.insert(Some("test".into()), &out_of_root), false);
        assert!(ft.remove_path(&nonexistent).is_none());
    }
}
