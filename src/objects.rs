use std::{io::SeekFrom, num::NonZeroU64};

use deku::prelude::*;

use crate::reader::AsyncFileRead;

pub(crate) trait SimpleRead: for<'a> DekuContainerRead<'a> {
	async fn read<R: AsyncFileRead + Unpin>(io: &mut R) -> std::io::Result<Self>
	where
		Self: Sized,
	{
		let data = io.read_some(std::mem::size_of::<Self>()).await?;
		Self::from_bytes((&data, 0))
			.map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
			.map(|(_, d)| d)
	}

	async fn read_at<R: AsyncFileRead + Unpin>(io: &mut R, offset: u64) -> std::io::Result<Self>
	where
		Self: Sized,
	{
		io.seek(SeekFrom::Start(offset)).await?;
		Self::read(io).await
	}
}

#[derive(Debug, Clone, PartialEq, Eq, DekuRead, DekuWrite)]
#[deku(id_type = "u8", endian = "endian", ctx = "endian: deku::ctx::Endian")]
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
		assert = "*compression != DataCompression::None || *r#type == ObjectType::Data"
	)]
	pub compression: DataCompression,

	pub size: u64,
}

pub const OBJECT_HEADER_SIZE: usize = std::mem::size_of::<ObjectHeader>();
const _: [(); OBJECT_HEADER_SIZE] = [(); 16];

impl SimpleRead for ObjectHeader {}

impl ObjectHeader {
	pub const fn payload_size(&self) -> u64 {
		self.size.saturating_sub(OBJECT_HEADER_SIZE as _)
	}
}

#[derive(Debug, Clone, PartialEq, Eq, DekuRead, DekuWrite)]
#[deku(endian = "little")]
pub struct DataObjectHeader {
	pub hash: u64,
	pub next_hash_offset: u64,
	pub next_field_offset: u64,
	pub entry_offset: u64,
	pub entry_array_offset: u64,
	pub n_entries: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, DekuRead, DekuWrite)]
#[deku(endian = "little")]
pub struct DataObjectCompactPayloadHeader {
	pub tail_entry_array_offset: u32,
	pub tail_entry_array_n_entries: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, DekuRead, DekuWrite)]
#[deku(endian = "little")]
pub struct FieldObjectHeader {
	pub hash: u64,
	pub next_hash_offset: u64,
	pub next_data_offset: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, DekuRead, DekuWrite)]
#[deku(endian = "little")]
pub struct EntryObjectHeader {
	pub seqnum: NonZeroU64,
	pub realtime: u64,
	pub monotonic: u64,
	pub boot_id: u128,
	pub xor_hash: u64,
}

#[derive(Debug, PartialEq, Eq, DekuRead, DekuWrite)]
#[deku(endian = "little")]
pub struct EntryObjectCompactItem {
	pub object_offset: u32,
}

#[derive(Debug, PartialEq, Eq, DekuRead, DekuWrite)]
#[deku(endian = "little")]
pub struct EntryObjectRegularItem {
	pub object_offset: u64,
	pub hash: u64,
}

#[derive(Debug, PartialEq, Eq, DekuRead, DekuWrite)]
#[deku(endian = "little")]
pub struct EntryArrayObjectHeader {
	pub next_entry_array_offset: Option<NonZeroU64>,
}

impl SimpleRead for EntryArrayObjectHeader {}

#[derive(Debug, Clone, PartialEq, Eq, DekuRead, DekuWrite)]
#[deku(endian = "little")]
pub struct EntryArrayRegularItem {
	pub offset: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, DekuRead, DekuWrite)]
#[deku(endian = "little")]
pub struct EntryArrayCompactItem {
	pub offset: u32,
}

pub const TAG_LENGTH: usize = 256 / 8;

#[derive(Debug, Clone, PartialEq, Eq, DekuRead, DekuWrite)]
#[deku(endian = "little")]
pub struct TagObjectHeader {
	pub seqnum: NonZeroU64,
	pub epoch: u64,
	pub tag: [u8; TAG_LENGTH],
}
