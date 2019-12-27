use redis::{Commands, RedisResult};

/// An iterator for a queue, pulls items out of the queue in batches
pub struct Iter<'a> {
  con: &'a mut redis::Connection,
  batch: Vec<String>,
  queue: &'a str,
  total: isize,
  batch_size: isize,
  start: isize,
}

impl<'a> Iter<'a> {
  pub fn load(
    con: &'a mut redis::Connection,
    queue: &'a str,
    batch_size: isize,
  ) -> RedisResult<Iter<'a>> {
    let iter = Iter {
      total: con.llen(queue)?,
      con: con,
      queue: queue,
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
    F: Fn(&mut redis::Connection, String) -> RedisResult<bool>,
  {
    while let Some(val) = self.next() {
      f(self.con, val)?;
    }
    Ok(true)
  }
}

impl<'a> Iterator for Iter<'a> {
  type Item = String;
  fn next(&mut self) -> Option<Self::Item> {
    loop {
      if let Some(v) = self.batch.pop() {
        return Some(v);
      }
      if self.start > self.total {
        return None;
      }
      let mut batch: Vec<String> = match self.con.lrange(self.queue, self.start, self.batch_size) {
        Ok(v) => v,
        Err(_) => return None,
      };
      batch.reverse();
      self.start = self.start + self.batch_size;
      self.batch = batch;
    }
  }
}
