use std::path::Path;

use futures_io::{AsyncRead, AsyncSeek};

pub trait AsyncFileRead: AsyncRead + AsyncSeek + Unpin {
	/// Open a file for reading.
	///
	/// This should close the current file (if any).
	fn open(
		&mut self,
		filename: &Path,
	) -> impl std::future::Future<Output = std::io::Result<()>> + Send;

	/// The path to the current file, if one is open.
	fn current(&self) -> Option<&Path>;
}

pub struct JournalReader<T> {
	io: T,
}

impl<T> std::fmt::Debug for JournalReader<T> {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("JournalReader")
			.field("io", &std::any::type_name::<T>())
			.finish()
	}
}

impl<T> JournalReader<T>
where
	T: AsyncFileRead,
{
	pub fn new(io: T) -> Self {
		Self { io }
	}

	/// Verify all data in the current journal file.
	///
	/// This will check every hash, every sealing tag, and every entry. It
	/// should be used to detect tampering; when reading the journal normally,
	/// only the data that is actually read is verified.
	pub async fn verify(&mut self) -> std::io::Result<bool> {
		todo!()
	}

	/// Verify all data in the entire journal.
	pub async fn verify_all(&mut self) -> std::io::Result<bool> {
		todo!()
	}

	/// Read entries from the current position.
	pub async fn entries(&mut self) -> std::io::Result<Vec<()>> {
		// TODO: return Stream
		todo!()
	}

	/// Seek to a timestamp, or as close as possible.
	pub async fn seek_to_timestamp(&mut self, _timestamp: u64) -> std::io::Result<()> {
		todo!()
	}

	/// Seek to a sequence number, or as close as possible.
	pub async fn seek_to_seqnum(&mut self, _seqnum: u64) -> std::io::Result<()> {
		todo!()
	}

	/// Seek to the start of a boot ID.
	pub async fn seek_to_boot_id(&mut self, _boot_id: u128) -> std::io::Result<()> {
		todo!()
	}
}
