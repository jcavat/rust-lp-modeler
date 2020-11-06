use std::collections::HashMap;

use dsl::{Problem, LpContinuous, LpBinary, LpInteger, LpProblem, LpExpression};

pub mod cbc;
pub use self::cbc::*;

pub mod gurobi;
pub use self::gurobi::*;

pub mod glpk;
pub use self::glpk::*;

#[cfg(feature = "native_cbc")]
pub mod native_cbc;
#[cfg(feature = "native_cbc")]
pub use self::native_cbc::*;

use std::fs::File;
use std::fs;
use util::is_zero;
use dsl::LpExpression::*;

#[derive(Debug, PartialEq, Clone)]
pub enum Status {
    Optimal,
    SubOptimal,
    Infeasible,
    Unbounded,
    NotSolved,
}

#[derive(Debug, Clone)]
pub struct Solution<'a> {
    pub status: Status,
    pub results: HashMap<String, f32>,
    pub related_problem: Option<&'a LpProblem>
}
impl Solution<'_> {
    pub fn new<'a>(status: Status, results: HashMap<String, f32>) -> Solution<'a> {
        Solution {
            status,
            results,
            related_problem: None
        }
    }
    pub fn with_problem(status: Status, results: HashMap<String, f32>, problem: &LpProblem) -> Solution {
        Solution {
            status,
            results,
            related_problem: Some(problem)
        }
    }
    fn check_possible_solution(&self) {
        match &self.status {
            Status::Unbounded | Status::NotSolved | Status::Infeasible => panic!("Solution must be optimal or suboptimal"),
            _ => ()
        }
    }
    pub fn get_raw_value(&self, name: &str) -> f32 {
        self.check_possible_solution();
        *self.results.get(name).expect("No value found for this variable. Check if the variable has been used in the related problem.")
    }
    pub fn get_bool(&self, var: &LpBinary) -> bool {
        self.check_possible_solution();
        self.results.get(&var.name).and_then(|&f| if is_zero(1.0-f) { Some(true) } else if is_zero(f) { Some(false) } else { None } ).expect("Result value cannot be interpreted as boolean")
    }
    pub fn get_float(&self, var: &LpContinuous) -> f32 {
        self.check_possible_solution();
        *self.results.get(&var.name).expect("No value found for this variable. Check if the variable has been used in the related problem.")
    }
    pub fn get_int(&self, var: &LpInteger) -> i32 {
        self.check_possible_solution();
        let &f = self.results.get(&var.name).expect("No value found for this variable. Check if the variable has been used in the related problem.");
        let i = f as i32;
        assert!( is_zero( f-(i as f32)), format!("Value {} cannot be interpreted as integer.", f) );
        i
    }
    pub fn eval(&self) -> Option<f32> {
        self.related_problem.and_then( |problem| problem.obj_expr.as_ref().map( |obj_expr| Self::eval_with(obj_expr, &self.results ) ))
    }
    fn eval_with(expr: &LpExpression, values: &HashMap<String, f32>) -> f32 {
        match expr {
            AddExpr(left, right) => Self::eval_with(left, values) + Self::eval_with(right, values),
            ConsBin(LpBinary { name })
            | ConsCont(LpContinuous { name, .. })
            | ConsInt(LpInteger { name, .. }) => *values.get(name).unwrap_or(&0f32),
            MulExpr(left, right) => Self::eval_with(left, values) * Self::eval_with(right, values),
            SubExpr(left, right) => Self::eval_with(left, values) - Self::eval_with(right, values),
            LitVal(n) => *n,
            EmptyExpr => 0.0
        }
    }
}

pub trait SolverTrait {
    type P: Problem;
    fn run<'a>(&self, problem: &'a Self::P) -> Result<Solution<'a>, String>;
}

pub trait SolverWithSolutionParsing {
    fn read_solution<'a>(&self, temp_solution_file: &String, problem: Option<&'a LpProblem>) -> Result<Solution<'a>, String> {
        match File::open( temp_solution_file ) {
            Ok(f) => {
                let res = self.read_specific_solution(&f, problem)?;
                let _ = fs::remove_file(temp_solution_file);
                Ok(res)
            }
            Err(_) => return Err("Cannot open file".to_string()),
        }
    }
    fn read_specific_solution<'a>(&self, f: &File, problem: Option<&'a LpProblem>) -> Result<Solution<'a>, String>;
}

pub trait WithMaxSeconds<T> {
    fn max_seconds(&self) -> Option<u32>;
    fn with_max_seconds(&self, seconds: u32) -> T;
}

pub trait WithNbThreads<T> {
    fn nb_threads(&self) -> Option<u32>;
    fn with_nb_threads(&self, threads: u32) -> T;
}
