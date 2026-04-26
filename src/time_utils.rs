use std::time::{SystemTime, UNIX_EPOCH};

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

#[link(name = "kernel32")]
unsafe extern "system" {
    fn GetTimeZoneInformation(lptzi: *mut TimeZoneInformationRaw) -> u32;
    fn SystemTimeToTzSpecificLocalTime(
        lptimezoneinformation: *const core::ffi::c_void,
        lpuniversaltime: *const SystemTimeRaw,
        lplocaltime: *mut SystemTimeRaw,
    ) -> i32;
}

#[repr(C)]
struct TimeZoneInformationRaw {
    bias: i32,
    _pad: [u8; 168],
}

pub(crate) fn gregorian_to_days(y: i32, m: i32, d: i32) -> i64 {
    let y = y as i64;
    let m = m as i64;
    let d = d as i64;
    let a = (14 - m) / 12;
    let yy = y + 4800 - a;
    let mm = m + 12 * a - 3;
    let jd = d + (153 * mm + 2) / 5 + 365 * yy + yy / 4 - yy / 100 + yy / 400 - 32045;
    jd - 2440588
}

pub(crate) fn days_to_gregorian(days: i64) -> (i32, i32, i32) {
    let z = days + 719468;
    let era = (if z >= 0 { z } else { z - 146096 }) / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as i32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as i32;
    let y = if m <= 2 { y + 1 } else { y } as i32;
    (y, m, d)
}

pub(crate) fn days_to_sqlite_date(days: i64) -> String {
    let (y, m, d) = days_to_gregorian(days);
    format!("{:04}-{:02}-{:02}", y, m, d)
}

fn legacy_local_offset_secs() -> i64 {
    unsafe {
        let mut tzi: TimeZoneInformationRaw = core::mem::zeroed();
        GetTimeZoneInformation(&mut tzi);
        -(tzi.bias as i64) * 60
    }
}

pub(crate) fn unix_secs_to_parts(secs: i64) -> (i32, i32, i32, i32, i32, i32) {
    let sec = (secs % 60) as i32;
    let total_min = secs / 60;
    let min = (total_min % 60) as i32;
    let total_h = total_min / 60;
    let hour = (total_h % 24) as i32;
    let total_days = total_h / 24;
    let z = total_days + 719468;
    let era = (if z >= 0 { z } else { z - 146096 }) / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as i32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as i32;
    let y = if m <= 2 { y + 1 } else { y } as i32;
    (y, m, d, hour, min, sec)
}

pub(crate) fn unix_secs_to_sqlite_str(secs: i64) -> String {
    let (y, m, d, h, min, s) = unix_secs_to_parts(secs);
    format!("{:04}-{:02}-{:02} {:02}:{:02}:{:02}", y, m, d, h, min, s)
}

pub(crate) fn now_utc_sqlite() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    unix_secs_to_sqlite_str(secs)
}

pub(crate) fn utc_secs_to_local_parts(secs: i64) -> (i32, i32, i32, i32, i32, i32) {
    let (y, m, d, h, min, sec) = unix_secs_to_parts(secs);
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
        if SystemTimeToTzSpecificLocalTime(core::ptr::null(), &utc, &mut local) != 0 {
            return (
                local.year as i32,
                local.month as i32,
                local.day as i32,
                local.hour as i32,
                local.minute as i32,
                local.second as i32,
            );
        }
    }

    let local_secs = secs + legacy_local_offset_secs();
    unix_secs_to_parts(local_secs)
}

fn parse_created_at_prefix(created_at: &str) -> Option<(i32, i32, i32, i32, i32)> {
    let mut date_time = created_at.split_whitespace();
    let date = date_time.next()?;
    let time = date_time.next()?;

    let mut date_parts = date.split('-');
    let year = date_parts.next()?.parse::<i32>().ok()?;
    let month = date_parts.next()?.parse::<i32>().ok()?;
    let day = date_parts.next()?.parse::<i32>().ok()?;

    let mut time_parts = time.split(':');
    let hour = time_parts.next()?.parse::<i32>().ok()?;
    let minute = time_parts.next()?.parse::<i32>().ok()?;

    Some((year, month, day, hour, minute))
}

pub(crate) fn format_created_at_local(created_at: &str, fallback: &str) -> String {
    let Some((utc_y, utc_m, utc_d, utc_h, utc_min)) = parse_created_at_prefix(created_at) else {
        return fallback.to_string();
    };
    let utc_days = gregorian_to_days(utc_y, utc_m, utc_d);
    let utc_secs = utc_days * 86400 + (utc_h as i64) * 3600 + (utc_min as i64) * 60;
    let (_, lm, ld, lh, lmin, _) = utc_secs_to_local_parts(utc_secs);
    format!("{:02}-{:02} {:02}:{:02}", lm, ld, lh, lmin)
}

pub(crate) fn format_local_time_for_image_preview() -> String {
    let now_secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let (_, m, d, h, min, _) = utc_secs_to_local_parts(now_secs);
    format!("{:02}-{:02} {:02}:{:02}", m, d, h, min)
}
