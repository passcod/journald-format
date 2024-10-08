use std::{
	collections::BTreeSet,
	num::{NonZeroU128, NonZeroU64},
	path::{Path, PathBuf},
};

use futures_util::{
	io::{AsyncReadExt, AsyncSeekExt},
	Stream,
};
use jiff::Timestamp;

use crate::header::MIN_HEADER_SIZE;

use super::JournalSelection;

pub trait AsyncFileRead: AsyncReadExt + AsyncSeekExt + Unpin {
	/// Open a file for reading.
	///
	/// This may close the current file (if any).
	fn open(
		&mut self,
		filename: &Path,
	) -> impl std::future::Future<Output = std::io::Result<()>> + Send;

	/// Close the current file.
	fn close(&mut self) -> impl std::future::Future<Output = ()> + Send;

	/// The path to the current file, if one is open.
	fn current(&self) -> Option<&Path>;

	/// Recursively list all journal files available.
	///
	/// The optional prefix filters the results. If `None`, all files are listed.
	/// The prefix may have a partial filename as the last component.
	///
	/// The library will interpret every file returned as a journal, so you may want to filter by
	/// the `.journal` extension for the systemd on-disk file scheme. However, [`JournalReader`]
	/// does not itself check the extension, so you can implement custom storage schemes; possibly
	/// overwriting the default [`make_filename`](AsyncFileRead::make_filename) and
	/// [`parse_filename`](AsyncFileRead::parse_filename) associated functions.
	///
	/// Must ignore the `fss` file present when Forward Secure Sealing is enabled.
	///
	/// ```plain
	/// # list_files(None)
	/// alpha/system@a-b-c.journal
	/// alpha/system@d-e-f.journal
	/// beta/user-123@g-h-i.journal
	/// ```
	///
	/// ```plain
	/// # list_files(Some("dir/system@"))
	/// alpha/system@a-b-c.journal
	/// alpha/system@d-e-f.journal
	/// ```
	fn list_files(
		&self,
		prefix: Option<&Path>,
	) -> impl Stream<Item = std::io::Result<FilenameInfo>> + Unpin;

	/// List all journal files available, sorted lexicographically.
	///
	/// This is a convenience method that calls [`list_files`](AsyncFileRead::list_files) and sorts the results.
	///
	/// You may want to override this method if you have a more efficient way to list files in sorted order.
	///
	/// The order is:
	///
	/// ```plain
	/// alpha/system@a-b-c.journal
	/// alpha/system@d-e-f.journal
	/// alpha/system.journal
	/// ```
	#[tracing::instrument(level = "trace", skip(self))]
	fn list_files_sorted(
		&self,
		prefix: Option<&Path>,
	) -> impl Stream<Item = std::io::Result<FilenameInfo>> + Unpin {
		Box::pin(async_stream::try_stream! {
			use futures_util::stream::StreamExt;
			let mut sorted = BTreeSet::new();
			let mut files = self.list_files(prefix);
			while let Some(file) = files.next().await {
				sorted.insert(file?);
			}
			for file in sorted {
				yield file;
			}
		})
	}

	/// Make a journal filename.
	///
	/// In the systemd on-disk file scheme, this is either:
	///
	/// ```plain
	/// (machine_id)/(scope).journal
	/// (machine_id)/(scope)@(file_seqnum)-(head_seqnum)-(head_realtime).journal
	/// ```
	///
	/// where `(machine_id)`, `(file_seqnum)`, `(head_seqnum)`, and `(head_realtime)` are lowercase hex-encoded in
	/// little-endian.
	///
	/// This MUST be the inverse of [`parse_filename`](AsyncFileRead::parse_filename), and you should ensure that
	/// [`make_prefix`](AsyncFileRead::make_prefix) remains compatible.
	#[tracing::instrument(level = "trace")]
	fn make_filename(info: &FilenameInfo) -> PathBuf {
		match info {
			FilenameInfo::Latest { machine_id, scope } => {
				PathBuf::from(hex::encode(machine_id.to_be_bytes()))
					.join(format!("{scope}.journal"))
			}
			FilenameInfo::Archived {
				machine_id,
				scope,
				file_seqnum,
				head_seqnum,
				head_realtime,
			} => PathBuf::from(hex::encode(machine_id.to_be_bytes())).join(format!(
				"{scope}@{file_seqnum}-{head_seqnum}-{head_realtime}.journal",
				file_seqnum = hex::encode(file_seqnum.get().to_be_bytes()),
				head_seqnum = hex::encode(head_seqnum.get().to_be_bytes()),
				head_realtime = hex::encode(
					u64::try_from(head_realtime.as_microsecond())
						.unwrap_or_default()
						.to_be_bytes()
				),
			)),
		}
	}

