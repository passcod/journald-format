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
		match info {
			FilenameInfo::Latest { machine_id, scope } => Self { machine_id, scope },
			FilenameInfo::Archived {
				machine_id, scope, ..
			} => Self { machine_id, scope },
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
	/// If the journal does not exist, this will return an error and will also have unselected the
	/// current journal.
	///
	/// This invalidates the current position.
	pub async fn select(&mut self, journal: JournalSelection) -> std::io::Result<()> {
		self.io.close().await;
		self.select = None;

		let latest = T::make_filename(FilenameInfo::Latest {
			machine_id: journal.machine_id,
			scope: journal.scope.clone(),
		});
		if let Err(err) = self.io.open(&latest).await {
			if err.kind() != std::io::ErrorKind::NotFound {
				return Err(err);
			}

			// Latest does not exist, try to find an archived journal.
			let prefix = T::make_prefix(&journal);
			let file = {
				let mut files = self.io.list_files(Some(&prefix));
				let Some(file) = files.next().await else {
					return Err(std::io::Error::new(
						std::io::ErrorKind::NotFound,
						"journal not found",
					));
				};
				file?
			};
			self.io.open(&file).await?;
		}

		self.select = Some(journal);
		Ok(())
	}

	/// Seek to a position in the journal.
	pub async fn seek(&mut self, _seek: Seek) -> std::io::Result<()> {
		todo!()
	}

	/// Read entries from the current position.
	pub async fn entries(
		&mut self,
	) -> std::io::Result<impl Stream<Item = std::io::Result<()>> + Unpin> {
		Ok(futures_util::stream::empty(/* TODO */))
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

#[derive(Debug, Clone, Copy)]
pub enum Seek {
	/// Seek to just after the newest entry.
	Newest,

	/// Seek to just before the oldest entry.
	Oldest,

	/// Seek to the entry closest to the given timestamp.
	Timestamp(u64),

	/// Seek to the entry closest to the given sequence number.
	Seqnum(u64),

	/// Seek to the start of the given boot ID.
	BootId(u128),

	/// Seek to the given number of entries before or after the current position.
	Entries(i64),
}
