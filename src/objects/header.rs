use deku::prelude::*;

use super::SimpleRead;

#[derive(Debug, Clone, PartialEq, Eq, DekuRead, DekuWrite)]
#[deku(id_type = "u8", endian = "endian", ctx = "endian: deku::ctx::Endian")]
pub enum ObjectType {
	/// Encapsulates the contents of one field of an entry, i.e. a string such
	/// as `_SYSTEMD_UNIT=avahi-daemon.service`, or `MESSAGE=Foo had a booboo`.
	#[deku(id = "1")]
	Data,

	/// Encapsulates a field name, i.e. a string such as `_SYSTEMD_UNIT` or
	/// `MESSAGE`, without any `=` or even value.
	#[deku(id = "2")]
	Field,

	/// Binds several `Data` objects together into a log entry.
	#[deku(id = "3")]
	Entry,

	/// Encapsulates a hash table for finding existing `Data` objects.
	#[deku(id = "4")]
	DataHashTable,

	/// Encapsulates a hash table for finding existing `Field` objects.
	#[deku(id = "5")]
	FieldHashTable,

	/// Encapsulates a sorted array of offsets to entries, used for seeking by
	/// binary search.
	#[deku(id = "6")]
	EntryArray,

	/// Consists of a Forward Secure Sealing tag for all data from the beginning
	/// of the file or the last tag written (whichever is later).
	#[deku(id = "7")]
	Tag,

	/// Unknown objects are skipped.
	#[deku(id_pat = "_")]
	Unknown(u8),
}

/// Compression algorithm used for a Data object.
#[derive(Debug, Clone, PartialEq, Eq, DekuRead, DekuWrite)]
#[deku(id_type = "u8", endian = "endian", ctx = "endian: deku::ctx::Endian")]
#[repr(u8)]
#[rustfmt::skip]
pub enum DataCompression {
	/// No compression.
	None = 0b000,

	/// The object is compressed with XZ.
	Xz   = 0b__1,

	/// The object is compressed with LZ4.
	Lz4  = 0b_10,

	/// The object is compressed with Zstd.
	Zstd = 0b100,
}

#[derive(Debug, Clone, PartialEq, Eq, DekuRead, DekuWrite)]
#[deku(endian = "little")]
pub struct ObjectHeader {
	pub r#type: ObjectType,

	#[deku(
		pad_bytes_after = "6",
		// assert = "*compression != DataCompression::None || *r#type == ObjectType::Data"
	)]
	pub compression: DataCompression,

	pub size: u64,
}

pub const OBJECT_HEADER_SIZE: u64 = std::mem::size_of::<ObjectHeader>() as _;
const _: [(); OBJECT_HEADER_SIZE as _] = [(); 16];

impl SimpleRead for ObjectHeader {}

impl ObjectHeader {
	pub const fn payload_size(&self) -> u64 {
		self.size.saturating_sub(OBJECT_HEADER_SIZE as _)
	}

	pub fn check_type(self, check: ObjectType) -> std::io::Result<Self> {
		if self.r#type != check {
			Err(std::io::Error::new(
				std::io::ErrorKind::InvalidData,
				format!("expected object of type {check:?}, found {:?}", self.r#type),
			))
		} else {
			Ok(self)
		}
	}
}
