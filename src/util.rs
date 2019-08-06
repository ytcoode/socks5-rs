pub fn key_to_token(k: usize) -> usize {
    let t = k + 1;
    if t > isize::max_value() as usize {
        panic!("too many connections");
    }
    t
}

pub fn token_to_key(t: usize) -> usize {
    (t as isize).abs() as usize - 1
}

pub fn peer_token(t: usize) -> usize {
    -(t as isize) as usize
}
