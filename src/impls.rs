#[cfg(feature = "on-disk")]
pub use on_disk::JournalOnDisk;

mod in_memory;

#[cfg(feature = "on-disk")]
mod on_disk;
