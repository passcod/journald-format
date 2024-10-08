use std::{
	io,
	path::{Path, PathBuf},
	pin::Pin,
	task::Poll,
};

use async_stream::try_stream;
use futures_io::{AsyncRead, AsyncSeek};
use futures_util::{io::Cursor, Stream};
use tokio::fs;

use crate::reader::{AsyncFileRead, FilenameInfo};

struct OpenFile {
	path: PathBuf,
	file: Cursor<Vec<u8>>,
}

pub struct ReadWholeFile {
	root: PathBuf,
	open: Option<OpenFile>,
}

impl ReadWholeFile {
	pub fn new(root: PathBuf) -> Self {
		Self { root, open: None }
	}
}

impl AsyncFileRead for ReadWholeFile {
	#[tracing::instrument(level = "trace", skip(self))]
	fn open(
		&mut self,
		filename: &Path,
	) -> impl std::future::Future<Output = io::Result<()>> + Send {
		async move {
			let path = self.root.join(filename);
			let file = Cursor::new(fs::read(&path).await?);
			self.open = Some(OpenFile { path, file });
			Ok(())
		}
	}

	#[tracing::instrument(level = "trace", skip(self))]
	fn close(&mut self) -> impl std::future::Future<Output = ()> + Send {
		async move {
			self.open = None;
		}
	}

	#[tracing::instrument(level = "trace", skip(self))]
	fn current(&self) -> Option<&Path> {
		self.open.as_ref().map(|file| file.path.as_ref())
	}

	#[tracing::instrument(level = "trace", skip(self))]
	fn list_files(
		&self,
		prefix: Option<&Path>,
	) -> impl Stream<Item = io::Result<FilenameInfo>> + Unpin {
		Box::pin(try_stream! {
			let root = match prefix {
				Some(prefix) => self.root.join(prefix.parent().unwrap_or(prefix)),
				None => self.root.clone(),
			};

			let mut todo = vec![root.clone()];

			loop {
				let Some(current) = todo.pop() else {
					break;
				};

				let mut read_dir = tokio::fs::read_dir(&current).await?;
				while let Some(entry) = read_dir.next_entry().await? {
					let file_type = entry.file_type().await?;
					if file_type.is_dir() {
						todo.push(entry.path());
					} else if file_type.is_file()
						&& entry
							.path()
							.to_string_lossy()
							.starts_with(root.to_string_lossy().as_ref())
					{
						if let Some(file) = Self::parse_filename(&entry.path()) {
							yield file;
						}
					}
				}
			}
		})
	}
}

impl AsyncSeek for ReadWholeFile {
	fn poll_seek(
		mut self: Pin<&mut Self>,
		cx: &mut std::task::Context<'_>,
		pos: io::SeekFrom,
	) -> Poll<io::Result<u64>> {

		self.open.as_mut().map_or_else(
			|| {
				Poll::Ready(Err(io::Error::new(
					io::ErrorKind::NotConnected,
					"no file open",
				)))
			},
			|open| {
				Pin::new(&mut open.file).poll_seek(cx, pos)
			},
		)
	}
}

impl AsyncRead for ReadWholeFile {
	fn poll_read(
		mut self: Pin<&mut Self>,
		cx: &mut std::task::Context<'_>,
		buf: &mut [u8],
	) -> Poll<io::Result<usize>> {
		self.open.as_mut().map_or_else(
			|| {
				Poll::Ready(Err(io::Error::new(
					io::ErrorKind::NotConnected,
					"no file open",
				)))
			},
			|open| {
				Pin::new(&mut open.file).poll_read(cx, buf)
			},
		)
	}
}
