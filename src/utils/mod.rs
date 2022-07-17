pub mod plot;
pub mod run;
#[cfg(test)]
mod test {
    use core::slice;

    use enum_as_inner::EnumAsInner;
    #[derive(EnumAsInner, Debug, Default)]
    enum TestEnum1 {
        A(i32),
        #[default]
        B,
    }
    enum TestEnum2 {
        A(TestEnum1),
        B,
    }
    #[test]
    fn test() {
        println!("{:?}", std::mem::size_of::<TestEnum1>());
        println!("{:?}", std::mem::size_of::<TestEnum2>());
        let aa10 = TestEnum2::A(TestEnum1::A(65536));
        let b = TestEnum2::B;
        let ab = TestEnum2::A(TestEnum1::B);
        println!("{:?}", std::mem::size_of_val(&b));
        let b: &[u8] = unsafe {
            slice::from_raw_parts(
                &b as *const TestEnum2 as *const u8,
                std::mem::size_of_val(&b),
            )
        };

        let aa10: &[u8] = unsafe {
            slice::from_raw_parts(
                &aa10 as *const TestEnum2 as *const u8,
                std::mem::size_of_val(&aa10),
            )
        };
        let ab: &[u8] = unsafe {
            slice::from_raw_parts(
                &ab as *const TestEnum2 as *const u8,
                std::mem::size_of_val(&ab),
            )
        };
        println!("{:?}", aa10);
        println!("{:?}", ab);
        println!("{:?}", b);
    }
}
