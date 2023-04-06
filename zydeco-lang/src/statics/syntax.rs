use crate::utils::span::{span, Span};
use std::rc::Rc;
use zydeco_derive::{EnumGenerator, FmtArgs};

pub use crate::syntax::*;

/* ---------------------------------- Kind ---------------------------------- */

pub use crate::syntax::Kind;

/* ---------------------------------- Type ---------------------------------- */

#[derive(Clone, Debug, PartialEq)]
pub struct AbstVar(pub usize);
#[derive(EnumGenerator, FmtArgs, Clone, Debug)]
pub enum SynType {
    TypeApp(TypeApp<TypeV, RcType>),
    Forall(Forall<(TypeV, Kind), RcType>),
    Exists(Exists<(TypeV, Kind), RcType>),
    AbstVar(AbstVar),
    Hole(Hole),
}

#[derive(Clone, Debug)]
pub struct Type {
    pub synty: SynType,
}
pub type RcType = Rc<Span<Type>>;
impl TypeT for Type {}

impl Type {
    pub fn internal(name: &'static str, args: Vec<RcType>) -> Self {
        TypeApp::internal(name, args).into()
    }
    pub fn make_thunk(arg: RcType) -> Self {
        TypeApp::internal("Thunk", vec![arg]).into()
    }
    pub fn make_ret(arg: RcType) -> Self {
        TypeApp::internal("Ret", vec![arg]).into()
    }
    pub fn make_os() -> Self {
        TypeApp::internal("OS", vec![]).into()
    }
}
impl TypeApp<TypeV, RcType> {
    pub fn internal(name: &'static str, args: Vec<RcType>) -> Self {
        TypeApp { tvar: TypeV::new(name.into(), span(0, 0)), args }
    }
    pub fn elim_thunk(&self) -> Option<Type> {
        if self.tvar.name() == "Thunk" {
            Some(self.args.first().unwrap().inner_ref().clone())
        } else {
            None
        }
    }
    pub fn elim_ret(&self) -> Option<Type> {
        if self.tvar.name() == "Ret" {
            Some(self.args.first().unwrap().inner_ref().clone())
        } else {
            None
        }
    }
    pub fn elim_os(&self) -> Option<()> {
        if self.tvar.name() == "OS" {
            Some(())
        } else {
            None
        }
    }
}
macro_rules! impl_from {
    ($T:ty) => {
        impl From<$T> for Type {
            fn from(synty: $T) -> Self {
                Self { synty: synty.into() }
            }
        }
    };
}
impl_from!(TypeApp<TypeV, RcType>);
impl_from!(Forall<(TypeV, Kind), RcType>);
impl_from!(Exists<(TypeV, Kind), RcType>);
impl_from!(AbstVar);
impl_from!(Hole);
impl From<TypeV> for Type {
    fn from(tvar: TypeV) -> Self {
        TypeApp { tvar, args: vec![] }.into()
    }
}

/* ---------------------------------- Term ---------------------------------- */

#[derive(EnumGenerator, FmtArgs, Clone, Debug)]
pub enum TermValue {
    Annotation(Annotation<RcValue, RcType>),
    Var(TermV),
    Thunk(Thunk<RcComp>),
    Ctor(Ctor<CtorV, RcValue>),
    Literal(Literal),
    Pack(Pack<RcType, RcValue>),
}
pub type RcValue = Rc<Span<TermValue>>;
impl ValueT for TermValue {}

#[derive(EnumGenerator, FmtArgs, Clone, Debug)]
pub enum TermComputation {
    Annotation(Annotation<RcComp, RcType>),
    Ret(Ret<RcValue>),
    Force(Force<RcValue>),
    Let(Let<TermV, RcValue, RcComp>),
    Do(Do<TermV, RcComp, RcComp>),
    Rec(Rec<TermV, RcComp>),
    Match(Match<CtorV, TermV, RcValue, RcComp>),
    Comatch(Comatch<DtorV, TermV, RcComp>),
    Dtor(Dtor<RcComp, DtorV, RcValue>),
    TyAbsTerm(TyAbsTerm<TypeV, Kind, RcComp>),
    TyAppTerm(TyAppTerm<RcComp, RcType>),
    MatchPack(MatchPack<RcValue, TypeV, TermV, RcComp>),
}
pub type RcComp = Rc<Span<TermComputation>>;
impl ComputationT for TermComputation {}

#[derive(EnumGenerator, Clone, Debug)]
pub enum Term {
    Value(TermValue),
    Computation(TermComputation),
}

/* --------------------------------- Module --------------------------------- */

#[derive(Clone, Debug)]
pub struct Module {
    pub name: Option<String>,
    pub data: Vec<DeclSymbol<Data<TypeV, Kind, CtorV, RcType>>>,
    pub codata: Vec<DeclSymbol<Codata<TypeV, Kind, DtorV, RcType>>>,
    pub alias: Vec<DeclSymbol<Alias<TypeV, Kind, RcType>>>,
    pub define: Vec<DeclSymbol<Define<TermV, RcValue>>>,
    pub define_ext: Vec<DeclSymbol<Define<(TermV, RcType), ()>>>,
}

#[derive(Clone, Debug)]
pub struct Program {
    pub module: Span<Module>,
    pub entry: Span<TermComputation>,
}
