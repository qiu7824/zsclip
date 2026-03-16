use std::time::{SystemTime, UNIX_EPOCH};

pub(crate) fn gregorian_to_days(y: i32, m: i32, d: i32) -> i64 {
    let y = y as i64; let m = m as i64; let d = d as i64;
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

pub(crate) fn local_offset_secs() -> i64 {
    unsafe {
        #[repr(C)]
        struct TimeZoneInformation {
            bias: i32,
            _pad: [u8; 168],
        }
        #[link(name = "kernel32")]
        unsafe extern "system" {
            fn GetTimeZoneInformation(lptzi: *mut TimeZoneInformation) -> u32;
        }
        let mut tzi: TimeZoneInformation = core::mem::zeroed();
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
    let yoe = (doe - doe/1460 + doe/36524 - doe/146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365*yoe + yoe/4 - yoe/100);
    let mp = (5*doy + 2) / 153;
    let d = (doy - (153*mp + 2)/5 + 1) as i32;
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

pub(crate) fn format_created_at_local(created_at: &str, fallback: &str) -> String {
    if created_at.len() < 16 {
        return fallback.to_string();
    }
    let utc_y: i64 = created_at[..4].parse().unwrap_or(0);
    let utc_m: i64 = created_at[5..7].parse().unwrap_or(0);
    let utc_d: i64 = created_at[8..10].parse().unwrap_or(0);
    let utc_h: i64 = created_at[11..13].parse().unwrap_or(0);
    let utc_min: i64 = created_at[14..16].parse().unwrap_or(0);
    let utc_days = gregorian_to_days(utc_y as i32, utc_m as i32, utc_d as i32);
    let utc_secs = utc_days * 86400 + utc_h * 3600 + utc_min * 60;
    let local_secs = utc_secs + local_offset_secs();
    let (_, lm, ld, lh, lmin, _) = unix_secs_to_parts(local_secs);
    format!("{:02}-{:02} {:02}:{:02}", lm, ld, lh, lmin)
}

pub(crate) fn format_local_time_for_image_preview() -> String {
    let now_secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let local_secs = now_secs + local_offset_secs();
    let (_, m, d, h, min, _) = unix_secs_to_parts(local_secs);
    format!("{:02}-{:02} {:02}:{:02}", m, d, h, min)
}
