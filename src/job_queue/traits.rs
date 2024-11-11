use std::any::Any;
use tokio::sync::oneshot;
use tokio::sync::watch;

pub(super) type JobCancelReceiver = watch::Receiver<()>;
pub(super) type JobCancelSender = watch::Sender<()>;

pub(super) type JobResultReceiver = oneshot::Receiver<JobCompletion>;
pub(super) type JobResultSender = oneshot::Sender<JobCompletion>;

/// represents a job result, which can be any type.
pub trait JobResult: Any + Send + Sync + std::fmt::Debug {
    fn as_any(&self) -> &dyn Any;
}

/// represents completion state of a job
#[derive(Debug)]
pub enum JobCompletion {
    Finished(Box<dyn JobResult>),
    Cancelled,
}

// represents any kind of job
#[async_trait::async_trait]
pub trait Job: Send + Sync {
    fn is_async(&self) -> bool;

    // note: we provide unimplemented default methods for
    // run and run_async.  This is so that implementing types
    // only need to impl the appropriate method.

    fn run(&self, _rx: JobCancelReceiver) -> JobCompletion {
        unimplemented!()
    }

    /// This method is called by JobQueue.  The default implementation handles job
    /// cancellation, so most Job implementors can simply impl run_async() and
    /// cancellation is automatic.
    async fn run_async_cancellable(&self, mut rx: JobCancelReceiver) -> JobCompletion {
        tokio::select! {
            _ = rx.changed() => {
                tracing::debug!("async job got cancel message. cancelling.");
                JobCompletion::Cancelled
            }

            job_result = self.run_async() => {
                JobCompletion::Finished(job_result)
            },
        }
    }

    /// implement this method to perform the work of the job.
    async fn run_async(&self) -> Box<dyn JobResult> {
        unimplemented!()
    }
}
