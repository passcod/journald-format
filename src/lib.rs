use std::{num::NonZeroU64, path::Path};

use deku::prelude::*;
use futures_io::{AsyncRead, AsyncSeek, AsyncWrite};

#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
#[deku(id_type = "u8")]
pub enum ObjectType {
	// 0 is unused
	/// Encapsulates the contents of one field of an entry, i.e. a string such
	/// as `_SYSTEMD_UNIT=avahi-daemon.service`, or `MESSAGE=Foo had a booboo`.
	Data = 1,

	/// Encapsulates a field name, i.e. a string such as `_SYSTEMD_UNIT` or
	/// `MESSAGE`, without any `=` or even value.
	Field,

	/// Binds several `Data` objects together into a log entry.
	Entry,

	/// Encapsulates a hash table for finding existing `Data` objects.
	DataHashTable,

	/// Encapsulates a hash table for finding existing `Field` objects.
	FieldHashTable,

	/// Encapsulates a sorted array of offsets to entries, used for seeking by
	/// binary search.
	EntryArray,

	/// Consists of a Forward Secure Sealing tag for all data from the beginning
	/// of the file or the last tag written (whichever is later).
	Tag,
}

#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
#[deku(endian = "little", magic = b"LPKSHHRH")]
pub struct Header {
	pub compatible_flags: u32,   // 4 = 12
	pub incompatible_flags: u32, // 4 = 16

	#[deku(pad_bytes_after = "7")]
	pub state: u8, // 8 = 24

	pub file_id: u128,            // 16 = 40
	pub machine_id: u128,         // 16 = 56
	pub tail_entry_boot_id: u128, // 16 = 72
	pub seqnum_id: u128,          // 16 = 88

	pub header_size: u64,             // 8 = 96
	pub arena_size: u64,              // 8 = 104
	pub data_hash_table_offset: u64,  // 8 = 112
	pub data_hash_table_size: u64,    // 8 = 120
	pub field_hash_table_offset: u64, // 8 = 128
	pub field_hash_table_size: u64,   // 8 = 136
	pub tail_object_offset: u64,      // 8 = 144

	pub n_objects: u64, // 8 = 152
	pub n_entries: u64, // 8 = 160

	pub tail_entry_seqnum: u64,    // 8 = 168
	pub head_entry_seqnum: u64,    // 8 = 176
	pub entry_array_offset: u64,   // 8 = 184
	pub head_entry_realtime: u64,  // 8 = 192
	pub tail_entry_realtime: u64,  // 8 = 200
	pub tail_entry_monotonic: u64, // 8 = 208

	// added in systemd 187
	#[deku(cond = "*header_size > 208")]
	pub n_data: u64, // 8 = 216
	#[deku(cond = "*header_size > 216")]
	pub n_fields: u64, // 8 = 224

	// added in systemd 189
	#[deku(cond = "*header_size > 224")]
	pub n_tags: u64, // 8 = 232
	#[deku(cond = "*header_size > 232")]
	pub n_entry_arrays: u64, // 8 = 240

	// added in systemd 246
	#[deku(cond = "*header_size > 240")]
	pub data_hash_chain_depth: u64, // 8 = 248
	#[deku(cond = "*header_size > 248")]
	pub field_hash_chain_depth: u64, // 8 = 256

	// added in systemd 252
	#[deku(cond = "*header_size > 256")]
	pub tail_entry_array_offset: u64, // 8 = 264
	#[deku(cond = "*header_size > 264")]
	pub tail_entry_array_n_entries: u64, // 8 = 272

	// added in systemd 254
	#[deku(cond = "*header_size > 272")]
	pub tail_entry_offset: u64, // 8 = 280
}

/// Feature flags that can be ignored if not understood.
///
/// If a reader encounters a compatible flag it does not understand, it should
/// ignore it and continue reading the file.
#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
#[deku(id_type = "u32", endian = "little")]
#[repr(u32)]
#[rustfmt::skip]
pub enum CompatibleFlag {
	/// The file includes `Tag` objects required for Forward Secure Sealing.
	///
	/// Available from systemd 189.
	Sealed           = 0b__1,

