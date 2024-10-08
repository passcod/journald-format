#[cfg(feature = "on-disk")]
pub use on_disk::JournalOnDisk;
#[cfg(feature = "on-disk")]
pub use read_whole::ReadWholeFile;

mod in_memory;

#[cfg(feature = "on-disk")]
mod on_disk;

#[cfg(feature = "on-disk")]
mod read_whole;
