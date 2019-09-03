use async_std::sync::RwLock as AsyncRwLock;
use async_trait::async_trait;
use futures::{stream::FuturesUnordered, StreamExt};
use rand::{thread_rng, Rng};
use std::{
    sync::{Arc, RwLock},
    time::Instant,
};
use tokio::executor::Executor;

struct AsyncCounter {
    inner: Arc<AsyncRwLock<u64>>,
}

impl AsyncCounter {
    fn new() -> Self {
        AsyncCounter {
            inner: Arc::new(AsyncRwLock::new(0)),
        }
    }

    async fn get(&self) -> u64 {
        let val = self.inner.read().await;
        *val
    }

    async fn set(&self, val: u64) {
        let mut curr = self.inner.write().await;
        *curr = val;
    }
}

impl Clone for AsyncCounter {
    fn clone(&self) -> AsyncCounter {
        AsyncCounter {
            inner: Arc::clone(&self.inner),
        }
    }
}

struct Counter {
    inner: Arc<RwLock<u64>>,
}

impl Counter {
    fn new() -> Self {
        Counter {
            inner: Arc::new(RwLock::new(0)),
        }
    }

    async fn get(&self) -> u64 {
        let val = self.inner.read().unwrap();
        *val
    }

    async fn set(&self, val: u64) {
        let mut curr = self.inner.write().unwrap();
        *curr = val;
    }
}

impl Clone for Counter {
    fn clone(&self) -> Counter {
        Counter {
            inner: Arc::clone(&self.inner),
        }
    }
}

#[async_trait]
trait CounterTrait: Clone + 'static + Send {
    fn new() -> Self;
    fn name() -> &'static str;
    async fn get(&self) -> u64;
    async fn set(&self, value: u64);
}

#[async_trait]
impl CounterTrait for Counter {
    fn name() -> &'static str {
        "Counter"
    }

    fn new() -> Self {
        Counter::new()
    }

    async fn get(&self) -> u64 {
        self.get().await
    }

    async fn set(&self, value: u64) {
        self.set(value).await
    }
}

#[async_trait]
impl CounterTrait for AsyncCounter {
    fn name() -> &'static str {
        "AsyncCounter"
    }

    fn new() -> Self {
        AsyncCounter::new()
    }

    async fn get(&self) -> u64 {
        self.get().await
    }

    async fn set(&self, value: u64) {
        self.set(value).await
    }
}

async fn test<C: CounterTrait>(
    executor: &mut (dyn Executor + 'static),
    max_counters: usize,
    per_counter_operations_cnt: usize,
    read_write_ratio: f64,
) {
    let mut rng = thread_rng();
    let mut counters = Vec::new();
    for _ in 0..max_counters {
        counters.push(C::new());
    }

    let mut futures = FuturesUnordered::new();
    for index in 0..max_counters {
        for _ in 0..per_counter_operations_cnt {
            let counter = counters[index].clone();
            let read = rng.gen_bool(read_write_ratio);
            if read {
                let fut = async move { counter.get().await };
                futures.push(executor.spawn_with_handle(fut).unwrap());
            } else {
                let new_val = rng.gen_range(0, 10000);
                let fut = async move {
                    counter.set(new_val).await;
                    new_val
                };
                futures.push(executor.spawn_with_handle(fut).unwrap());
            };
        }
    }
    let start = Instant::now();
    let results = futures.collect::<Vec<_>>().await;
    let elapsed = start.elapsed();
    println!(
        "{}, time spent: {} milliseconds, ratio: {}, first val: {}",
        C::name(),
        elapsed.as_millis(),
        read_write_ratio,
        results[0]
    );
}

#[tokio::main]
async fn main() {
    let mut executor = tokio::executor::DefaultExecutor::current();
    let max_counters = 100;
    let per_counter_operations_cnt = 10000;
    let read_write_ratio = 0.9;
    test::<Counter>(
        &mut executor,
        max_counters,
        per_counter_operations_cnt,
        read_write_ratio,
    )
    .await;
    test::<AsyncCounter>(
        &mut executor,
        max_counters,
        per_counter_operations_cnt,
        read_write_ratio,
    )
    .await;
    let read_write_ratio = 0.5;
    test::<Counter>(
        &mut executor,
        max_counters,
        per_counter_operations_cnt,
        read_write_ratio,
    )
        .await;
    test::<AsyncCounter>(
        &mut executor,
        max_counters,
        per_counter_operations_cnt,
        read_write_ratio,
    )
        .await;
}
