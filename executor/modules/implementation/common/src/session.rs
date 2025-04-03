pub struct Pool<T> {
    pool: crossbeam::queue::ArrayQueue<Box<T>>,
}

impl<T> Default for Pool<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Pool<T> {
    pub fn new() -> Self {
        Self {
            pool: crossbeam::queue::ArrayQueue::new(8),
        }
    }

    pub fn get(&self) -> Option<Box<T>> {
        self.pool.pop()
    }

    pub fn retn(&self, obj: Box<T>) {
        let _ = self.pool.push(obj);
    }
}