	/// Make a journal filename prefix from a machine ID and scope.
	///
	/// In the systemd on-disk file scheme, this is:
	///
	/// ```plain
	/// (machine_id)/(scope)@
	/// ```
	///
	/// where `(machine_id)` is lowercase hex-encoded in little-endian.
	///
	/// This MUST be compatible with [`make_filename`](AsyncFileRead::parse_filename).
	#[tracing::instrument(level = "trace")]
	fn make_prefix(JournalSelection { machine_id, scope }: &JournalSelection) -> PathBuf {
		PathBuf::from(hex::encode(machine_id.to_be_bytes())).join(format!("{scope}@"))
	}

	/// Parse a journal filename.
	///
	/// Returns `None` if the filename cannot be parsed.
	///
	/// In the systemd on-disk file scheme, this is either:
	///
	/// ```plain
	/// (machine_id)/(scope).journal
	/// (machine_id)/(scope)@(file_seqnum)-(head_seqnum)-(head_realtime).journal
	/// ```
	///
	/// where `(machine_id)`, `(file_seqnum)`, `(head_seqnum)`, and `(head_realtime)` are lowercase hex-encoded in
	/// little-endian.
	///
	/// This MUST be the inverse of [`make_filename`](AsyncFileRead::make_filename), though it may be more lenient.
	/// The default implementation ignores the extension (or even the presence of a file extension), and is
	/// case-insensitive on the hex fields.
	#[tracing::instrument(level = "trace")]
	fn parse_filename(path: &Path) -> Option<FilenameInfo> {
		let mut components = path.components().rev();
		let filename = components.next()?.as_os_str().to_str()?;
		let machine_id = u128::from_be_bytes(
			hex::decode(components.next()?.as_os_str().to_str()?)
				.ok()?
				.try_into()
				.ok()?,
		);

		let Some((scope, rest)) = filename.split_once('@') else {
			let (scope, _) = filename.split_once('.').unwrap_or((filename, ""));
			if scope == "fss" {
				return None;
			}
			return Some(FilenameInfo::Latest {
				machine_id,
				scope: scope.to_string(),
			});
		};

		let (file_seqnum, rest) = rest.split_once('-')?;
		let (head_seqnum, rest) = rest.split_once('-')?;
		let (head_realtime, _) = rest.split_once('.').unwrap_or((rest, ""));

		let file_seqnum = u128::from_be_bytes(hex::decode(file_seqnum).ok()?.try_into().ok()?);
		let head_seqnum = u64::from_be_bytes(hex::decode(head_seqnum).ok()?.try_into().ok()?);
		let head_realtime = u64::from_be_bytes(hex::decode(head_realtime).ok()?.try_into().ok()?);

		Some(FilenameInfo::Archived {
			machine_id,
			scope: scope.to_string(),
			file_seqnum: NonZeroU128::new(file_seqnum)?,
			head_seqnum: NonZeroU64::new(head_seqnum)?,
			head_realtime: Timestamp::from_microsecond(head_realtime.try_into().ok()?).ok()?,
		})
	}

	/// For internal use only.
	#[allow(async_fn_in_trait)]
	#[doc(hidden)]
	#[must_use]
	#[tracing::instrument(level = "trace", skip(self))]
	async fn read_some(&mut self, size: usize) -> std::io::Result<Vec<u8>>
	where
		Self: Unpin,
	{
		let mut buf = vec![0; size];
		self.read_exact(&mut buf).await?;
		Ok(buf)
	}

