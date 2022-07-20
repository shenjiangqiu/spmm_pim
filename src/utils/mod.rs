pub mod plot;
pub mod run;
#[cfg(test)]
mod test {
    use core::slice;

    use enum_as_inner::EnumAsInner;
    use log::{debug, info};
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
    struct Char<const C: char>;
    #[test]
    fn test() {
        println!("{:?}", std::mem::size_of::<TestEnum1>());
        println!("{:?}", std::mem::size_of::<TestEnum2>());

        let _a = Char::<'🦀'>;
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

    #[test]
    fn test_debug() {
        // ---- first create neccessary status structures
        let config_str = include_str!("../../log_config.yml");
        let config = serde_yaml::from_str(config_str).unwrap();
        log4rs::init_raw_config(config).unwrap_or(());
        let mut a = 10;
        info!("{:?}", {
            a = a + 1;
            a
        });
        // the info will be excecuted
        assert_eq!(a, 11);
        debug!("{:?}", {
            a = a + 1;
            a
        });
        // the debug will be ignored
        assert_eq!(a, 11);
    }
}
