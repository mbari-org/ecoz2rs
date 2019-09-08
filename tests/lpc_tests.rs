#[cfg(test)]
mod tests {

    fn get_input() -> Vec<i32> {
        vec![
            0, 1, 7, 2, 5, 8, 16, 3, 19, 6, 14, 9, 9, 17, 17, 4, 12, 20, 20, 7, 7, 15, 15, 10, 23,
        ]
    }

    #[test]
    fn test_clone() {
        let inp = get_input();

        let signal = {
            let mut x = vec![0i32; inp.len()];
            x.clone_from_slice(&inp[..inp.len()]);
            x
        };
        assert_eq!(inp, signal);
    }
}
