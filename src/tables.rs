use std::num::NonZeroU64;

use deku::prelude::*;
use futures_util::{Stream, StreamExt as _};

use crate::reader::AsyncFileRead;

// used for both data and field hash tables
// the hash table is an array of these
// key is hash % size of the hash table

/// Hash table item.
#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
#[deku(endian = "little")]
pub struct HashItem {
	pub head_hash_offset: Option<NonZeroU64>,
	pub tail_hash_offset: Option<NonZeroU64>,
}

pub const HASH_ITEM_SIZE: usize = 16;

/// Hash table abstraction.
pub struct HashTable<'h> {
	pub(crate) offset: NonZeroU64,
	pub(crate) size: NonZeroU64,
	pub(crate) _phantom: std::marker::PhantomData<&'h ()>,
}

impl<'h> HashTable<'h> {
	/// Number of item slots in the hash table.
	#[tracing::instrument(level = "trace", skip(self))]
	pub fn capacity(&self) -> u64 {
		self.size.get() / HASH_ITEM_SIZE as u64
	}

	/// Iterate over all items in the hash table.
	#[tracing::instrument(level = "trace", skip(self, io))]
	pub fn items<'io: 'h, R: AsyncFileRead + Unpin>(
		&'h self,
		io: &'io mut R,
	) -> impl Stream<Item = std::io::Result<HashItem>> + Unpin + 'h {
		Box::pin(async_stream::try_stream! {
			let mut offset = self.offset.get();
			let end = self.offset.get() + self.size.get();
			while offset < end {
				let item = io.read_some_at(offset, HASH_ITEM_SIZE).await?;
				offset += HASH_ITEM_SIZE as u64;
				let (_, item) = HashItem::from_bytes((&item, 0))?;
				yield item;
			}
		})
	}

	/// Count the number of items in the hash table.
	///
	/// This is computed by reading the entire hash table, and ignores errors.
	#[tracing::instrument(level = "trace", skip(self, io))]
	pub async fn count<R: AsyncFileRead + Unpin>(&self, io: &mut R) -> u64 {
		let stream = self.items(io);
		stream.count().await as _
	}

	/// How full the hash table is.
	///
	/// This is computed by reading the entire hash table, for performance prefer to use
	/// [`Header::data_fill_level`](crate::header::Header::data_fill_level) or
	/// [`Header::field_fill_level`](crate::header::Header::field_fill_level) instead.
	#[tracing::instrument(level = "trace", skip(self, io))]
	pub async fn fill_level<R: AsyncFileRead + Unpin>(&self, io: &mut R) -> f64 {
		self.count(io).await as f64 / self.capacity() as f64
	}
}
