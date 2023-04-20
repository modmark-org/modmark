use crate::utils::PlotContext;
use evalexpr::*;

pub const PLOT_EPSILON: f64 = 8.1e-5; // has to be larger than ERROR_TOLERANCE
pub const ERROR_TOLERANCE: f64 = 8e-5;

// Helper function to reduce duplicate code in new_function_context
fn value_to_float(v: &Value) -> Option<f64> {
    match v {
        Value::Float(f) => Some(*f),
        Value::Int(i) => Some(*i as f64),
        _ => None,
    }
}

// Create a new function context with functions sin, cos, tan, sqrt, log, mod
pub fn new_function_context() -> HashMapContext {
    let mut ctx = HashMapContext::new();
    ctx.set_function(
        String::from("sin"),
        Function::new(|arg| match value_to_float(arg) {
            Some(float) => Ok(Value::Float(float.sin())),
            None => Err(EvalexprError::expected_float(arg.clone())),
        }),
    )
    .unwrap();
    ctx.set_function(
        String::from("cos"),
        Function::new(|arg| match value_to_float(arg) {
            Some(float) => Ok(Value::Float(float.cos())),
            None => Err(EvalexprError::expected_float(arg.clone())),
        }),
    )
    .unwrap();
    ctx.set_function(
        String::from("tan"),
        Function::new(|arg| match value_to_float(arg) {
            Some(float) => Ok(Value::Float(float.tan())),
            None => Err(EvalexprError::expected_float(arg.clone())),
        }),
    )
    .unwrap();
    ctx.set_function(
        String::from("sqrt"),
        Function::new(|arg| match value_to_float(arg) {
            Some(float) => Ok(Value::Float(float.sqrt())),
            None => Err(EvalexprError::expected_float(arg.clone())),
        }),
    )
    .unwrap();
    ctx.set_function(
        String::from("log"),
        Function::new(|arg| {
            let args = arg.as_tuple()?;
            match (value_to_float(&args[0]), value_to_float(&args[1])) {
                (Some(a), Some(b)) => Ok(Value::Float(b.log(a))),
                (_, _) => Err(EvalexprError::expected_float(arg.clone())),
            }
        }),
    )
    .unwrap();
    ctx.set_function(
        String::from("mod"),
        Function::new(|arg| {
            let args = arg.as_tuple()?;
            match (value_to_float(&args[0]), value_to_float(&args[1])) {
                (Some(a), Some(b)) => Ok(Value::Float(b % a)),
                (_, _) => Err(EvalexprError::expected_float(arg.clone())),
            }
        }),
    )
    .unwrap();

    ctx
}

// Assign values to variables. This function was designed to work with an arbitrary number of
// variables (to support 3d plotting)
fn update_function_context(ctx: &mut HashMapContext, names: &[String], values: &[f64]) {
    let mut names_iter = names.iter();
    let mut values_iter = values.iter();
    while let (Some(name), Some(&value)) = (names_iter.next(), values_iter.next()) {
        // Only values from float ranges will be used here, fine to unwrap
        ctx.set_value(name.into(), value.into()).unwrap();
    }
}

// Approximate a point at the boundary, given two points and the distance to the boundary
pub fn get_point_at_boundary(outside: (f64, f64), inside: (f64, f64), y_margin: f64) -> (f64, f64) {
    let (x, y) = outside;
    let (a, b) = inside;
    let (dx, dy) = (x - a, y - b);
    let q = 1.0 - y_margin / dy;
    (a + dx * q, b + dy * q)
}

// Used to make divisions by 0 usable for plotting
pub fn clamp_eval(eval: f64) -> f64 {
    if eval == f64::INFINITY {
        f64::MAX
    } else if eval == f64::NEG_INFINITY {
        f64::MIN
    } else {
        eval
    }
}

