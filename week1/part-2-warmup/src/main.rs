/* The following exercises were borrowed from Will Crichton's CS 242 Rust lab. */

use std::collections::HashSet;

fn main() {
    println!("Hi! Try running \"cargo test\" to run tests.");
}

fn add_n(v: Vec<i32>, n: i32) -> Vec<i32> {
    let mut new_v = Vec::new();
    for i in v.iter() {
        // v is the vector from above
        new_v.push(n + i);
    }

    new_v
}

fn add_n_inplace(v: &mut Vec<i32>, n: i32) {
    for i in v.iter_mut() {
        *i = *i + n;
    }
}

fn dedup(v: &mut Vec<i32>) {
    let mut indexes = vec![];
    let mut hs: HashSet<i32> = HashSet::new();

    let mut i = 0;
    for ele in v.iter() {
        if hs.contains(ele) {
            indexes.push(i);
        } else {
            hs.insert(*ele);
        }

        i += 1;
    }

    let mut x: usize = indexes.len() - 1;

    // true
    // while x > 0 will have warning ..........
    loop {
        v.remove(indexes[x]);
        if x == 0 {
            break;
        }
        x = x - 1;
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_add_n() {
        assert_eq!(add_n(vec![1], 2), vec![3]);
    }

    #[test]
    fn test_add_n_inplace() {
        let mut v = vec![1];
        add_n_inplace(&mut v, 2);
        assert_eq!(v, vec![3]);
    }

    #[test]
    fn test_dedup() {
        let mut v = vec![3, 1, 0, 1, 4, 4];
        dedup(&mut v);
        assert_eq!(v, vec![3, 1, 0, 4]);
    }
}