	/// For internal use only.
	#[allow(async_fn_in_trait)]
	#[doc(hidden)]
	#[must_use]
	#[tracing::instrument(level = "trace", skip(self))]
	async fn read_some_at(&mut self, offset: u64, size: usize) -> std::io::Result<Vec<u8>>
	where
		Self: Unpin,
	{
		debug_assert!(
			offset >= MIN_HEADER_SIZE as u64,
			"small seek protection! [{offset}]"
		);

		let mut buf = vec![0; size];
		self.seek(std::io::SeekFrom::Start(offset)).await?;
		self.read_exact(&mut buf).await?;
		Ok(buf)
	}

	/// For internal use only.
	#[allow(async_fn_in_trait)]
	#[doc(hidden)]
	async fn read_bounded_into(
		&mut self,
		buf: &mut [u8],
		min: usize,
		max: usize,
	) -> std::io::Result<usize>
	where
		Self: Unpin,
	{
		let mut n = 0;
		while n < min {
			let m = self.read(&mut buf[n..]).await?;
			if m == 0 {
				return Err(std::io::Error::new(
					std::io::ErrorKind::UnexpectedEof,
					"reached EOF before min bound",
				));
			}
			n += m;
		}
		while n < max {
			let m = self.read(&mut buf[n..]).await?;
			if m == 0 {
				break;
			}
			n += m;
		}
		Ok(n)
	}

	/// For internal use only.
	#[allow(async_fn_in_trait)]
	#[doc(hidden)]
	#[must_use]
	#[tracing::instrument(level = "trace", skip(self))]
	async fn read_bounded(&mut self, min: usize, max: usize) -> std::io::Result<Vec<u8>>
	where
		Self: Unpin,
	{
		let mut buf = vec![0; max];
		let n = self.read_bounded_into(&mut buf, min, max).await?;
		buf.truncate(n);
		Ok(buf)
	}
}

/// Information contained in a journal filename.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FilenameInfo {
	Archived {
		machine_id: u128,
		scope: String,
		file_seqnum: NonZeroU128,
		head_seqnum: NonZeroU64,
		head_realtime: Timestamp,
	},
	Latest {
		machine_id: u128,
		scope: String,
	},
}

impl FilenameInfo {
	pub fn is_archived(&self) -> bool {
		match self {
			Self::Archived { .. } => true,
			_ => false,
		}
	}

	pub fn is_latest(&self) -> bool {
		match self {
			Self::Latest { .. } => true,
			_ => false,
		}
	}
}

impl PartialOrd for FilenameInfo {
	fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
		match (self, other) {
			(
				Self::Archived {
					machine_id: a_machine_id,
					scope: a_scope,
					file_seqnum: a_file_seqnum,
					head_seqnum: a_head_seqnum,
					head_realtime: a_head_realtime,
				},
				Self::Archived {
					machine_id: b_machine_id,
					scope: b_scope,
					file_seqnum: b_file_seqnum,
					head_seqnum: b_head_seqnum,
					head_realtime: b_head_realtime,
				},
			) => a_head_realtime
				.partial_cmp(b_head_realtime)
				.or_else(|| a_head_seqnum.partial_cmp(b_head_seqnum))
				.or_else(|| a_file_seqnum.partial_cmp(b_file_seqnum))
				.or_else(|| a_scope.partial_cmp(b_scope))
				.or_else(|| a_machine_id.partial_cmp(b_machine_id)),
			(
				Self::Latest {
					machine_id: a_machine_id,
					scope: a_scope,
				},
				Self::Latest {
					machine_id: b_machine_id,
					scope: b_scope,
				},
			) => a_scope
				.partial_cmp(b_scope)
				.or_else(|| a_machine_id.partial_cmp(b_machine_id)),
			(Self::Archived { .. }, Self::Latest { .. }) => Some(std::cmp::Ordering::Less),
			(Self::Latest { .. }, Self::Archived { .. }) => Some(std::cmp::Ordering::Greater),
		}
	}
}

impl Ord for FilenameInfo {
	fn cmp(&self, other: &Self) -> std::cmp::Ordering {
		// UNWRAP: we know partial_cmp is always Some from above
		self.partial_cmp(other).unwrap()
	}
}
