use super::syntax::*;
use crate::utils::fmt::*;

impl FmtArgs for Abstract {
    fn fmt_args(&self, _fargs: Args) -> String {
        format!("${}", self.0)
    }
}

impl FmtArgs for Hole {
    fn fmt_args(&self, _fargs: Args) -> String {
        "_?".into()
    }
}

impl FmtArgs for SynType {
    fn fmt_args(&self, fargs: Args) -> String {
        match self {
            SynType::TypeApp(t) => t.fmt_args(fargs),
            SynType::Forall(t) => t.fmt_args(fargs),
            SynType::Exists(t) => t.fmt_args(fargs),
            SynType::Abstract(t) => t.fmt_args(fargs),
            SynType::Hole(t) => t.fmt_args(fargs),
        }
    }
}

impl FmtArgs for Type {
    fn fmt_args(&self, fargs: Args) -> String {
        self.synty.fmt_args(fargs)
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
            TermValue::Pack(t) => t.fmt_args(args),
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
            TermComputation::TypAbs(t) => t.fmt_args(args),
            TermComputation::TypApp(t) => t.fmt_args(args),
            TermComputation::MatchPack(t) => t.fmt_args(args),
        }
    }
}

impl FmtArgs for Module {
    fn fmt_args(&self, args: Args) -> String {
        let mut s = String::new();
        let Module { name, data, codata, define, define_ext } = self;
        if let Some(name) = name {
            s += &format!("module {} where", name);
            s += &args.br_indent();
        }
        for d in data {
            s += &d.fmt_args(args);
            s += &args.br_indent();
        }
        for d in codata {
            s += &d.fmt_args(args);
            s += &args.br_indent();
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
            s += &args.br_indent();
        }
        for d in define {
            s += &d.fmt_args(args);
            s += &args.br_indent();
        }
        s
    }
}

impl FmtArgs for Program {
    fn fmt_args(&self, args: Args) -> String {
        let Program { module, entry } = self;
        let mut s = String::new();
        s += &module.fmt_args(args);
        s += &args.br_indent();
        s += &entry.fmt_args(args);
        s
    }
}
