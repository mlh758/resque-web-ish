use futures_util::future::BoxFuture;
use futures_util::FutureExt;
use redis::{aio::ConnectionLike, Cmd, RedisResult, Value};
use std::sync::Arc;
use std::sync::Mutex;

pub struct MockConnection {
    pub received: Vec<Cmd>,
    pub to_send: Vec<Value>,
}
#[derive(Clone)]
pub struct RedisStore {
    pub connection: Arc<Mutex<MockConnection>>,
}

impl RedisStore {
    // received stores any commands the connection would have received
    // to_send is a list of Redis Value to send in response to any commands. Commands
    // are pulled in reverse order so the last command in to_send will be the first
    // sent back to the app.
    pub fn new(received: Vec<Cmd>, to_send: Vec<Value>) -> Self {
        RedisStore {
            connection: Arc::new(Mutex::new(MockConnection { received, to_send })),
        }
    }
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
        (async move {
            let mut connection = self.connection.lock().unwrap();
            connection.received = cmd.cmd_iter().cloned().collect();
            Ok(connection.to_send.clone())
        })
        .boxed()
    }

    fn req_packed_command<'a>(&'a mut self, cmd: &'a Cmd) -> BoxFuture<'a, RedisResult<Value>> {
        (async move {
            let mut connection = self.connection.lock().unwrap();
            connection.received.push(cmd.clone());
            Ok(connection.to_send.pop().unwrap())
        })
        .boxed()
    }
}
