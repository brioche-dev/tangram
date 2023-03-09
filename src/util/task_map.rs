use either::Either;
use futures::future::BoxFuture;
use std::{collections::HashMap, hash::Hash};

pub type TaskFn<K, T> = Box<dyn Fn(K) -> BoxFuture<'static, T> + Send + Sync + 'static>;

pub struct TaskMap<K, T = ()>
where
	K: Clone + Eq + Hash,
	T: Clone,
{
	map: std::sync::Mutex<HashMap<K, tokio::sync::broadcast::Receiver<T>>>,
	f: TaskFn<K, T>,
}

impl<K, T> TaskMap<K, T>
where
	K: Clone + Eq + Hash,
	T: Clone,
{
	/// Create a new [`TaskMap`].
	#[must_use]
	pub fn new(f: TaskFn<K, T>) -> TaskMap<K, T> {
		TaskMap {
			map: std::sync::Mutex::new(HashMap::default()),
			f,
		}
	}

	/// Run a task with the provided key and get its output.
	pub async fn run(&self, key: K) -> T {
		// Determine if this call should await a running task or run the task itself.
		let receiver_or_sender = {
			let mut map = self.map.lock().unwrap();
			if let Some(receiver) = map.get(&key) {
				// If the map has an entry for the key, then resubscribe to the broadcast channel to receive the output.
				let receiver = receiver.resubscribe();
				Either::Left(receiver)
			} else {
				// If the map does not have an entry for the key, then create a broadcast channel and add its receiver to the map.
				let (sender, receiver) = tokio::sync::broadcast::channel(1);
				map.insert(key.to_owned(), receiver);
				Either::Right(sender)
			}
		};

		// Await a running task or run the task and send the output to the broadcast channel when it completes.
		match receiver_or_sender {
			// Await a running task.
			Either::Left(mut receiver) => receiver.recv().await.unwrap(),

			// Run the task and send the output to the broadcast channel when it completes.
			Either::Right(sender) => {
				let output = (self.f)(key.clone()).await;
				let mut map = self.map.lock().unwrap();
				sender.send(output.clone()).ok();
				map.remove(&key);
				output
			},
		}
	}
}