// This needs to match the functions added in new_function_context
pub fn get_var_names(function: &str) -> Vec<String> {
    let replaced = function
        .replace("sin", "")
        .replace("cos", "")
        .replace("tan", "")
        .replace("sqrt", "")
        .replace("log", "")
        .replace("mod", "");

    let mut vars = vec![];
    for c in replaced.chars() {
        if c.is_alphabetic() {
            let s = c.to_string();
            if !vars.contains(&s) {
                vars.push(s);
            }
        }
    }
    vars
}

// Verify all the functions provided by the user
pub fn verify_functions(ctx: &mut PlotContext, var_count: usize) -> Result<(), String> {
    let mut idx = 1;
    for function in ctx.data.split('\n') {
        if function.is_empty() {
            return Err(format!("Function {idx} is invalid: empty"));
        }

        let names = get_var_names(function);
        if names.len() > var_count {
            return Err(format!("Function {idx} is invalid: too many variables"));
        }

        let values = vec![0.0; names.len()];
        update_function_context(&mut ctx.fn_ctx, &names, &values);

        if eval_number_with_context_mut(function, &mut ctx.fn_ctx).is_err() {
            return Err(format!(
                "Function {idx} is invalid: does not produce a number"
            ));
        }
        idx += 1;
    }

    Ok(())
}

// Set variables to the provided values and evaluate function
pub fn eval_function(
    ctx: &mut HashMapContext,
    function: &str,
    values: &[f64],
) -> EvalexprResult<f64> {
    let names = get_var_names(function);
    update_function_context(ctx, &names, values);
    eval_number_with_context_mut(function, ctx)
}

// Finds all top-level denominators (denominators inside denominators are not included)
pub fn find_denominators(function: &str) -> Vec<String> {
    let mut denoms = vec![];
    let mut buf = String::new();
    let mut level = 0;
    let mut denom_at = None;
    let mut paren_last = false;
    for c in function.chars() {
        if let Some(denom_level) = denom_at {
            if buf.is_empty() || denom_level != level {
                buf.push(c);
            } else if " +-*)".contains(c) || paren_last {
                denoms.push(buf);
                buf = String::new();
                denom_at = None;
            } else if c != ')' {
                buf.push(c);
            }
        }

        match c {
            '/' => {
                if denom_at.is_none() {
                    denom_at = Some(level);
                }
            }
            '(' => level += 1,
            ')' => level -= 1,
            _ => {}
        }

        paren_last = c == ')';
    }

    if !buf.is_empty() {
        denoms.push(buf);
    }
    denoms
}

// Finds solutions to f(x) = 0. Uses Newton-Raphson with Finite Differences, with the addition of
// boundaries for the search.
pub fn solve_zero(
    ctx: &mut HashMapContext,
    function: &str,
    mut x0: f64,
    range: (f64, f64),
) -> Option<f64> {
    let mut dx = f64::MAX;
    let mut iter = 1;
    let iter_max = 50;
    let step = f64::EPSILON.sqrt();
    while dx.abs() > ERROR_TOLERANCE && iter < iter_max {
        let ym = eval_function(ctx, function, &[x0 - step]).unwrap();
        let y = eval_function(ctx, function, &[x0]).unwrap();
        let yp = eval_function(ctx, function, &[x0 + step]).unwrap();
        dx = y / ((yp - ym) / (step + step));
        x0 -= dx;
        iter += 1;

        if x0 < range.0 || x0 > range.1 {
            iter = iter_max;
        }
    }

    if iter >= iter_max || x0.is_nan() {
        None
    } else {
        Some(x0)
    }
}

// Search for an asymptote in the given range. This function assumes only one asymptote is present
// within the given range. Should there be more than one asymptote, only one will be returned.
pub fn find_asymptote(ctx: &mut HashMapContext, function: &str, range: (f64, f64)) -> Option<f64> {
    let denoms = find_denominators(function);
    for denom in denoms {
        if !get_var_names(&denom).is_empty() {
            let x0 = (range.0 + range.1) / 2.0;
            if let Some(sol) = solve_zero(ctx, &denom, x0, range) {
                return Some(sol);
            }
        }
    }
    None
}
