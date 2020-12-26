use crate::author::Author;
use crate::database::{ObjectID, Storable};

pub struct Commit {
    message: String,
    data: Vec<u8>,
}

impl Commit {
    pub fn new(parent: &Option<ObjectID>, oid: &ObjectID, author: Author, message: String) -> Self {
        let parent_msg = if let Some(parent) = parent {
            format!("parent {}\n", parent.as_str())
        } else {
            "".to_string()
        };
        let data = format!(
            "{}\
             tree {}\n\
             author {}\n\
             committer {}\n\
             \n\
             {}",
            parent_msg,
            oid.as_str(),
            author.to_str(),
            author.to_str(),
            message
        )
        .as_bytes()
        .to_vec();

        Commit { message, data }
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
