use coinduction::*;
use std::fmt::{Display, UpperHex};

#[traitdef((($t1: ty, $t2: ty)) => {$t1: TraitA<S>, $t2: TraitB<S>, S: Display + Default})]
trait TraitA<S> {
    fn get_a(&self) -> String;
}

impl<T1, T2, S> TraitA<S> for (T1, T2)
where
    T1: TraitA<S>,
    T2: TraitB<S>,
    S: Display + Default,
{
    fn get_a(&self) -> String {
        format!(
            "A:{} ({}, {})",
            S::default(),
            self.0.get_a(),
            self.1.get_b()
        )
    }
}

#[traitdef((($t1: ty, $t2: ty)) => {$t1: TraitB<S>, $t2: TraitA<S>, S: Display + Default})]
trait TraitB<S> {
    fn get_b(&self) -> String;
}

impl<T1, T2, S> TraitB<S> for (T1, T2)
where
    T1: TraitB<S>,
    T2: TraitA<S>,
    S: Display + Default,
{
    fn get_b(&self) -> String {
        format!(
            "B:{} ({}, {})",
            S::default(),
            self.0.get_b(),
            self.1.get_a()
        )
    }
}

#[typedef(TraitA, TraitB)]
mod typedef_mod {
    use super::*;

    pub struct TypeA<U>(U);

    impl<S, U> TraitA<S> for TypeA<U>
    where
        U: UpperHex,
    {
        fn get_a(&self) -> String {
            format!("A {:X}", &self.0)
        }
    }

    pub struct TypeB<U>(U);

    impl<S, U> TraitB<S> for TypeB<U>
    where
        U: UpperHex,
    {
        fn get_b(&self) -> String {
            format!("B {:X}", &self.0)
        }
    }

    pub struct Wrapper<T>(T);

    impl<T, S> TraitA<S> for Wrapper<T>
    where
        T: TraitA<S>,
    {
        fn get_a(&self) -> String {
            format!("A [{}]", self.0.get_a())
        }
    }

    impl<T, S> TraitB<S> for Wrapper<T>
    where
        T: TraitB<S>,
    {
        fn get_b(&self) -> String {
            format!("B [{}]", self.0.get_b())
        }
    }

    pub struct Wrapper2<T, U>(T, core::marker::PhantomData<U>);

    impl<T, S, U> TraitA<S> for Wrapper2<T, U>
    where
        T: TraitA<S>,
        U: Default + Display,
    {
        fn get_a(&self) -> String {
            format!("A:{} [{}]", U::default(), self.0.get_a())
        }
    }

    impl<T, S, U> TraitB<S> for Wrapper2<T, U>
    where
        T: TraitB<S>,
        U: Default + Display,
    {
        fn get_b(&self) -> String {
            format!("B:{} [{}]", U::default(), self.0.get_b())
        }
    }
}

use typedef_mod::*;

#[coinduction(TraitA, TraitB)]
mod coinduction_mod {

    use super::*;

    struct RecA<T>(Option<RecB<T>>, core::marker::PhantomData<T>);

    impl<S, T> TraitA<S> for RecA<T>
    where
        RecB<T>: TraitB<S>,
        T: UpperHex + std::default::Default,
    {
        fn get_a(&self) -> String {
            if let Some(b) = &self.0 {
                format!("{:X} {}", T::default(), <RecB<T> as TraitB<S>>::get_b(b))
            } else {
                format!("None")
            }
        }
    }

    struct RecB<T>(Option<Box<RecA<T>>>, core::marker::PhantomData<T>);

    impl<S, T> TraitB<S> for RecB<T>
    where
        RecA<T>: TraitA<S>,
        T: Display + std::default::Default,
    {
        fn get_b(&self) -> String {
            if let Some(a) = &self.0 {
                format!(
                    "{} {}",
                    T::default(),
                    <RecA<T> as TraitA<S>>::get_a(a.as_ref())
                )
            } else {
                format!("None")
            }
        }
    }
}

use coinduction_mod::*;

