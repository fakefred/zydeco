use super::syntax::*;
use crate::utils::fmt::*;

impl FmtArgs for Type {
    fn fmt_args(&self, fargs: Args) -> String {
        let app = self.app.fmt_args(fargs);
        if let Some(kd) = &self.kd {
            format!("({} :: {})", app, kd.fmt_args(fargs))
        } else {
            app
        }
    }
}

impl FmtArgs for TermValue {
    fn fmt_args(&self, args: Args) -> String {
        match self {
            TermValue::TermAnn(t) => t.fmt_args(args),
            TermValue::Var(t) => t.fmt_args(args),
            TermValue::Thunk(t) => t.fmt_args(args),
            TermValue::Ctor(t) => t.fmt_args(args),
            TermValue::Literal(t) => t.fmt_args(args),
        }
    }
}

impl FmtArgs for TermComputation {
    fn fmt_args(&self, args: Args) -> String {
        match self {
            TermComputation::TermAnn(t) => t.fmt_args(args),
            TermComputation::Ret(t) => t.fmt_args(args),
            TermComputation::Force(t) => t.fmt_args(args),
            TermComputation::Let(t) => t.fmt_args(args),
            TermComputation::Do(t) => t.fmt_args(args),
            TermComputation::Rec(t) => t.fmt_args(args),
            TermComputation::Match(t) => t.fmt_args(args),
            TermComputation::CoMatch(t) => t.fmt_args(args),
            TermComputation::Dtor(t) => t.fmt_args(args),
        }
    }
}

impl FmtArgs for Module {
    fn fmt_args(&self, args: Args) -> String {
        let mut s = String::new();
        let Module { name, data, codata, define, define_ext, entry } = self;
        if let Some(name) = name {
            s += &format!("module {} where", name);
            s += &args.force_space();
        }
        for d in data {
            s += &d.fmt_args(args);
            s += &args.force_space();
        }
        for d in codata {
            s += &d.fmt_args(args);
            s += &args.force_space();
        }
        for DeclSymbol {
            public,
            external: _,
            inner: Define { name: (var, ty), def: () },
        } in define_ext
        {
            if *public {
                s += &format!("pub ");
            }
            s += &format!(
                "extern {} : {} end",
                var.fmt_args(args),
                ty.fmt_args(args)
            );
            s += &args.force_space();
        }
        for d in define {
            s += &d.fmt_args(args);
            s += &args.force_space();
        }
        s += &format!("{}", entry.fmt_args(args));
        s
    }
}
