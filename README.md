# Task-managed resource
This provides a way to share a resource that is `Send`, but not `Sync` between multiple tokio tasks.
When creating a `TaskMangedResource`, a `TaskSender` is returned. This object is then able to execute closures on the resource.

## Motivation
This was made to be used in Rust backend applications that need to share a common `Sync!` state, such as database connections (see Diesel's [`PgConnection`](https://docs.rs/diesel/latest/diesel/pg/struct.PgConnection.html)).

## Example
```rs
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
```

## License
This project is licensed under the MIT license.