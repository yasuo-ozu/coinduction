use coinduction::coinduction;

#[coinduction]
mod calculator {
    #[coinduction]
    pub trait Evaluate {
        fn evaluate(&self, input: &[&'static str], index: &mut usize) -> i32;
    }

    pub struct Expr;
    pub struct Term;

    impl Evaluate for Expr
    where
        Term: Evaluate,
    {
        fn evaluate(&self, input: &[&'static str], index: &mut usize) -> i32 {
            let left_val = Term.evaluate(input, index);
            if *index >= input.len() || input[*index] == ")" {
                return left_val;
            }
            let op = input[*index];
            *index += 1;
            let right_val = Term.evaluate(input, index);
            match op {
                "+" => left_val + right_val,
                "-" => left_val - right_val,
                _ => left_val,
            }
        }
    }

    impl Evaluate for Term
    where
        Expr: Evaluate,
    {
        fn evaluate(&self, input: &[&'static str], index: &mut usize) -> i32 {
            let token = input[*index];
            *index += 1;
            if token == "(" {
                let result = Expr.evaluate(input, index);
                *index += 1; // skip closing ')'
                result
            } else {
                token.parse::<i32>().unwrap()
            }
        }
    }
}

#[test]
fn test_inner_trait() {
    use calculator::{Evaluate, Expr};

    let input = vec!["1", "+", "(", "2", "-", "3", ")"];
    let mut index = 0;
    assert_eq!(Expr.evaluate(&input, &mut index), 0);
}
