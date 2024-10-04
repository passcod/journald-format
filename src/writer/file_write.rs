use std::path::Path;

use futures_io::AsyncWrite;

use crate::reader::AsyncFileRead;

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
