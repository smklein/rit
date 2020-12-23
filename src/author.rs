use chrono::{DateTime, Utc};
use std::time::SystemTime;

pub struct Author {
    name: String,
    email: String,
    time: SystemTime,
}

impl Author {
    pub fn new(name: String, email: String, time: SystemTime) -> Self {
        Author { name, email, time }
    }

    pub fn to_str(&self) -> String {
        let timestamp = DateTime::<Utc>::from(self.time).format("%s %z");
        format!("{} <{}> {}", self.name, self.email, timestamp)
    }
}
