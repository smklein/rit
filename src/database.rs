use anyhow::Result;
use sha1::{Digest, Sha1};
use std::fs::{create_dir_all, rename, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

/// An entity which may be stored within the git object database.
pub trait Storable {
    // Identifies which object is serialized here.
    fn type_name(&self) -> &str;

    // Returns a byte representation of the underlying data.
    fn data(&self) -> &Vec<u8>;

    // TODO: Could be fancy, use types to enforce encryption.

    fn encoded_raw(&self) -> Vec<u8> {
        let mut content = Vec::new();
        content.extend_from_slice(self.type_name().as_bytes());
        content.extend_from_slice(" ".as_bytes());
        content.extend_from_slice(format!("{}\0", self.data().len()).as_bytes());
        content.extend_from_slice(self.data());
        content
    }

    /// Serialize the opbject to a byte steam.
    fn serialize(&self) -> Result<Vec<u8>> {
        let content = self.encoded_raw();

        // Use zlib to compress the file so it uses less on-disk storage.
        let compression = flate2::Compression::fast();
        let mut encoder = flate2::write::ZlibEncoder::new(Vec::new(), compression);
        encoder.write_all(&content)?;
        let compressed = encoder.finish()?;

        println!(
            "Content compressed from {} --> {} bytes",
            content.len(),
            compressed.len()
        );

        Ok(compressed)
    }

    /// Return the sha1 of the object, as a hex encoded string.
    fn sha1(&self) -> String {
        let mut hasher = Sha1::new();
        hasher.update(self.encoded_raw().clone());
        hex::encode(hasher.finalize())
    }
}

/// Utility for storing Blob objects within git.
pub struct Database {
    root: PathBuf,
}

impl Database {
    /// Generates a new database object around the provided
    /// git database path.
    pub fn new<P: AsRef<Path>>(path: P) -> Self {
        Database {
            root: PathBuf::from(path.as_ref().clone()),
        }
    }

    /// Add a new blob object to the Git object store.
    pub fn store(&self, object: impl Storable) -> Result<()> {
        let content = object.serialize()?;
        let object_id = object.sha1();
        println!("Object ID: {}", object_id);
        let prefix = &object_id[0..2];
        let suffix = &object_id[2..];

        // First two characters of the object ID form a directory.
        // The latter characters of the object ID form the regular file name.
        let mut path = self.root.clone();
        path.push(&prefix);
        create_dir_all(&path)?;
        let temp_filename = format!("{}.tmp", suffix);
        let mut final_path = path.clone();
        final_path.push(suffix);
        let mut tmp_path = path;
        tmp_path.push(&temp_filename);

        // Create the file exclusively so we won't clobber anyone else
        // generating this object.
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .open(&tmp_path)?;
        file.write_all(&content)?;

        rename(tmp_path, final_path)?;

        Ok(())
    }
}

/// Encapsulates the contents of a file.
pub struct Blob {
    data: Vec<u8>,
}

impl Storable for Blob {
    fn type_name(&self) -> &str {
        "blob"
    }
    fn data(&self) -> &Vec<u8> {
        &self.data
    }
}

impl Blob {
    pub fn new(data: Vec<u8>) -> Self {
        Blob { data }
    }
}
