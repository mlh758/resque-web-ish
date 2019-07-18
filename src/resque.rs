use redis::Commands;
use serde_derive::Serialize;
use std::collections::HashSet;

#[derive(Serialize)]
pub struct Worker {
  id: String,
  payload: String,
}

#[derive(Serialize)]
pub struct QueueDetails {
  total_jobs: u64,
  jobs: serde_json::Value,
}

pub fn get_queues(con: &mut redis::Connection) -> redis::RedisResult<HashSet<String>> {
  con.smembers("resque:queues")
}

pub fn failure_count(con: &mut redis::Connection) -> u64 {
  con.get("resque:stat:failed").unwrap_or(0)
}

pub fn processed_count(con: &mut redis::Connection) -> u64 {
  con.get("resque:stat:processed").unwrap_or(0)
}

pub fn get_failed(
  con: &mut redis::Connection,
  start: isize,
  end: isize,
) -> redis::RedisResult<Vec<String>> {
  con.lrange("resque:failed", start, end)
}

pub fn current_failures(con: &mut redis::Connection) -> redis::RedisResult<u64> {
  con.llen("resque:failed")
}

pub fn active_workers(con: &mut redis::Connection) -> redis::RedisResult<Vec<Worker>> {
  let workers: Vec<String> = con.smembers("resque:workers")?;
  let results: Vec<Worker> = workers
    .into_iter()
    .map(|worker| Worker {
      payload: match con.get(format!("resque:worker:{}", &worker)) {
        Ok(val) => val,
        Err(_) => String::from(""),
      },
      id: worker,
    })
    .collect();
  Ok(results)
}

pub fn queue_details(
  con: &mut redis::Connection,
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

pub fn clear_queue(con: &mut redis::Connection, queue: &str) -> redis::RedisResult<isize> {
  con.del(format!("resque:{}", queue))
}

pub fn delete_failed_job(con: &mut redis::Connection, job: &str) -> redis::RedisResult<isize> {
  let mut found = false;
  let key = "resque:failed";
  let mut start = 0;
  let max = con.llen(key)?;
  while !found {
    if start > max {
      break;
    }
    let jobs: Vec<String> = con.lrange(key, start, start + 99)?;
    for failed_job in jobs.iter() {
      if failed_job.as_str().contains(job) {
        con.lrem(key, 0, failed_job)?;
        found = true
      }
    }
    start += 100;
  }
  Ok(1)
}
