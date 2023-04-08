#![allow(unused)]
use zydeco_lang::{dynamics::syntax as ds, prelude::*, statics::syntax as ss, zydeco::ZydecoExpr};

pub fn launch() -> Result<i32, String> {
    println!("Zydeco v0.2.0");
    let mut zydeco_expr = ZydecoExpr::new();
    loop {
        let mut line = String::new();
        {
            use std::io::Write;
            print!("> ");
            std::io::stdout().flush().unwrap();
            let stdin = std::io::stdin();
            let n = stdin.read_line(&mut line).map_err(|e| e.to_string())?;
            // Ctrl-D to exit
            if n == 0 {
                break Ok(0);
            }
        }
        // check for commands
        if line.trim_start().starts_with("#") {
            // currently, the only command is #env
            if line.starts_with("#env") {
                for (var, val) in zydeco_expr.env.iter() {
                    println!("{} = {}", var.fmt(), val.fmt())
                }
            } else {
                println!("Unknown command {}", line.trim());
            }
            continue;
        }
        // parse and elaborate
        let term = match ZydecoExpr::parse(&line) {
            Err(e) => {
                println!("Parse Error: {}", e);
                continue;
            }
            Ok(term) => match ZydecoExpr::elab(term) {
                Err(e) => {
                    println!("Elaboration Error: {}", e);
                    continue;
                }
                Ok(term) => term,
            },
        };
        // typecheck and evaluate
        match term.inner_ref() {
            ss::Term::Value(v) => {
                match zydeco_expr.tyck_value(term.span().make(v.clone())) {
                    Err(e) => println!("Type Error: {}", e),
                    Ok(ty) => {
                        // NOTE: not evaluating the value, just printing its type
                        // let v = ZydecoExpr::link_value(v);
                        // let v = zydeco_expr.eval_value(v);
                        // println!("{} :: {}", v.fmt(), ty.fmt())
                        println!("{} :: {}", v.fmt(), ty.fmt())
                    }
                }
            }
            ss::Term::Computation(c) => {
                match zydeco_expr.tyck_computation(term.span().make(c.clone())) {
                    Err(e) => println!("Type Error: {}", e),
                    Ok(ty) => {
                        // HACK: The final call to OS will destroy the environment,
                        // so we need to save a snapshot of it before we run.
                        let snapshot = zydeco_expr.clone();
                        if let Some(()) =
                            ty.clone().elim_os(zydeco_expr.ctx.clone(), &SpanInfo::new(0, 0))
                        {
                            let c = ZydecoExpr::link_computation(c);
                            let c = zydeco_expr.eval_os(c, &[]);
                            match c.entry {
                                ds::ProgKont::Ret(value) => {
                                    unreachable!()
                                }
                                ds::ProgKont::ExitCode(i) => {
                                    println!("exited with code {}", i)
                                }
                            }
                        } else if let Some(ty) =
                            ty.clone().elim_ret(zydeco_expr.ctx.clone(), &SpanInfo::new(0, 0))
                        {
                            let c = ZydecoExpr::link_computation(c);
                            let c = zydeco_expr.eval_ret_computation(c);
                            match c {
                                ds::ProgKont::Ret(value) => {
                                    println!("{} :: {}", value.fmt(), ty.fmt())
                                }
                                ds::ProgKont::ExitCode(i) => {
                                    unreachable!()
                                }
                            }
                        } else {
                            println!("Can't run computation of type {}", ty.fmt());
                            println!("Can only run computations of type OS or Ret(a)")
                        }
                        // HACK: Restore the environment
                        zydeco_expr = snapshot;
                    }
                }
            }
        }
    }
}
