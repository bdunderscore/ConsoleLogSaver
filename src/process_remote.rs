mod lldb;

#[derive(Debug, thiserror::Error)]
pub enum ProcessRemoteError {
    #[error("byte order mismatch")]
    ByteOrderMismatch,
    #[error("pointer size mismatch")]
    PointerSizeMismatch,
}

pub use lldb::get_buffer;
pub type ProcessId = ::lldb::lldb_pid_t;
