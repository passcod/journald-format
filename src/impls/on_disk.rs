use std::{
	io,
	path::{Path, PathBuf},
	pin::Pin,
	task::Poll,
};

use async_stream::try_stream;
use futures_io::{AsyncRead, AsyncSeek};
use futures_util::Stream;
use tokio::{fs::File, io::ReadBuf};

use crate::reader::AsyncFileRead;

struct OpenFile {
	path: PathBuf,
	file: File,
}

pub struct JournalOnDisk {
	root: PathBuf,
	open: Option<OpenFile>,
}

impl JournalOnDisk {
	pub fn new(root: PathBuf) -> Self {
		Self { root, open: None }
	}
}

impl AsyncFileRead for JournalOnDisk {
	fn open(
		&mut self,
		filename: &Path,
	) -> impl std::future::Future<Output = io::Result<()>> + Send {
		async move {
			let path = self.root.join(filename);
			let file = File::open(&path).await?;
			self.open = Some(OpenFile { path, file });
			Ok(())
		}
	}

	fn close(&mut self) -> impl std::future::Future<Output = ()> + Send {
		async move {
			self.open = None;
		}
	}

	fn current(&self) -> Option<&Path> {
		self.open.as_ref().map(|file| file.path.as_ref())
	}

	fn list_files(&self, prefix: Option<&Path>) -> impl Stream<Item = io::Result<PathBuf>> + Unpin {
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
						&& JournalOnDisk::parse_filename(&entry.path()).is_some()
					{
						yield entry.path();
					}
				}
			}
		})
	}
}

impl AsyncSeek for JournalOnDisk {
	fn poll_seek(
		mut self: Pin<&mut Self>,
		cx: &mut std::task::Context<'_>,
		pos: io::SeekFrom,
	) -> Poll<io::Result<u64>> {
		use tokio::io::AsyncSeek as _;

		self.open.as_mut().map_or_else(
			|| {
				Poll::Ready(Err(io::Error::new(
					io::ErrorKind::NotConnected,
					"no file open",
				)))
			},
			|open| {
				if let Err(err) = Pin::new(&mut open.file).start_seek(pos) {
					return Poll::Ready(Err(err));
				}

				Pin::new(&mut open.file).poll_complete(cx)
			},
		)
	}
}

impl AsyncRead for JournalOnDisk {
	fn poll_read(
		mut self: Pin<&mut Self>,
		cx: &mut std::task::Context<'_>,
		buf: &mut [u8],
	) -> Poll<io::Result<usize>> {
		use tokio::io::AsyncRead as _;

		self.open.as_mut().map_or_else(
			|| {
				Poll::Ready(Err(io::Error::new(
					io::ErrorKind::NotConnected,
					"no file open",
				)))
			},
			|open| {
				let pre = buf.len();
				match Pin::new(&mut open.file).poll_read(cx, &mut ReadBuf::new(buf)) {
					Poll::Ready(Ok(())) => Poll::Ready(Ok(buf.len().saturating_sub(pre))),
					Poll::Ready(Err(err)) => Poll::Ready(Err(err)),
					Poll::Pending => Poll::Pending,
				}
			},
		)
	}
}
