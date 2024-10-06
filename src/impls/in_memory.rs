use std::path::Path;

use futures_util::{io::Cursor, Stream};

use crate::reader::{AsyncFileRead, FilenameInfo};

impl AsyncFileRead for Cursor<&[u8]> {
	fn open(
		&mut self,
		_filename: &Path,
	) -> impl std::future::Future<Output = std::io::Result<()>> + Send {
		async move { Ok(()) }
	}

	fn close(&mut self) -> impl std::future::Future<Output = ()> + Send {
		async move {}
	}

	fn current(&self) -> Option<&Path> {
		None
	}

	fn list_files(
		&self,
		_prefix: Option<&Path>,
	) -> impl Stream<Item = std::io::Result<FilenameInfo>> {
		futures_util::stream::empty()
	}
}
