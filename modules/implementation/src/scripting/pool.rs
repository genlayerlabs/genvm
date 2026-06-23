use std::sync::Arc;

struct Inner<T, C, E: 'static> {
    free: crossbeam::queue::ArrayQueue<Arc<super::UserVM<T, C, E>>>,
}

/// A fixed-size pool of Lua VMs. A VM is checked out exclusively via [`Pool::get`]
/// and returned to the pool when the [`PoolGuard`] is dropped, so a single
/// `lua_State` is never driven by two executions concurrently.
pub struct Pool<T, C, E: 'static> {
    inner: Arc<Inner<T, C, E>>,
    // `available` counts the VMs currently in `free`; acquiring a permit
    // guarantees a subsequent `pop` succeeds.
    available: Arc<tokio::sync::Semaphore>,
}

impl<T, C, E: 'static> Clone for Pool<T, C, E> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            available: self.available.clone(),
        }
    }
}

/// Exclusive checkout of one VM. Dereferences to the underlying [`super::UserVM`]
/// and returns it to the pool on drop.
pub struct PoolGuard<T, C, E: 'static> {
    vm: Option<Arc<super::UserVM<T, C, E>>>,
    inner: Arc<Inner<T, C, E>>,
    // Released only after the VM has been returned to `free` (see `Drop`).
    _permit: tokio::sync::OwnedSemaphorePermit,
}

impl<T, C, E: 'static> std::ops::Deref for PoolGuard<T, C, E> {
    type Target = super::UserVM<T, C, E>;

    fn deref(&self) -> &Self::Target {
        self.vm
            .as_ref()
            .expect("vm is present until the guard is dropped")
    }
}

impl<T, C, E: 'static> Drop for PoolGuard<T, C, E> {
    fn drop(&mut self) {
        if let Some(vm) = self.vm.take() {
            // Return the VM before `_permit` is released so the next waiter that
            // wakes on the freed permit is guaranteed to find a VM in `free`.
            let _ = self.inner.free.push(vm);
        }
    }
}

impl<T, C, E: 'static> Pool<T, C, E> {
    /// Atomically checks out a VM, waiting if all VMs are currently in use.
    pub async fn get(&self) -> PoolGuard<T, C, E> {
        let permit = self
            .available
            .clone()
            .acquire_owned()
            .await
            .expect("vm pool semaphore is never closed");
        let vm = self
            .inner
            .free
            .pop()
            .expect("holding a permit guarantees a free vm");
        PoolGuard {
            vm: Some(vm),
            inner: self.inner.clone(),
            _permit: permit,
        }
    }
}

pub async fn new<T, C, E: 'static, F>(
    cnt: usize,
    vm_supplier: impl Fn() -> F,
) -> anyhow::Result<Pool<T, C, E>>
where
    F: std::future::Future<Output = anyhow::Result<super::UserVM<T, C, E>>>,
{
    if cnt == 0 {
        anyhow::bail!("vm pool size must be >= 1");
    }
    let free = crossbeam::queue::ArrayQueue::new(cnt);
    for _i in 0..cnt {
        free.push(Arc::new(vm_supplier().await?))
            .ok()
            .expect("queue capacity matches the number of VMs");
    }

    Ok(Pool {
        inner: Arc::new(Inner { free }),
        available: Arc::new(tokio::sync::Semaphore::new(cnt)),
    })
}
