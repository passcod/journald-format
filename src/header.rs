use std::{
	num::{NonZeroU128, NonZeroU32, NonZeroU64},
	path::PathBuf,
};

use deku::{ctx::Endian, no_std_io, prelude::*};
use flagset::{flags, FlagSet};

use crate::reader::{AsyncFileRead, FilenameInfo};

// magic 8 = 8
#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
#[deku(endian = "little", magic = b"LPKSHHRH")]
pub struct Header {
	#[deku(
		reader = "CompatibleFlag::deku_reader(deku::reader)",
		writer = "CompatibleFlag::deku_writer(deku::writer, &self.compatible_flags)"
	)]
	pub compatible_flags: FlagSet<CompatibleFlag>, // 4 = 12

	#[deku(
		reader = "IncompatibleFlag::deku_reader(deku::reader)",
		writer = "IncompatibleFlag::deku_writer(deku::writer, &self.incompatible_flags)"
	)]
	pub incompatible_flags: FlagSet<IncompatibleFlag>, // 4 = 16

	#[deku(pad_bytes_after = "7")]
	pub state: State, // 8 = 24

	pub file_id: u128,                           // 16 = 40
	pub machine_id: u128,                        // 16 = 56
	pub tail_entry_boot_id: Option<NonZeroU128>, // 16 = 72
	pub seqnum_id: NonZeroU128,                  // 16 = 88

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
	pub n_data: Option<u64>, // 8 = 216
	#[deku(cond = "*header_size > 216")]
	pub n_fields: Option<u64>, // 8 = 224

	// added in systemd 189
	#[deku(cond = "*header_size > 224")]
	pub n_tags: Option<u64>, // 8 = 232
	#[deku(cond = "*header_size > 232")]
	pub n_entry_arrays: Option<u64>, // 8 = 240

	// added in systemd 246
	#[deku(cond = "*header_size > 240")]
	pub data_hash_chain_depth: Option<u64>, // 8 = 248
	#[deku(cond = "*header_size > 248")]
	pub field_hash_chain_depth: Option<u64>, // 8 = 256

	// added in systemd 252
	#[deku(cond = "*header_size > 256")]
	pub tail_entry_array_offset: Option<NonZeroU32>, // 4 = 260
	#[deku(cond = "*header_size > 260")]
	pub tail_entry_array_n_entries: Option<NonZeroU32>, // 4 = 264

	// added in systemd 254
	#[deku(cond = "*header_size > 264")]
	pub tail_entry_offset: Option<NonZeroU64>, // 8 = 272
}

const MIN_HEADER_SIZE: usize = 208;
const MAX_HEADER_SIZE: usize = 272;

impl From<Header> for FilenameInfo {
	fn from(value: Header) -> Self {
		FilenameInfo::Archived {
			machine_id: value.machine_id,
			scope: String::new(),
			file_seqnum: value.seqnum_id,
			head_seqnum: value.head_entry_seqnum,
			head_realtime: value.head_entry_realtime,
		}
	}
}

impl Header {
	pub fn filename(&self, scope: &str) -> PathBuf {
		PathBuf::from(hex::encode(self.machine_id.to_le_bytes())).join(format!(
			"{scope}@{file_seqnum}-{head_seqnum}-{head_realtime}.journal",
			file_seqnum = hex::encode(self.seqnum_id.get().to_le_bytes()),
			head_seqnum = hex::encode(self.head_entry_seqnum.to_le_bytes()),
			head_realtime = hex::encode(self.head_entry_realtime.to_le_bytes()),
		))
	}

	pub async fn read<R: AsyncFileRead + Unpin>(io: &mut R) -> std::io::Result<Self> {
		let head = io.read_bounded(MIN_HEADER_SIZE, MAX_HEADER_SIZE).await?;

		let (_, header) = Header::from_bytes((&head, 0))
			.map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

		Ok(header)
	}
}

