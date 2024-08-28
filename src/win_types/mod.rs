use bevy::prelude::{Deref, DerefMut};
use windows::Win32::Foundation::HANDLE;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deref, DerefMut)]
pub struct WinHandle(pub HANDLE);

unsafe impl Send for WinHandle {}
unsafe impl Sync for WinHandle {}
