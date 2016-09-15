use std;
use variables::*;
use variables::LpExpression::*;
use variables::Constraint::*;
use std::rc::Rc;
use std::collections::HashSet;

//use variables::LpExpression::{AddExpr, MulExpr};
use std::ops::{AddAssign};

//use std::collections::HashMap;

/// Enum helping to specify the objective function of the linear problem.
///
/// # Exemples:
///
/// ```
/// use lp_modeler::problem::{LpObjective, LpProblem};
///
/// let mut problem = LpProblem::new("One Problem", Objective::Maximize);
/// ```
#[derive(Debug)]
pub enum LpObjective {
    Minimize,
    Maximize
}

/// Structure used for creating the model and solving a linear problem.
///
/// # Exemples:
///
/// ```
/// use lp_modeler::problem::{LpObjective, LpProblem};
/// use lp_modeler::variables::{LpVariable, LpType, LpOperations};
///
/// let ref a = LpVariable::new("a", LpType::Integer);
/// let ref b = LpVariable::new("b", LpType::Integer);
///
/// let mut problem = LpProblem::new("One Problem", LpObjective::Maximize);
/// problem += (a + b).lt(100);
/// problem += a.gt(b);
/// problem += 2*a + 3*b;
///
/// problem.solve();
/// ```
#[derive(Debug)]
pub struct LpProblem {
    name: &'static str,
    objective_type: LpObjective,
    obj_expr: Option<LpExpression>,
    constraints: Vec<LpConstraint>

}

impl LpProblem {

    /// Create a new problem
    pub fn new(name: &'static str, objective: LpObjective) -> LpProblem {
        LpProblem { name: name, objective_type: objective, obj_expr: None, constraints: Vec::new() }
    }

    // TODO: Call once and pass into parameter
    // TODO: Check variables on the objective function
    fn variables(&self) -> HashSet<&LpExpression> {

        fn var<'a>(expr: &'a LpExpression, lst: &mut Vec<&'a LpExpression>) {
            match expr {
                &BinaryVariable {..} | &IntegerVariable {..} | &ContinuousVariable {..} => { lst.push(expr); },
                &MulExpr(_, ref e) => { var(&*e, lst); },
                &AddExpr(ref e1, ref e2) | &SubExpr(ref e1, ref e2) => {
                    var(&*e1, lst);
                    var(&*e2, lst);
                },
                _ => ()
            }
        }