	/// The `tail_entry_boot_id` field is strictly updated on initial creation
	/// of the file, and whener an entry is updated. If this flag is not set,
	/// the field is also updated when the file is archived.
	///
	/// Available from systemd 254.
	TailEntryBootId  = 0b_10,

	/// Forward Secure Sealing happens once per epoch. This protects against an
	/// attack where a sealed log is truncated and that cannot be detected, see
	/// CVE-2023-31438.
	///
	/// Available from systemd 255.
	SealedContinuous = 0b100,
}

/// Feature flags that must be understood for compatibility.
///
/// If a reader encounters an incompatible flag it does not understand, it must
/// refuse to read the file, and ask the user to upgrade their software.
#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
#[deku(id_type = "u32", endian = "little")]
#[repr(u32)]
#[rustfmt::skip]
pub enum IncompatibleFlag {
	/// The file includes `Data` objects that are compressed with XZ.
	///
	/// Available from systemd 38.
	CompressedXz   = 0b____1,

	/// The file includes `Data` objects that are compressed with LZ4.
	///
	/// Available from systemd 216.
	CompressedLz4  = 0b___10,

	/// The hash tables use the SipHash-2-4 keyed hash algorithm.
	///
	/// Available from systemd 246.
	KeyedHash      = 0b__100,

	/// The file includes `Data` objects that are compressed with Zstd.
	///
	/// Available from systemd 246.
	CompressedZstd = 0b_1000,

	/// The file uses the "new" binary format, which uses less space.
	///
	/// Available from systemd 252.
	Compact        = 0b10000,
}

#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
#[deku(id_type = "u8")]
#[repr(u8)]
pub enum State {
	/// The file is closed for writing.
	Offline = 0,

	/// The file is open for writing.
	Online = 1,

	/// The file is closed for writing and has been rotated.
	Archived = 2,
}

/// Format flags for objects.
#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
#[deku(id_type = "u8")]
#[repr(u8)]
#[rustfmt::skip]
pub enum ObjectFlag {
	/// The object is compressed with XZ.
	CompressedXz   = 0b__1,

	/// The object is compressed with LZ4.
	CompressedLz4  = 0b_10,

	/// The object is compressed with Zstd.
	CompressedZstd = 0b100,
}

#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
pub struct ObjectHeader {
	pub r#type: ObjectType,
	#[deku(pad_bytes_after = "6")]
	pub flags: u8,

	#[deku(endian = "little")]
	pub size: u64,
}

impl ObjectHeader {
	pub const fn payload_size(&self) -> u64 {
		self.size
			.saturating_sub(std::mem::size_of::<ObjectHeader>() as _)
	}
}

#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
#[deku(endian = "little")]
pub struct DataObjectHeader {
	pub hash: u64,
	pub next_hash_offset: u64,
	pub next_field_offset: u64,
	pub entry_offset: u64,
	pub entry_array_offset: u64,
	pub n_entries: u64,
}

#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
#[deku(endian = "little")]
pub struct DataObjectCompactPayloadHeader {
	pub tail_entry_array_offset: u32,
	pub tail_entry_array_n_entries: u32,
}

#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
#[deku(endian = "little")]
pub struct FieldObjectHeader {
	pub hash: u64,
	pub next_hash_offset: u64,
	pub next_data_offset: u64,
}

#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
#[deku(endian = "little")]
pub struct EntryObjectHeader {
	pub seqnum: NonZeroU64,
	pub realtime: u64,
	pub monotonic: u64,
	pub boot_id: u128,
	pub xor_hash: u64,
}

#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
#[deku(endian = "little")]
pub struct EntryObjectCompactItem {
	pub object_offset: u32,
}

