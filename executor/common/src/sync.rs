mod darc {
    use std::marker::PhantomData;
    use std::ptr::NonNull;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct DArcControlBlock {
        ref_count: AtomicUsize,
        root_ptr: NonNull<()>,
        deleter: fn(*mut ()),
    }

    #[repr(C)]
    struct DArcControlWithData<T> {
        control_block: DArcControlBlock,
        data: T,
    }

    pub struct DArc<T>
    where
        T: 'static + ?Sized,
    {
        control_block: NonNull<DArcControlBlock>,
        actual_ptr: NonNull<T>,
        _phantom: PhantomData<T>,
    }

    pub struct DArcStruct<T> {
        control_block: NonNull<DArcControlBlock>,
        actual_data: std::mem::ManuallyDrop<T>,
    }

    impl<T> DArcStruct<&T> {
        pub fn into_arc(self) -> DArc<T> {
            let zelf = std::mem::ManuallyDrop::new(self);
            DArc {
                control_block: zelf.control_block,
                actual_ptr: unsafe {
                    std::ptr::NonNull::new_unchecked(*zelf.actual_data as *const T as *mut T)
                },
                _phantom: PhantomData,
            }
        }
    }

    impl<R, F> DArcStruct<F>
    where
        F: std::future::Future<Output = R>,
    {
        pub async fn await_inner(self) -> DArcStruct<R> {
            let mut zelf = std::mem::ManuallyDrop::new(self);
            // preserve dropping on panic
            let temp_arc = DArcStruct {
                control_block: zelf.control_block,
                actual_data: std::mem::ManuallyDrop::new(()),
            };
            let my_data = unsafe { std::mem::ManuallyDrop::take(&mut zelf.actual_data) };
            let new_data = my_data.await;

            let droppable_arc = std::mem::ManuallyDrop::new(temp_arc);

            DArcStruct {
                control_block: droppable_arc.control_block,
                actual_data: std::mem::ManuallyDrop::new(new_data),
            }
        }
    }

    impl<T, E> DArcStruct<core::result::Result<T, E>> {
        pub fn lift_result(self) -> core::result::Result<DArcStruct<T>, E> {
            let mut zelf = std::mem::ManuallyDrop::new(self);
            let zelf_data = unsafe { std::mem::ManuallyDrop::take(&mut zelf.actual_data) };
            match zelf_data {
                Ok(data) => Ok(DArcStruct {
                    control_block: zelf.control_block,
                    actual_data: std::mem::ManuallyDrop::new(data),
                }),
                Err(e) => {
                    std::mem::drop(DArcStruct {
                        control_block: zelf.control_block,
                        actual_data: std::mem::ManuallyDrop::new(()),
                    });
                    Err(e)
                }
            }
        }
    }

    impl<T> DArcStruct<Option<T>> {
        pub fn lift_option(self) -> Option<DArcStruct<T>> {
            let mut zelf = std::mem::ManuallyDrop::new(self);
            let zelf_data = unsafe { std::mem::ManuallyDrop::take(&mut zelf.actual_data) };
            match zelf_data {
                Some(data) => Some(DArcStruct {
                    control_block: zelf.control_block,
                    actual_data: std::mem::ManuallyDrop::new(data),
                }),
                None => {
                    std::mem::drop(DArcStruct {
                        actual_data: std::mem::ManuallyDrop::new(()),
                        control_block: zelf.control_block,
                    });
                    None
                }
            }
        }
    }

    impl<T> std::ops::Deref for DArcStruct<T> {
        type Target = T;

        fn deref(&self) -> &Self::Target {
            &self.actual_data
        }
    }

    impl<T> std::ops::Drop for DArcStruct<T> {
        fn drop(&mut self) {
            std::mem::drop(unsafe { std::mem::ManuallyDrop::take(&mut self.actual_data) });

            // SAFETY: control_block is valid
            let prev_count = unsafe {
                (*self.control_block.as_ptr())
                    .ref_count
                    .fetch_sub(1, Ordering::Release)
            };

            if prev_count == 1 {
                // This was the last reference
                std::sync::atomic::fence(Ordering::Acquire);

                // SAFETY: control_block is valid
                unsafe {
                    let control = &*self.control_block.as_ptr();
                    (control.deleter)(control.root_ptr.as_ptr());
                }
            }
        }
    }

    impl<T> serde::Serialize for DArc<T>
    where
        T: serde::Serialize + 'static,
    {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            let data_ref: &T = self;
            data_ref.serialize(serializer)
        }
    }

    impl<T> DArc<T>
    where
        T: 'static,
    {
        pub fn new(value: T) -> Self {
            let boxed = Box::new(DArcControlWithData {
                control_block: DArcControlBlock {
                    ref_count: AtomicUsize::new(1),
                    root_ptr: NonNull::dangling(), // Will be set below
                    deleter: Self::delete_control_with_data::<T>,
                },
                data: value,
            });

            let ptr = NonNull::from(Box::leak(boxed));

            // SAFETY: We just created this pointer and know it's valid
            unsafe {
                (*ptr.as_ptr()).control_block.root_ptr = ptr.cast();
            }

            // SAFETY: The control block is at the beginning of the struct
            let control_block = ptr.cast::<DArcControlBlock>();

            // SAFETY: We know the data field exists and is properly aligned
            let actual_ptr = unsafe { NonNull::new_unchecked(&mut (*ptr.as_ptr()).data as *mut T) };

            DArc {
                control_block,
                actual_ptr,
                _phantom: PhantomData,
            }
        }
    }

    impl<T> DArc<T>
    where
        T: 'static + ?Sized,
    {
        pub fn into_get_sub<'a, R>(self, getter: impl FnOnce(&'a T) -> R) -> DArcStruct<R>
        where
            R: 'a,
        {
            // SAFETY: actual_ptr is valid for the lifetime of self
            let data_ref = unsafe { self.actual_ptr.as_ref() };
            let sub = getter(data_ref);
            let cb = self.control_block;
            std::mem::forget(self);
            DArcStruct {
                control_block: cb,
                actual_data: std::mem::ManuallyDrop::new(sub),
            }
        }

        /// this function is unsound because forall 'a should be in function, but it's nearly impossible to type
        pub async fn into_get_sub_async<'a, R, F>(
            self,
            getter: impl FnOnce(&'a T) -> F,
        ) -> DArcStruct<R>
        where
            F: std::future::Future<Output = R>,
            R: 'a,
        {
            // SAFETY: actual_ptr is valid for the lifetime of self
            let data_ref = unsafe { self.actual_ptr.as_ref() };
            let sub = getter(data_ref).await;
            let cb = self.control_block;
            std::mem::forget(self);
            DArcStruct {
                control_block: cb,
                actual_data: std::mem::ManuallyDrop::new(sub),
            }
        }

        pub fn into_gep<R>(self, getter: impl for<'a> FnOnce(&'a T) -> &'a R) -> DArc<R>
        where
            R: 'static + ?Sized,
        {
            // SAFETY: actual_ptr is valid for the lifetime of self
            let data_ref = unsafe { self.actual_ptr.as_ref() };
            let derived_ref = getter(data_ref);

            // SAFETY: derived_ref is a reference to a subobject of T
            let derived_ptr = unsafe { NonNull::new_unchecked(derived_ref as *const R as *mut R) };

            let control_block = self.control_block;

            std::mem::forget(self);

            DArc {
                control_block,
                actual_ptr: derived_ptr,
                _phantom: PhantomData,
            }
        }

        /// forall 'a is placed incorrectly, but it's impossible to fix it here
        pub async fn into_gep_async<'a, R, F>(self, getter: impl FnOnce(&'a T) -> F) -> DArc<R>
        where
            R: 'static + ?Sized,
            F: std::future::Future<Output = &'a R>,
        {
            // SAFETY: actual_ptr is valid for the lifetime of self
            let data_ref = unsafe { self.actual_ptr.as_ref() };
            let derived_ref = getter(data_ref).await;

            // SAFETY: derived_ref is a reference to a subobject of T
            let derived_ptr = unsafe { NonNull::new_unchecked(derived_ref as *const R as *mut R) };

            let control_block = self.control_block;

            std::mem::forget(self);

            DArc {
                control_block,
                actual_ptr: derived_ptr,
                _phantom: PhantomData,
            }
        }

        /// Get a derived pointer to a field or subobject
        pub fn gep<R>(&self, getter: impl for<'a> FnOnce(&'a T) -> &'a R) -> DArc<R>
        where
            R: 'static + ?Sized,
        {
            self.clone().into_gep(getter)
        }

        /// Get the current reference count
        pub fn strong_count(&self) -> usize {
            // SAFETY: control_block is valid
            unsafe {
                (*self.control_block.as_ptr())
                    .ref_count
                    .load(Ordering::Relaxed)
            }
        }

        fn delete_control_with_data<U>(ptr: *mut ()) {
            // SAFETY: This function is only called with the correct type
            std::mem::drop(unsafe { Box::from_raw(ptr as *mut DArcControlWithData<U>) })
        }
    }

    impl<T> Clone for DArc<T>
    where
        T: 'static + ?Sized,
    {
        fn clone(&self) -> Self {
            // SAFETY: control_block is valid
            unsafe {
                (*self.control_block.as_ptr())
                    .ref_count
                    .fetch_add(1, Ordering::Relaxed);
            }

            DArc {
                control_block: self.control_block,
                actual_ptr: self.actual_ptr,
                _phantom: PhantomData,
            }
        }
    }

    impl<T> Drop for DArc<T>
    where
        T: 'static + ?Sized,
    {
        fn drop(&mut self) {
            // SAFETY: control_block is valid
            let prev_count = unsafe {
                (*self.control_block.as_ptr())
                    .ref_count
                    .fetch_sub(1, Ordering::Release)
            };

            if prev_count == 1 {
                // This was the last reference
                std::sync::atomic::fence(Ordering::Acquire);

                // SAFETY: control_block is valid
                unsafe {
                    let control = &*self.control_block.as_ptr();
                    (control.deleter)(control.root_ptr.as_ptr());
                }
            }
        }
    }

    impl<T> std::ops::Deref for DArc<T>
    where
        T: 'static + ?Sized,
    {
        type Target = T;

        fn deref(&self) -> &Self::Target {
            // SAFETY: actual_ptr is valid for the lifetime of self
            unsafe { self.actual_ptr.as_ref() }
        }
    }

    // SAFETY: DArc can be sent between threads if T can be
    unsafe impl<T: Send + Sync + 'static + ?Sized> Send for DArc<T> {}
    unsafe impl<T: Send + Sync + 'static + ?Sized> Sync for DArc<T> {}

    unsafe impl<T: Send + Sync> Send for DArcStruct<T> {}
    unsafe impl<T: Send + Sync> Sync for DArcStruct<T> {}
}

