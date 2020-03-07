use redis::{ConnectionLike, RedisResult, Value};

pub struct RedisStore<'a> {
    pub received: &'a mut Vec<u8>,
    pub to_send: &'a mut Vec<Value>,
}

impl<'a> ConnectionLike for RedisStore<'a> {
    fn get_db(&self) -> i64 {
        3
    }

    fn req_packed_commands(
        &mut self,
        cmd: &[u8],
        _offset: usize,
        _count: usize,
    ) -> RedisResult<Vec<Value>> {
        self.received.extend_from_slice(cmd);
        Ok(self.to_send.clone())
    }

    fn req_packed_command(&mut self, cmd: &[u8]) -> RedisResult<Value> {
        self.received.extend_from_slice(cmd);
        Ok(self.to_send.pop().unwrap())
    }

    fn check_connection(&mut self) -> bool {
        true
    }

    fn is_open(&self) -> bool {
        true
    }
}
