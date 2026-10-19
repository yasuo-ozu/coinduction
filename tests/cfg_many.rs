//! Only the impls of the traits the module works on go through the `macro_rules!` evaluating the
//! `cfg`s, one level of macro expansion each: other conditional impls are left to rustc, however
//! many there are.
#![allow(dead_code)]
use coinduction::*;

#[announce::announce]
pub trait Count {
    fn count(&self) -> usize;
}

pub trait Other {
    fn other() -> usize;
}

#[coinduction]
#[rustfmt::skip]
pub mod many {
    #[coinduction]
    use super::Count;
    use super::Other;

    pub struct Node(pub Option<Box<Leaf>>);
    pub struct Leaf(pub Option<Box<Node>>);

    impl Count for Node where Leaf: Count {
        fn count(&self) -> usize { self.0.as_ref().map_or(0, |n| n.count()) + 1 }
    }

    #[cfg(not(any()))]
    impl Count for Leaf where Node: Count {
        fn count(&self) -> usize { self.0.as_ref().map_or(0, |n| n.count()) + 1 }
    }

    // A disabled impl of another trait: it would conflict with the enabled one
    #[cfg(any())]
    impl Other for Node { fn other() -> usize { 0 } }
    #[cfg(not(any()))]
    impl Other for Node { fn other() -> usize { 1 } }

