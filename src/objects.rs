use std::io::SeekFrom;

use deku::prelude::*;

use crate::{header::MIN_HEADER_SIZE, reader::AsyncFileRead};

pub use self::data::*;
pub use self::entry::*;
pub use self::entry_array::*;
pub use self::field::*;
pub use self::header::*;
pub use self::tag::*;

mod data;
mod entry;
mod entry_array;
mod field;
mod header;
mod tag;

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
		debug_assert!(
			offset >= MIN_HEADER_SIZE as u64,
			"small seek protection! ({offset})"
		);
		io.seek(SeekFrom::Start(offset)).await?;
		Self::read(io).await
	}
}