#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
#[deku(endian = "little")]
pub struct EntryObjectRegularItem {
	pub object_offset: u64,
	pub hash: u64,
}

#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
#[deku(endian = "little")]
pub struct HashItem {
	pub head_hash_offset: u64,
	pub tail_hash_offset: u64,
}

#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
#[deku(endian = "little")]
pub struct EntryArrayObjectHeader {
	pub next_entry_array_offset: u64,
}

#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
#[deku(endian = "little")]
pub struct EntryArrayRegularItem {
	pub offset: u64,
}

#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
#[deku(endian = "little")]
pub struct EntryArrayCompactItem {
	pub offset: u32,
}

pub const TAG_LENGTH: usize = 256 / 8;

#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
#[deku(endian = "little")]
pub struct TagObjectHeader {
	pub seqnum: NonZeroU64,
	pub epoch: u64,
	pub tag: [u8; TAG_LENGTH],
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Compression {
	/// Compress new objects with XZ.
	Xz,

	/// Compress new objects with LZ4.
	Lz4,

	/// Compress new objects with Zstd.
	#[default]
	Zstd,
}

pub trait AsyncFileRead: AsyncRead + AsyncSeek + Unpin {
	/// Open a file for reading.
	///
	/// This should close the current file (if any).
	fn open(
		&mut self,
		filename: &Path,
	) -> impl std::future::Future<Output = std::io::Result<()>> + Send;

	/// The path to the current file, if one is open.
	fn current(&self) -> Option<&Path>;
}

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

pub struct JournalReader<T> {
	io: T,
}

impl<T> std::fmt::Debug for JournalReader<T> {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("JournalReader")
			.field("io", &std::any::type_name::<T>())
			.finish()
	}
}

impl<T> JournalReader<T>
where
	T: AsyncFileRead,
{
	pub fn new(io: T) -> Self {
		Self { io }
	}

	/// Verify all data in the current journal file.
	///
	/// This will check every hash, every sealing tag, and every entry. It
	/// should be used to detect tampering; when reading the journal normally,
	/// only the data that is actually read is verified.
	pub async fn verify(&mut self) -> std::io::Result<bool> {
		todo!()
	}

	/// Verify all data in the entire journal.
	pub async fn verify_all(&mut self) -> std::io::Result<bool> {
		todo!()
	}

	/// Read entries from the current position.
	pub async fn entries(&mut self) -> std::io::Result<Vec<()>> {
		// TODO: return Stream
		todo!()
	}

	/// Seek to a timestamp, or as close as possible.
	pub async fn seek_to_timestamp(&mut self, _timestamp: u64) -> std::io::Result<()> {
		todo!()
	}

	/// Seek to a sequence number, or as close as possible.
	pub async fn seek_to_seqnum(&mut self, _seqnum: u64) -> std::io::Result<()> {
		todo!()
	}

	/// Seek to the start of a boot ID.
	pub async fn seek_to_boot_id(&mut self, _boot_id: u128) -> std::io::Result<()> {
		todo!()
	}
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WriteOptions {
	pub machine_id: u128,
	pub boot_id: u128,
	pub seal: bool,
	pub compact: bool,
	pub compression: Option<Compression>,
}

impl WriteOptions {
	pub fn new(machine_id: u128, boot_id: u128) -> Self {
		Self {
			machine_id,
			boot_id,
			seal: true,
			compact: true,
			compression: Some(Compression::default()),
		}
	}

	// enabling seal also enables seal-continuous bc that's backwards compatible
	pub fn with_seal(mut self, seal: bool) -> Self {
		self.seal = seal;
		self
	}

	pub fn with_compact(mut self, compact: bool) -> Self {
		self.compact = compact;
		self
	}

	pub fn with_compression(mut self, compression: Option<Compression>) -> Self {
		self.compression = compression;
		self
	}
}

pub struct JournalWriter<T> {
	options: WriteOptions,
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
	pub fn with_options(io: T, options: WriteOptions) -> Self {
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
