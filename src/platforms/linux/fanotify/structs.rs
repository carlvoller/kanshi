use std::mem;

#[derive(Debug, Clone, Copy)]
pub struct FanotifyEventMetaData {
    pub event_len: libc::__u32,
    pub vers: libc::__u8,
    pub reserved: libc::__u8,
    pub metadata_len: libc::__u16,
    pub mask: libc::__u64,
    pub fd: libc::__s32,
    pub pid: libc::__s32,
}

pub const FAN_EVENT_METADATA_LEN: usize = mem::size_of::<FanotifyEventMetaData>();
