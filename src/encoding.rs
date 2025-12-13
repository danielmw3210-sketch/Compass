#![allow(dead_code)]
use std::io::{self, Write};

/// Trait for objects that have a canonical binary representation for Hashing/Signing.
/// careful: This must be deterministic across platforms/versions.
pub trait CanonicalSerialize {
    fn canonical_serialize<W: Write>(&self, writer: &mut W) -> io::Result<()>;
    
    fn to_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        self.canonical_serialize(&mut buf).expect("memory write failed");
        buf
    }
}

// --- Primitives ---

impl CanonicalSerialize for u8 {
    fn canonical_serialize<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        writer.write_all(&[*self])
    }
}

impl CanonicalSerialize for u64 {
    fn canonical_serialize<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        writer.write_all(&self.to_le_bytes())
    }
}

impl CanonicalSerialize for u128 {
    fn canonical_serialize<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        writer.write_all(&self.to_le_bytes())
    }
}

impl CanonicalSerialize for String {
    fn canonical_serialize<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        let bytes = self.as_bytes();
        let len = bytes.len() as u32;
        writer.write_all(&len.to_le_bytes())?;
        writer.write_all(bytes)
    }
}

impl CanonicalSerialize for bool {
    fn canonical_serialize<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        writer.write_all(&[if *self { 1u8 } else { 0u8 }])
    }
}

impl<T: CanonicalSerialize> CanonicalSerialize for Vec<T> {
    fn canonical_serialize<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        let len = self.len() as u32;
        writer.write_all(&len.to_le_bytes())?;
        for item in self {
            item.canonical_serialize(writer)?;
        }
        Ok(())
    }
}
