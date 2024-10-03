use std::path::Path;

use futures_io::AsyncWrite;

use crate::reader::AsyncFileRead;
use self::options::CreateOptions;

pub mod options;

pub trait AsyncFileWrite: AsyncFileRead + AsyncWrite {
	/// Close the current file (if any) and open a new one for writing.
	fn rotate(
		&mut self,
		filename: &Path,
	) -> impl std::future::Future<Output = std::io::Result<()>> + Send;

	/// Whether the current file is writable.
	///
	/// `None` if no file is open.
	fn writeable(&self) -> Option<bool>;
}

pub struct JournalWriter<T> {
	options: CreateOptions,
	io: T,
	prepared: bool,
}

impl<T> std::fmt::Debug for JournalWriter<T> {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("JournalWriter")
			.field("options", &self.options)
			.field("io", &std::any::type_name::<T>())
			.field("prepared", &self.prepared)
			.finish()
	}
}

impl<T> JournalWriter<T>
where
	T: AsyncFileWrite,
{
	pub fn with_options(io: T, options: CreateOptions) -> Self {
		Self {
			options,
			io,
			prepared: false,
		}
	}

	/// Prepare the journal for writing.
	///
	/// This must be called before writing any entries. It will error if:
	/// - the journal is already open (e.g. by another process)
	/// - opening the journal file fails
	/// - reading the journal header fails
	/// - writing the journal status fails
	pub async fn prepare(&mut self) -> std::io::Result<()> {
		self.prepared = true;
		todo!()
	}

	/// Write an entry (a set of key-value items) to the journal.
	pub async fn write_entry(
		&mut self,
		_fields: impl Iterator<Item = (String, bstr::BString)>,
	) -> std::io::Result<()> {
		if !self.prepared {
			self.prepare().await?;
		}
		todo!()
	}

	/// Seal the journal.
	///
	/// This should be called at a regular interval to prevent tampering.
	pub async fn seal(&mut self) -> std::io::Result<()> {
		if !self.prepared {
			self.prepare().await?;
		}
		todo!()
	}
}
