use redis::{Commands, ErrorKind};
use serde_derive::{Deserialize, Serialize};
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

#[derive(Deserialize)]
struct FailedJob {
  payload: serde_json::Value,
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
  let key = "resque:failed";
  remove_job(con, key, job)?;
  Ok(1)
}

pub fn retry_failed_job(con: &mut redis::Connection, job: &str) -> redis::RedisResult<isize> {
  let key = "resque:failed";
  let job = remove_job(con, key, job)?;
  let job_payload: FailedJob = serde_json::from_str(job.as_str()).map_err(json_failed)?;
  con.rpush("resque:queue:default", job_payload.payload.to_string())?;
  Ok(1)
}

pub fn retry_all_jobs(con: &mut redis::Connection) -> redis::RedisResult<isize> {
  let key = "resque:failed";
  let fail_count = con.llen(key)?;
  let jobs: Vec<String> = con.lrange(key, 0, fail_count)?;
  for job in jobs.iter() {
    let job_payload: FailedJob = serde_json::from_str(job.as_str()).map_err(json_failed)?;
    con.rpush("resque:queue:default", job_payload.payload.to_string())?;
  }
  clear_queue(con, "failed")?;
  Ok(1)
}

fn json_failed(_err: serde_json::Error) -> redis::RedisError {
  std::convert::From::from((ErrorKind::IoError, "failed to parse job json"))
}

fn remove_job(con: &mut redis::Connection, key: &str, job: &str) -> redis::RedisResult<String> {
  let mut start = 0;
  let max = con.llen(key)?;
  while start < max {
    let jobs: Vec<String> = con.lrange(key, start, start + 99)?;
    for failed_job in jobs.iter() {
      if failed_job.as_str().contains(job) {
        con.lrem(key, 0, failed_job)?;
        return Ok(failed_job.to_string());
      }
    }
    start += 100;
  }
  Err(std::convert::From::from((
    ErrorKind::IoError,
    "job not found",
  )))
}
