use crate::author::Author;
use crate::database::{ObjectID, Storable};

pub struct Commit {
    _oid: ObjectID,
    _author: Author,
    message: String,
    data: Vec<u8>,
}

impl Commit {
    pub fn new(oid: ObjectID, author: Author, message: String) -> Self {
        let data = format!(
            "tree {}\n\
             author {}\n\
             committer {}\n\
             \n\
             {}",
            oid.as_str(),
            author.to_str(),
            author.to_str(),
            message
        )
        .as_bytes()
        .to_vec();

        Commit {
            _oid: oid,
            _author: author,
            message,
            data,
        }
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

impl Storable for Commit {
    fn type_name(&self) -> &str {
        "commit"
    }
    fn data(&self) -> &Vec<u8> {
        &self.data
    }
}
