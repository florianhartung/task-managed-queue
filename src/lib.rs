use std::{any::Any, num::NonZeroUsize};

use tokio::sync::{mpsc, oneshot};

#[derive(Debug)]
pub struct TaskManagedResource<R> {
    r: R,
    messages: mpsc::Receiver<Message<R>>,
}

impl<R: Send> TaskManagedResource<R> {
    pub fn new(r: R) -> (Self, TaskSender<R>) {
        Self::new_with_capacity(r, NonZeroUsize::new(32).unwrap())
    }

    pub fn new_with_capacity(r: R, task_capacity: NonZeroUsize) -> (Self, TaskSender<R>) {
        let (messages_tx, messages_rx) = mpsc::channel(task_capacity.get());

        (
            Self {
                r,
                messages: messages_rx,
            },
            TaskSender {
                sender: messages_tx,
            },
        )
    }

    pub async fn run(mut self) {
        while let Some(msg) = self.messages.recv().await {
            let res: AnyResponse = (msg.task)(&mut self.r);
            msg.response.send(res).expect("that the response channel is not closed already because responses must always be received");
        }
    }
}

#[derive(Clone)]
pub struct TaskSender<R> {
    sender: mpsc::Sender<Message<R>>,
}

type Task<T, R> = Box<dyn FnOnce(&mut R) -> T + Send + 'static>;
type AnyResponse = Box<dyn Any + Send + 'static>;

struct Message<R> {
    task: Task<AnyResponse, R>,
    response: oneshot::Sender<AnyResponse>,
}

impl<R> TaskSender<R> {
    pub async fn execute<T, F>(&mut self, f: F) -> T
    where
        T: Send + 'static,
        F: FnOnce(&mut R) -> T + Send + 'static,
    {
        let task: Task<AnyResponse, R> = Box::new(move |conn| Box::new(f(conn)));

        let (response_tx, response_rx) = oneshot::channel();

        let message = Message {
            task,
            response: response_tx,
        };

        self.sender
            .send(message)
            .await
            .expect("the channel to not ever be closed");

        let response = response_rx
            .await
            .expect("the response channel to never be closed without response");

        *response.downcast::<T>().expect("this to be of type T")
    }
}

#[cfg(test)]
mod tests {
    use crate::TaskManagedResource;

    #[tokio::test]
    pub async fn test() {
        // Create a task-managed resource
        let (resource, mut sender) = TaskManagedResource::new(7);

        // Run the managing task
        tokio::spawn(resource.run());

        // We can create as many senders as we like
        let mut sender2 = sender.clone();
        let mut _sender3 = sender.clone();

        // Let's create two tasks to execute afterwards.
        let increment_and_get = |num: &mut i32| {
            *num += 1;
            *num
        };
        let double_and_get = |num: &mut i32| {
            *num *= 2;
            *num
        };

        let first_task = sender.execute(increment_and_get);
        let second_task = sender2.execute(double_and_get);

        let res = tokio::join!(first_task, second_task);

        // Different result depending on order of execution
        assert!(res == (8, 16) || res == (15, 14));
    }
}
