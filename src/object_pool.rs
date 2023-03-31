pub struct ObjectPool<T: Resettable> {
    pool: Vec<T>,
    max_size: usize,
}

impl<T> ObjectPool<T> where T: Resettable {
    // Create a new ObjectPool with a specified maximum size
    pub fn new(max_size: usize) -> Self {
        ObjectPool {
            pool: Vec::new(),
            max_size,
        }
    }

    // Get an object from the pool if available, otherwise return a default object
    pub fn get(&mut self) -> T
        where
            T: Default,
    {
        match self.pool.pop() {
            Some(obj) => obj,
            None => T::default(),
        }
    }

    // Return an object to the pool if the pool is not full, otherwise discard the object
    // Call the reset method before returning it
    pub fn return_object(&mut self, mut obj: T) {
        if self.pool.len() < self.max_size {
            obj.reset();
            self.pool.push(obj);
        }
    }


    // Clear all objects from the pool
    pub fn clear(&mut self) {
        self.pool.clear();
    }
}

impl<T: Resettable> Drop for ObjectPool<T> {
    fn drop(&mut self) {
        for obj in &mut self.pool {
            obj.reset();
        }
    }
}


// Define the Resettable trait
pub trait Resettable {
    fn reset(&mut self);
}