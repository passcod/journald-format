use std::path::PathBuf;

use deku::prelude::*;

#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
#[deku(endian = "little", magic = b"LPKSHHRH")]
pub struct Header {
	// magic 8 = 8
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
	pub tail_entry_array_offset: u32, // 4 = 260
	#[deku(cond = "*header_size > 260")]
	pub tail_entry_array_n_entries: u32, // 4 = 264

	// added in systemd 254
	#[deku(cond = "*header_size > 264")]
	pub tail_entry_offset: u64, // 8 = 272
}

impl Header {
	pub fn filename(&self, scope: &str) -> PathBuf {
		PathBuf::from(hex::encode(self.machine_id.to_le_bytes())).join(format!(
			"{scope}@{file_seqnum}-{head_seqnum}-{head_realtime}.journal",
			file_seqnum = hex::encode(self.seqnum_id.to_le_bytes()),
			head_seqnum = hex::encode(self.head_entry_seqnum.to_le_bytes()),
			head_realtime = hex::encode(self.head_entry_realtime.to_le_bytes()),
		))
	}
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
