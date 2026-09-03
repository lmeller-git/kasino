use crate::{
    InlineBandit,
    strategy::{DCBO, DoubleCollect},
    sync::{
        Arc,
        Mutex,
        atomic::{AtomicUsize, Ordering},
        thread,
    },
    tests::test_library::{
        LockedDeque,
        MutAccessForkCollection,
        assert_exact_set,
        retry_dequeue,
        retry_enqueue,
    },
};

pub(crate) fn linearizable<Q>(q: Q)
where
    Q: MutAccessForkCollection<Item = u32> + Sync + Send + 'static,
{
    const COUNT: usize = 1;
    const THREADS: usize = 2;

    let mut threads = Vec::new();

    for _ in 0..THREADS {
        let mut q2 = q.fork();
        threads.push(thread::spawn(move || {
            for _ in 0..COUNT {
                while q2.enqueue(42).is_err() {
                    thread::yield_now();
                }
                q2.dequeue().unwrap();
            }
        }));
    }

    for t in threads {
        t.join().unwrap();
    }
}

pub(crate) fn spsc<Q>(q: Q)
where
    Q: MutAccessForkCollection<Item = u32> + Sync + Send + 'static,
{
    const COUNT: usize = 2;

    let mut push_arm = q.fork();
    let mut pop_arm = q.fork();
    let popped = Arc::new(Mutex::new(Vec::with_capacity(COUNT)));
    let p_h = popped.clone();

    let consumer = thread::spawn(move || {
        for _ in 0..COUNT {
            p_h.lock().push(retry_dequeue(&mut pop_arm));
        }
        assert!(pop_arm.dequeue().is_none());
    });

    let producer = thread::spawn(move || {
        for i in 0..COUNT {
            retry_enqueue(&mut push_arm, i as u32);
        }
    });

    consumer.join().unwrap();
    producer.join().unwrap();
    assert_exact_set(&popped.lock(), 0..COUNT as u32);
}

pub(crate) fn mpsc<Q>(q: Q)
where
    Q: MutAccessForkCollection<Item = u32> + Sync + Send + 'static,
{
    const COUNT: usize = 2;
    const THREADS: usize = 2;

    let mut pop_arm = q.fork();
    let v = Arc::new((0..COUNT).map(|_| AtomicUsize::new(0)).collect::<Vec<_>>());

    let handles: Vec<_> = (0..THREADS)
        .map(|_| {
            let mut arm = q.fork();
            thread::spawn(move || {
                for i in 0..COUNT {
                    retry_enqueue(&mut arm, i as u32);
                }
            })
        })
        .collect();

    for _ in 0..THREADS {
        for _ in 0..COUNT {
            let n = retry_dequeue(&mut pop_arm);
            v[n as usize].fetch_add(1, Ordering::SeqCst);
        }
    }

    for h in handles {
        h.join().unwrap();
    }

    for c in v.iter() {
        assert_eq!(c.load(Ordering::SeqCst), THREADS);
    }
}

#[test]
fn spsc_impl() {
    loom::model(|| {
        let q: &'static InlineBandit<LockedDeque<u32>, DCBO, 3, 1> =
            Box::leak(Box::new(InlineBandit::new()));
        spsc(q.buy_in());
    });
}

#[test]
fn mpsc_impl() {
    loom::model(|| {
        let q: &'static InlineBandit<LockedDeque<u32>, DCBO, 3, 1> =
            Box::leak(Box::new(InlineBandit::new()));
        mpsc(q.buy_in());
    });
}

#[test]
fn linearizable_impl() {
    loom::model(|| {
        let q: &'static InlineBandit<LockedDeque<u32>, DoubleCollect<DCBO>, 3, 1> =
            Box::leak(Box::new(InlineBandit::new()));
        linearizable(q.buy_in());
    })
}
