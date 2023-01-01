use crossbeam_channel::unbounded;
use std::{thread, time};

fn parallel_map<T, U, F>(mut input_vec: Vec<T>, num_threads: usize, f: F) -> Vec<U>
where
    F: FnOnce(T) -> U + Send + Copy + 'static,
    T: Send + 'static,
    U: Send + 'static + Default + std::fmt::Debug,
{
    let mut threads = Vec::new();
    let (sender, receiver) = unbounded();
    let (sender2, receiver2) = unbounded();
    let l = input_vec.len();
    let mut output_vec: Vec<U> = Vec::with_capacity(l);
    for _ in 0..input_vec.len() {
        output_vec.push(Default::default());
    }
    for _ in 0..num_threads {
        let receiver = receiver.clone();
        let sender2 = sender2.clone();
        threads.push(thread::spawn(move || {
            while let Ok((i, next_num)) = receiver.recv() {
                sender2.send((i, f(next_num))).expect("send result");
            }
            drop(sender2);
        }));
    }
    let mut i = 0;
    while let Some(num) = input_vec.pop() {
        sender
            .send((i, num))
            .expect("Tried writing to channel, but there are no receivers!");
        i += 1;
    }

    drop(sender);
    drop(sender2);

    while let Ok((i, rv)) = receiver2.recv() {
        output_vec[l - i - 1] = rv;
    }
    drop(receiver2);

    for thread in threads {
        thread.join().expect("Panic occurred in thread");
    }

    output_vec
}

fn main() {
    let v = vec![6, 7, 8, 9, 10, 1, 2, 3, 4, 5, 12, 18, 11, 5, 20];
    let squares = parallel_map(v, 10, |num| {
        println!("{} squared is {}", num, num * num);
        thread::sleep(time::Duration::from_millis(500));
        num * num
    });
    println!("squares: {:?}", squares);
}
