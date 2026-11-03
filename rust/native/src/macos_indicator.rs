// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 appipinopi
//! Original compact/central mode-indicator content and frame via AppKit.
use std::ffi::c_void;
/// Return +1 NSView for ARC, or null if rejected. mode=automatic/direct/code 0/1/2.
#[no_mangle]
pub extern "C" fn meltype_macos_indicator_content(mode: u32, compact: i32) -> *mut c_void {
    if mode > 2 {
        return std::ptr::null_mut();
    }
    #[cfg(target_os = "macos")]
    unsafe {
        native::content(mode, compact != 0)
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = compact;
        std::ptr::null_mut()
    }
}
/// Main-thread SDK layout and caret validity. operation 0=caret validity,
/// 1=compact frame (caret required), 2=central frame. Return 1=success.
/// # Safety
/// caret/visible/output are null or readable/writable four-double rectangles.
#[no_mangle]
pub unsafe extern "C" fn meltype_macos_indicator_frame(
    operation: u32,
    caret: *const f64,
    visible: *const f64,
    output: *mut f64,
) -> i32 {
    #[cfg(target_os = "macos")]
    {
        native::frame(operation, caret, visible, output)
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (operation, caret, visible, output);
        0
    }
}
/// Create a +1 NSObject owning indicator lifecycle; enabled is queried at
/// deferred presentation time and must remain callable through owner destruction.
/// # Safety
/// enabled must not unwind and must be callable on the main thread.
#[no_mangle]
pub unsafe extern "C" fn meltype_macos_indicator_create(
    enabled: Option<unsafe extern "C" fn() -> i32>,
) -> *mut c_void {
    #[cfg(target_os = "macos")]
    {
        enabled.map_or(std::ptr::null_mut(), |enabled| native::create(enabled))
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = enabled;
        std::ptr::null_mut()
    }
}
/// Queue a show (operation 0), or invalidate/hide (1), on main thread.
/// # Safety
/// owner is a live native indicator. caret is null or four readable doubles.
#[no_mangle]
pub unsafe extern "C" fn meltype_macos_indicator_update(
    owner: *mut c_void,
    operation: u32,
    mode: u32,
    caret: *const f64,
) -> i32 {
    if owner.is_null() || operation > 1 || mode > 2 {
        return 0;
    }
    #[cfg(target_os = "macos")]
    {
        native::update(owner, operation, mode, caret)
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = caret;
        0
    }
}
/// Borrow/pin the owned panel for SDK inspection; return +1 or null.
/// # Safety
/// owner is a live native indicator; caller consumes returned +1 reference.
#[no_mangle]
pub unsafe extern "C" fn meltype_macos_indicator_panel(owner: *mut c_void) -> *mut c_void {
    #[cfg(target_os = "macos")]
    {
        native::panel(owner)
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = owner;
        std::ptr::null_mut()
    }
}
#[cfg(target_os = "macos")]
mod native {
    use super::*;
    use objc2::{
        msg_send,
        rc::{Allocated, Retained},
        runtime::{AnyClass, AnyObject},
        MainThreadMarker,
    };
    use objc2_foundation::{NSPoint, NSRect, NSSize, NSString};
    #[link(name = "AppKit", kind = "framework")]
    extern "C" {}
    #[link(name = "CoreGraphics", kind = "framework")]
    extern "C" {
        fn CGRectGetMinX(r: NSRect) -> f64;
        fn CGRectGetMinY(r: NSRect) -> f64;
        fn CGRectGetWidth(r: NSRect) -> f64;
        fn CGRectGetHeight(r: NSRect) -> f64;
    }
    unsafe fn class(name: &std::ffi::CStr) -> &'static AnyClass {
        AnyClass::get(name).expect("AppKit class unavailable")
    }
    unsafe fn label(text: &str, size: f64, weight: f64) -> Retained<AnyObject> {
        let text = NSString::from_str(text);
        let label: Retained<AnyObject> = msg_send![class(c"NSTextField"),labelWithString:&*text];
        let font: Retained<AnyObject> =
            msg_send![class(c"NSFont"),systemFontOfSize:size,weight:weight];
        let _: () = msg_send![&label,setFont:&*font];
        label
    }
    struct Layout {
        size: NSSize,
        rows: [NSRect; 3],
    }
    unsafe fn layout(compact: bool) -> Layout {
        if !compact {
            return Layout {
                size: NSSize::new(250.0, 180.0),
                rows: std::array::from_fn(|i| {
                    NSRect::new(
                        NSPoint::new(10.0, 118.0 - i as f64 * 50.0),
                        NSSize::new(230.0, 44.0),
                    )
                }),
            };
        }
        let mut sizes = [NSSize::new(0.0, 0.0); 3];
        for (i, text) in ["自動", "英数", "Code"].iter().enumerate() {
            let label = label(text, 12.0, 0.3);
            let _: () = msg_send![&label, sizeToFit];
            let frame: NSRect = msg_send![&label, frame];
            sizes[i] = NSSize::new(frame.size.width.ceil(), frame.size.height.ceil());
        }
        let height = sizes.iter().map(|s| s.height).fold(0.0f64, f64::max) + 4.0;
        let mut x = 2.0;
        let rows = std::array::from_fn(|i| {
            let row = NSRect::new(
                NSPoint::new(x, 2.0),
                NSSize::new(sizes[i].width + 4.0, height),
            );
            x += row.size.width + 6.0;
            row
        });
        Layout {
            size: NSSize::new(x - 6.0 + 2.0, height + 4.0),
            rows,
        }
    }
    unsafe fn view(class_name: &std::ffi::CStr, frame: NSRect) -> Retained<AnyObject> {
        let allocated: Allocated<AnyObject> = msg_send![class(class_name), alloc];
        msg_send![allocated,initWithFrame:frame]
    }
    pub unsafe fn content(mode: u32, compact: bool) -> *mut c_void {
        if MainThreadMarker::new().is_none() {
            return std::ptr::null_mut();
        }
        let layout = layout(compact);
        let background = view(
            c"NSVisualEffectView",
            NSRect::new(NSPoint::new(0.0, 0.0), layout.size),
        );
        let _: () = msg_send![&background,setMaterial:13isize];
        let _: () = msg_send![&background,setBlendingMode:0isize];
        let _: () = msg_send![&background,setState:1isize];
        let _: () = msg_send![&background,setWantsLayer:true];
        let layer: Option<Retained<AnyObject>> = msg_send![&background, layer];
        if let Some(layer) = layer {
            let _: () = msg_send![&layer,setCornerRadius:if compact{5.0f64}else{16.0}];
            let _: () = msg_send![&layer,setMasksToBounds:true];
        }
        let titles = [
            "日本語・英語の自動判定",
            "英数の直接入力",
            "コード入力 (コメント・文字列は日本語)",
        ];
        for (i, short) in ["自動", "英数", "Code"].iter().enumerate() {
            let selected = i as u32 == mode;
            let row = view(c"NSView", layout.rows[i]);
            let _: () = msg_send![&row,setAccessibilityElement:true];
            let role = NSString::from_str("AXRadioButton");
            let title = NSString::from_str(titles[i]);
            let _: () = msg_send![&row,setAccessibilityRole:&*role];
            let _: () = msg_send![&row,setAccessibilityLabel:&*title];
            let value: Retained<AnyObject> =
                msg_send![class(c"NSNumber"),numberWithInt:i32::from(selected)];
            let _: () = msg_send![&row,setAccessibilityValue:&*value];
            let _: () = msg_send![&row,setWantsLayer:true];
            let layer: Option<Retained<AnyObject>> = msg_send![&row, layer];
            if let Some(layer) = layer {
                let _: () = msg_send![&layer,setCornerRadius:if compact{3.0f64}else{10.0}];
                if selected {
                    let color: Retained<AnyObject> =
                        msg_send![class(c"NSColor"), controlAccentColor];
                    let cg_color: *const c_void = msg_send![&color, CGColor];
                    let _: () = msg_send![&layer,setBackgroundColor:cg_color];
                }
            }
            let label = label(
                short,
                if compact { 12.0 } else { 20.0 },
                if selected { 0.3 } else { 0.0 },
            );
            let color: Retained<AnyObject> = if selected {
                msg_send![class(c"NSColor"), whiteColor]
            } else {
                msg_send![class(c"NSColor"), labelColor]
            };
            let _: () = msg_send![&label,setTextColor:&*color];
            let _: () = msg_send![&label,setAlignment:1usize];
            let frame = if compact {
                NSRect::new(
                    NSPoint::new(2.0, 2.0),
                    NSSize::new(
                        layout.rows[i].size.width - 4.0,
                        layout.rows[i].size.height - 4.0,
                    ),
                )
            } else {
                NSRect::new(NSPoint::new(0.0, 10.0), NSSize::new(230.0, 28.0))
            };
            let _: () = msg_send![&label,setFrame:frame];
            let _: () = msg_send![&label,setAccessibilityElement:false];
            let _: () = msg_send![&row,addSubview:&*label];
            let _: () = msg_send![&background,addSubview:&*row];
        }
        if !compact {
            let caption = label("Meltype-FLASH", 11.0, 0.0);
            let color: Retained<AnyObject> = msg_send![class(c"NSColor"), secondaryLabelColor];
            let _: () = msg_send![&caption,setTextColor:&*color];
            let _: () = msg_send![&caption,setAlignment:1usize];
            let _: () = msg_send![&caption,setFrame:NSRect::new(NSPoint::new(10.0,160.0),NSSize::new(230.0,16.0))];
            let _: () = msg_send![&background,addSubview:&*caption];
        }
        Retained::into_raw(background).cast()
    }
    unsafe fn rect(p: *const f64) -> Option<NSRect> {
        if p.is_null() {
            None
        } else {
            let v = std::slice::from_raw_parts(p, 4);
            Some(NSRect::new(
                NSPoint::new(v[0], v[1]),
                NSSize::new(v[2], v[3]),
            ))
        }
    }
    pub unsafe fn frame(op: u32, caret: *const f64, visible: *const f64, output: *mut f64) -> i32 {
        if MainThreadMarker::new().is_none() {
            return 0;
        }
        let caret = rect(caret);
        if op == 0 {
            return i32::from(caret.is_some_and(|r| {
                CGRectGetMinX(r).is_finite()
                    && CGRectGetMinY(r).is_finite()
                    && CGRectGetWidth(r).is_finite()
                    && CGRectGetHeight(r).is_finite()
                    && CGRectGetWidth(r) >= 0.0
                    && CGRectGetHeight(r) > 0.0
            }));
        }
        if op != 1 && op != 2 || output.is_null() {
            return 0;
        }
        let Some(visible) = rect(visible) else {
            return 0;
        };
        let size = layout(op == 1).size;
        let (x, y) = if op == 1 {
            let Some(caret) = caret else {
                return 0;
            };
            (
                CGRectGetMinX(visible).max(
                    (CGRectGetMinX(caret) - size.width / 2.0)
                        .min(CGRectGetMinX(visible) + CGRectGetWidth(visible) - size.width),
                ),
                CGRectGetMinY(visible).max(
                    (CGRectGetMinY(caret) - size.height - 6.0)
                        .min(CGRectGetMinY(visible) + CGRectGetHeight(visible) - size.height),
                ),
            )
        } else {
            (
                CGRectGetMinX(visible) + CGRectGetWidth(visible) / 2.0 - size.width / 2.0,
                CGRectGetMinY(visible) + CGRectGetHeight(visible) / 2.0 - size.height / 2.0,
            )
        };
        std::ptr::copy_nonoverlapping([x, y, size.width, size.height].as_ptr(), output, 4);
        1
    }
    use objc2::{
        define_class,
        rc::Weak,
        runtime::{NSObject, NSObjectProtocol},
        sel, AnyThread, DefinedClass,
    };
    use std::cell::{Cell, RefCell};
    struct State {
        panel: RefCell<Option<Retained<AnyObject>>>,
        timer: RefCell<Option<Retained<AnyObject>>>,
        generation: Cell<u64>,
        enabled: unsafe extern "C" fn() -> i32,
    }
    define_class!(
        #[unsafe(super=NSObject)]
        #[ivars=State]
        struct Indicator;
        unsafe impl NSObjectProtocol for Indicator {}
    );
    impl Drop for Indicator {
        fn drop(&mut self) {
            if MainThreadMarker::new().is_some() {
                unsafe {
                    self.hide();
                }
            }
        }
    }
    struct TimerState {
        owner: Weak<Indicator>,
    }
    define_class!(
        #[unsafe(super=NSObject)]
        #[ivars=TimerState]
        struct TimerTarget;
        unsafe impl NSObjectProtocol for TimerTarget {}
        impl TimerTarget {
            #[unsafe(method(expire:))]
            unsafe fn expire(&self,_timer:*mut AnyObject) {
                if MainThreadMarker::new().is_some() {
                    if let Some(owner)=self.ivars().owner.load(){owner.hide();}
                }
            }
        }
    );
    struct Queued {
        owner: Weak<Indicator>,
        generation: u64,
        mode: u32,
        caret: Option<NSRect>,
    }
    #[link(name = "System")]
    extern "C" {
        static _dispatch_main_q: u8;
        fn dispatch_async_f(
            queue: *const c_void,
            context: *mut c_void,
            callback: unsafe extern "C" fn(*mut c_void),
        );
    }
    #[link(name = "CoreGraphics", kind = "framework")]
    extern "C" {
        fn CGRectContainsPoint(rect: NSRect, point: NSPoint) -> bool;
        fn CGRectIntersectsRect(a: NSRect, b: NSRect) -> bool;
    }
    #[link(name = "AppKit", kind = "framework")]
    extern "C" {
        fn NSMouseInRect(point: NSPoint, rect: NSRect, flipped: bool) -> bool;
    }
    unsafe extern "C" fn queued(context: *mut c_void) {
        let request = Box::from_raw(context.cast::<Queued>());
        if let Some(owner) = request.owner.load() {
            if owner.ivars().generation.get() == request.generation {
                owner.present(request.mode, request.caret, request.generation);
            }
        }
    }
    impl Indicator {
        unsafe fn cancel_timer(&self) {
            let timer = self.ivars().timer.borrow_mut().take();
            if let Some(timer) = timer {
                let _: () = msg_send![&timer, invalidate];
            }
        }
        unsafe fn hide(&self) {
            self.ivars()
                .generation
                .set(self.ivars().generation.get().wrapping_add(1));
            self.cancel_timer();
            let panel = self.ivars().panel.borrow().clone();
            if let Some(panel) = panel {
                let _: () = msg_send![&panel,orderOut:std::ptr::null::<AnyObject>()];
            }
        }
        unsafe fn present(&self, mode: u32, caret: Option<NSRect>, generation: u64) {
            let enabled = (self.ivars().enabled)() > 0;
            if self.ivars().generation.get() != generation {
                return;
            }
            if !enabled {
                self.hide();
                return;
            }
            let screens: Retained<objc2_foundation::NSArray<AnyObject>> =
                msg_send![class(c"NSScreen"), screens];
            let mouse: NSPoint = msg_send![class(c"NSEvent"), mouseLocation];
            let mut screen = None;
            for candidate in screens.iter() {
                let frame: NSRect = msg_send![&candidate, frame];
                let hit = if let Some(caret) = caret {
                    CGRectContainsPoint(frame, caret.origin) || CGRectIntersectsRect(frame, caret)
                } else {
                    NSMouseInRect(mouse, frame, false)
                };
                if hit {
                    screen = Some(candidate);
                    break;
                }
            }
            let Some(screen) = screen else {
                self.hide();
                return;
            };
            let visible: NSRect = msg_send![&screen, visibleFrame];
            let mut values = [0.0; 4];
            let c = caret.map(|r| [r.origin.x, r.origin.y, r.size.width, r.size.height]);
            let v = [
                visible.origin.x,
                visible.origin.y,
                visible.size.width,
                visible.size.height,
            ];
            frame(
                if caret.is_some() { 1 } else { 2 },
                c.as_ref().map_or(std::ptr::null(), |c| c.as_ptr()),
                v.as_ptr(),
                values.as_mut_ptr(),
            );
            let frame = NSRect::new(
                NSPoint::new(values[0], values[1]),
                NSSize::new(values[2], values[3]),
            );
            let previous = self.ivars().panel.borrow().clone();
            let panel = if let Some(panel) = previous {
                panel
            } else {
                let allocated: Allocated<AnyObject> = msg_send![class(c"NSPanel"), alloc];
                let panel: Retained<AnyObject> = msg_send![allocated,initWithContentRect:frame,styleMask:128usize,backing:2usize,defer:false];
                panel
            };
            let _: () = msg_send![&panel,setLevel:101isize];
            let _: () = msg_send![&panel,setOpaque:false];
            let color: Retained<AnyObject> = msg_send![class(c"NSColor"), clearColor];
            let _: () = msg_send![&panel,setBackgroundColor:&*color];
            let _: () = msg_send![&panel,setHasShadow:true];
            let _: () = msg_send![&panel,setIgnoresMouseEvents:true];
            let _: () = msg_send![&panel,setHidesOnDeactivate:false];
            let _: () = msg_send![&panel,setCollectionBehavior:257usize];
            let content =
                Retained::from_raw(content(mode, caret.is_some()).cast::<AnyObject>()).unwrap();
            let _: () = msg_send![&panel,setContentView:&*content];
            let _: () = msg_send![&panel,setFrame:frame,display:true];
            *self.ivars().panel.borrow_mut() = Some(panel.clone());
            let _: () = msg_send![&panel, orderFrontRegardless];
            let target = TimerTarget::alloc().set_ivars(TimerState {
                owner: Weak::new(self),
            });
            let target: Retained<TimerTarget> = msg_send![super(target), init];
            let timer: Retained<AnyObject> = msg_send![class(c"NSTimer"),scheduledTimerWithTimeInterval:1.0f64,target:&*target,selector:sel!(expire:),userInfo:std::ptr::null::<AnyObject>(),repeats:false];
            *self.ivars().timer.borrow_mut() = Some(timer);
        }
    }
    unsafe fn owner(pointer: *mut c_void) -> Option<Retained<Indicator>> {
        Retained::retain(pointer.cast::<AnyObject>())?
            .downcast::<Indicator>()
            .ok()
    }
    pub unsafe fn create(enabled: unsafe extern "C" fn() -> i32) -> *mut c_void {
        if MainThreadMarker::new().is_none() {
            return std::ptr::null_mut();
        }
        let this = Indicator::alloc().set_ivars(State {
            panel: RefCell::new(None),
            timer: RefCell::new(None),
            generation: Cell::new(0),
            enabled,
        });
        let result: Retained<Indicator> = msg_send![super(this), init];
        Retained::into_raw(result).cast()
    }
    pub unsafe fn update(
        pointer: *mut c_void,
        operation: u32,
        mode: u32,
        caret: *const f64,
    ) -> i32 {
        if MainThreadMarker::new().is_none() {
            return 0;
        }
        let Some(owner) = owner(pointer) else {
            return 0;
        };
        if operation == 1 {
            owner.hide();
            return 1;
        }
        owner.cancel_timer();
        let generation = owner.ivars().generation.get().wrapping_add(1);
        owner.ivars().generation.set(generation);
        let request = Box::new(Queued {
            owner: Weak::new(&owner),
            generation,
            mode,
            caret: rect(caret),
        });
        dispatch_async_f(
            std::ptr::addr_of!(_dispatch_main_q).cast(),
            Box::into_raw(request).cast(),
            queued,
        );
        1
    }
    pub unsafe fn panel(pointer: *mut c_void) -> *mut c_void {
        if MainThreadMarker::new().is_none() {
            return std::ptr::null_mut();
        }
        let Some(owner) = owner(pointer) else {
            return std::ptr::null_mut();
        };
        let panel = owner.ivars().panel.borrow().clone();
        panel.map_or(std::ptr::null_mut(), |panel| {
            Retained::into_raw(panel).cast()
        })
    }
}
