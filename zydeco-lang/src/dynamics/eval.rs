use super::{
    env::*,
    syntax::{ZCompute, ZValue},
};
use crate::{
    parse::syntax::{Dtor, VVar},
    utils::ann::{AnnHolder, AnnT},
};
use std::{mem::replace, rc::Rc, fmt::Debug};

#[derive(Debug, Clone)]
pub enum EvalError<Ann> {
    ErrStr(String, Ann),
}

#[derive(Clone)]
enum Frame<Ann: AnnT> {
    Kont(Rc<ZCompute<Ann>>, Env<Ann>, VVar<Ann>),
    Call(Rc<ZValue<Ann>>),
    Dtor(Dtor<Ann>, Vec<Rc<ZValue<Ann>>>),
}

impl<Ann: AnnT> Debug for Frame<Ann> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Frame::Kont(_, _, var) => write!(f, "Kont({})", var.name()),
            Frame::Call(_) => write!(f, "Call"),
            Frame::Dtor(dtor, _) => write!(f, "Dtor({})", dtor.name()),
        }
    }
}

#[derive(Debug, Clone)]
enum Stack<Ann: AnnT> {
    Done,
    Frame(Frame<Ann>, Rc<Stack<Ann>>),
}

impl<Ann: AnnT> Stack<Ann> {
    fn new() -> Self {
        Stack::Done
    }
}

#[derive(Clone)]
pub struct Runtime<Ann: AnnT> {
    stack: Rc<Stack<Ann>>,
    env: Env<Ann>,
}

