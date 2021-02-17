use redis::{AsyncCommands, ErrorKind};
use serde_derive::{Deserialize, Serialize};
use std::collections::HashMap;
use std::collections::HashSet;

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

#[derive(Serialize)]
pub struct ResqueStats {
    success_count: u64,
    failure_count: u64,
    available_queues: Vec<String>,
}

pub async fn queue_stats(mut con: impl AsyncCommands) -> redis::RedisResult<ResqueStats> {
    let (queues, fail_cnt, pass_cnt): (HashSet<String>, Option<u64>, Option<u64>) = redis::pipe()
        .smembers("resque:queues")
        .get("resque:stat:failed")
        .get("resque:stat:processed")
        .query_async(&mut con)
        .await?;
    Ok(ResqueStats {
        success_count: pass_cnt.unwrap_or(0),
        failure_count: fail_cnt.unwrap_or(0),
        available_queues: queues.into_iter().collect(),
    })
}

pub async fn get_failed(
    mut con: impl AsyncCommands,
    start: isize,
    end: isize,
) -> redis::RedisResult<Vec<String>> {
    con.lrange("resque:failed", start, end).await
}

pub async fn current_failures(mut con: impl AsyncCommands) -> redis::RedisResult<u64> {
    con.llen("resque:failed").await
}

pub async fn active_workers(mut con: impl AsyncCommands) -> redis::RedisResult<Vec<Worker>> {
    let (workers, heartbeats): (Vec<String>, HashMap<String, String>) = redis::pipe()
        .smembers("resque:workers")
        .hgetall("resque:workers:heartbeat")
        .query_async(&mut con)
        .await?;
    let mut results = Vec::new();
    for worker in workers.into_iter() {
        results.push(Worker {
            payload: con
                .get(format!("resque:worker:{}", &worker))
                .await
                .unwrap_or(None),
            heartbeat: heartbeats.get(&worker).map(|x| x.to_string()),
            id: worker,
        });
    }
    Ok(results)
}

pub async fn queue_details(
    mut con: impl AsyncCommands,
    queue_name: &str,
    start: isize,
    end: isize,
) -> redis::RedisResult<QueueDetails> {
    let key = format!("resque:queue:{}", queue_name);
    let queued_jobs: Vec<String> = con.lrange(&key, start, end).await?;
    Ok(QueueDetails {
        total_jobs: con.llen(&key).await?,
        jobs: queued_jobs
            .into_iter()
            .map(|job| match serde_json::from_str(&job) {
                Ok(val) => val,
                Err(_) => serde_json::Value::Null,
            })
            .collect(),
    })
}

pub async fn clear_queue(mut con: impl AsyncCommands, queue: &str) -> redis::RedisResult<isize> {
    con.del(format!("resque:{}", queue)).await
}

pub async fn delete_failed_job(mut con: impl AsyncCommands, job: &str) -> redis::RedisResult<()> {
    let key = "resque:failed";
    remove_job(&mut con, key, job).await?;
    Ok(())
}

pub async fn retry_failed_job(mut con: impl AsyncCommands, job: &str) -> redis::RedisResult<()> {
    let key = "resque:failed";
    let job = remove_job(&mut con, key, job).await?;
    let job_payload: FailedJob = serde_json::from_str(job.as_str()).map_err(json_failed)?;
    con.rpush("resque:queue:default", job_payload.payload.to_string())
        .await?;
    Ok(())
}

pub async fn retry_all_jobs(mut con: impl AsyncCommands) -> redis::RedisResult<()> {
    let key = "resque:failed";
    let mut start = 0;
    loop {
        let failed: Vec<String> = con.lrange(key, start, 99).await?;
        for job in failed.iter() {
            start += 1;
            let job_payload: FailedJob = serde_json::from_str(job).map_err(json_failed)?;
            con.rpush("resque:queue:default", job_payload.payload.to_string())
                .await?;
        }
        if failed.len() < 100 {
            break;
        }
    }
    clear_queue(con, "failed").await?;
    Ok(())
}

fn json_failed(_err: serde_json::Error) -> redis::RedisError {
    std::convert::From::from((ErrorKind::IoError, "failed to parse job json"))
}