    // More inherent impls under `cfg` than the recursion limit
    #[cfg(not(any()))] impl Node { pub fn f0() -> usize { 0 } }
    #[cfg(not(any()))] impl Node { pub fn f1() -> usize { 1 } }
    #[cfg(not(any()))] impl Node { pub fn f2() -> usize { 2 } }
    #[cfg(not(any()))] impl Node { pub fn f3() -> usize { 3 } }
    #[cfg(not(any()))] impl Node { pub fn f4() -> usize { 4 } }
    #[cfg(not(any()))] impl Node { pub fn f5() -> usize { 5 } }
    #[cfg(not(any()))] impl Node { pub fn f6() -> usize { 6 } }
    #[cfg(not(any()))] impl Node { pub fn f7() -> usize { 7 } }
    #[cfg(not(any()))] impl Node { pub fn f8() -> usize { 8 } }
    #[cfg(not(any()))] impl Node { pub fn f9() -> usize { 9 } }
    #[cfg(not(any()))] impl Node { pub fn f10() -> usize { 10 } }
    #[cfg(not(any()))] impl Node { pub fn f11() -> usize { 11 } }
    #[cfg(not(any()))] impl Node { pub fn f12() -> usize { 12 } }
    #[cfg(not(any()))] impl Node { pub fn f13() -> usize { 13 } }
    #[cfg(not(any()))] impl Node { pub fn f14() -> usize { 14 } }
    #[cfg(not(any()))] impl Node { pub fn f15() -> usize { 15 } }
    #[cfg(not(any()))] impl Node { pub fn f16() -> usize { 16 } }
    #[cfg(not(any()))] impl Node { pub fn f17() -> usize { 17 } }
    #[cfg(not(any()))] impl Node { pub fn f18() -> usize { 18 } }
    #[cfg(not(any()))] impl Node { pub fn f19() -> usize { 19 } }
    #[cfg(not(any()))] impl Node { pub fn f20() -> usize { 20 } }
    #[cfg(not(any()))] impl Node { pub fn f21() -> usize { 21 } }
    #[cfg(not(any()))] impl Node { pub fn f22() -> usize { 22 } }
    #[cfg(not(any()))] impl Node { pub fn f23() -> usize { 23 } }
    #[cfg(not(any()))] impl Node { pub fn f24() -> usize { 24 } }
    #[cfg(not(any()))] impl Node { pub fn f25() -> usize { 25 } }
    #[cfg(not(any()))] impl Node { pub fn f26() -> usize { 26 } }
    #[cfg(not(any()))] impl Node { pub fn f27() -> usize { 27 } }
    #[cfg(not(any()))] impl Node { pub fn f28() -> usize { 28 } }
    #[cfg(not(any()))] impl Node { pub fn f29() -> usize { 29 } }
    #[cfg(not(any()))] impl Node { pub fn f30() -> usize { 30 } }
    #[cfg(not(any()))] impl Node { pub fn f31() -> usize { 31 } }
    #[cfg(not(any()))] impl Node { pub fn f32() -> usize { 32 } }
    #[cfg(not(any()))] impl Node { pub fn f33() -> usize { 33 } }
    #[cfg(not(any()))] impl Node { pub fn f34() -> usize { 34 } }
    #[cfg(not(any()))] impl Node { pub fn f35() -> usize { 35 } }
    #[cfg(not(any()))] impl Node { pub fn f36() -> usize { 36 } }
    #[cfg(not(any()))] impl Node { pub fn f37() -> usize { 37 } }
    #[cfg(not(any()))] impl Node { pub fn f38() -> usize { 38 } }
    #[cfg(not(any()))] impl Node { pub fn f39() -> usize { 39 } }
    #[cfg(not(any()))] impl Node { pub fn f40() -> usize { 40 } }
    #[cfg(not(any()))] impl Node { pub fn f41() -> usize { 41 } }
    #[cfg(not(any()))] impl Node { pub fn f42() -> usize { 42 } }
    #[cfg(not(any()))] impl Node { pub fn f43() -> usize { 43 } }
    #[cfg(not(any()))] impl Node { pub fn f44() -> usize { 44 } }
    #[cfg(not(any()))] impl Node { pub fn f45() -> usize { 45 } }
    #[cfg(not(any()))] impl Node { pub fn f46() -> usize { 46 } }
    #[cfg(not(any()))] impl Node { pub fn f47() -> usize { 47 } }
    #[cfg(not(any()))] impl Node { pub fn f48() -> usize { 48 } }
    #[cfg(not(any()))] impl Node { pub fn f49() -> usize { 49 } }
    #[cfg(not(any()))] impl Node { pub fn f50() -> usize { 50 } }
    #[cfg(not(any()))] impl Node { pub fn f51() -> usize { 51 } }
    #[cfg(not(any()))] impl Node { pub fn f52() -> usize { 52 } }
    #[cfg(not(any()))] impl Node { pub fn f53() -> usize { 53 } }
    #[cfg(not(any()))] impl Node { pub fn f54() -> usize { 54 } }
    #[cfg(not(any()))] impl Node { pub fn f55() -> usize { 55 } }
    #[cfg(not(any()))] impl Node { pub fn f56() -> usize { 56 } }
    #[cfg(not(any()))] impl Node { pub fn f57() -> usize { 57 } }
    #[cfg(not(any()))] impl Node { pub fn f58() -> usize { 58 } }
    #[cfg(not(any()))] impl Node { pub fn f59() -> usize { 59 } }
    #[cfg(not(any()))] impl Node { pub fn f60() -> usize { 60 } }
    #[cfg(not(any()))] impl Node { pub fn f61() -> usize { 61 } }
    #[cfg(not(any()))] impl Node { pub fn f62() -> usize { 62 } }
    #[cfg(not(any()))] impl Node { pub fn f63() -> usize { 63 } }
    #[cfg(not(any()))] impl Node { pub fn f64() -> usize { 64 } }
    #[cfg(not(any()))] impl Node { pub fn f65() -> usize { 65 } }
    #[cfg(not(any()))] impl Node { pub fn f66() -> usize { 66 } }
    #[cfg(not(any()))] impl Node { pub fn f67() -> usize { 67 } }
    #[cfg(not(any()))] impl Node { pub fn f68() -> usize { 68 } }
    #[cfg(not(any()))] impl Node { pub fn f69() -> usize { 69 } }
    #[cfg(not(any()))] impl Node { pub fn f70() -> usize { 70 } }
    #[cfg(not(any()))] impl Node { pub fn f71() -> usize { 71 } }
    #[cfg(not(any()))] impl Node { pub fn f72() -> usize { 72 } }
    #[cfg(not(any()))] impl Node { pub fn f73() -> usize { 73 } }
    #[cfg(not(any()))] impl Node { pub fn f74() -> usize { 74 } }
    #[cfg(not(any()))] impl Node { pub fn f75() -> usize { 75 } }
    #[cfg(not(any()))] impl Node { pub fn f76() -> usize { 76 } }
    #[cfg(not(any()))] impl Node { pub fn f77() -> usize { 77 } }
    #[cfg(not(any()))] impl Node { pub fn f78() -> usize { 78 } }
    #[cfg(not(any()))] impl Node { pub fn f79() -> usize { 79 } }
    #[cfg(not(any()))] impl Node { pub fn f80() -> usize { 80 } }
    #[cfg(not(any()))] impl Node { pub fn f81() -> usize { 81 } }
    #[cfg(not(any()))] impl Node { pub fn f82() -> usize { 82 } }
    #[cfg(not(any()))] impl Node { pub fn f83() -> usize { 83 } }
    #[cfg(not(any()))] impl Node { pub fn f84() -> usize { 84 } }
    #[cfg(not(any()))] impl Node { pub fn f85() -> usize { 85 } }
    #[cfg(not(any()))] impl Node { pub fn f86() -> usize { 86 } }
    #[cfg(not(any()))] impl Node { pub fn f87() -> usize { 87 } }
    #[cfg(not(any()))] impl Node { pub fn f88() -> usize { 88 } }
    #[cfg(not(any()))] impl Node { pub fn f89() -> usize { 89 } }
    #[cfg(not(any()))] impl Node { pub fn f90() -> usize { 90 } }
    #[cfg(not(any()))] impl Node { pub fn f91() -> usize { 91 } }
    #[cfg(not(any()))] impl Node { pub fn f92() -> usize { 92 } }
    #[cfg(not(any()))] impl Node { pub fn f93() -> usize { 93 } }
    #[cfg(not(any()))] impl Node { pub fn f94() -> usize { 94 } }
    #[cfg(not(any()))] impl Node { pub fn f95() -> usize { 95 } }
    #[cfg(not(any()))] impl Node { pub fn f96() -> usize { 96 } }
    #[cfg(not(any()))] impl Node { pub fn f97() -> usize { 97 } }
    #[cfg(not(any()))] impl Node { pub fn f98() -> usize { 98 } }
    #[cfg(not(any()))] impl Node { pub fn f99() -> usize { 99 } }
    #[cfg(not(any()))] impl Node { pub fn f100() -> usize { 100 } }
    #[cfg(not(any()))] impl Node { pub fn f101() -> usize { 101 } }
    #[cfg(not(any()))] impl Node { pub fn f102() -> usize { 102 } }
    #[cfg(not(any()))] impl Node { pub fn f103() -> usize { 103 } }
    #[cfg(not(any()))] impl Node { pub fn f104() -> usize { 104 } }
    #[cfg(not(any()))] impl Node { pub fn f105() -> usize { 105 } }
    #[cfg(not(any()))] impl Node { pub fn f106() -> usize { 106 } }
    #[cfg(not(any()))] impl Node { pub fn f107() -> usize { 107 } }
    #[cfg(not(any()))] impl Node { pub fn f108() -> usize { 108 } }
    #[cfg(not(any()))] impl Node { pub fn f109() -> usize { 109 } }
    #[cfg(not(any()))] impl Node { pub fn f110() -> usize { 110 } }
    #[cfg(not(any()))] impl Node { pub fn f111() -> usize { 111 } }
    #[cfg(not(any()))] impl Node { pub fn f112() -> usize { 112 } }
    #[cfg(not(any()))] impl Node { pub fn f113() -> usize { 113 } }
    #[cfg(not(any()))] impl Node { pub fn f114() -> usize { 114 } }
    #[cfg(not(any()))] impl Node { pub fn f115() -> usize { 115 } }
    #[cfg(not(any()))] impl Node { pub fn f116() -> usize { 116 } }
    #[cfg(not(any()))] impl Node { pub fn f117() -> usize { 117 } }
    #[cfg(not(any()))] impl Node { pub fn f118() -> usize { 118 } }
    #[cfg(not(any()))] impl Node { pub fn f119() -> usize { 119 } }
    #[cfg(not(any()))] impl Node { pub fn f120() -> usize { 120 } }
    #[cfg(not(any()))] impl Node { pub fn f121() -> usize { 121 } }
    #[cfg(not(any()))] impl Node { pub fn f122() -> usize { 122 } }
    #[cfg(not(any()))] impl Node { pub fn f123() -> usize { 123 } }
    #[cfg(not(any()))] impl Node { pub fn f124() -> usize { 124 } }
    #[cfg(not(any()))] impl Node { pub fn f125() -> usize { 125 } }
    #[cfg(not(any()))] impl Node { pub fn f126() -> usize { 126 } }
    #[cfg(not(any()))] impl Node { pub fn f127() -> usize { 127 } }
    #[cfg(not(any()))] impl Node { pub fn f128() -> usize { 128 } }
    #[cfg(not(any()))] impl Node { pub fn f129() -> usize { 129 } }
    #[cfg(not(any()))] impl Node { pub fn f130() -> usize { 130 } }
    #[cfg(not(any()))] impl Node { pub fn f131() -> usize { 131 } }
    #[cfg(not(any()))] impl Node { pub fn f132() -> usize { 132 } }
    #[cfg(not(any()))] impl Node { pub fn f133() -> usize { 133 } }
    #[cfg(not(any()))] impl Node { pub fn f134() -> usize { 134 } }
    #[cfg(not(any()))] impl Node { pub fn f135() -> usize { 135 } }
    #[cfg(not(any()))] impl Node { pub fn f136() -> usize { 136 } }
    #[cfg(not(any()))] impl Node { pub fn f137() -> usize { 137 } }
    #[cfg(not(any()))] impl Node { pub fn f138() -> usize { 138 } }
    #[cfg(not(any()))] impl Node { pub fn f139() -> usize { 139 } }
    #[cfg(not(any()))] impl Node { pub fn f140() -> usize { 140 } }
    #[cfg(not(any()))] impl Node { pub fn f141() -> usize { 141 } }
    #[cfg(not(any()))] impl Node { pub fn f142() -> usize { 142 } }
    #[cfg(not(any()))] impl Node { pub fn f143() -> usize { 143 } }
    #[cfg(not(any()))] impl Node { pub fn f144() -> usize { 144 } }
    #[cfg(not(any()))] impl Node { pub fn f145() -> usize { 145 } }
    #[cfg(not(any()))] impl Node { pub fn f146() -> usize { 146 } }
    #[cfg(not(any()))] impl Node { pub fn f147() -> usize { 147 } }
    #[cfg(not(any()))] impl Node { pub fn f148() -> usize { 148 } }
    #[cfg(not(any()))] impl Node { pub fn f149() -> usize { 149 } }
}

#[test]
fn many_conditional_impls() {
    use many::{Leaf, Node};
    let leaf = Leaf(Some(Box::new(Node(None))));
    assert_eq!(leaf.count(), 2);
    assert_eq!(<Node as Other>::other(), 1);
    assert_eq!(Node::f0() + Node::f149(), 149);
}
