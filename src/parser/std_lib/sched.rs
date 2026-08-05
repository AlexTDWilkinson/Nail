//! Scheduled jobs beside a running program - the nightly cleanup, the weekly
//! digest - without a system crontab. A schedule blocks forever, so it runs
//! in a spawn block next to the web server, and the work itself is a named
//! function the program owns.

use std::future::Future;
use std::pin::Pin;

pub type JobFuture = Pin<Box<dyn Future<Output = ()> + Send>>;

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct SCHED_Job {
    pub name: String,
    pub cron: String,
}

fn unix_now(what: &str) -> Result<i64, String> {
    return std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .map_err(|_| format!("{}: the system clock reads before 1970", what));
}

/// Run jobs on their cron schedules, forever. Each due moment calls the
/// program's handle_job function with the job's name. A job that is still
/// running when its next moment comes simply delays it - jobs run one at a
/// time, in this loop, so they never overlap themselves.
pub async fn run<F>(jobs: Vec<SCHED_Job>, handler: F) -> Result<(), String>
where
    F: Fn(String) -> JobFuture + Clone + Send + Sync + 'static,
{
    if jobs.is_empty() {
        return Err("sched_run: there are no jobs to run".to_string());
    }
    let mut after = unix_now("sched_run")?;
    for job in &jobs {
        super::time::cron_next(job.cron.clone(), after).map_err(|failure| format!("sched_run: job `{}`: {}", job.name, failure))?;
    }
    loop {
        let mut due_time = i64::MAX;
        let mut due_jobs: Vec<&SCHED_Job> = Vec::new();
        for job in &jobs {
            let next = super::time::cron_next(job.cron.clone(), after).map_err(|failure| format!("sched_run: job `{}`: {}", job.name, failure))?;
            if next < due_time {
                due_time = next;
                due_jobs = vec![job];
            } else if next == due_time {
                due_jobs.push(job);
            }
        }
        let wait = due_time - unix_now("sched_run")?;
        if wait > 0 {
            tokio::time::sleep(std::time::Duration::from_secs(wait as u64)).await;
        }
        for job in &due_jobs {
            handler(job.name.clone()).await;
        }
        after = due_time;
    }
}

/// Call the program's handle_job function with the given name every so many
/// seconds, forever. The wait is between finishes, not starts, so slow work
/// never overlaps itself.
pub async fn every<F>(name: String, seconds: i64, handler: F) -> Result<(), String>
where
    F: Fn(String) -> JobFuture + Clone + Send + Sync + 'static,
{
    if seconds < 1 {
        return Err(format!("sched_every: the wait must be at least a second, not {}", seconds));
    }
    loop {
        tokio::time::sleep(std::time::Duration::from_secs(seconds as u64)).await;
        handler(name.clone()).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicI64, Ordering};
    use std::sync::Arc;

    #[tokio::test]
    async fn a_bad_cron_expression_is_refused_before_anything_runs() {
        let jobs = vec![SCHED_Job { name: "broken".to_string(), cron: "not cron".to_string() }];
        let failure = run(jobs, |_| Box::pin(async {}) as JobFuture).await.unwrap_err();
        assert!(failure.contains("broken"), "got: {}", failure);
        assert!(run(vec![], |_| Box::pin(async {}) as JobFuture).await.unwrap_err().contains("no jobs"));
    }

    #[tokio::test]
    async fn every_refuses_a_zero_wait_and_otherwise_ticks() {
        assert!(every("tick".to_string(), 0, |_| Box::pin(async {}) as JobFuture).await.unwrap_err().contains("at least a second"));

        let count = Arc::new(AtomicI64::new(0));
        let seen = count.clone();
        let ticker = every("tick".to_string(), 1, move |_| {
            let seen = seen.clone();
            Box::pin(async move {
                seen.fetch_add(1, Ordering::SeqCst);
            }) as JobFuture
        });
        // The loop never returns on its own; give it time for one tick and drop it.
        let _ = tokio::time::timeout(std::time::Duration::from_millis(1300), ticker).await;
        assert_eq!(count.load(Ordering::SeqCst), 1);
    }
}
