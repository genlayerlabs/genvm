pub mod interfaces;

#[repr(C)]
#[derive(Clone, Copy, PartialEq)]
pub struct Version {
    pub major: u32,
    pub minor: u32,
}

#[repr(C)]
pub struct SharedThreadPoolABI {
    pub ctx: *const (),
    pub submit_task: extern "C-unwind" fn(zelf: *const (), ctx: *const (), cb: extern "C-unwind" fn(ctx: *const ())),
}

#[repr(C)]
pub struct CtorArgs {
    pub version: Version, // first to be ABI compatible
    pub thread_pool: SharedThreadPoolABI,
    pub module_config: *const u8,
    pub module_config_len: usize,
}

impl CtorArgs {
    pub fn config(&self) -> Result<&str, std::str::Utf8Error> {
        std::str::from_utf8(unsafe { std::slice::from_raw_parts(self.module_config, self.module_config_len) })
    }
}

#[macro_export]
macro_rules! default_base_functions {
    ($api:tt, $name:ty) => {
        #[no_mangle]
        pub unsafe extern "C-unwind" fn ctor(args: &CtorArgs) -> *const () {
            if $api::VERSION != args.version {
                panic!("version mismatch");
            }
            match <$name>::try_new(args) {
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

pub struct SharedThreadPool {
    abi: SharedThreadPoolABI,
}

impl SharedThreadPool {
    pub fn new(abi: SharedThreadPoolABI) -> Self {
        Self { abi }
    }

    pub fn submit<F>(&self, f: F)
    where F: FnOnce() -> ()
    {
        let ctx = unsafe {
            let layout = std::alloc::Layout::new::<std::mem::MaybeUninit<F>>();
            let res: *mut std::mem::MaybeUninit<F> = std::alloc::alloc(layout).cast();
            (*res).write(f);
            res as *const ()
        };
        extern "C-unwind" fn run<F: FnOnce() -> ()>(ctx_ptr: *const ()) {
            let ctx = unsafe { std::ptr::read(ctx_ptr as *mut F) };
            let layout = std::alloc::Layout::new::<std::mem::MaybeUninit<F>>();
            unsafe { std::alloc::dealloc(ctx_ptr as *mut u8, layout); }
            // this call will drop the memory
            ctx();
        }
        (self.abi.submit_task)(self.abi.ctx, ctx, run::<F>);
    }
}