// #[coinduction(TraitA, TraitB)]
// mod complex_recursive {
//     use super::*;
//
//     struct RecC<T1, T2, T3, T4>((T1, Wrapper2<(T2, (T3, RecD<T1, T2, T3, T4>)), T4>));
//
//     struct RecD<T1, T2, T3, T4>(Option<Box<RecC<T1, T2, T3, T4>>>);
//
//     impl<T1, T2, T3, T4, S> TraitA<S> for RecC<T1, T2, T3, T4>
//     where
//         (T1, Wrapper2<(T2, (T3, RecD<T1, T2, T3, T4>)), T4>): TraitB<S>,
//     {
//         fn get_a(&self) -> String {
//             format!("RecC: {}", self.0.get_b())
//         }
//     }
//
//     impl<T1, T2, T3, T4, S> TraitB<S> for RecD<T1, T2, T3, T4>
//     where
//         RecC<T1, T2, T3, T4>: TraitA<S>,
//     {
//         fn get_b(&self) -> String {
//             if let Some(ref rec_c) = self.0 {
//                 format!("RecD {}", rec_c.get_a())
//             } else {
//                 format!("RecD None")
//             }
//         }
//     }
// }
//
// #[cfg(test)]
// mod tests {
//     use super::*;
//
//     #[test]
//     fn test_rec_a_get_a_with_none() {
//         let rec_a: RecA<i32> = RecA(None);
//         assert_eq!(rec_a.get_a(), "None");
//     }
//
//     #[test]
//     fn test_rec_a_get_a_with_some() {
//         let rec_b = RecB::<i32>(None);
//         let rec_a = RecA(Some(rec_b));
//         assert_eq!(rec_a.get_a(), "0 None");
//     }
//
//     #[test]
//     fn test_rec_a_get_a_nested() {
//         let inner_rec_a = RecA::<u8>(None);
//         let rec_b = RecB(Some(Box::new(inner_rec_a)));
//         let rec_a = RecA(Some(rec_b));
//         assert_eq!(rec_a.get_a(), "0 0None");
//     }
//
//     #[test]
//     fn test_rec_b_get_b_with_none() {
//         let rec_b: RecB<String> = RecB(None);
//         assert_eq!(rec_b.get_b(), "None");
//     }
//
//     #[test]
//     fn test_rec_b_get_b_with_some() {
//         let rec_a = RecA::<String>(None);
//         let rec_b = RecB(Some(Box::new(rec_a)));
//         assert_eq!(rec_b.get_b(), "None");
//     }
//
//     #[test]
//     fn test_rec_b_get_b_nested() {
//         let inner_rec_b = RecB::<u16>(None);
//         let rec_a = RecA(Some(inner_rec_b));
//         let rec_b = RecB(Some(Box::new(rec_a)));
//         assert_eq!(rec_b.get_b(), "0 None");
//     }
//
//     #[test]
//     fn test_rec_a_deep_nesting() {
//         let deepest_rec_a = RecA::<u64>(None);
//         let deep_rec_b = RecB(Some(Box::new(deepest_rec_a)));
//         let mid_rec_a = RecA(Some(deep_rec_b));
//         let mid_rec_b = RecB(Some(Box::new(mid_rec_a)));
//         let top_rec_a = RecA(Some(mid_rec_b));
//         assert_eq!(top_rec_a.get_a(), "0 00 None");
//     }
//
//     #[test]
//     fn test_rec_b_deep_nesting() {
//         let deepest_rec_b = RecB::<char>(None);
//         let deep_rec_a = RecA(Some(deepest_rec_b));
//         let mid_rec_b = RecB(Some(Box::new(deep_rec_a)));
//         let mid_rec_a = RecA(Some(mid_rec_b));
//         let top_rec_b = RecB(Some(Box::new(mid_rec_a)));
//         assert_eq!(top_rec_b.get_b(), " None");
//     }
//
//     #[test]
//     fn test_rec_a_with_float_type() {
//         let inner_rec_b = RecB::<f32>(None);
//         let rec_a = RecA(Some(inner_rec_b));
//         assert_eq!(rec_a.get_a(), "0 None");
//     }
//
//     #[test]
//     fn test_rec_b_with_bool_type() {
//         let inner_rec_a = RecA::<bool>(None);
//         let rec_b = RecB(Some(Box::new(inner_rec_a)));
//         assert_eq!(rec_b.get_b(), "falseNone");
//     }
//
//     #[test]
//     fn test_rec_alternating_chain() {
//         let level4_a = RecA::<u8>(None);
//         let level3_b = RecB(Some(Box::new(level4_a)));
//         let level2_a = RecA(Some(level3_b));
//         let level1_b = RecB(Some(Box::new(level2_a)));
//         let level0_a = RecA(Some(level1_b));
//         assert_eq!(level0_a.get_a(), "0 00 None");
//         assert_eq!(level1_b.get_b(), "00 None");
//     }
//
//     #[test]
//     fn test_rec_with_usize_type() {
//         let rec_b = RecB::<usize>(None);
//         let rec_a = RecA(Some(rec_b));
//         assert_eq!(rec_a.get_a(), "0 None");
//     }
//
//     #[test]
//     fn test_rec_mixed_numeric_types() {
//         let i8_rec_a = RecA::<i8>(None);
//         let i16_rec_b = RecB(Some(Box::new(i8_rec_a)));
//         let i32_rec_a = RecA(Some(i16_rec_b));
//         let i64_rec_b = RecB(Some(Box::new(i32_rec_a)));
//         assert_eq!(i64_rec_b.get_b(), "00 None");
//     }
// }
