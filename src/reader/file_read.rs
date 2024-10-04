use std::path::{Path, PathBuf};

use futures_util::{
	io::{AsyncReadExt, AsyncSeekExt, Cursor},
	Stream,
};

pub trait AsyncFileRead: AsyncReadExt + AsyncSeekExt + Unpin {
	/// Open a file for reading.
	///
	/// This should close the current file (if any).
	fn open(
		&mut self,
		filename: &Path,
	) -> impl std::future::Future<Output = std::io::Result<()>> + Send;

	/// The path to the current file, if one is open.
	fn current(&self) -> Option<&Path>;

	/// Recursively list all journal files available, sorted lexicographically.
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
	) -> impl Stream<Item = std::io::Result<PathBuf>> + Unpin;

	/// Make a journal filename.
	///
	/// In the systemd on-disk file scheme, this is:
	///
	/// ```plain
	/// (machine_id)/(scope)@(file_seqnum)-(head_seqnum)-(head_realtime).journal
	/// ```
	///
	/// where `(machine_id)`, `(file_seqnum)`, `(head_seqnum)`, and `(head_realtime)` are lowercase hex-encoded in
	/// little-endian.
	///
	/// This MUST be the inverse of [`parse_filename`](AsyncFileRead::parse_filename), and you should ensure that
	/// [`make_prefix`](AsyncFileRead::make_prefix) remains compatible.
	fn make_filename(info: FilenameInfo) -> PathBuf {
		PathBuf::from(hex::encode(info.machine_id.to_le_bytes())).join(format!(
			"{scope}@{file_seqnum}-{head_seqnum}-{head_realtime}.journal",
			scope = info.scope,
			file_seqnum = hex::encode(info.file_seqnum.to_le_bytes()),
			head_seqnum = hex::encode(info.head_seqnum.to_le_bytes()),
			head_realtime = hex::encode(info.head_realtime.to_le_bytes()),
		))
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
	fn make_prefix(machine_id: u128, scope: &str) -> PathBuf {
		PathBuf::from(hex::encode(machine_id.to_le_bytes())).join(format!("{scope}@"))
	}

	/// Parse a journal filename.
	///
	/// Returns `None` if the filename cannot be parsed.
	///
	/// In the systemd on-disk file scheme, this is:
	///
	/// ```plain
	/// (machine_id)/(scope)@(file_seqnum)-(head_seqnum)-(head_realtime).journal
	/// ```
	///
	/// where `(machine_id)`, `(file_seqnum)`, `(head_seqnum)`, and `(head_realtime)` are lowercase hex-encoded in
	/// little-endian.
	///
	/// This MUST be the inverse of [`make_filename`](AsyncFileRead::make_filename), though it may be more lenient.
	/// The default implementation ignores the extension (or even the presence of a file extension), and is
	/// case-insensitive on the hex fields.
	fn parse_filename(path: &Path) -> Option<FilenameInfo> {
		let mut components = path.components().rev();
		let filename = components.next()?.as_os_str().to_str()?;
		let machine_id = u128::from_le_bytes(
			hex::decode(components.next()?.as_os_str().to_str()?)
				.ok()?
				.try_into()
				.ok()?,
		);

		let (scope, rest) = filename.split_once('@')?;
		let (file_seqnum, rest) = rest.split_once('-')?;
		let (head_seqnum, rest) = rest.split_once('-')?;
		let (head_realtime, _) = rest.split_once('.').unwrap_or((rest, ""));

		let file_seqnum = u128::from_le_bytes(hex::decode(file_seqnum).ok()?.try_into().ok()?);
		let head_seqnum = u64::from_le_bytes(hex::decode(head_seqnum).ok()?.try_into().ok()?);
		let head_realtime = u64::from_le_bytes(hex::decode(head_realtime).ok()?.try_into().ok()?);

		Some(FilenameInfo {
			machine_id,
			scope: scope.to_string(),
			file_seqnum,
			head_seqnum,
			head_realtime,
		})
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
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FilenameInfo {
	pub machine_id: u128,
	pub scope: String,
	pub file_seqnum: u128,
	pub head_seqnum: u64,
	pub head_realtime: u64,
}

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
