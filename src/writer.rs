pub use file_write::AsyncFileWrite;
pub use options::CreateOptions;

mod file_write;
mod options;

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
