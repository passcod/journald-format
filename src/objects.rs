use std::num::NonZeroU64;

use deku::prelude::*;

#[derive(Debug, PartialEq, Eq, DekuRead, DekuWrite)]
#[deku(id_type = "u8")]
pub enum ObjectType {
	/// Encapsulates the contents of one field of an entry, i.e. a string such
	/// as `_SYSTEMD_UNIT=avahi-daemon.service`, or `MESSAGE=Foo had a booboo`.
	Data = 1,

	/// Encapsulates a field name, i.e. a string such as `_SYSTEMD_UNIT` or
	/// `MESSAGE`, without any `=` or even value.
	Field,

	/// Binds several `Data` objects together into a log entry.
	Entry,

	/// Encapsulates a hash table for finding existing `Data` objects.
	DataHashTable,

	/// Encapsulates a hash table for finding existing `Field` objects.
	FieldHashTable,

	/// Encapsulates a sorted array of offsets to entries, used for seeking by
	/// binary search.
	EntryArray,

	/// Consists of a Forward Secure Sealing tag for all data from the beginning
	/// of the file or the last tag written (whichever is later).
	Tag,
}

/// Compression algorithm used for a Data object.
#[derive(Debug, PartialEq, Eq, DekuRead, DekuWrite)]
#[deku(id_type = "u8")]
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

#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
pub struct ObjectHeader {
	pub r#type: ObjectType,

	#[deku(
		pad_bytes_after = "6",
		assert = "*compression != DataCompression::None || *r#type == ObjectType::Data"
	)]
	pub compression: DataCompression,

	#[deku(endian = "little")]
	pub size: u64,
}

impl ObjectHeader {
	pub const fn payload_size(&self) -> u64 {
		self.size
			.saturating_sub(std::mem::size_of::<ObjectHeader>() as _)
	}
}

#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
#[deku(endian = "little")]
pub struct DataObjectHeader {
	pub hash: u64,
	pub next_hash_offset: u64,
	pub next_field_offset: u64,
	pub entry_offset: u64,
	pub entry_array_offset: u64,
	pub n_entries: u64,
}

#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
#[deku(endian = "little")]
pub struct DataObjectCompactPayloadHeader {
	pub tail_entry_array_offset: u32,
	pub tail_entry_array_n_entries: u32,
}

#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
#[deku(endian = "little")]
pub struct FieldObjectHeader {
	pub hash: u64,
	pub next_hash_offset: u64,
	pub next_data_offset: u64,
}

#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
#[deku(endian = "little")]
pub struct EntryObjectHeader {
	pub seqnum: NonZeroU64,
	pub realtime: u64,
	pub monotonic: u64,
	pub boot_id: u128,
	pub xor_hash: u64,
}

#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
#[deku(endian = "little")]
pub struct EntryObjectCompactItem {
	pub object_offset: u32,
}

#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
#[deku(endian = "little")]
pub struct EntryObjectRegularItem {
	pub object_offset: u64,
	pub hash: u64,
}

#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
#[deku(endian = "little")]
pub struct EntryArrayObjectHeader {
	pub next_entry_array_offset: Option<NonZeroU64>,
}

#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
#[deku(endian = "little")]
pub struct EntryArrayRegularItem {
	pub offset: u64,
}

#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
#[deku(endian = "little")]
pub struct EntryArrayCompactItem {
	pub offset: u32,
}

pub const TAG_LENGTH: usize = 256 / 8;

#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
#[deku(endian = "little")]
pub struct TagObjectHeader {
	pub seqnum: NonZeroU64,
	pub epoch: u64,
	pub tag: [u8; TAG_LENGTH],
}