impl<'rt, Ann: AnnT> Runtime<Ann> {
    pub fn new() -> Self {
        Runtime { stack: Rc::new(Stack::new()), env: Env::new() }
    }

    fn get(&self, var: &VVar<Ann>) -> Result<Rc<ZValue<Ann>>, EvalError<Ann>> {
        self.env.get(var).ok_or_else(|| {
            EvalError::ErrStr(
                format!("Variable {} not found", var),
                var.ann().clone(),
            )
        })
    }

    fn resolve_value(
        &self, val: Rc<ZValue<Ann>>,
    ) -> Result<Rc<ZValue<Ann>>, EvalError<Ann>> {
        use ZValue::*;
        match val.as_ref() {
            Var(var, _) => self.get(var),
            Thunk(thunk, None, ann) => {
                let env = self.env.clone().push();
                Ok(Rc::new(Thunk(thunk.clone(), Some(env), ann.clone())))
            }
            Ctor(ctor, args, ann) => {
                let args = args
                    .iter()
                    .map(|arg| self.resolve_value(arg.clone()))
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(Rc::new(Ctor(ctor.clone(), args, ann.clone())))
            }
            _ => Ok(val),
        }
    }

    fn call(&mut self, arg: Rc<ZValue<Ann>>) -> Result<(), EvalError<Ann>> {
        let arg = self.resolve_value(arg)?;
        self.stack =
            Rc::new(Stack::Frame(Frame::Call(arg.clone()), self.stack.clone()));
        Ok(())
    }

    fn dtor(&mut self, dtor: Dtor<Ann>, args: Vec<Rc<ZValue<Ann>>>) {
        self.stack =
            Rc::new(Stack::Frame(Frame::Dtor(dtor, args), self.stack.clone()));
    }

    fn kont(&mut self, comp: Rc<ZCompute<Ann>>, var: VVar<Ann>) {
        self.push();
        self.stack = Rc::new(Stack::Frame(
            Frame::Kont(comp, self.env.clone(), var),
            self.stack.clone(),
        ));
    }

    fn push(&mut self) {
        let env = replace(&mut self.env, Env::new());
        self.env = env.push();
    }

    pub fn insert(
        &mut self, var: VVar<Ann>, val: Rc<ZValue<Ann>>,
    ) -> Result<(), EvalError<Ann>> {
        self.env.insert(var, self.resolve_value(val)?.clone());
        Ok(())
    }

    fn step(
        &mut self, comp: ZCompute<Ann>,
    ) -> Result<Rc<ZCompute<Ann>>, EvalError<Ann>> {
        use {ZCompute::*, ZValue::*};
        match comp {
            Let { binding: (var, val), body, .. } => {
                self.insert(var, val)?;
                Ok(body)
            }
            Do { binding: (var, comp), body, .. } => {
                self.kont(body, var);
                Ok(comp)
            }
            Force(val, _) => {
                if let Thunk(comp, Some(env), _) =
                    self.resolve_value(val.clone())?.as_ref()
                {
                    self.env = env.clone();
                    Ok(comp.clone())
                } else {
                    Err(EvalError::ErrStr(
                        format!("Force on non-thunk value: {:?}", val),
                        val.ann().clone(),
                    ))
                }
            }
            Return(val, _) => {
                let val = self.resolve_value(val)?;
                let stack = self.stack.to_owned();
                if let Stack::Frame(Frame::Kont(comp, env, var), prev) =
                    stack.as_ref()
                {
                    self.stack = prev.clone();
                    self.env = env.pop().ok_or_else(|| {
                        EvalError::ErrStr(
                            format!("EnvStack is empty"),
                            val.ann().clone(),
                        )
                    })?;
                    self.insert(var.clone(), val)?;
                    Ok(comp.clone())
                } else {
                    Err(EvalError::ErrStr(
                        format!("Return on non-kont frame: {:?}", val),
                        val.ann().clone(),
                    ))
                }
            }
            Lam { arg: var, body, ann } => {
                let stack = self.stack.to_owned();
                if let Stack::Frame(Frame::Call(arg), prev) = stack.as_ref() {
                    self.stack = prev.clone();
                    self.insert(var, arg.clone())?;
                    Ok(body)
                } else {
                    Err(EvalError::ErrStr(
                        format!("Lam on non-call frame"),
                        ann.clone(),
                    ))
                }
            }
            Prim { arity, body, ann } => {
                let mut args = Vec::new();
                for _ in 0..arity {
                    let stack = self.stack.to_owned();
                    if let Stack::Frame(Frame::Call(arg), prev) = stack.as_ref()
                    {
                        self.stack = prev.clone();
                        args.push((**arg).clone());
                    }
                }
                Ok(Rc::new(ZCompute::Return(Rc::new(body(args)), ann.clone())))
            }
            Rec { arg, body, ann } => {
                self.insert(
                    arg.clone(),
                    Rc::new(Thunk(
                        Rc::new(Rec {
                            arg,
                            body: body.clone(),
                            ann: ann.clone(),
                        }),
                        Some(self.env.clone()),
                        ann,
                    )),
                )?;
                Ok(body)
            }
            App(f, arg, _) => {
                self.call(arg.clone())?;
                Ok(f)
            }
            If { cond, thn, els, ann } => {
                let cond = self.resolve_value(cond)?;
                if let Bool(cond, _) = cond.as_ref() {
                    Ok(if *cond { thn } else { els })
                } else {
                    Err(EvalError::ErrStr(
                        format!("If on non-bool value: {:?}", cond),
                        ann,
                    ))
                }
            }
            Match { scrut, cases, ann } => {
                if let Ctor(ctor, args, _) =
                    self.resolve_value(scrut.clone())?.as_ref()
                {
                    let (_, vars, comp) = cases
                        .into_iter()
                        .find(|(pat, ..)| pat == ctor)
                        .ok_or_else(|| {
                            EvalError::ErrStr(
                                format!("Ctor {:?} mismatch", ctor),
                                ann,
                            )
                        })?;
                    for (var, arg) in vars.iter().zip(args.iter()) {
                        self.insert(var.clone(), arg.clone())?;
                    }
                    Ok(comp)
                } else {
                    Err(EvalError::ErrStr(
                        format!(
                            "Match on non-ctor value: {:?}",
                            scrut.as_ref()
                        ),
                        ann,
                    ))
                }
            }
            CoMatch { cases, ann } => {
                let stack = self.stack.to_owned();
                if let Stack::Frame(Frame::Dtor(dtor, args), prev) =
                    stack.as_ref()
                {
                    self.stack = prev.clone();
                    let (_, vars, comp) = cases
                        .iter()
                        .find(|(pat, ..)| *pat == *dtor)
                        .ok_or_else(|| {
                            EvalError::ErrStr(format!("Dtor mismatch"), ann)
                        })?;
                    for (var, arg) in vars.iter().zip(args.iter()) {
                        self.insert(var.clone(), arg.clone())?;
                    }
                    Ok(comp.clone())
                } else {
                    Err(EvalError::ErrStr(
                        format!("CoMatch on non-dtor frame"),
                        ann,
                    ))
                }
            }
            CoApp { scrut, dtor, args, .. } => {
                self.dtor(
                    dtor,
                    args.into_iter()
                        .map(|val| self.resolve_value(val))
                        .collect::<Result<_, _>>()?,
                );
                Ok(scrut)
            }
        }
    }

    fn eval(
        &mut self, mut comp: ZCompute<Ann>,
    ) -> Result<ZValue<Ann>, EvalError<Ann>> {
        use ZCompute::*;
        const MAX_STEPS: usize = 1000;
        let mut steps = 0;
        while steps <= MAX_STEPS {
            match (comp, self.stack.as_ref()) {
                (Return(val, _ann), Stack::Done) => {
                    return Ok(self.resolve_value(val)?.as_ref().clone());
                }
                (c, _) => {
                    comp = self.step(c)?.as_ref().clone();
                    // println!("|- {:#?}", self.env);
                    // println!();
                    // println!(":: {:#?}", self.stack);
                    // println!();
                    // println!("|> {:?}", comp);
                    // println!();
                    // println!();
                    // println!();
                }
            }
            steps += 1;
        }
        Err(EvalError::ErrStr(
            format!("My name is megumi! Exceeded max steps: {}", MAX_STEPS),
            comp.ann().clone(),
        ))
    }
}

pub fn eval<Ann: AnnT>(
    comp: ZCompute<Ann>, runtime: &mut Runtime<Ann>
) -> Result<ZValue<Ann>, EvalError<Ann>> {
    runtime.eval(comp)
}