#[cfg(test)]
#[tokio::test]
async fn test_header_parse() {
	use futures_util::io::Cursor;

	use crate::tables::HASH_ITEM_SIZE;

	const HEADER_DATA: &[u8] = &[
		0x4c, 0x50, 0x4b, 0x53, 0x48, 0x48, 0x52, 0x48, 0x02, 0x00, 0x00, 0x00, 0x1c, 0x00, 0x00,
		0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xa0, 0x71, 0x3a, 0xc1, 0x94, 0xe5,
		0x40, 0xcc, 0xa6, 0x62, 0xd1, 0x98, 0x8b, 0x5d, 0xd9, 0x24, 0xc4, 0x44, 0xc7, 0x1c, 0x03,
		0x8d, 0x45, 0xb0, 0xaf, 0x20, 0x14, 0x44, 0xa8, 0x3b, 0x91, 0xc9, 0x82, 0xed, 0xa8, 0xaf,
		0x55, 0x80, 0x4a, 0xbe, 0x8e, 0xca, 0x8e, 0xfb, 0x40, 0x72, 0xc6, 0x98, 0xae, 0x25, 0x7a,
		0x22, 0x4b, 0x70, 0x40, 0x5a, 0x90, 0x42, 0xa9, 0x9a, 0xef, 0x05, 0x7c, 0xe0, 0x10, 0x01,
		0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xf0, 0xfe, 0x7f, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00,
		0x16, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x80, 0xe3, 0x38, 0x00, 0x00, 0x00, 0x00, 0x00,
		0x20, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xd0, 0x14, 0x00, 0x00, 0x00, 0x00, 0x00,
		0x00, 0x70, 0x17, 0x68, 0x02, 0x00, 0x00, 0x00, 0x00, 0x87, 0x4e, 0x03, 0x00, 0x00, 0x00,
		0x00, 0x00, 0xe8, 0x4a, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x75, 0x12, 0x2f, 0x00, 0x00,
		0x00, 0x00, 0x00, 0x94, 0x59, 0x2d, 0x00, 0x00, 0x00, 0x00, 0x00, 0x98, 0x09, 0x39, 0x00,
		0x00, 0x00, 0x00, 0x00, 0x84, 0x11, 0x3e, 0x05, 0x68, 0x23, 0x06, 0x00, 0x23, 0xff, 0xf7,
		0x14, 0x92, 0x23, 0x06, 0x00, 0xf6, 0x6f, 0x55, 0x54, 0x56, 0x00, 0x00, 0x00, 0xa4, 0x8e,
		0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x6c, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
		0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x8d, 0x74, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
		0x04, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
		0x00, 0x18, 0x16, 0xf3, 0x00, 0xda, 0xdb, 0x00, 0x00, 0x70, 0x17, 0x68, 0x02, 0x00, 0x00,
		0x00, 0x00, 0x05, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xe0, 0x14, 0x00, 0x00, 0x00,
		0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
	];

	let mut io = Cursor::new(HEADER_DATA);
	let header = Header::read(&mut io).await.unwrap();
	assert_eq!(
		header,
		Header {
			compatible_flags: CompatibleFlag::TailEntryBootId.into(),
			incompatible_flags: IncompatibleFlag::KeyedHash
				| (IncompatibleFlag::CompressedZstd)
				| (IncompatibleFlag::Compact),
			state: State::Online,
			file_id: u128::from_le_bytes([
				0xa0, 0x71, 0x3a, 0xc1, 0x94, 0xe5, 0x40, 0xcc, 0xa6, 0x62, 0xd1, 0x98, 0x8b, 0x5d,
				0xd9, 0x24
			]),
			machine_id: u128::from_le_bytes([
				0xc4, 0x44, 0xc7, 0x1c, 0x03, 0x8d, 0x45, 0xb0, 0xaf, 0x20, 0x14, 0x44, 0xa8, 0x3b,
				0x91, 0xc9
			]),
			tail_entry_boot_id: NonZeroU128::new(u128::from_le_bytes([
				0x82, 0xed, 0xa8, 0xaf, 0x55, 0x80, 0x4a, 0xbe, 0x8e, 0xca, 0x8e, 0xfb, 0x40, 0x72,
				0xc6, 0x98,
			])),
			seqnum_id: NonZeroU128::new(u128::from_le_bytes([
				0xae, 0x25, 0x7a, 0x22, 0x4b, 0x70, 0x40, 0x5a, 0x90, 0x42, 0xa9, 0x9a, 0xef, 0x05,
				0x7c, 0xe0,
			]))
			.unwrap(),
			header_size: MAX_HEADER_SIZE as _,
			arena_size: 41942768,
			data_hash_table_offset: 5632,
			data_hash_table_size: 233016 * HASH_ITEM_SIZE as u64,
			field_hash_table_offset: 288,
			field_hash_table_size: 333 * HASH_ITEM_SIZE as u64,
			tail_object_offset: 40376176,
			n_objects: 216711,
			n_entries: 84712,
			tail_entry_seqnum: 3084917,
			head_entry_seqnum: 2972052,
			entry_array_offset: 3738008,
			head_entry_realtime: 1727779531788676,
			tail_entry_realtime: 1727960184258339,
			tail_entry_monotonic: 370782072822,
			n_data: Some(102052),
			n_fields: Some(108),
			n_tags: Some(0),
			n_entry_arrays: Some(29837),
			data_hash_chain_depth: Some(4),
			field_hash_chain_depth: Some(2),
			tail_entry_array_offset: NonZeroU32::new(15930904),
			tail_entry_array_n_entries: NonZeroU32::new(56282),
			tail_entry_offset: NonZeroU64::new(40376176),
		}
	);
}

flags! {
	/// Feature flags that can be ignored if not understood.
	///
	/// If a reader encounters a compatible flag it does not understand, it should
	/// ignore it and continue reading the file.
	pub enum CompatibleFlag: u32 {
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
	pub enum IncompatibleFlag: u32 {
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
}

impl CompatibleFlag {
	fn deku_reader<R: no_std_io::Read + no_std_io::Seek>(
		reader: &mut Reader<R>,
	) -> Result<FlagSet<Self>, DekuError> {
		let value = u32::from_reader_with_ctx(reader, Endian::Little)?;
		Ok(FlagSet::new_truncated(value))
	}

	fn deku_writer<W: std::io::Write + std::io::Seek>(
		writer: &mut Writer<W>,
		field: &FlagSet<Self>,
	) -> Result<(), DekuError> {
		field.bits().to_writer(writer, Endian::Little)
	}
}

impl IncompatibleFlag {
	fn deku_reader<R: no_std_io::Read + no_std_io::Seek>(
		reader: &mut Reader<R>,
	) -> Result<FlagSet<Self>, DekuError> {
		let value = u32::from_reader_with_ctx(reader, Endian::Little)?;
		FlagSet::new(value).map_err(|_| DekuError::Assertion("Unknown incompatible flags".into()))
	}

	fn deku_writer<W: std::io::Write + std::io::Seek>(
		writer: &mut Writer<W>,
		field: &FlagSet<Self>,
	) -> Result<(), DekuError> {
		field.bits().to_writer(writer, Endian::Little)
	}
}

#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
#[deku(id_type = "u8", endian = "endian", ctx = "endian: deku::ctx::Endian")]
#[repr(u8)]
pub enum State {
	/// The file is closed for writing.
	Offline = 0,

	/// The file is open for writing.
	Online = 1,

	/// The file is closed for writing and has been rotated.
	Archived = 2,
}
