use std::path::{Path, PathBuf};

use futures_util::{io::Cursor, Stream};

use crate::reader::AsyncFileRead;

impl AsyncFileRead for Cursor<&[u8]> {
	fn open(
		&mut self,
		_filename: &Path,
	) -> impl std::future::Future<Output = std::io::Result<()>> + Send {
		async move { Ok(()) }
	}

	fn current(&self) -> Option<&Path> {
		None
	}

	fn list_files(&self, _prefix: Option<&Path>) -> impl Stream<Item = std::io::Result<PathBuf>> {
		futures_util::stream::empty()
	}
}
