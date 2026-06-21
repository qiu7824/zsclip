use std::path::Path;

use crate::platform::string::to_wide;

#[link(name = "winmm")]
unsafe extern "system" {
    fn PlaySoundW(pszsound: *const u16, hmod: isize, fdwsound: u32) -> i32;
}

const SND_ASYNC: u32 = 0x0001;
const SND_FILENAME: u32 = 0x00020000;
const SND_NODEFAULT: u32 = 0x0002;
const SND_MEMORY: u32 = 0x0004;

pub(crate) fn play_wav_file(path: &Path) -> bool {
    if !path.is_file() {
        return false;
    }
    let wide = to_wide(&path.to_string_lossy());
    unsafe { PlaySoundW(wide.as_ptr(), 0, SND_ASYNC | SND_FILENAME | SND_NODEFAULT) != 0 }
}

pub(crate) fn play_wav_memory(bytes: &'static [u8]) -> bool {
    if bytes.is_empty() {
        return false;
    }
    unsafe {
        PlaySoundW(
            bytes.as_ptr() as *const u16,
            0,
            SND_ASYNC | SND_MEMORY | SND_NODEFAULT,
        ) != 0
    }
}
