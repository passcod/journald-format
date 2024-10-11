use std::num::NonZeroU64;

use bstr::BString;
use deku::prelude::*;

use crate::{
	objects::{DataCompression, ObjectHeader, ObjectType, OBJECT_HEADER_SIZE},
	reader::AsyncFileRead,
};

use super::SimpleRead;

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

pub const DATA_OBJECT_HEADER_SIZE: u64 = std::mem::size_of::<DataObjectHeader>() as _;
const _: [(); DATA_OBJECT_HEADER_SIZE as _] = [(); 48];

impl SimpleRead for DataObjectHeader {}

#[derive(Debug, Clone, PartialEq, Eq, DekuRead, DekuWrite)]
#[deku(endian = "little")]
pub struct DataObjectCompactPayloadHeader {
	pub tail_entry_array_offset: u32,
	pub tail_entry_array_n_entries: u32,
}

pub const DATA_OBJECT_COMPACT_PAYLOAD_HEADER_SIZE: u64 =
	std::mem::size_of::<DataObjectCompactPayloadHeader>() as _;
const _: [(); DATA_OBJECT_COMPACT_PAYLOAD_HEADER_SIZE as _] = [(); 8];

impl SimpleRead for DataObjectCompactPayloadHeader {}

#[derive(Debug, Clone, PartialEq, Eq, DekuRead, DekuWrite)]
struct DataPayload {
	#[deku(
		until = "|v: &u8| *v == b'='",
		map = "|mut field: Vec<u8>| -> Result<_, DekuError> { Ok({ field.pop(); field }) }"
	)]
	pub key: Vec<u8>,

	#[deku(read_all)]
	pub value: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Data {
	pub offset: NonZeroU64,
	pub header: DataObjectHeader,
	pub entry_array: Option<DataObjectCompactPayloadHeader>,
	pub key: BString,
	pub value: BString,
}

impl Data {
	#[tracing::instrument(level = "trace", skip(io))]
	pub(crate) async fn read_at<R: AsyncFileRead + Unpin>(
		io: &mut R,
		offset: u64,
		is_compact: bool,
	) -> std::io::Result<Self>
	where
		Self: Sized,
	{
		tracing::trace!(?offset, "reading object header");
		let object = ObjectHeader::read_at(io, offset)
			.await?
			.check_type(ObjectType::Data)?;
		tracing::trace!(?object, "read object header");

		assert_eq!(
			object.compression,
			DataCompression::None,
			"TODO: uncompress"
		);

		let header_offset = offset + OBJECT_HEADER_SIZE;
		tracing::trace!(offset=?header_offset, "reading data header");
		let header = DataObjectHeader::read_at(io, header_offset).await?;
		tracing::trace!(?header, "read data header");

		let (payload_rel_offset, entry_array) = if is_compact {
			let compact_header_offset = header_offset + DATA_OBJECT_COMPACT_PAYLOAD_HEADER_SIZE;
			tracing::trace!(offset=?compact_header_offset, "reading compact data header");
			let compact_header =
				DataObjectCompactPayloadHeader::read_at(io, compact_header_offset).await?;
			tracing::trace!(?compact_header, "read compact data header");
			(
				OBJECT_HEADER_SIZE
					+ DATA_OBJECT_HEADER_SIZE
					+ DATA_OBJECT_COMPACT_PAYLOAD_HEADER_SIZE,
				Some(compact_header),
			)
		} else {
			(OBJECT_HEADER_SIZE + DATA_OBJECT_HEADER_SIZE, None)
		};

		let payload_offset = offset + payload_rel_offset;
		let payload_size = object.payload_size() - payload_rel_offset + OBJECT_HEADER_SIZE;
		tracing::trace!(offset=?payload_offset, size=?payload_size, "reading payload");
		let payload = io.read_some_at(payload_offset, payload_size as _).await?;
		tracing::trace!(?payload, "read payload");
		let payload = DataPayload::from_bytes((&payload, 0))
			.map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
			.map(|(_, d)| d)?;
		tracing::trace!(?payload, "parsed payload");

		Ok(Self {
			offset: offset.try_into().unwrap(),
			header,
			entry_array,
			key: BString::new(payload.key),
			value: BString::new(payload.value),
		})
	}
}
