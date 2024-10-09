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

use crate::reader::{AsyncFileRead, FilenameInfo};

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
	#[tracing::instrument(level = "trace", skip(self))]
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
				let _ = Pin::new(&mut open.file).poll_complete(cx);
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
				let mut buf = ReadBuf::new(buf);
				match Pin::new(&mut open.file).poll_read(cx, &mut buf) {
					Poll::Ready(Ok(())) => Poll::Ready(Ok(buf.filled().len())),
					Poll::Ready(Err(err)) => Poll::Ready(Err(err)),
					Poll::Pending => Poll::Pending,
				}
			},
		)
	}
}

#[test]
fn test_parse_filename_latest() {
	assert_eq!(
		JournalOnDisk::parse_filename(Path::new(
			"/var/log/journal/c444c71c038d45b0af201444a83b91c9/system.journal"
		)),
		Some(FilenameInfo::Latest {
			machine_id: 0xc444c71c038d45b0af201444a83b91c9,
			scope: "system".into()
		})
	);
}

#[test]
fn test_parse_filename_archived() {
	use jiff::Timestamp;
	use std::num::{NonZeroU128, NonZeroU64};

	assert_eq!(
		JournalOnDisk::parse_filename(Path::new(
			"/var/log/journal/c444c71c038d45b0af201444a83b91c9/system@ae257a224b70405a9042a99aef057ce0-00000000002d5994-00062368053e1184.journal"
		)),
		Some(FilenameInfo::Archived {
			machine_id: 0xc444c71c038d45b0af201444a83b91c9,
			scope: "system".into(),
			file_seqnum: NonZeroU128::new(0xae257a224b70405a9042a99aef057ce0).unwrap(),
			head_seqnum: NonZeroU64::new(0x00000000002d5994).unwrap(),
			head_realtime: Timestamp::from_microsecond(0x00062368053e1184).unwrap()
		})
	);
}

#[test]
fn test_make_filename_latest() {
	assert_eq!(
		JournalOnDisk::make_filename(&FilenameInfo::Latest {
			machine_id: 0xc444c71c038d45b0af201444a83b91c9,
			scope: "system".into()
		}),
		PathBuf::from("c444c71c038d45b0af201444a83b91c9/system.journal"),
	);
}

#[test]
fn test_make_filename_archived() {
	use jiff::Timestamp;
	use std::num::{NonZeroU128, NonZeroU64};

	assert_eq!(
		JournalOnDisk::make_filename(&FilenameInfo::Archived {
			machine_id: 0xc444c71c038d45b0af201444a83b91c9,
			scope: "system".into(),
			file_seqnum: NonZeroU128::new(0xae257a224b70405a9042a99aef057ce0).unwrap(),
			head_seqnum: NonZeroU64::new(0x00000000002d5994).unwrap(),
			head_realtime: Timestamp::from_microsecond(0x00062368053e1184).unwrap()
		}),
		PathBuf::from(
			"c444c71c038d45b0af201444a83b91c9/system@ae257a224b70405a9042a99aef057ce0-00000000002d5994-00062368053e1184.journal"
		),
	);
}

#[test]
fn test_make_prefix() {
	assert_eq!(
		JournalOnDisk::make_prefix(&crate::reader::JournalSelection {
			machine_id: 0xc444c71c038d45b0af201444a83b91c9,
			scope: "system".into()
		}),
		PathBuf::from("c444c71c038d45b0af201444a83b91c9/system@"),
	);
}
