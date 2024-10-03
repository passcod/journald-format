/// Options used when creating new journal files.
///
/// The machine ID, boot ID, and scope are required, the rest have defaults, which are like
/// systemd's own.
///
/// The meaning and defaults of the options are described for systemd, but you can choose your own
/// semantics when writing your own journal files. In general, you shouldn't use this library to
/// write to systemd's own journal files (talk directly to journald instead), so when writing your
/// own independent journals you'll be free to invent your own conventions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateOptions {
	/// The machine ID of the current system.
	///
	/// In systemd, this is the value of `/etc/machine-id`.
	pub machine_id: u128,

	/// The current boot ID of the system.
	///
	/// In systemd, this is the value of `/proc/sys/kernel/random/boot_id`.
	pub boot_id: u128,

	/// Scope of the journal.
	///
	/// This is used to logically group journal files together.
	///
	/// In systemd, the scope is either `system` for the system-wide journal, or `user-$UID` for
	/// a user journal.
	pub scope: String,

	/// Whether to seal the journal with forward secure sealing.
	///
	/// Enabling also enables seal-continuous, because that's more secure and backwards compatible.
	///
	/// Defaults to false.
	pub seal: bool,

	/// Whether to use the "compact" binary format.
	///
	/// Defaults to true.
	pub compact: bool,

	/// The compression algorithm to use for new objects.
	///
	/// Defaults to Zstd.
	pub compression: Option<Compression>,

	/// The capacity of the data hash table, in entries.
	///
	/// This should be scaled according to the desired maximum file size for the journal.
	///
	/// When the data hash table is 75% full, or on the first collision, the journal will rotate.
	///
	/// Defaults to 2048.
	pub data_hash_table_capacity: u64,

	/// The capacity of the field hash table, in entries.
	///
	/// This should be scaled according to the amount of unique field names in the journal.
	///
	/// Defaults to 333.
	pub field_hash_table_capacity: u64,
}

impl CreateOptions {
	pub fn new(machine_id: u128, boot_id: u128, scope: impl ToString) -> Self {
		Self {
			machine_id,
			boot_id,
			scope: scope.to_string(),
			seal: false,
			compact: true,
			compression: Some(Compression::default()),
			data_hash_table_capacity: 2048,
			field_hash_table_capacity: 333,
		}
	}

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

	pub fn with_data_hash_table_capacity(mut self, data_hash_table_capacity: u64) -> Self {
		self.data_hash_table_capacity = data_hash_table_capacity;
		self
	}

	pub fn with_field_hash_table_capacity(mut self, field_hash_table_capacity: u64) -> Self {
		self.field_hash_table_capacity = field_hash_table_capacity;
		self
	}
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