pub use darc::{DArc, DArcStruct};

struct WaiterInner {
    counter: std::sync::atomic::AtomicUsize,
    reached_zero: tokio::sync::Notify,
}

pub struct Waiter(std::sync::Arc<WaiterInner>);

impl Default for Waiter {
    fn default() -> Self {
        Self::new()
    }
}

impl Waiter {
    pub fn new() -> Self {
        Self(std::sync::Arc::new(WaiterInner {
            counter: std::sync::atomic::AtomicUsize::new(1),
            reached_zero: tokio::sync::Notify::new(),
        }))
    }

    pub fn increment(&self) {
        #[allow(unused_variables)]
        let old = self
            .0
            .counter
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        #[cfg(debug_assertions)]
        if old == 0 {
            panic!("Waiter incremented after reaching zero");
        }
    }

    pub fn decrement(&self) {
        let old_val = self
            .0
            .counter
            .fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
        if old_val == 1 {
            self.0.reached_zero.notify_one();
        }
        #[cfg(debug_assertions)]
        if old_val == 0 {
            panic!("Waiter decremented below zero");
        }
    }

    pub async fn wait(&self) {
        if self.0.counter.load(std::sync::atomic::Ordering::SeqCst) > 0 {
            self.0.reached_zero.notified().await;
        }
    }
}

