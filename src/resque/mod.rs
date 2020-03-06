use redis::{Commands, ErrorKind};
use serde_derive::{Deserialize, Serialize};
use std::collections::HashMap;
use std::collections::HashSet;
mod queue;

#[derive(Serialize)]
pub struct Worker {
  id: String,
  payload: Option<String>,
  heartbeat: Option<String>,
}

#[derive(Serialize)]
pub struct QueueDetails {
  total_jobs: u64,
  jobs: serde_json::Value,
}

#[derive(Deserialize)]
struct FailedJob {
  payload: serde_json::Value,
}

pub fn get_queues(con: &mut impl Commands) -> redis::RedisResult<HashSet<String>> {
  con.smembers("resque:queues")
}

pub fn failure_count(con: &mut impl Commands) -> u64 {
  con.get("resque:stat:failed").unwrap_or(0)
}

pub fn processed_count(con: &mut impl Commands) -> u64 {
  con.get("resque:stat:processed").unwrap_or(0)
}

pub fn get_failed(
  con: &mut impl Commands,
  start: isize,
  end: isize,
) -> redis::RedisResult<Vec<String>> {
  con.lrange("resque:failed", start, end)
}

pub fn current_failures(con: &mut impl Commands) -> redis::RedisResult<u64> {
  con.llen("resque:failed")
}

pub fn active_workers(con: &mut impl Commands) -> redis::RedisResult<Vec<Worker>> {
  let workers: Vec<String> = con.smembers("resque:workers")?;
  let heartbeats: HashMap<String, String> = con.hgetall("resque:workers:heartbeat")?;
  let results: Vec<Worker> = workers
    .into_iter()
    .map(|worker| Worker {
      payload: con
        .get(format!("resque:worker:{}", &worker))
        .unwrap_or(None),
      heartbeat: heartbeats.get(&worker).map(|x| x.to_string()),
      id: worker,
    })
    .collect();
  Ok(results)
}

pub fn queue_details(
  con: &mut impl Commands,
  queue_name: &str,
  start: isize,
  end: isize,
) -> redis::RedisResult<QueueDetails> {
  let key = format!("resque:queue:{}", queue_name);
  let queued_jobs: Vec<String> = con.lrange(&key, start, end)?;
  Ok(QueueDetails {
    total_jobs: con.llen(&key)?,
    jobs: queued_jobs
      .into_iter()
      .map(|job| match serde_json::from_str(&job) {
        Ok(val) => val,
        Err(_) => serde_json::Value::Null,
      })
      .collect(),
  })
}

pub fn clear_queue(con: &mut impl Commands, queue: &str) -> redis::RedisResult<isize> {
  con.del(format!("resque:{}", queue))
}

pub fn delete_failed_job(con: &mut impl Commands, job: &str) -> redis::RedisResult<()> {
  let key = "resque:failed";
  remove_job(con, key, job)?;
  Ok(())
}

pub fn retry_failed_job(con: &mut impl Commands, job: &str) -> redis::RedisResult<()> {
  let key = "resque:failed";
  let job = remove_job(con, key, job)?;
  let job_payload: FailedJob = serde_json::from_str(job.as_str()).map_err(json_failed)?;
  con.rpush("resque:queue:default", job_payload.payload.to_string())?;
  Ok(())
}

pub fn retry_all_jobs(con: &mut impl Commands) -> redis::RedisResult<()> {
  let key = "resque:failed";
  let mut iter = queue::Iter::load(con, key, 100)?;
  iter.each(|con, job| {
    let job_payload: FailedJob = serde_json::from_str(job.as_str()).map_err(json_failed)?;
    con.rpush("resque:queue:default", job_payload.payload.to_string())?;
    Ok(true)
  })?;
  clear_queue(con, "failed")?;
  Ok(())
}

fn json_failed(_err: serde_json::Error) -> redis::RedisError {
  std::convert::From::from((ErrorKind::IoError, "failed to parse job json"))
}

fn remove_job(con: &mut impl Commands, key: &str, job: &str) -> redis::RedisResult<String> {
  let iter = queue::Iter::load(con, key, 100)?;
  for failed_job in iter {
    if failed_job.as_str().contains(job) {
      con.lrem(key, 0, failed_job.as_str())?;
      return Ok(failed_job.to_string());
    }
  }
  Err(std::convert::From::from((
    ErrorKind::IoError,
    "job not found",
  )))
}

pub fn remove_worker(con: &mut impl Commands, id: &str) -> redis::RedisResult<()> {
  redis::pipe()
    .cmd("DEL")
    .arg(format!("resque:stat:processed:{}", id))
    .ignore()
    .cmd("DEL")
    .arg(format!("resque:stat:failed:{}", id))
    .ignore()
    .cmd("SREM")
    .arg("resque:workers")
    .arg(id)
    .ignore()
    .cmd("HDEL")
    .arg("resque:workers:heartbeat")
    .arg(id)
    .ignore()
    .cmd("DEL")
    .arg(format!("resque:worker:{}:started", id))
    .ignore()
    .query(con)?;
  Ok(())
}

#[cfg(test)]
mod tests {
  use super::*;
  use mock_redis::RedisStore;
  use redis::Value;
  mod mock_redis;
  #[test]
  fn test_clear_queue() {
    let mut store = RedisStore {
      received: &mut Vec::new(),
      to_send: &mut vec![Value::Int(1)],
    };
    let rslt = clear_queue(&mut store, "default");
    let cmd = std::str::from_utf8(store.received).unwrap();
    assert_eq!(rslt, Ok(1));
    assert!(cmd.contains("resque:default"));
  }

  #[test]
  fn test_get_failed() {
    let mut store = RedisStore {
      received: &mut Vec::new(),
      to_send: &mut vec![Value::Bulk(vec![
        Value::Data(Vec::from("failed1")),
        Value::Data(Vec::from("failed2")),
      ])],
    };
    let rslt = get_failed(&mut store, 1, 100).unwrap();
    assert_eq!(rslt, vec!(String::from("failed1"), String::from("failed2")))
  }

  #[test]
  fn test_processed_count() {
    let mut store = RedisStore {
      received: &mut Vec::new(),
      to_send: &mut vec![Value::Int(100), Value::Nil],
    };
    assert_eq!(processed_count(&mut store), 0);
    assert_eq!(processed_count(&mut store), 100);
  }

  #[test]
  fn delete_failed_job_succeeds() {
    let mut store = RedisStore {
      received: &mut Vec::new(),
      to_send: &mut vec![
        Value::Int(1),
        Value::Bulk(vec![Value::Data(Vec::from("id1")), Value::Data(Vec::from("id2")), Value::Data(Vec::from("id3"))]),
        Value::Int(3)
      ],
    };
    let rslt = delete_failed_job(&mut store, "id2");
    assert_eq!(rslt, Ok(()));
  }

  #[test]
  fn delete_failed_job_queue_empties() {
    let mut store = RedisStore {
      received: &mut Vec::new(),
      to_send: &mut vec![
        Value::Int(1),
        Value::Bulk(vec![Value::Data(Vec::from("id1"))]),
        Value::Int(3)
      ],
    };
    if let Ok(()) = delete_failed_job(&mut store, "id2") {
      panic!("should not have found a value")
    }
  }
}
