#![allow(non_snake_case)]

use windows_sys::Win32::{
    Foundation::{HWND, LPARAM, POINT, RECT, WPARAM},
    UI::WindowsAndMessaging::{GUITHREADINFO, WM_IME_CONTROL},
};

use crate::app_core::{
    NativeImeCandidateAnchor, NativeImeCompositionAnchor, NativeImeHost, Point, UiRect,
};
use crate::platform::{input as platform_input, process as platform_process, window as platform_window};

#[link(name = "user32")]
unsafe extern "system" {
    fn GetKeyboardLayout(idThread: u32) -> isize;
}

#[link(name = "imm32")]
unsafe extern "system" {
    fn ImmGetContext(hwnd: HWND) -> isize;
    fn ImmReleaseContext(hwnd: HWND, context: isize) -> i32;
    fn ImmGetOpenStatus(context: isize) -> i32;
    fn ImmGetConversionStatus(context: isize, conversion: *mut u32, sentence: *mut u32) -> i32;
    fn ImmIsIME(layout: isize) -> i32;
    fn ImmGetCompositionStringW(context: isize, index: u32, buffer: *mut core::ffi::c_void, bytes: u32) -> i32;
    fn ImmNotifyIME(context: isize, action: u32, index: u32, value: u32) -> i32;
}

const IMC_GETCANDIDATEPOS: WPARAM = 0x0007;
const IMC_GETCOMPOSITIONWINDOW: WPARAM = 0x000B;
const IMC_GETCONVERSIONMODE: WPARAM = 0x0001;
const IMC_GETOPENSTATUS: WPARAM = 0x0005;
const CFS_RECT_V: u32 = 0x0001;
const CFS_POINT_V: u32 = 0x0002;
const CFS_FORCE_POSITION_V: u32 = 0x0020;
const CFS_CANDIDATEPOS_V: u32 = 0x0040;
const CFS_EXCLUDE_V: u32 = 0x0080;
const IME_CMODE_NATIVE_V: u32 = 0x0001;
const IME_CONVERSION_KNOWN_FLAGS: u32 = 0x0FFB;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum WindowsImeInputMode {
    Native,
    Alphanumeric,
    Unknown,
}

#[cfg(test)]
thread_local! {
    static TEST_IME_OBSERVATION: std::cell::Cell<Option<(WindowsImeInputMode, bool)>> = const { std::cell::Cell::new(None) };
}

#[cfg(test)]
pub(crate) fn with_test_ime_observation<T>(mode: WindowsImeInputMode, cancelled: bool, action: impl FnOnce() -> T) -> T {
    struct Restore(Option<(WindowsImeInputMode, bool)>);
    impl Drop for Restore {
        fn drop(&mut self) { TEST_IME_OBSERVATION.with(|value| value.set(self.0)); }
    }
    let _restore = Restore(TEST_IME_OBSERVATION.with(|value| value.replace(Some((mode, cancelled)))));
    action()
}

#[repr(C)]
struct CandidateForm {
    dwIndex: u32,
    dwStyle: u32,
    ptCurrentPos: POINT,
    rcArea: RECT,
}

#[repr(C)]
struct CompositionForm {
    dwStyle: u32,
    ptCurrentPos: POINT,
    rcArea: RECT,
}

#[derive(Clone, Copy, Default)]
pub(crate) struct WindowsImeHost;

impl WindowsImeHost {
    pub(crate) const fn new() -> Self {
        Self
    }

    /// Cancel only the exact composition owned by VV, never a generic editor Escape.
    pub(crate) fn cancel_exact_vv_composition(self, focus: HWND) -> bool {
        #[cfg(test)]
        if let Some((_, cancelled)) = TEST_IME_OBSERVATION.with(|value| value.get()) {
            return cancelled;
        }
        let context = unsafe { ImmGetContext(focus) };
        if context == 0 { return false; }
        let length = unsafe { ImmGetCompositionStringW(context, 8, core::ptr::null_mut(), 0) };
        if length != 4 { unsafe { ImmReleaseContext(focus, context); } return false; }
        let mut text = [0u16; 2];
        let bytes = unsafe { ImmGetCompositionStringW(context, 8, text.as_mut_ptr().cast(), 4) };
        let exact = bytes == 4 && text == [b'v' as u16, b'v' as u16];
        let cancelled = exact && unsafe { ImmNotifyIME(context, 0x15, 0x4, 0) } != 0;
        unsafe { ImmReleaseContext(focus, context); }
        cancelled
    }

