use anyhow::{anyhow, Result};
use sha1::{Digest, Sha1};
use std::fs::{create_dir_all, rename, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, PartialEq, PartialOrd, Eq, Ord)]
pub struct ObjectID {
    id: Vec<u8>,
}

impl ObjectID {
    fn new(storable: &(impl Storable + ?Sized)) -> Self {
        let mut hasher = Sha1::new();
        hasher.update(storable.encoded_raw());
        ObjectID {
            id: hasher.finalize().as_mut_slice().to_vec(),
        }
    }

    /// Creates an ObjectID from a hexadecimal encoded string.
    pub fn from_str<S: AsRef<str>>(s: S) -> Result<Self> {
        let id = hex::decode(s.as_ref())?;
        if id.len() != sha1::Sha1::output_size() {
            return Err(anyhow!("Invalid ObjectID length"));
        }
        Ok(ObjectID { id })
    }

    /*
    /// Creates an ObjectID from a raw byte sequence.
    pub fn from_bytes(b: &[u8]) -> Result<Self> {
        if b.len() != sha1::Sha1::output_size() {
            return Err(anyhow!("Invalid ObjectID length"));
        }
        Ok(ObjectID { id: b.to_vec() })
    }
    */

    pub fn as_bytes(&self) -> &[u8] {
        self.id.as_slice()
    }

    pub fn as_str(&self) -> String {
        hex::encode(self.as_bytes())
    }
}

/// An entity which may be stored within the git object database.
pub trait Storable {
    // Identifies which object is serialized here.
    fn type_name(&self) -> &str;

    // Returns a byte representation of the underlying data.
    fn data(&self) -> &Vec<u8>;

    // TODO: Could be fancy, use types to enforce compression.

    // TODO: We re-invoked "encoded_raw" pretty frequently; might be
    // worth restructuring this code to reduce the number of invocations.

    fn encoded_raw(&self) -> Vec<u8> {
        let data = self.data();
        let mut content = Vec::new();
        content.extend_from_slice(self.type_name().as_bytes());
        content.extend_from_slice(b" ");
        content.extend_from_slice(format!("{}\0", data.len()).as_bytes());
        content.extend_from_slice(data);
        content
    }

    /// Serializes the object to a byte steam.
    fn serialize(&self) -> Result<Vec<u8>> {
        let content = self.encoded_raw();

        // Use zlib to compress the file so it uses less on-disk storage.
        let compression = flate2::Compression::fast();
        let mut encoder = flate2::write::ZlibEncoder::new(Vec::new(), compression);
        encoder.write_all(&content)?;
        let compressed = encoder.finish()?;

        Ok(compressed)
    }

    /// Returns the ID of the object.
    fn oid(&self) -> ObjectID {
        ObjectID::new(self)
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
            root: PathBuf::from(path.as_ref()),
        }
    }

    /// Add a new object to the Git object store.
    pub fn store(&self, object: &impl Storable) -> Result<()> {
        let content = object.serialize()?;
        let object_id = object.oid().as_str();
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
        // Someone else already made this object. Since the object store
        // *should* be immutable, this means it already has the content we want
        // too.
        if final_path.exists() {
            return Ok(());
        }

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
