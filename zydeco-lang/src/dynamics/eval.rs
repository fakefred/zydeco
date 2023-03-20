use super::syntax::*;
use std::rc::Rc;

impl<'rt> Eval<'rt> for ls::TermValue {
    type Out = TermValue;

    fn step<'e>(
        self, runtime: &'e mut Runtime<'rt>,
    ) -> Step<Self, Self::Out> {
        match self {
            ls::TermValue::Var(var) => Step::Done(
                runtime
                    .env
                    .lookup(&var)
                    .expect("variable does not exist")
                    .clone(),
            ),
            ls::TermValue::Thunk(ls::Thunk(body)) => {
                Step::Done(Thunk { body, env: runtime.env.clone() }.into())
            }
            ls::TermValue::Ctor(ls::Ctor { ctor, args }) => {
                let args = args
                    .iter()
                    .map(|arg| Rc::new(arg.as_ref().clone().eval(runtime)))
                    .collect();
                Step::Done(Ctor { ctor, args }.into())
            }
            ls::TermValue::Literal(lit) => Step::Done(lit.into()),
        }
    }
}

impl<'rt> Eval<'rt> for ls::TermComputation {
    type Out = TermComputation;

    fn step<'e>(
        self, runtime: &'e mut Runtime<'rt>,
    ) -> Step<Self, Self::Out> {
        match self {
            ls::TermComputation::Ret(ls::Ret(v)) => {
                let v = v.as_ref().clone().eval(runtime);
                match runtime.stack.pop_back() {
                    Some(Frame::Kont(comp, var)) => {
                        let env = runtime.env.update(var, v);
                        runtime.env = env;
                        Step::Step(comp.as_ref().clone())
                    }
                    None => Step::Done(TermComputation::Ret(v)),
                    _ => panic!("Kont not at stacktop"),
                }
            }
            ls::TermComputation::Force(ls::Force(v)) => {
                let v = v.as_ref().clone().eval(runtime);
                let TermValue::Thunk(thunk) = v else {
                    panic!("Force on non-thunk")
                };
                runtime.env = thunk.env;
                Step::Step(thunk.body.as_ref().clone())
            }
            ls::TermComputation::Let(ls::Let { var, def, body }) => {
                let def = def.as_ref().clone().eval(runtime);
                let env = runtime.env.update(var, def);
                runtime.env = env;
                Step::Step(body.as_ref().clone())
            }
            ls::TermComputation::Do(ls::Do { var, comp, body }) => {
                runtime.stack.push_back(Frame::Kont(body, var));
                Step::Step(comp.as_ref().clone())
            }
            ls::TermComputation::Rec(ls::Rec { var, body }) => {
                let env = runtime.env.update(
                    var,
                    Thunk { body: body.clone(), env: runtime.env.clone() }
                        .into(),
                );
                runtime.env = env;
                Step::Step(body.as_ref().clone())
            }
            ls::TermComputation::Match(ls::Match { scrut, arms }) => {
                let scrut = scrut.as_ref().clone().eval(runtime);
                let TermValue::Ctor(Ctor { ctor, args }) = scrut else {
                    panic!("Match on non-ctor")
                };
                let Matcher { ctor: _, vars, body } = arms
                    .into_iter()
                    .find(|arm| arm.ctor == ctor)
                    .expect("no matching arm");
                for (var, arg) in vars.into_iter().zip(args.into_iter()) {
                    let env = runtime.env.update(var, arg.as_ref().clone());
                    runtime.env = env;
                }
                Step::Step(body.as_ref().clone())
            }
            ls::TermComputation::CoMatch(ls::CoMatch { arms }) => {
                let Some(Frame::Dtor(dtor, args)) = runtime.stack.pop_back() else {
                    panic!("CoMatch on non-Dtor")
                };
                let CoMatcher { dtor: _, vars, body } = arms
                    .into_iter()
                    .find(|arm| arm.dtor == dtor)
                    .expect("no matching arm");
                for (var, arg) in vars.into_iter().zip(args.into_iter()) {
                    let env = runtime.env.update(var, arg.as_ref().clone());
                    runtime.env = env;
                }
                Step::Step(body.as_ref().clone())
            }
            ls::TermComputation::Dtor(ls::Dtor { body, dtor, args }) => {
                let args = args
                    .iter()
                    .map(|arg| Rc::new(arg.as_ref().clone().eval(runtime)))
                    .collect();
                runtime.stack.push_back(Frame::Dtor(dtor, args));
                Step::Step(body.as_ref().clone())
            }
            ls::TermComputation::Prim(ls::Prim { arity, body }) => {
                let mut args = Vec::new();
                for _ in 0..arity {
                    let Some(Frame::Dtor(_, arg)) = runtime.stack.pop_back() else {
                        panic!("Prim on non-Dtor")
                    };
                    args.push(
                        arg.first().expect("empty arg").as_ref().clone(),
                    );
                }
                match body(
                    args,
                    runtime.input,
                    runtime.output,
                    runtime.args,
                ) {
                    Ok(e) => Step::Step(e),
                    Err(exit_code) => {
                        Step::Done(TermComputation::ExitCode(exit_code))
                    }
                }
            }
        }
    }
}

impl Module {
    pub fn new<'rt>(m: ls::Module, runtime: &'rt mut Runtime<'rt>) -> Self {
        for (x, v) in m.define {
            let v = v.clone().eval(runtime);
            let env = runtime.env.update(x, v);
            runtime.env = env;
        }
        Module { name: m.name, entry: m.entry.eval(runtime) }
    }
}
