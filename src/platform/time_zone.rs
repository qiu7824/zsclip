#[repr(C)]
struct SystemTimeRaw {
    year: u16,
    month: u16,
    day_of_week: u16,
    day: u16,
    hour: u16,
    minute: u16,
    second: u16,
    milliseconds: u16,
}

#[repr(C)]
struct TimeZoneInformationRaw {
    bias: i32,
    _pad: [u8; 168],
}

#[link(name = "kernel32")]
unsafe extern "system" {
    fn GetTimeZoneInformation(lptzi: *mut TimeZoneInformationRaw) -> u32;
    fn SystemTimeToTzSpecificLocalTime(
        lptimezoneinformation: *const core::ffi::c_void,
        lpuniversaltime: *const SystemTimeRaw,
        lplocaltime: *mut SystemTimeRaw,
    ) -> i32;
}

pub(crate) fn local_offset_secs() -> i64 {
    unsafe {
        let mut tzi: TimeZoneInformationRaw = core::mem::zeroed();
        GetTimeZoneInformation(&mut tzi);
        -(tzi.bias as i64) * 60
    }
}

pub(crate) fn utc_parts_to_local_parts(
    y: i32,
    m: i32,
    d: i32,
    h: i32,
    min: i32,
    sec: i32,
) -> Option<(i32, i32, i32, i32, i32, i32)> {
    let utc = SystemTimeRaw {
        year: y as u16,
        month: m as u16,
        day_of_week: 0,
        day: d as u16,
        hour: h as u16,
        minute: min as u16,
        second: sec as u16,
        milliseconds: 0,
    };
    unsafe {
        let mut local: SystemTimeRaw = core::mem::zeroed();
        if SystemTimeToTzSpecificLocalTime(core::ptr::null(), &utc, &mut local) == 0 {
            return None;
        }
        Some((
            local.year as i32,
            local.month as i32,
            local.day as i32,
            local.hour as i32,
            local.minute as i32,
            local.second as i32,
        ))
    }
}
