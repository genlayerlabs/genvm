pub mod interfaces;

#[repr(C)]
#[derive(Clone, Copy, PartialEq)]
pub struct Version {
    pub major: u32,
    pub minor: u32,
}

#[macro_export]
macro_rules! default_base_functions {
    ($api:tt, $name:ty) => {
        #[no_mangle]
        pub extern "C-unwind" fn check_version(v: *const Version) -> bool {
            return $api::VERSION == unsafe { *v };
        }

        #[no_mangle]
        pub unsafe extern "C-unwind" fn ctor(config: *const u8) -> *const () {
            let config = CStr::from_ptr(config as *const i8);
            match <$name>::try_new(config) {
                Ok(v) => {
                    let layout = std::alloc::Layout::new::<std::mem::MaybeUninit<$name>>();
                    let res: *mut std::mem::MaybeUninit<$name> = std::alloc::alloc(layout).cast();
                    (*res).write(v);
                    res as *const ()
                }
                Err(e) => {
                    eprintln!("{}\nbt: {}", e, e.backtrace());
                    panic!("couldn't initialize module");
                }
            }
        }

        #[no_mangle]
        pub unsafe extern "C-unwind" fn dtor(ptr: *const ()) {
            let ctx = get_ptr(ptr);
            std::ptr::drop_in_place(ctx);
            let layout = std::alloc::Layout::new::<std::mem::MaybeUninit<$name>>();
            std::alloc::dealloc(ptr as *mut u8, layout);
        }

        fn get_ptr(ptr: *const ()) -> &'static mut $name {
            unsafe {
                let ptr = ptr as *mut $name;
                return &mut *ptr;
            }
        }
    };
}

pub fn str_to_shared(s: &str) -> *const u8 {
    unsafe {
        let res = libc::malloc(s.len() + 1) as *mut u8;
        *res.add(s.len()) = 0;
        libc::memcpy(
            res as *mut std::ffi::c_void,
            s.as_ptr() as *const std::ffi::c_void,
            s.len(),
        );
        res as *const u8
    }
}
