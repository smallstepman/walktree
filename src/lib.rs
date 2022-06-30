use bimap::BiMap;
use indextree::{Arena, NodeId};
use std::{
    cmp::Ordering,
    path::{Path, PathBuf},
};
use walkdir::{DirEntry, WalkDir};

#[derive(Debug)]
pub enum WalkTreeError {}

#[derive(Debug, PartialEq, Eq)]
pub struct WalkTree<T> {
    pub arena: Arena<T>,
    pub map: BiMap<PathBuf, NodeId>,
}

impl<T> WalkTree<T> {
    pub fn load(path: &Path) -> WalkTreeBuilder<T> {
        WalkTreeBuilder::load(path)
    }
    fn build(mut arena: Arena<T>, map: BiMap<PathBuf, NodeId>) -> Self {
        for (path, node_id) in map.iter() {
            if let Some(parent) = path.parent() {
                if let Some(t) = map.get_by_left(&parent.to_path_buf()) {
                    t.append(*node_id, &mut arena);
                }
            }
        }
        Self { arena, map }
    }
    pub fn get_path_by_node_id(&self, node_id: NodeId) -> Option<&PathBuf> {
        self.map.get_by_right(&node_id)
    }
    pub fn get_node_id_by_path(&self, path: &Path) -> Option<&NodeId> {
        self.map.get_by_left(path)
    }
    pub fn get_item_by_path(&self, path: &Path) -> Option<&T> {
        if let Some(node_id) = self.get_node_id_by_path(path) {
            return self.get_item_by_node_id(*node_id);
        }
        None
    }
    pub fn get_item_by_node_id(&self, node_id: NodeId) -> Option<&T> {
        if let Some(node) = self.arena.get(node_id) {
            return Some(&node.get());
        }
        None
    }
    // pub fn get_mut_item_by_path(&mut self, path: &Path) -> Option<&T> {
    //     if let Some(node_id) = self.get_node_id_by_path(path) {
    //         if let Some(node) = self.arena.get_mut(*node_id) {
    //             return Some(&node.get_mut());
    //         }
    //     }
    //     None
    // }
    // pub fn get_mut_item_by_node_id(&mut self, node_id: NodeId) -> Option<&T> {
    //     if let Some(node) = self.arena.get_mut(node_id) {
    //         return Some(&node.get_mut());
    //     }
    //     None
    // }
}

pub struct WalkTreeBuilder<T> {
    root_dir: PathBuf,
    fn_filter: Option<fn(&walkdir::DirEntry) -> bool>,
    fn_map: Option<fn(walkdir::DirEntry) -> T>,
    walkdir_modes: WalkDirModes,
}

struct WalkDirModes(Vec<WalkDirOption>);

pub enum WalkDirOption {
    ContentsFirst,
    FollowLinks,
    MaxDepth(usize),
    MaxOpen(usize),
    MinDepth(usize),
    SameFileSystem,
    SortBy(fn(&DirEntry, &DirEntry) -> Ordering),
    SortByFileName,
    SortByKey(fn(&DirEntry) -> Ordering),
}

#[derive(Debug, PartialEq, Eq)]
struct WalkTreeNode<T> {
    pub path: PathBuf,
    pub data: T,
}

impl WalkDirModes {
    fn new() -> Self {
        Self(Vec::<WalkDirOption>::new())
    }
    fn check_compability(&self) -> Result<(), WalkTreeError> {
        Ok(())
    }
    fn compile_walkdir(&self, root_dir: &Path) -> WalkDir {
        self.0
            .iter()
            .fold(WalkDir::new(root_dir), |walkdir, option| match option {
                WalkDirOption::ContentsFirst => walkdir.contents_first(true),
                WalkDirOption::FollowLinks => walkdir.follow_links(true),
                WalkDirOption::MaxDepth(i) => walkdir.max_depth(*i),
                WalkDirOption::MaxOpen(i) => walkdir.max_open(*i),
                WalkDirOption::MinDepth(i) => walkdir.min_depth(*i),
                WalkDirOption::SameFileSystem => walkdir.same_file_system(true),
                WalkDirOption::SortBy(f) => walkdir.sort_by(*f),
                WalkDirOption::SortByFileName => walkdir.sort_by_file_name(),
                WalkDirOption::SortByKey(f) => walkdir.sort_by_key(*f),
            })
    }
}

impl Default for WalkDirModes {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> WalkTreeBuilder<T> {
    pub fn load(p: &Path) -> WalkTreeBuilder<T> {
        WalkTreeBuilder {
            root_dir: p.to_path_buf(),
            fn_filter: None,
            fn_map: None,
            walkdir_modes: WalkDirModes::new(),
        }
    }
    pub fn with_map(self, f: fn(walkdir::DirEntry) -> T) -> Self {
        WalkTreeBuilder {
            fn_map: Some(f),
            ..self
        }
    }
    pub fn with_fliter(self, f: fn(&walkdir::DirEntry) -> bool) -> Self {
        WalkTreeBuilder {
            fn_filter: Some(f),
            ..self
        }
    }
    pub fn with_walkdir_mode(mut self, mode: WalkDirOption) -> Self {
        self.walkdir_modes.0.push(mode);
        self
    }
    pub fn walk(self) -> Result<WalkTree<T>, WalkTreeError> {
        self.walkdir_modes.check_compability()?;
        let walkdir = self.walkdir_modes.compile_walkdir(&self.root_dir);
        let entries = self.apply_fiter_fn(walkdir);
        let entries = self.apply_map_fn(entries);

        let mut arena = Arena::<T>::new();
        let map = entries
            .into_iter()
            .map(|e| (e.path.clone(), arena.new_node(e.data)))
            .collect::<BiMap<_, _>>();

        Ok(WalkTree::build(arena, map))
    }
    fn apply_fiter_fn(&self, w: WalkDir) -> Vec<DirEntry> {
        if let Some(fn_filter) = self.fn_filter {
            w.into_iter()
                .filter_entry(fn_filter)
                .filter_map(|x| x.ok())
                .collect::<Vec<_>>()
        } else {
            w.into_iter().filter_map(|x| x.ok()).collect::<Vec<_>>()
        }
    }
    fn apply_map_fn(&self, w: Vec<DirEntry>) -> Vec<WalkTreeNode<T>> {
        w.into_iter()
            .map(|v| WalkTreeNode {
                path: v.path().to_path_buf(),
                data: self.fn_map.unwrap()(v),
            })
            .collect::<Vec<_>>()
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use std::fs::read_dir;

    #[test]
    fn new() {
        let p = Path::new("~/Documents/");
        let _x = WalkTree::load(p)
            .with_fliter(|_| true)
            .with_map(|x| {
                if x.path().is_dir() {
                    read_dir(x.path())
                        .unwrap()
                        .filter_map(|x| x.ok())
                        .map(|x| x.path())
                        .collect::<Vec<_>>()
                } else {
                    vec![]
                }
            })
            .walk()
            .unwrap();
        // assert_eq!(x, vec![]);
    }
}