async fn remove_job(
    con: &mut impl AsyncCommands,
    key: &str,
    job: &str,
) -> redis::RedisResult<String> {
    let mut start = 0;
    loop {
        let failed: Vec<String> = con.lrange(key, start, 99).await?;
        for failed_job in failed.iter() {
            start += 1;
            if failed_job.contains(job) {
                con.lrem(key, 0, failed_job.as_str()).await?;
                return Ok(failed_job.to_string());
            }
        }
        if failed.len() < 100 {
            break;
        }
    }

    Err(std::convert::From::from((
        ErrorKind::IoError,
        "job not found",
    )))
}

pub async fn remove_worker(mut con: impl AsyncCommands, id: &str) -> redis::RedisResult<()> {
    redis::pipe()
        .del(format!("resque:stat:processed:{}", id))
        .ignore()
        .del(format!("resque:stat:failed:{}", id))
        .ignore()
        .srem("resque:workers", id)
        .ignore()
        .hdel("resque:workers:heartbeat", id)
        .ignore()
        .del(format!("resque:worker:{}:started", id))
        .ignore()
        .query_async(&mut con)
        .await
}

#[cfg(test)]
mod tests {
    use core::panic;

    use super::*;
    use mock_redis::RedisStore;
    use redis::Value;
    mod mock_redis;
    // #[actix_rt::test]
    // async fn test_clear_queue() {
    //     let store = RedisStore {
    //         received: Vec::new(),
    //         to_send: vec![Value::Int(1)],
    //     };
    //     let rslt = clear_queue(store, "default").await;
    //     let args: Vec<&str> = store.received[0]
    //         .args_iter()
    //         .map(|arg| match arg {
    //             redis::Arg::Simple(arg) => std::str::from_utf8(arg).unwrap(),
    //             _ => panic!("unexpected cursor arg"),
    //         })
    //         .collect();
    //     assert_eq!(rslt, Ok(1));
    //     assert!(args[1].contains("resque:default"));
    // }

    #[actix_rt::test]
    async fn test_get_failed() {
        let store = RedisStore {
            received: Vec::new(),
            to_send: vec![Value::Bulk(vec![
                Value::Data(Vec::from("failed1")),
                Value::Data(Vec::from("failed2")),
            ])],
        };
        let rslt = get_failed(store, 1, 100).await.unwrap();
        assert_eq!(rslt, vec!(String::from("failed1"), String::from("failed2")))
    }

    #[actix_rt::test]
    async fn delete_failed_job_succeeds() {
        let store = RedisStore {
            received: Vec::new(),
            to_send: vec![
                Value::Int(1),
                Value::Bulk(vec![
                    Value::Data(Vec::from("id1")),
                    Value::Data(Vec::from("id2")),
                    Value::Data(Vec::from("id3")),
                ]),
                Value::Int(3),
            ],
        };
        let rslt = delete_failed_job(store, "id2").await;
        assert_eq!(rslt, Ok(()));
    }

    #[actix_rt::test]
    async fn delete_failed_job_queue_empties() {
        let store = RedisStore {
            received: Vec::new(),
            to_send: vec![
                Value::Int(1),
                Value::Bulk(vec![Value::Data(Vec::from("id1"))]),
                Value::Int(3),
            ],
        };
        if let Ok(()) = delete_failed_job(store, "id2").await {
            panic!("should not have found a value")
        }
    }
    #[actix_rt::test]
    async fn queue_stats_populated() {
        let store = RedisStore {
            received: Vec::new(),
            to_send: vec![
                Value::Bulk(vec![Value::Data(Vec::from("default"))]),
                Value::Int(123),
                Value::Int(456),
            ],
        };
        let rslt = queue_stats(store).await.unwrap();
        let expected = ResqueStats {
            available_queues: vec!["default".to_string()],
            failure_count: 123,
            success_count: 456,
        };
        assert_eq!(rslt.available_queues, expected.available_queues);
        assert_eq!(rslt.failure_count, expected.failure_count);
        assert_eq!(rslt.success_count, expected.success_count);
    }

    #[actix_rt::test]
    async fn queue_stats_no_counts() {
        let store = RedisStore {
            received: Vec::new(),
            to_send: vec![
                Value::Bulk(vec![Value::Data(Vec::from("default"))]),
                Value::Nil,
                Value::Nil,
            ],
        };
        let rslt = queue_stats(store).await.unwrap();
        let expected = ResqueStats {
            available_queues: vec!["default".to_string()],
            failure_count: 0,
            success_count: 0,
        };
        assert_eq!(rslt.available_queues, expected.available_queues);
        assert_eq!(rslt.failure_count, expected.failure_count);
        assert_eq!(rslt.success_count, expected.success_count);
    }
}
