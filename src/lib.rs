#![allow(clippy::must_use_candidate)]
#![allow(clippy::too_many_lines)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::cognitive_complexity)]
#![allow(clippy::unseparated_literal_suffix)]

use anyhow::Error;
use deadqueue::unlimited::Queue;
use stack_string::StackString;
use std::{ops::Deref, sync::Arc};
use tokio::{
    io::{stderr, stdout, AsyncWriteExt},
    sync::Mutex,
    task::{spawn, JoinHandle},
};

enum MessageType {
    Mesg(StackString),
    Close,
}

type ChanType = Queue<MessageType>;
type TaskType = JoinHandle<Result<(), Error>>;

#[derive(Clone)]
pub struct StdoutChannel {
    stdout_queue: Arc<ChanType>,
    stderr_queue: Arc<ChanType>,
    stdout_task: Arc<Mutex<Option<TaskType>>>,
    stderr_task: Arc<Mutex<Option<TaskType>>>,
}

impl Default for StdoutChannel {
    fn default() -> Self {
        Self::new()
    }
}

impl StdoutChannel {
    pub fn new() -> Self {
        let stdout_queue = Arc::new(Queue::new());
        let stderr_queue = Arc::new(Queue::new());
        let stdout_task = Arc::new(Mutex::new(Some(spawn({
            let queue = stdout_queue.clone();
            async move { Self::stdout_task(&queue).await }
        }))));
        let stderr_task = Arc::new(Mutex::new(Some(spawn({
            let queue = stderr_queue.clone();
            async move { Self::stderr_task(&queue).await }
        }))));
        Self {
            stdout_queue,
            stderr_queue,
            stdout_task,
            stderr_task,
        }
    }

    pub fn with_mock_stdout(mock_stdout: MockStdout, mock_stderr: MockStdout) -> Self {
        let stdout_queue = Arc::new(Queue::new());
        let stderr_queue = Arc::new(Queue::new());
        let stdout_task = Arc::new(Mutex::new(Some(spawn({
            let queue = stdout_queue.clone();
            async move { Self::mock_stdout(&queue, &mock_stdout).await }
        }))));
        let stderr_task = Arc::new(Mutex::new(Some(spawn({
            let queue = stderr_queue.clone();
            async move { Self::mock_stdout(&queue, &mock_stderr).await }
        }))));
        Self {
            stdout_queue,
            stderr_queue,
            stdout_task,
            stderr_task,
        }
    }

    pub fn send<T: Into<StackString>>(&self, item: T) {
        self.stdout_queue.push(MessageType::Mesg(item.into()));
    }

    pub fn send_err<T: Into<StackString>>(&self, item: T) {
        self.stderr_queue.push(MessageType::Mesg(item.into()));
    }

    pub async fn close(&self) -> Result<(), Error> {
        self.stdout_queue.push(MessageType::Close);
        self.stderr_queue.push(MessageType::Close);
        if let Some(stdout_task) = self.stdout_task.lock().await.take() {
            stdout_task.await??;
        }
        if let Some(stderr_task) = self.stderr_task.lock().await.take() {
            stderr_task.await??;
        }
        Ok(())
    }

    async fn stdout_task(queue: &ChanType) -> Result<(), Error> {
        while let MessageType::Mesg(line) = queue.pop().await {
            stdout().write_all(format!("{}\n", line).as_bytes()).await?;
        }
        Ok(())
    }

    async fn stderr_task(queue: &ChanType) -> Result<(), Error> {
        while let MessageType::Mesg(line) = queue.pop().await {
            stderr().write_all(format!("{}\n", line).as_bytes()).await?;
        }
        Ok(())
    }

    async fn mock_stdout(queue: &ChanType, mock_stdout: &MockStdout) -> Result<(), Error> {
        while let MessageType::Mesg(line) = queue.pop().await {
            mock_stdout.lock().await.push(line);
        }
        Ok(())
    }
}

#[derive(Clone)]
pub struct MockStdout(Arc<Mutex<Vec<StackString>>>);

impl Default for MockStdout {
    fn default() -> Self {
        Self::new()
    }
}

impl Deref for MockStdout {
    type Target = Mutex<Vec<StackString>>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl MockStdout {
    pub fn new() -> Self {
        Self(Arc::new(Mutex::new(Vec::new())))
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Error;

    use super::{MockStdout, StdoutChannel};

    #[tokio::test]
    async fn test_default_mockstdout() -> Result<(), Error> {
        let mock = MockStdout::default();
        mock.lock().await.push("HEY".into());
        assert_eq!(mock.lock().await.len(), 1);
        assert_eq!(mock.lock().await[0].as_str(), "HEY");
        Ok(())
    }

    #[tokio::test]
    async fn test_default() -> Result<(), Error> {
        let chan = StdoutChannel::default();

        chan.send("stdout: Hey There");
        chan.send("What's happening".to_string());
        chan.send_err("stderr: How it goes");

        chan.close().await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_stdout_task() -> Result<(), Error> {
        let chan = StdoutChannel::new();

        chan.send("stdout: Hey There");
        chan.send("What's happening".to_string());
        chan.send_err("stderr: How it goes");

        chan.close().await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_mock_stdout() -> Result<(), Error> {
        let stdout = MockStdout::new();
        let stderr = MockStdout::new();

        let chan = StdoutChannel::with_mock_stdout(stdout.clone(), stderr.clone());

        chan.send("stdout: Hey There");
        chan.send("What's happening".to_string());
        chan.send_err("stderr: How it goes");
        chan.close().await?;

        assert_eq!(stdout.lock().await.len(), 2);
        assert_eq!(stdout.lock().await[0], "stdout: Hey There");
        assert_eq!(stdout.lock().await[1], "What's happening");
        assert_eq!(stderr.lock().await.len(), 1);
        assert_eq!(stderr.lock().await[0], "stderr: How it goes");

        Ok(())
    }
}
