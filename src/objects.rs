use std::{
	io::SeekFrom,
	num::{NonZeroU32, NonZeroU64},
};

use deku::prelude::*;

use crate::{header::Header, reader::AsyncFileRead};

pub(crate) trait SimpleRead: for<'a> DekuContainerRead<'a> {
	#[tracing::instrument(level = "trace", skip(io))]
	async fn read<R: AsyncFileRead + Unpin>(io: &mut R) -> std::io::Result<Self>
	where
		Self: Sized,
	{
		let data = io.read_some(std::mem::size_of::<Self>()).await?;
		Self::from_bytes((&data, 0))
			.map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
			.map(|(_, d)| d)
	}

	#[tracing::instrument(level = "trace", skip(io))]
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

pub const ENTRY_OBJECT_HEADER_SIZE: usize = std::mem::size_of::<EntryObjectHeader>();
const _: [(); ENTRY_OBJECT_HEADER_SIZE] = [(); 48];

impl SimpleRead for EntryObjectHeader {}

#[derive(Debug, Clone, PartialEq, Eq, DekuRead, DekuWrite)]
#[deku(endian = "little")]
pub struct EntryObjectCompactItem {
	pub object_offset: u32,
}

impl SimpleRead for EntryObjectCompactItem {}

#[derive(Debug, Clone, PartialEq, Eq, DekuRead, DekuWrite)]
#[deku(endian = "little")]
pub struct EntryObjectRegularItem {
	pub object_offset: u64,
	pub hash: u64,
}

impl SimpleRead for EntryObjectRegularItem {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
	pub offset: NonZeroU64,
	pub header: EntryObjectHeader,
	pub objects: Vec<NonZeroU32>,
}

impl Entry {
	#[tracing::instrument(level = "trace", skip(io, file_header))]
	pub(crate) async fn read_at<R: AsyncFileRead + Unpin>(
		io: &mut R,
		offset: u64,
		file_header: &Header,
	) -> std::io::Result<Self>
	where
		Self: Sized,
	{
		let object = ObjectHeader::read_at(io, offset).await?;

		let header_offset = offset + OBJECT_HEADER_SIZE as u64;
		let header = EntryObjectHeader::read_at(io, header_offset).await?;

		let array_offset = header_offset + ENTRY_OBJECT_HEADER_SIZE as u64;
		let size = file_header.sizeof_entry_object_item();
		let capacity = object.payload_size() / size;
		let mut objects = Vec::with_capacity(capacity as _);
		for n in 0..capacity {
			let object_offset = if file_header.is_compact() {
				let item = EntryObjectCompactItem::read_at(io, array_offset + n * size).await?;
				item.object_offset
			} else {
				let item = EntryObjectRegularItem::read_at(io, array_offset + n * size).await?;
				u32::try_from(item.object_offset).map_err(|err| {
					std::io::Error::new(
						std::io::ErrorKind::InvalidData,
						format!("object offset of item {n} in EntryArray:{offset} is larger than u32: {err}")
					)
				})?
			};

			if let Some(object_offset) = NonZeroU32::new(object_offset) {
				objects.push(object_offset);
			} else {
				break;
			}
		}

		Ok(Self {
			// UNWRAP: offsets are always non-zero
			offset: NonZeroU64::new(offset).unwrap(),
			header,
			objects,
		})
	}
}

#[derive(Debug, Clone, PartialEq, Eq, DekuRead, DekuWrite)]
#[deku(endian = "little")]
pub struct EntryArrayObjectHeader {
	pub next_entry_array_offset: Option<NonZeroU64>,
}

pub const ENTRY_ARRAY_HEADER_SIZE: usize = std::mem::size_of::<EntryArrayObjectHeader>();
const _: [(); ENTRY_ARRAY_HEADER_SIZE] = [(); 8];

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