        let mut lst = Vec::new();
        for e in &self.constraints {
            var(&e.0, &mut lst);
        }
        lst.iter().map(|&x| x).collect::<HashSet<_>>()
    }

    fn dfs(expr: &LpExpression, lst: &mut String) {
        match expr {
            &MulExpr(ref e1, ref e2) => {
                Self::dfs(e1, lst);
                lst.push_str(" ");
                Self::dfs(e2, lst);
            },
            &AddExpr(ref e1, ref e2) => {
                Self::dfs(e1, lst);
                lst.push_str(" + ");
                Self::dfs(e2, lst);
            },
            &SubExpr(ref e1, ref e2) => {
                Self::dfs(e1, lst);
                lst.push_str(" - ");
                Self::dfs(e2, lst);
            },
            &BinaryVariable {name: ref n, .. } => {
                lst.push_str(n);
            },
            &IntegerVariable {name: ref n, .. } => {
                lst.push_str(n);
            },
            &ContinuousVariable {name: ref n, .. } => {
                lst.push_str(n);
            },
            &LitVal(n) => {
                lst.push_str(&n.to_string());
            },
            _ => ()
        }
    }

    fn objective_string(&self) -> String {

        let mut s = String::new();
        if let Some(ref expr) = self.obj_expr {
            Self::dfs(expr, &mut s);
        }
        s

    }

    fn constraints_string(&self) -> String {
        let mut res = String::new();
        let mut cstrs = self.constraints.iter();
        let mut index = 1;
        while let Some(&LpConstraint(ref e1, ref op, ref e2)) = cstrs.next() {
            res.push_str("  c");
            res.push_str(&index.to_string());
            res.push_str(": ");
            index += 1;
            Self::dfs(e1, &mut res);
            match op {
                //TODO: Remove > <, not standard
                &Greater => res.push_str(" > "),
                &Less => res.push_str(" < "),
                &GreaterOrEqual => res.push_str(" >= "),
                &LessOrEqual => res.push_str(" <= "),
                &Equal => res.push_str(" = "),
            }
            Self::dfs(e2, &mut res);
            res.push_str("\n");
        }
        res
    }

    fn bounds_string(&self) -> String {
        let mut res = String::new();
        for v in self.variables() {
            match v {
                &IntegerVariable { ref name, lower_bound, upper_bound }
                    | &ContinuousVariable { ref name, lower_bound, upper_bound } => {
                    if let Some(l) = lower_bound {
                        res.push_str("  ");
                        res.push_str(&l.to_string());
                        res.push_str(" <= ");
                        res.push_str(&name);
                        if let Some(u) = upper_bound {
                            res.push_str(" <= ");
                            res.push_str(&u.to_string());
                        }
                        res.push_str("\n");
                    } else if let Some(u) = upper_bound {
                        res.push_str("  ");
                        res.push_str(&name);
                        res.push_str(" <= ");
                        res.push_str(&u.to_string());
                        res.push_str("\n");
                    } else {
                        match v {
                            &ContinuousVariable {..} => {
                                res.push_str("  ");
                                res.push_str(&name);
                                res.push_str(" free\n");
                            }, // TODO: IntegerVar => -INF to INF
                            _ => ()
                        }
                    }
                },
                _ => (),
            }
        }
        res
    }

    fn generals_string(&self) -> String {
        let mut res = String::new();
        for v in self.variables() {
            match v {
                &IntegerVariable { ref name, .. } => {
                    res.push_str(name);
                    res.push_str(" ");
                },
                _ => (),
            }
        }
        res
    }

    fn binaries_string(&self) -> String {
        let mut res = String::new();
        for v in self.variables() {
            match v {
                &BinaryVariable { ref name } => {
                    res.push_str(name);
                    res.push_str(" ");
                },
                _ => (),
            }
        }
        res
    }

    pub fn write_lp<T: Into<String>>(&self, file_name: T) -> std::io::Result<()> {
        use std::fs::File;
        use std::io::prelude::*;

        let mut buffer = try!(File::create(file_name.into()));

        try!(buffer.write("\\ ".as_bytes()));
        try!(buffer.write(self.name.as_bytes()));
        try!(buffer.write("\n\n".as_bytes()));

        // Write objectives
        match self.objective_type {
            LpObjective::Maximize => { try!(buffer.write(b"Maximize\n  ")); },
            LpObjective::Minimize => { try!(buffer.write(b"Minimize\n  ")); },
        }
        let obj_str = self.objective_string();
        try!(buffer.write(obj_str.as_bytes()));

        // Write constraints
        let cstr_str = self.constraints_string();
        if cstr_str.len() > 0 {
            try!(buffer.write(b"\n\nSubject To\n"));
            try!(buffer.write(cstr_str.as_bytes()));
        }

        // Write bounds for Integer and Continuous
        let bound_str = self.bounds_string();
        if bound_str.len() > 0 {
            try!(buffer.write(b"\nBounds\n"));
            try!(buffer.write(bound_str.as_bytes()));
        }

        // Write Integer vars
        let generals_str = self.generals_string();
        if generals_str.len() > 0 {
            try!(buffer.write(b"\nGenerals\n  "));
            try!(buffer.write(generals_str.as_bytes()));
            try!(buffer.write(b"\n"));
        }

        // Write Binaries vars
        let binaries_str = self.binaries_string();
        if binaries_str.len() > 0 {
            println!("----------------->{}", binaries_str.len());
            try!(buffer.write(b"\nBinary\n  "));
            try!(buffer.write(binaries_str.as_bytes()));
            try!(buffer.write(b"\n"));
        }

        try!(buffer.write(b"\nEnd"));

        Ok(())

    }

    /// Solve the LP model
    pub fn solve(&self) {
        println!("Mhmmmm, solving :-)");
    }

}

/// Add constraints
impl AddAssign<LpConstraint> for LpProblem {
    fn add_assign(&mut self, _rhs: LpConstraint) {
        self.constraints.push(_rhs);
    }
}

/// Add an expression as an objective function
impl<T> AddAssign<T> for LpProblem where T: Into<LpExpression>{
    fn add_assign(&mut self, _rhs: T) {
        //TODO: improve without cloning
        if let Some(e) = self.obj_expr.clone() {
            self.obj_expr = Some(AddExpr(Rc::new(_rhs.into()), Rc::new(e.clone())));
        } else {
            self.obj_expr = Some(_rhs.into());
        }
    }
}

#[cfg(test)]
fn test(){
    assert!(false);
}
