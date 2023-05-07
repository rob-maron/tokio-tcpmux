use std::collections::HashSet;

pub struct StreamIdGenerator {
    counter: u32,
    open_streams: HashSet<u32>,
}

impl StreamIdGenerator {
    pub fn new() -> Self {
        Self {
            counter: 1,
            open_streams: HashSet::new(),
        }
    }

    pub fn next(&mut self) -> u32 {
        while self.open_streams.contains(&self.counter) {
            self.counter = if self.counter == 1073741822 {
                1
            } else {
                self.counter + 1
            };
        }

        let next_id = self.counter;
        self.open_streams.insert(next_id);

        self.counter = if self.counter == 1073741822 {
            1
        } else {
            self.counter + 1
        };
        next_id
    }

    pub fn release_id(&mut self, id: u32) {
        self.open_streams.remove(&id);
    }

    pub fn acquire_id(&mut self, id: u32) {
        self.open_streams.insert(id);
    }
}

