use std::collections::HashSet;

pub use file_read::{AsyncFileRead, FilenameInfo};
use futures_util::{Stream, StreamExt as _};

mod file_read;

// pub(crate) const READ_SIZE: usize = 4096;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct JournalSelection {
	pub machine_id: u128,
	pub scope: String,
}

impl From<FilenameInfo> for JournalSelection {
	fn from(info: FilenameInfo) -> Self {
		Self {
			machine_id: info.machine_id,
			scope: info.scope,
		}
	}
}

pub struct JournalReader<T> {
	io: T,
	select: Option<JournalSelection>,
}

impl<T> std::fmt::Debug for JournalReader<T> {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("JournalReader")
			.field("io", &std::any::type_name::<T>())
			.field("select", &self.select)
			.finish()
	}
}

impl<T> JournalReader<T>
where
	T: AsyncFileRead,
{
	/// Initialize a new journal reader.
	pub fn new(io: T) -> Self {
		Self { io, select: None }
	}

	/// List all available journals (machine ID, scope).
	pub async fn list(&self) -> std::io::Result<HashSet<JournalSelection>> {
		let mut set = HashSet::new();
		let mut files = self.io.list_files(None);
		while let Some(file) = files.next().await {
			let file = file?;
			if let Some(info) = T::parse_filename(&file) {
				set.insert(info.into());
			}
		}

		Ok(set)
	}

	/// Get the current journal selection.
	pub fn selection(&self) -> Option<&JournalSelection> {
		self.select.as_ref()
	}

	/// Select a journal to read from.
	///
	/// This invalidates the current position.
	pub fn select(&mut self, journal: JournalSelection) {
		self.select = Some(journal);
	}

	/// Read entries from the current position.
	pub async fn entries(
		&mut self,
	) -> std::io::Result<impl Stream<Item = std::io::Result<()>> + Unpin> {
		Ok(futures_util::stream::empty(/* TODO */))
	}

	/// Seek to the end of the journal.
	///
	/// Reading entries from here will output nothing and block until new entries are written.
	pub async fn seek_to_newest(&mut self, _scope: &str) -> std::io::Result<()> {
		todo!()
	}

	/// Seek to the start of the journal.
	///
	/// Reading entries from here will output the entire journal.
	pub async fn seek_to_oldest(&mut self, _scope: &str) -> std::io::Result<()> {
		todo!()
	}

	/// Seek to a timestamp, or as close as possible.
	pub async fn seek_to_timestamp(
		&mut self,
		_scope: &str,
		_timestamp: u64,
	) -> std::io::Result<()> {
		todo!()
	}

	/// Seek to a sequence number, or as close as possible.
	pub async fn seek_to_seqnum(&mut self, _scope: &str, _seqnum: u64) -> std::io::Result<()> {
		todo!()
	}

	/// Seek to the start of a boot ID.
	pub async fn seek_to_boot_id(&mut self, _scope: &str, _boot_id: u128) -> std::io::Result<()> {
		todo!()
	}

	/// Verify all data in all available journals.
	///
	/// This will check every hash, every sealing tag, and every entry. It
	/// should be used to detect tampering; when reading the journal normally,
	/// only the data that is actually read is verified.
	pub async fn verify_all(&mut self) -> std::io::Result<bool> {
		todo!()
	}
}
