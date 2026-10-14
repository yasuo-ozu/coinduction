//! A chain of 24 constraints dispatched through trait rules must fit in the default
//! `recursion_limit`.

use coinduction::coinduction;

#[coinduction(
    (($T:ty, $U:ty)) => { $T: Size, $U: Size },
    (u8) => {}
)]
pub trait Size {
    fn size(&self) -> usize;
}

impl Size for u8 {
    fn size(&self) -> usize {
        1
    }
}

impl<T: Size, U: Size> Size for (T, U) {
    fn size(&self) -> usize {
        self.0.size() + self.1.size()
    }
}

#[coinduction]
pub mod chain {
    #[coinduction]
    use super::Size;

    pub struct A;

    impl Size for A
    where
        (u8, A): Size,
        (u8, (u8, A)): Size,
        (u8, (u8, (u8, A))): Size,
        (u8, (u8, (u8, (u8, A)))): Size,
        (u8, (u8, (u8, (u8, (u8, A))))): Size,
        (u8, (u8, (u8, (u8, (u8, (u8, A)))))): Size,
        (u8, (u8, (u8, (u8, (u8, (u8, (u8, A))))))): Size,
        (u8, (u8, (u8, (u8, (u8, (u8, (u8, (u8, A)))))))): Size,
        (u8, (u8, (u8, (u8, (u8, (u8, (u8, (u8, (u8, A))))))))): Size,
        (u8, (u8, (u8, (u8, (u8, (u8, (u8, (u8, (u8, (u8, A)))))))))): Size,
        (u8, (u8, (u8, (u8, (u8, (u8, (u8, (u8, (u8, (u8, (u8, A))))))))))): Size,
        (u8, (u8, (u8, (u8, (u8, (u8, (u8, (u8, (u8, (u8, (u8, (u8, A)))))))))))): Size,
        (u8, (u8, (u8, (u8, (u8, (u8, (u8, (u8, (u8, (u8, (u8, (u8, (u8, A))))))))))))): Size,
        (u8, (u8, (u8, (u8, (u8, (u8, (u8, (u8, (u8, (u8, (u8, (u8, (u8, (u8, A)))))))))))))): Size,
        (u8, (u8, (u8, (u8, (u8, (u8, (u8, (u8, (u8, (u8, (u8, (u8, (u8, (u8, (u8, A))))))))))))))): Size,
        (u8, (u8, (u8, (u8, (u8, (u8, (u8, (u8, (u8, (u8, (u8, (u8, (u8, (u8, (u8, (u8, A)))))))))))))))): Size,
        (u8, (u8, (u8, (u8, (u8, (u8, (u8, (u8, (u8, (u8, (u8, (u8, (u8, (u8, (u8, (u8, (u8, A))))))))))))))))): Size,
        (u8, (u8, (u8, (u8, (u8, (u8, (u8, (u8, (u8, (u8, (u8, (u8, (u8, (u8, (u8, (u8, (u8, (u8, A)))))))))))))))))): Size,
        (u8, (u8, (u8, (u8, (u8, (u8, (u8, (u8, (u8, (u8, (u8, (u8, (u8, (u8, (u8, (u8, (u8, (u8, (u8, A))))))))))))))))))): Size,
        (u8, (u8, (u8, (u8, (u8, (u8, (u8, (u8, (u8, (u8, (u8, (u8, (u8, (u8, (u8, (u8, (u8, (u8, (u8, (u8, A)))))))))))))))))))): Size,
        (u8, (u8, (u8, (u8, (u8, (u8, (u8, (u8, (u8, (u8, (u8, (u8, (u8, (u8, (u8, (u8, (u8, (u8, (u8, (u8, (u8, A))))))))))))))))))))): Size,
        (u8, (u8, (u8, (u8, (u8, (u8, (u8, (u8, (u8, (u8, (u8, (u8, (u8, (u8, (u8, (u8, (u8, (u8, (u8, (u8, (u8, (u8, A)))))))))))))))))))))): Size,
        (u8, (u8, (u8, (u8, (u8, (u8, (u8, (u8, (u8, (u8, (u8, (u8, (u8, (u8, (u8, (u8, (u8, (u8, (u8, (u8, (u8, (u8, (u8, A))))))))))))))))))))))): Size,
        (u8, (u8, (u8, (u8, (u8, (u8, (u8, (u8, (u8, (u8, (u8, (u8, (u8, (u8, (u8, (u8, (u8, (u8, (u8, (u8, (u8, (u8, (u8, (u8, A)))))))))))))))))))))))): Size,
    {
        fn size(&self) -> usize {
            1
        }
    }
}

#[test]
fn test_chain_depth() {
    assert_eq!(chain::A.size(), 1);
}
