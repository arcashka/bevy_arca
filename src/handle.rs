use windows::Win32::Foundation::HANDLE;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Handle(pub HANDLE);

unsafe impl Send for Handle {}
unsafe impl Sync for Handle {}
