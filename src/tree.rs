use crate::database::Storable;
use crate::entry::Entry;

static MODE: &str = "100644";

/// Implements a git tree object, a storable list of
/// entries.
pub struct Tree {
    // Sorted list of entries by name.
    _entries: Vec<Entry>,
    data: Vec<u8>,
}

impl Tree {
    pub fn new(mut entries: Vec<Entry>) -> Tree {
        entries.sort();
        let data = entries
            .iter()
            .map(|entry| {
                // Entry format: "{MODE} {NAME}\0{OID}"
                vec![
                    format!("{} ", MODE).as_bytes(),
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
            .collect::<Vec<u8>>();

        Tree {
            _entries: entries,
            data,
        }
    }
}

impl Storable for Tree {
    fn type_name(&self) -> &str {
        "tree"
    }

    fn data(&self) -> &Vec<u8> {
        &self.data
    }
}
