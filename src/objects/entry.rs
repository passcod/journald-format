use std::num::{NonZeroU128, NonZeroU32, NonZeroU64};

use deku::prelude::*;
use futures_util::Stream;
use jiff::Timestamp;

use crate::{header::Header, monotonic::Monotonic, reader::AsyncFileRead};

use super::{Data, ObjectHeader, ObjectType, SimpleRead, OBJECT_HEADER_SIZE};

#[derive(Debug, Clone, PartialEq, Eq, DekuRead, DekuWrite)]
#[deku(endian = "little")]
pub struct EntryObjectHeader {
	pub seqnum: NonZeroU64,

	#[deku(
		reader = "crate::deku_helpers::reader_realtime(deku::reader)",
		writer = "crate::deku_helpers::writer_realtime(deku::writer, &self.realtime)"
	)]
	pub realtime: Timestamp,

	pub monotonic: Monotonic,
	pub boot_id: NonZeroU128,
	pub xor_hash: u64,
}

pub const ENTRY_OBJECT_HEADER_SIZE: u64 = 48;

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
		tracing::trace!(?offset, "reading object header");
		let object = ObjectHeader::read_at(io, offset)
			.await?
			.check_type(ObjectType::Entry)?;
		tracing::trace!(?object, "read object header");

		tracing::trace!(?offset, "reading entry header");
		let header_offset = offset + OBJECT_HEADER_SIZE;
		let header = EntryObjectHeader::read_at(io, header_offset).await?;
		tracing::trace!(?header, "read entry header");

		let array_offset = header_offset + ENTRY_OBJECT_HEADER_SIZE;
		let array_size = object.payload_size() - ENTRY_OBJECT_HEADER_SIZE;

		let size = file_header.sizeof_entry_object_item();
		let capacity = array_size / size;
		tracing::trace!(
			?size,
			?capacity,
			?array_offset,
			?array_size,
			"initialising object vec"
		);
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

	#[tracing::instrument(level = "trace", skip(self, io, file_header))]
	pub(crate) fn data<'io, R: AsyncFileRead + Unpin>(
		&'io self,
		io: &'io mut R,
		file_header: &'io Header,
	) -> impl Stream<Item = std::io::Result<Data>> + Unpin + 'io
	where
		Self: Sized,
	{
		Box::pin(async_stream::try_stream! {
			let is_compact = file_header.is_compact();
			for offset in &self.objects {
				yield Data::read_at(io, offset.get().into(), is_compact).await?;
			}
		})
	}
}
