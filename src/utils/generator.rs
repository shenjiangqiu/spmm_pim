#[cfg(test)]
mod test {
    use genawaiter::{rc::gen, yield_};

    #[test]
    fn gen_test() {
        let abc = gen!({
            let mut i = 0;
            while i < 10 {
                i += 1;
                if i % 2 == 1 {
                    yield_!(i);
                }
            }
        });
        for i in abc {
            println!("{}", i);
        }
    }
}
