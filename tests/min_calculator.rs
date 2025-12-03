use coinduction::*;

#[traitdef]
trait Evaluate {
    fn evaluate(&self, input: &[&'static str], index: &mut usize) -> i32;
}

#[coinduction(Evaluate)]
mod calculator {
    use super::Evaluate;

    pub struct Expr;
    pub struct Term;

    impl Evaluate for Expr
    where
        Term: Evaluate,
    {
        fn evaluate(&self, input: &[&'static str], index: &mut usize) -> i32 {
            let left_val = Term.evaluate(input, index);
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
fn test_simple_addition() {
    use calculator::*;

    // Test: 2 + 3
    let expr = Expr;
    let input = vec!["2", "+", "3"];
    let mut index = 0;
    let result = expr.evaluate(&input, &mut index);
    assert_eq!(result, 5);
}

#[test]
fn test_simple_subtraction() {
    use calculator::*;

    // Test: 5 - 2
    let expr = Expr;
    let input = vec!["5", "-", "2"];
    let mut index = 0;
    let result = expr.evaluate(&input, &mut index);
    assert_eq!(result, 3);
}

#[test]
fn test_single_number() {
    use calculator::*;

    // Test: 42
    let term = Term;
    let input = vec!["42"];
    let mut index = 0;
    let result = term.evaluate(&input, &mut index);
    assert_eq!(result, 42);
}

#[test]
fn test_parenthesized_simple() {
    use calculator::*;

    // Test: (1 + 2)
    let term = Term;
    let input = vec!["(", "1", "+", "2", ")"];
    let mut index = 0;
    let result = term.evaluate(&input, &mut index);
    assert_eq!(result, 3);
}

#[test]
fn test_parenthesized_expression() {
    use calculator::*;

    // Test: (2 + 3)
    let term = Term;
    let input = vec!["(", "2", "+", "3", ")"];
    let mut index = 0;
    let result = term.evaluate(&input, &mut index);
    assert_eq!(result, 5);
}

#[test]
fn test_nested_expression() {
    use calculator::*;

    // Test: 1 + (2 - 3)
    let expr = Expr;
    let input = vec!["1", "+", "(", "2", "-", "3", ")"];
    let mut index = 0;
    let result = expr.evaluate(&input, &mut index);
    assert_eq!(result, 0); // 1 + (2 - 3) = 1 + (-1) = 0
}
