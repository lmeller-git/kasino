use std::{collections::VecDeque, sync::Mutex, thread};

use kasino::{InlineBandit, WithCapacity, components::PushPopCollection, strategy::DCBO};

struct MyQueue<T> {
    raw: Mutex<VecDeque<T>>,
}

impl<T> PushPopCollection for MyQueue<T> {
    type Item = T;

    fn push(&self, item: Self::Item) -> Result<(), Self::Item> {
        self.raw.lock().unwrap().push_back(item);
        Ok(())
    }

    fn pop(&self) -> Option<Self::Item> {
        self.raw.lock().unwrap().pop_front()
    }

    fn len(&self) -> usize {
        self.raw.lock().unwrap().len()
    }

    fn capacity(&self) -> usize {
        self.raw.lock().unwrap().capacity()
    }
}

impl<T, const N: usize> WithCapacity<N> for MyQueue<T> {
    fn with_capacity() -> Self {
        Self {
            raw: Mutex::new(VecDeque::with_capacity(N)),
        }
    }
}

fn main() {
    let mab: InlineBandit<MyQueue<_>, DCBO, 8> = InlineBandit::new();
    let mut player = mab.buy_in();
    let mut player2 = player.fork();
    let mut player3 = player.fork();

    thread::scope(|scope| {
        scope.spawn(move || {
            for _ in 0..10 {
                player.push(1).unwrap();
            }
        });

        scope.spawn(move || {
            for _ in 0..10 {
                player2.push(2).unwrap();
            }
        });
    });

    let mut total = 0;
    while let Some(item) = player3.pop() {
        total += item;
    }

    assert_eq!(total, 30);
}
