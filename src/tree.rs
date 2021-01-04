use crate::database::Storable;
use crate::entry::Entry;
use crate::workspace::{Workspace, WorkspacePath};
use anyhow::{anyhow, Result};
use lazy_init::Lazy;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

// A single component of a path - should have no parents or separators.
//
// This is just a PathBuf, but it helps enforce that it's only *one*
// component within a PathBuf. This is a hacky enforcement, but IMO
// that's fine because this structure isn't public.
#[derive(Debug, Eq, Hash, PartialEq, Ord, PartialOrd, Clone)]
struct Component(PathBuf);

impl Component {
    fn new<P: AsRef<Path>>(p: P) -> Self {
        assert_eq!(p.as_ref().parent().unwrap(), Path::new(""));
        Component(p.as_ref().into())
    }
}

// Trees consist of either:
// - Subtrees, or
// - Entries
#[derive(Debug)]
enum Node {
    Tree(TreeNode),
    Entry(WorkspacePath),
}

#[derive(Default, Debug)]
struct TreeNode {
    map: HashMap<Component, Node>,
}

impl TreeNode {
    // Inserts an entry into a TreeNode, in a recursive-friendly manner.
    //
    // Suppose you're trying to insert the following:
    //   "a/b/c/d.txt"
    // This entry should be parsed as follows:
    //   parents: ["a", "b", "c"], entry: "a/b/c/d.txt"
    // By calling "add_entry", intermediate nodes are created, such that:
    //   a -> b -> c -> d.txt
    fn add_entry(
        &mut self,
        workspace: &Workspace,
        parents: &[Component],
        entry: &WorkspacePath,
    ) -> Result<()> {
        if parents.is_empty() {
            // We have accessed the TreeNode storing the entry. Add away!
            let basename = entry.as_partial_path().file_name().unwrap();
            let node = if workspace.metadata(&entry)?.is_dir() {
                Node::Tree(TreeNode::default())
            } else {
                Node::Entry(entry.clone())
            };
            let old = self.map.insert(Component::new(basename), node);
            assert!(
                old.is_none(),
                "We kicked something out to insert this entry!"
            );
        } else {
            // We need to do some tree traversal to reach the entry.
            match self.map.get_mut(&parents[0]) {
                // This intermediate node already exists - lets try to add
                // the entry to that node, instead of this one.
                Some(node) => {
                    match node {
                        Node::Tree(node) => node.add_entry(workspace, &parents[1..], entry)?,
                        Node::Entry(_) => panic!("Parsed a directory as a file?"),
                    };
                }
                // No intermediate node exists, but one SHOULD exist here.
                None => {
                    let mut node = TreeNode::default();
                    node.add_entry(workspace, &parents[1..], entry)?;
                    self.map.insert(parents[0].clone(), Node::Tree(node));
                }
            };
        }
        Ok(())
    }
}

/// Implements a git tree object, a storable list of entries.
pub struct Tree {
    // Sorted list of entries by name.
    entries: Vec<Entry>,
    // Lazily initialized view of data.
    data: Lazy<Vec<u8>>,
}

impl Tree {
    // XXX this should just become the "new" method...
    pub fn build(workspace: &Workspace, mut entries: Vec<WorkspacePath>) -> Result<Self> {
        entries.sort();

        // 'entries' is full paths relative to workspace root
        let mut root = TreeNode::default();

        for entry in &entries {
            let path = entry.as_partial_path();
            println!("Tree::build entry: {}", path.display());

            let parents: Vec<Component> = entry
                .as_partial_path()
                .iter()
                .map(|p| Component::new(&p))
                .collect();
            println!("  parents: {:#?}", parents);
            root.add_entry(workspace, &parents[..parents.len() - 1], entry)?;
        }
        println!("Root: {:#?}", root);
        Err(anyhow!("nah"))
    }

    pub fn serialize(&self) -> Vec<u8> {
        self.entries
            .iter()
            .map(|entry| {
                println!("Tree entry path: {}", entry.path().display());
                // Entry format: "{MODE} {NAME}\0{OID}"
                vec![
                    format!("{} ", entry.mode().as_str()).as_bytes(),
                    entry.path_bytes(),
                    &[b'\0'],
                    entry.oid().as_bytes(),
                ]
                .iter()
                .map(|slice| slice.to_vec())
                .flatten()
                .collect::<Vec<u8>>()
            })
            .flatten()
            .collect::<Vec<u8>>()
    }

    pub fn new(mut entries: Vec<Entry>) -> Tree {
        entries.sort();
        Tree {
            entries,
            data: Lazy::new(),
        }
    }
}

impl Storable for Tree {
    fn type_name(&self) -> &str {
        "tree"
    }

    fn data(&self) -> &Vec<u8> {
        &self.data.get_or_create(|| self.serialize())
    }
}
