use futures_util::future::BoxFuture;
use futures_util::FutureExt;
use redis::{aio::ConnectionLike, Cmd, RedisResult, Value};
use std::unimplemented;

pub struct RedisStore {
    pub received: Vec<Cmd>,
    pub to_send: Vec<Value>,
}

impl ConnectionLike for RedisStore {
    fn get_db(&self) -> i64 {
        3
    }

    fn req_packed_commands<'a>(
        &'a mut self,
        cmd: &'a redis::Pipeline,
        _offset: usize,
        _count: usize,
    ) -> BoxFuture<'a, RedisResult<Vec<Value>>> {
        unimplemented!()
    }

    fn req_packed_command<'a>(&'a mut self, cmd: &'a Cmd) -> BoxFuture<'a, RedisResult<Value>> {
        (async move {
            self.received.push(cmd.clone());
            Ok(self.to_send.pop().unwrap())
        })
        .boxed()
    }
}