    pub(crate) fn input_mode(self, focus: HWND) -> WindowsImeInputMode {
        #[cfg(test)]
        if let Some((mode, _)) = TEST_IME_OBSERVATION.with(|value| value.get()) {
            return mode;
        }
        if !platform_window::exists(focus) {
            return WindowsImeInputMode::Unknown;
        }
        let thread_id = platform_window::window_thread_id(focus);
        if thread_id != 0 {
            let layout = unsafe { GetKeyboardLayout(thread_id) };
            if layout != 0 && unsafe { ImmIsIME(layout) } == 0 {
                return WindowsImeInputMode::Alphanumeric;
            }
        }

        let context = unsafe { ImmGetContext(focus) };
        if context == 0 {
            return ime_window_input_mode(focus);
        }
        let open = unsafe { ImmGetOpenStatus(context) } != 0;
        let mut conversion = 0u32;
        let mut sentence = 0u32;
        let conversion_known =
            unsafe { ImmGetConversionStatus(context, &mut conversion, &mut sentence) } != 0;
        unsafe {
            ImmReleaseContext(focus, context);
        }

        if open && !conversion_known {
            ime_window_input_mode(focus)
        } else {
            classify_windows_ime_input_mode(open, conversion_known, conversion)
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ImeModeQueryIdentity {
    focus: usize,
    ime: usize,
    thread: u32,
    process: u32,
    foreground: usize,
    foreground_thread: u32,
    foreground_process: u32,
    layout: isize,
}

fn ime_mode_query_identity(focus: HWND) -> Option<ImeModeQueryIdentity> {
    if !platform_window::exists(focus) {return None;}
    let (thread,process)=platform_window::window_thread_process_id(focus);
    if thread==0 || process==0 {return None;}
    let foreground=platform_window::foreground();
    let (foreground_thread,foreground_process)=platform_window::window_thread_process_id(foreground);
    if foreground_thread==0 || foreground_process==0 {return None;}
    let mut info: GUITHREADINFO=unsafe {core::mem::zeroed()};
    info.cbSize=core::mem::size_of::<GUITHREADINFO>() as u32;
    if !platform_window::gui_thread_info(foreground_thread,&mut info) || info.hwndFocus!=focus {return None;}
    let ime=platform_input::default_ime_window(focus);
    if !platform_window::exists(ime) || !platform_window::class_name(ime).eq_ignore_ascii_case("IME")
        || platform_window::window_thread_process_id(ime)!=(thread,process) {return None;}
    let layout=unsafe {GetKeyboardLayout(thread)};
    if layout==0 {return None;}
    Some(ImeModeQueryIdentity {focus:focus as usize,ime:ime as usize,thread,process,
        foreground:foreground as usize,foreground_thread,foreground_process,layout})
}

fn read_ime_window_mode(
    mut query: impl FnMut(WPARAM) -> Option<isize>,
    mut identity_is_current: impl FnMut() -> bool,
) -> WindowsImeInputMode {
    // Zero is also the default reply from IMEs that do not implement these
    // compatibility queries. Two explicit open replies and matching conversion
    // replies are required, since Shift can switch mode without changing HKL.
    if !identity_is_current() || query(IMC_GETOPENSTATUS)!=Some(1) || !identity_is_current() {
        return WindowsImeInputMode::Unknown;
    }
    let Some(conversion)=query(IMC_GETCONVERSIONMODE).and_then(|value|u32::try_from(value).ok()) else {
        return WindowsImeInputMode::Unknown;
    };
    if conversion & !IME_CONVERSION_KNOWN_FLAGS != 0 {return WindowsImeInputMode::Unknown;}
    if !identity_is_current() || query(IMC_GETOPENSTATUS)!=Some(1) || !identity_is_current() {
        return WindowsImeInputMode::Unknown;
    }
    let repeated=query(IMC_GETCONVERSIONMODE).and_then(|value|u32::try_from(value).ok());
    if repeated!=Some(conversion) || !identity_is_current() {return WindowsImeInputMode::Unknown;}
    classify_windows_ime_input_mode(true,true,conversion)
}

fn query_ime_window_mode(ime: HWND, identity_is_current: impl FnMut() -> bool) -> WindowsImeInputMode {
    // Primitive return values only: no process-local HIMC or pointer-bearing
    // IME structure crosses the boundary, and each external call has a 50ms cap.
    read_ime_window_mode(|command|platform_window::send_message_bounded(ime,WM_IME_CONTROL,command,0),identity_is_current)
}

fn ime_window_input_mode(focus: HWND) -> WindowsImeInputMode {
    let Some(identity)=ime_mode_query_identity(focus) else {return WindowsImeInputMode::Unknown;};
    query_ime_window_mode(identity.ime as HWND,||ime_mode_query_identity(focus)==Some(identity))
}

fn classify_windows_ime_input_mode(
    open: bool,
    conversion_known: bool,
    conversion: u32,
) -> WindowsImeInputMode {
    if !open {
        WindowsImeInputMode::Alphanumeric
    } else if !conversion_known {
        WindowsImeInputMode::Unknown
    } else if conversion & IME_CMODE_NATIVE_V != 0 {
        WindowsImeInputMode::Native
    } else {
        WindowsImeInputMode::Alphanumeric
    }
}

fn ime_anchor_query_on_current_thread(focus_thread: u32, current_thread: u32) -> bool {
    focus_thread != 0 && focus_thread == current_thread
}

impl NativeImeHost for WindowsImeHost {
    type Handle = HWND;

    fn candidate_anchor(
        &mut self,
        focus: Self::Handle,
        index: u32,
    ) -> Option<NativeImeCandidateAnchor> {
        if !platform_window::exists(focus)
            || !ime_anchor_query_on_current_thread(platform_window::window_thread_id(focus), platform_process::current_thread_id())
        {
            return None;
        }
        // External IME windows can wait on their input thread indefinitely. The
        // caller already has asynchronous accessibility/native-caret fallbacks.
        let ime = platform_input::default_ime_window(focus);
        if !platform_window::exists(ime) {
            return None;
        }

        let mut candidate = CandidateForm {
            dwIndex: index,
            dwStyle: 0,
            ptCurrentPos: POINT { x: 0, y: 0 },
            rcArea: empty_rect(),
        };
        if platform_window::send_message(
            ime,
            WM_IME_CONTROL,
            IMC_GETCANDIDATEPOS,
            &mut candidate as *mut _ as LPARAM,
        ) != 0
        {
            return None;
        }

        match candidate.dwStyle {
            CFS_CANDIDATEPOS_V => point_to_screen(focus, candidate.ptCurrentPos)
                .map(|position| NativeImeCandidateAnchor::CandidatePoint { position }),
            CFS_EXCLUDE_V if rect_has_area(&candidate.rcArea) => {
                rect_to_screen(focus, candidate.rcArea)
                    .map(|rect| NativeImeCandidateAnchor::ExcludeRect { rect })
            }
            _ => None,
        }
    }

    fn composition_anchor(&mut self, focus: Self::Handle) -> Option<NativeImeCompositionAnchor> {
        if !platform_window::exists(focus)
            || !ime_anchor_query_on_current_thread(platform_window::window_thread_id(focus), platform_process::current_thread_id())
        {
            return None;
        }
        let ime = platform_input::default_ime_window(focus);
        if !platform_window::exists(ime) {
            return None;
        }

        let mut composition = CompositionForm {
            dwStyle: 0,
            ptCurrentPos: POINT { x: 0, y: 0 },
            rcArea: empty_rect(),
        };
        if platform_window::send_message(
            ime,
            WM_IME_CONTROL,
            IMC_GETCOMPOSITIONWINDOW,
            &mut composition as *mut _ as LPARAM,
        ) != 0
        {
            return None;
        }

        match composition.dwStyle {
            CFS_POINT_V | CFS_FORCE_POSITION_V => point_to_screen(focus, composition.ptCurrentPos)
                .map(|position| NativeImeCompositionAnchor::Point { position }),
            CFS_RECT_V if rect_has_area(&composition.rcArea) => {
                rect_to_screen(focus, composition.rcArea)
                    .map(|rect| NativeImeCompositionAnchor::Rect { rect })
            }
            _ => None,
        }
    }

    fn has_default_ime_window(&mut self, focus: Self::Handle) -> bool {
        platform_window::exists(platform_input::default_ime_window(focus))
    }
}

const fn empty_rect() -> RECT {
    RECT {
        left: 0,
        top: 0,
        right: 0,
        bottom: 0,
    }
}

fn rect_has_area(rect: &RECT) -> bool {
    rect.right > rect.left && rect.bottom > rect.top
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scalar_ime_mode_requires_two_explicit_open_replies_and_valid_conversion_flags() {
        fn read(replies: &[Option<isize>]) -> (WindowsImeInputMode, Vec<WPARAM>) {
            let mut replies=replies.iter().copied();
            let mut commands=Vec::new();
            let mode=read_ime_window_mode(|command| {commands.push(command);replies.next().unwrap_or(None)},||true);
            (mode,commands)
        }
        for (conversion,expected) in [(0,WindowsImeInputMode::Alphanumeric),(0x8,WindowsImeInputMode::Alphanumeric),
            (1,WindowsImeInputMode::Native),(1025,WindowsImeInputMode::Native)] {
            let (mode,commands)=read(&[Some(1),Some(conversion),Some(1),Some(conversion)]);
            assert_eq!(mode,expected);
            assert_eq!(commands,[IMC_GETOPENSTATUS,IMC_GETCONVERSIONMODE,IMC_GETOPENSTATUS,IMC_GETCONVERSIONMODE]);
        }
        for replies in [
            [Some(0),Some(0),Some(0),Some(0)], [None,Some(0),Some(1),Some(0)], [Some(2),Some(0),Some(1),Some(0)],
            [Some(1),None,Some(1),Some(0)], [Some(1),Some(0),None,Some(0)], [Some(1),Some(0),Some(0),Some(0)],
            [Some(1),Some(-1),Some(1),Some(-1)], [Some(1),Some(u32::MAX as isize),Some(1),Some(u32::MAX as isize)],
            [Some(1),Some(4),Some(1),Some(4)], [Some(1),Some(0x1000),Some(1),Some(0x1000)],
            [Some(1),Some(0),Some(1),Some(1025)], [Some(1),Some(1025),Some(1),Some(0)], [Some(1),Some(0),Some(1),None],
        ] {assert_eq!(read(&replies).0,WindowsImeInputMode::Unknown,"replies={replies:?}");}
    }

    #[test]
    fn scalar_ime_mode_rejects_identity_changes_at_each_message_boundary() {
        for changed_at in 0..5 {
            let mut observations=0;
            let mode=read_ime_window_mode(|command|Some(if command==IMC_GETOPENSTATUS {1}else{0}),|| {
                let stable=observations!=changed_at;observations+=1;stable
            });
            assert_eq!(mode,WindowsImeInputMode::Unknown,"identity check {changed_at}");
        }
    }

    #[test]
    fn scalar_ime_mode_queries_a_real_foreign_thread_window_and_times_out() {
        use std::sync::{Arc, Condvar, Mutex, mpsc, atomic::{AtomicBool, AtomicIsize, AtomicUsize, Ordering}};
        use std::time::{Duration, Instant};
        use windows_sys::Win32::Foundation::LRESULT;
        use windows_sys::Win32::UI::WindowsAndMessaging::{CREATESTRUCTW, CreateWindowExW, DefWindowProcW, DestroyWindow,
            DispatchMessageW, GetMessageW, MSG, PostMessageW, PostQuitMessage, RegisterClassExW, UnregisterClassW,
            WM_CLOSE, WM_NCCREATE, WNDCLASSEXW, WS_POPUP};
        struct Context {open:AtomicIsize,conversion:AtomicIsize,block:AtomicBool,calls:AtomicUsize,
            pointer_seen:AtomicBool,release:Mutex<bool>,wake:Condvar}
        unsafe extern "system" fn procedure(hwnd:HWND,msg:u32,wp:WPARAM,lp:LPARAM)->LRESULT {
            if msg==WM_NCCREATE {let create=&*(lp as *const CREATESTRUCTW);platform_window::set_user_data(hwnd,create.lpCreateParams as isize);return 1;}
            if msg==WM_IME_CONTROL {
                let context=&*(platform_window::user_data(hwnd) as *const Context);
                context.calls.fetch_add(1,Ordering::SeqCst);
                if lp!=0 {context.pointer_seen.store(true,Ordering::SeqCst);}
                if context.block.load(Ordering::SeqCst) {
                    let release=context.release.lock().unwrap();
                    drop(context.wake.wait_timeout_while(release,Duration::from_secs(2),|released|!*released).unwrap());
                }
                return if wp==IMC_GETOPENSTATUS {context.open.load(Ordering::SeqCst)}
                    else if wp==IMC_GETCONVERSIONMODE {context.conversion.load(Ordering::SeqCst)}else{-1};
            }
            if msg==WM_CLOSE {DestroyWindow(hwnd);PostQuitMessage(0);return 0;}
            DefWindowProcW(hwnd,msg,wp,lp)
        }
        struct Fixture {hwnd:usize,context:Arc<Context>,worker:Option<std::thread::JoinHandle<()>>}
        impl Drop for Fixture {
            fn drop(&mut self) {
                *self.context.release.lock().unwrap()=true;self.context.wake.notify_all();
                unsafe {PostMessageW(self.hwnd as HWND,WM_CLOSE,0,0);}
                if let Some(worker)=self.worker.take() {let _=worker.join();}
            }
        }
        let context=Arc::new(Context {open:AtomicIsize::new(1),conversion:AtomicIsize::new(0),block:AtomicBool::new(false),
            calls:AtomicUsize::new(0),pointer_seen:AtomicBool::new(false),release:Mutex::new(false),wake:Condvar::new()});
        let worker_context=context.clone();
        let (ready,received)=mpsc::channel();
        let worker=std::thread::spawn(move||unsafe {
            let class=crate::platform::string::to_wide(&format!("ZSClipImeScalarFixture{}",platform_process::current_thread_id()));
            let instance=platform_window::module_handle();
            let definition=WNDCLASSEXW {cbSize:core::mem::size_of::<WNDCLASSEXW>() as u32,lpfnWndProc:Some(procedure),
                hInstance:instance,lpszClassName:class.as_ptr(),..core::mem::zeroed()};
            assert_ne!(RegisterClassExW(&definition),0);
            let hwnd=CreateWindowExW(0,class.as_ptr(),class.as_ptr(),WS_POPUP,0,0,10,10,core::ptr::null_mut(),core::ptr::null_mut(),instance,Arc::as_ptr(&worker_context).cast());
            ready.send(hwnd as usize).unwrap();
            if !hwnd.is_null() {let mut message:MSG=core::mem::zeroed();while GetMessageW(&mut message,core::ptr::null_mut(),0,0)>0 {DispatchMessageW(&message);}}
            UnregisterClassW(class.as_ptr(),instance);
        });
        let fixture=Fixture {hwnd:received.recv_timeout(Duration::from_secs(3)).unwrap(),context,worker:Some(worker)};
        assert_ne!(fixture.hwnd,0);
        let hwnd=fixture.hwnd as HWND;
        let identity=platform_window::window_thread_process_id(hwnd);
        assert_ne!(identity.0,platform_process::current_thread_id());
        let current=||platform_window::exists(hwnd)&&platform_window::window_thread_process_id(hwnd)==identity;
        // The system's reserved IME class is not impersonated. This exercises
        // the production bounded scalar transport, not a third-party IME.
        assert_eq!(query_ime_window_mode(hwnd,current),WindowsImeInputMode::Alphanumeric);
        fixture.context.conversion.store(1025,Ordering::SeqCst);
        assert_eq!(query_ime_window_mode(hwnd,current),WindowsImeInputMode::Native);
        fixture.context.open.store(0,Ordering::SeqCst);
        assert_eq!(query_ime_window_mode(hwnd,current),WindowsImeInputMode::Unknown);
        fixture.context.open.store(1,Ordering::SeqCst);
        fixture.context.conversion.store(-1,Ordering::SeqCst);
        assert_eq!(query_ime_window_mode(hwnd,current),WindowsImeInputMode::Unknown);
        fixture.context.block.store(true,Ordering::SeqCst);
        let started=Instant::now();
        assert_eq!(query_ime_window_mode(hwnd,current),WindowsImeInputMode::Unknown);
        assert!(started.elapsed()<Duration::from_millis(500),"Scalar IME query waited beyond its bounded timeout");
        assert!(!fixture.context.pointer_seen.load(Ordering::SeqCst));
        assert!(fixture.context.calls.load(Ordering::SeqCst)>=8);
        drop(fixture);
        assert_eq!(query_ime_window_mode(hwnd,||false),WindowsImeInputMode::Unknown);
    }

    #[test]
    #[ignore = "Read-only live foreground/focus IME probe; logs only window IDs, mode and scalar state, never composition or clipboard content"]
    fn live_foreground_ime_mode_probe() {
        let foreground=platform_window::foreground();
        let thread=platform_window::window_thread_id(foreground);
        let mut info:GUITHREADINFO=unsafe {core::mem::zeroed()};info.cbSize=core::mem::size_of::<GUITHREADINFO>() as u32;
        assert!(platform_window::gui_thread_info(thread,&mut info));
        let focus=info.hwndFocus;
        assert!(platform_window::exists(focus));
        let identity=ime_mode_query_identity(focus);
        let direct=unsafe {ImmGetContext(focus)};
        if direct!=0 {unsafe {ImmReleaseContext(focus,direct);}}
        let mode=WindowsImeHost::new().input_mode(focus);
        let scalar=identity.map(|identity| {
            let ime=identity.ime as HWND;
            [platform_window::send_message_bounded(ime,WM_IME_CONTROL,IMC_GETOPENSTATUS,0),
             platform_window::send_message_bounded(ime,WM_IME_CONTROL,IMC_GETCONVERSIONMODE,0),
             platform_window::send_message_bounded(ime,WM_IME_CONTROL,IMC_GETOPENSTATUS,0),
             platform_window::send_message_bounded(ime,WM_IME_CONTROL,IMC_GETCONVERSIONMODE,0)]
        });
        eprintln!("live IME identity={identity:?} direct_context={} mode={mode:?} scalar={scalar:?}",direct!=0);
        if let Ok(expected)=std::env::var("ZSCLIP_TEST_EXPECT_IME_MODE") {
            let expected=match expected.as_str() {"alphanumeric"=>WindowsImeInputMode::Alphanumeric,"native"=>WindowsImeInputMode::Native,"unknown"=>WindowsImeInputMode::Unknown,_=>panic!("Unsupported expected IME mode")};
            assert_eq!(mode,expected);
        }
    }

    #[test]
    fn ime_anchor_queries_reject_unknown_and_foreign_threads() {
        assert!(ime_anchor_query_on_current_thread(7, 7));
        assert!(!ime_anchor_query_on_current_thread(7, 8));
        assert!(!ime_anchor_query_on_current_thread(0, 7));
        assert!(!ime_anchor_query_on_current_thread(0, 0));
    }

    #[test]
    fn foreign_thread_anchor_does_not_wait_for_a_blocked_window() {
        use std::sync::{Arc, Condvar, Mutex, mpsc, atomic::{AtomicUsize, Ordering}};
        use std::time::{Duration, Instant};
        use windows_sys::Win32::Foundation::LRESULT;
        use windows_sys::Win32::UI::WindowsAndMessaging::{
            CREATESTRUCTW, CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW,
            GetMessageW, MSG, PostMessageW, PostQuitMessage, RegisterClassExW, UnregisterClassW,
            WM_CLOSE, WM_NCCREATE, WNDCLASSEXW, WS_POPUP,
        };
        struct Context {
            gate: Arc<(Mutex<bool>, Condvar)>,
            entered: mpsc::Sender<()>,
            messages: Arc<AtomicUsize>,
        }
        unsafe extern "system" fn procedure(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
            if msg == WM_NCCREATE {
                let create=&*(lp as *const CREATESTRUCTW);
                platform_window::set_user_data(hwnd,create.lpCreateParams as isize);
                return 1;
            }
            if msg == 0x8000 + 99 {
                return platform_window::send_message(hwnd,WM_IME_CONTROL,0,0);
            }
            if msg == WM_IME_CONTROL {
                let context=&*(platform_window::user_data(hwnd) as *const Context);
                context.messages.fetch_add(1,Ordering::SeqCst);
                let _=context.entered.send(());
                let (release,wake)=&*context.gate;
                let guard=release.lock().unwrap();
                // A finite watchdog also lets a regression fail instead of hanging the test process.
                drop(wake.wait_timeout_while(guard,Duration::from_secs(2),|released|!*released).unwrap());
                return 0;
            }
            if msg == WM_CLOSE {DestroyWindow(hwnd);PostQuitMessage(0);return 0;}
            DefWindowProcW(hwnd,msg,wp,lp)
        }
        struct Fixture {
            hwnd: usize,
            gate: Arc<(Mutex<bool>,Condvar)>,
            worker: Option<std::thread::JoinHandle<()>>,
        }
        impl Drop for Fixture {
            fn drop(&mut self) {
                let (released,wake)=&*self.gate;
                *released.lock().unwrap()=true;
                wake.notify_all();
                unsafe {PostMessageW(self.hwnd as HWND,WM_CLOSE,0,0);}
                if let Some(worker)=self.worker.take() {let _=worker.join();}
            }
        }
        let gate=Arc::new((Mutex::new(false),Condvar::new()));
        let messages=Arc::new(AtomicUsize::new(0));
        let (ready_tx,ready_rx)=mpsc::channel();
        let (entered_tx,entered_rx)=mpsc::channel();
        let context=Context {gate:gate.clone(),entered:entered_tx,messages:messages.clone()};
        let worker=std::thread::spawn(move||unsafe {
            let mut context=Box::new(context);
            let class=crate::platform::string::to_wide(&format!("ZSClipImeAnchorFixture{}",platform_process::current_thread_id()));
            let instance=platform_window::module_handle();
            let wc=WNDCLASSEXW {cbSize:core::mem::size_of::<WNDCLASSEXW>() as u32,lpfnWndProc:Some(procedure),hInstance:instance,lpszClassName:class.as_ptr(),..core::mem::zeroed()};
            assert_ne!(RegisterClassExW(&wc),0);
            let hwnd=CreateWindowExW(0,class.as_ptr(),class.as_ptr(),WS_POPUP,0,0,10,10,core::ptr::null_mut(),core::ptr::null_mut(),instance,(&mut *context as *mut Context).cast());
            ready_tx.send(hwnd as usize).unwrap();
            if !hwnd.is_null() {
                let mut message:MSG=core::mem::zeroed();
                while GetMessageW(&mut message,core::ptr::null_mut(),0,0)>0 {DispatchMessageW(&message);}
            }
            UnregisterClassW(class.as_ptr(),instance);
        });
        let fixture=Fixture {hwnd:ready_rx.recv_timeout(Duration::from_secs(3)).unwrap(),gate,worker:Some(worker)};
        assert_ne!(fixture.hwnd,0);
        assert_ne!(platform_window::window_thread_id(fixture.hwnd as HWND),platform_process::current_thread_id());
        assert_ne!(unsafe {PostMessageW(fixture.hwnd as HWND,0x8000+99,0,0)},0);
        entered_rx.recv_timeout(Duration::from_secs(3)).unwrap();
        let start=Instant::now();
        let mut host=WindowsImeHost::new();
        assert!(host.candidate_anchor(fixture.hwnd as HWND,0).is_none());
        assert!(host.composition_anchor(fixture.hwnd as HWND).is_none());
        assert!(start.elapsed()<Duration::from_millis(500),"Foreign-thread anchor lookup waited for the blocked window");
        drop(fixture);
        assert_eq!(messages.load(Ordering::SeqCst),1,"Only the fixture's deliberate blocking message is allowed");
    }

    #[test]
    fn ime_input_mode_distinguishes_native_and_english_states() {
        assert_eq!(
            classify_windows_ime_input_mode(false, true, IME_CMODE_NATIVE_V),
            WindowsImeInputMode::Alphanumeric
        );
        assert_eq!(
            classify_windows_ime_input_mode(true, true, IME_CMODE_NATIVE_V),
            WindowsImeInputMode::Native
        );
        assert_eq!(
            classify_windows_ime_input_mode(true, true, 0),
            WindowsImeInputMode::Alphanumeric
        );
        assert_eq!(
            classify_windows_ime_input_mode(true, false, 0),
            WindowsImeInputMode::Unknown
        );
    }
}

fn point_to_screen(hwnd: HWND, mut point: POINT) -> Option<Point> {
    if platform_window::client_to_screen(hwnd, &mut point) {
        Some(Point {
            x: point.x,
            y: point.y,
        })
    } else {
        None
    }
}

fn rect_to_screen(hwnd: HWND, rect: RECT) -> Option<UiRect> {
    let mut top_left = POINT {
        x: rect.left,
        y: rect.top,
    };
    let mut bottom_right = POINT {
        x: rect.right,
        y: rect.bottom,
    };
    if platform_window::client_to_screen(hwnd, &mut top_left)
        && platform_window::client_to_screen(hwnd, &mut bottom_right)
    {
        Some(UiRect::new(
            top_left.x,
            top_left.y,
            bottom_right.x,
            bottom_right.y,
        ))
    } else {
        None
    }
}