pub struct CacheMap<T: 'static>(
    dashmap::DashMap<symbol_table::GlobalSymbol, DArc<tokio::sync::OnceCell<T>>>,
);

impl<T> CacheMap<T> {
    pub fn new() -> Self {
        Self(dashmap::DashMap::new())
    }

    pub async fn get_or_create<Err, F>(
        &self,
        key: symbol_table::GlobalSymbol,
        creator: impl FnOnce() -> F,
    ) -> Result<DArc<T>, Err>
    where
        F: std::future::Future<Output = Result<T, Err>>,
    {
        let entry = match self.0.entry(key) {
            dashmap::Entry::Occupied(occupied_entry) => occupied_entry.get().clone(),
            dashmap::Entry::Vacant(vacant_entry) => vacant_entry
                .insert(DArc::new(tokio::sync::OnceCell::new()))
                .clone(),
        };

        let res = entry
            .into_get_sub_async(|cell| cell.get_or_try_init(creator))
            .await
            .lift_result()?
            .into_arc();
        Ok(res)
    }
}

impl<T> Default for CacheMap<T> {
    fn default() -> Self {
        Self::new()
    }
}

pub struct Lock<T, Token>(T, Token);

impl<T, Token> Lock<T, Token> {
    pub fn new(value: T, tok: Token) -> Self {
        Self(value, tok)
    }

    pub fn into_inner(self) -> T {
        self.deconstruct().0
    }

    pub fn deconstruct(self) -> (T, Token) {
        let zelf = std::mem::ManuallyDrop::new(self);

        let fst = unsafe { std::ptr::read(&zelf.0) };
        let snd = unsafe { std::ptr::read(&zelf.1) };

        (fst, snd)
    }
}

impl<T, Token> std::ops::Deref for Lock<T, Token> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T, Token> std::ops::DerefMut for Lock<T, Token> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
