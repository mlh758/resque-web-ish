use redis::{AsyncCommands, RedisResult};

/// An iterator for a queue, pulls items out of the queue in batches
pub struct Iter<'a, T: AsyncCommands> {
    con: &'a mut T,
    batch: Vec<String>,
    queue: &'a str,
    total: isize,
    batch_size: isize,
    start: isize,
}

impl<'a, T: AsyncCommands> Iter<'a, T> {
    pub async fn load(
        con: &'a mut T,
        queue: &'a str,
        batch_size: isize,
    ) -> RedisResult<Iter<'a, T>> {
        let iter = Iter {
            total: con.llen(queue).await?,
            con,
            queue,
            batch_size: batch_size - 1,
            batch: vec![],
            start: 0,
        };
        Ok(iter)
    }
    /// Runs the given function over every item in the queue
    /// Stops iteration and propagates any errors encountered
    /// The Ok value can be ignored
    pub fn each<F>(&mut self, f: F) -> RedisResult<bool>
    where
        F: Fn(&mut T, String) -> RedisResult<bool>,
    {
        while let Some(val) = self.next() {
            f(self.con, val)?;
        }
        Ok(true)
    }
}

impl<'a, T: AsyncCommands> Iterator for Iter<'a, T> {
    type Item = String;
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(v) = self.batch.pop() {
                return Some(v);
            }
            if self.start > self.total {
                return None;
            }
            let mut batch: Vec<String> =
                match self.con.lrange(self.queue, self.start, self.batch_size) {
                    Ok(v) => v,
                    Err(_) => return None,
                };
            // Something could have popped items on us between requests, avoid infinite loop
            if batch.len() == 0 {
                return None;
            }
            batch.reverse();
            self.start = self.start + self.batch_size;
            self.batch = batch;
        }
    }
}
