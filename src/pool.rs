use std::collections::HashMap;

pub struct IntegerPool {
    // Implements an integer pool for reusing unique integers.
    //
    // Clients of this struct will take unique integers and give them back when done.
    integers: Vec<usize>,
    pool_index_map: HashMap<usize, usize>,
    marker: usize,
    size: usize,
}

impl IntegerPool {
    pub fn new(size: usize) -> IntegerPool {
        let mut integer_pool = IntegerPool {
            integers: Vec::new(),
            pool_index_map: HashMap::new(),
            marker: 0,
            size: 0,
        };

        integer_pool.expand(size);

        integer_pool
    }

    pub fn take(&mut self) -> usize {
        // Take an integer from the pool.
        if self.marker >= self.size {
            let new_size = self.size * 2;
            self.expand(new_size);
        }

        let integer = self.integers[self.marker];
        self.pool_index_map.insert(integer, self.marker);
        self.marker += 1;

        integer
    }

    pub fn give(&mut self, pool_integer: usize) {
        // Give an integer back to the pool so it can be reused.
        // Swap with the last available integer.
        self.marker -= 1;
        let end_integer = self.integers[self.marker];
        let end_index = self.pool_index_map[&end_integer];

        let pool_integer_index = self.pool_index_map[&pool_integer];
        self.integers[self.marker] = pool_integer;
        self.integers[pool_integer_index] = end_integer;

        self.pool_index_map.insert(end_integer, pool_integer_index);
        self.pool_index_map.insert(pool_integer, end_index);
    }

    fn expand(&mut self, new_size: usize) {
        // Expand the pool to the specified new size.
        for i in 0..(new_size - self.size) {
            let new_index = self.size + i;
            self.integers.push(new_index);
            self.pool_index_map.insert(new_index, new_index);
        }

        self.size = new_size;
    }
}
