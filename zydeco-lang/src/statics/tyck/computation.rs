use std::collections::{HashMap, HashSet};

use super::*;

impl TypeCheck for Span<TermComputation> {
    type Ctx = Ctx;
    type Out = Type;
    fn syn_step(
        &self, mut ctx: Self::Ctx,
    ) -> Result<Step<(Self::Ctx, &Self), Self::Out>, Span<TypeCheckError>> {
        Ok(match self.inner_ref() {
            TermComputation::TermAnn(TermAnn { term, ty }) => {
                ty.span()
                    .make(ty.syn(ctx.clone())?)
                    .ensure(&Kind::CType, "computation term annotation")?;
                Step::AnaMode((ctx, term), ty.inner_ref().clone())
            }
            TermComputation::Ret(Ret(v)) => {
                // Err(self
                // .span()
                // .make(NeedAnnotation { content: format!("ret") }))?
                let app_body = v.syn(ctx.clone())?.head_reduction()?;
                let ty_body: Type = app_body.into();
                let kd = self.span().make(ty_body.clone()).syn(ctx)?;
                self.span().make(kd).ensure(&Kind::VType, "force")?;
                Step::Done(Type::make_ret(rc!(self.span().make(ty_body))))
            }
            TermComputation::Force(Force(v)) => {
                let app_body = v.syn(ctx.clone())?.head_reduction()?;
                let ty_body = app_body.elim_thunk().ok_or_else(|| {
                    self.span().make(TypeExpected {
                        context: format!("force"),
                        expected: format!("{{a}}"),
                        found: app_body.clone().into(),
                    })
                })?;
                let kd = self.span().make(ty_body.to_owned()).syn(ctx)?;
                self.span().make(kd).ensure(&Kind::CType, "force")?;
                Step::Done(ty_body)
            }
            TermComputation::Let(Let { var, def, body }) => {
                let ty_def = def.syn(ctx.clone())?;
                let kd = self.span().make(ty_def.clone()).syn(ctx.clone())?;
                self.span().make(kd).ensure(&Kind::VType, "let")?;
                ctx.term_ctx.insert(var.to_owned(), ty_def.clone());
                Step::SynMode((ctx, body))
            }
            TermComputation::Do(Do { var, comp, body }) => {
                let ty_comp = comp.syn(ctx.clone())?;
                let kd = self.span().make(ty_comp.clone()).syn(ctx.clone())?;
                self.span().make(kd).ensure(&Kind::CType, "do")?;
                let ty_app = ty_comp.head_reduction()?;
                let ty_val = ty_app.elim_ret().ok_or_else(|| {
                    self.span().make(TypeExpected {
                        context: format!("do"),
                        expected: format!("{{a}}"),
                        found: ty_comp.clone(),
                    })
                })?;
                ctx.term_ctx.insert(var.to_owned(), ty_val);
                Step::SynMode((ctx, body))
            }
            TermComputation::Rec(Rec { var: _, body: _ }) => Err(self
                .span()
                .make(NeedAnnotation { content: format!("rec") }))?,
            TermComputation::Match(Match { scrut, arms }) => {
                let ty_scrut = scrut.syn(ctx.clone())?;
                let kd = self.span().make(ty_scrut.clone()).syn(ctx.clone())?;
                self.span().make(kd).ensure(&Kind::VType, "match")?;
                let ty_app = ty_scrut.head_reduction()?;
                let TCtor::Var(tvar) = ty_app.tctor else {
                    Err(self.span().make(TypeExpected {
                        context: format!("match"),
                        expected: format!("codata type"),
                        found: ty_app.clone().into(),
                    }))?
                };
                let Data { name, params, ctors } =
                    ctx.data_ctx.get(&tvar).cloned().ok_or_else(|| {
                        self.span().make(
                            NameResolveError::UnboundTypeVariable { tvar }
                                .into(),
                        )
                    })?;
                // arity check on data type
                let diff = Env::init(&params, &ty_app.args, || {
                    self.span().make(ArityMismatch {
                        context: format!("data type `{}` instiantiation", name),
                        expected: params.len(),
                        found: ty_app.args.len(),
                    })
                })?;
                let ctors: HashMap<_, _> = ctors
                    .into_iter()
                    .map(|DataBr(ctor, tys)| (ctor, tys))
                    .collect();
                let mut unexpected = Vec::new();
                let mut ctorv_set_arm: HashSet<CtorV> = HashSet::new();
                let mut ty_arms = Vec::new();
                for Matcher { ctor, vars, body } in arms {
                    let Some(tys) = ctors.get(ctor) else {
                        unexpected.push(ctor.to_owned());
                        continue;
                    };
                    ctorv_set_arm.insert(ctor.to_owned());
                    let tys = tys.into_iter().map(|ty| {
                        ty.inner_ref().to_owned().subst(diff.clone())
                    });
                    let mut ctx = ctx.clone();
                    for (var, ty) in vars.iter().zip(tys) {
                        ctx.term_ctx.insert(var.to_owned(), ty);
                    }
                    let ty = body.syn(ctx.clone())?;
                    let span = body.span();
                    let kd = span.make(ty.clone()).syn(ctx)?;
                    span.make(kd).ensure(&Kind::CType, "match arm")?;
                    ty_arms.push(ty);
                }
                let ctorv_set_data: HashSet<CtorV> =
                    ctors.keys().cloned().collect();
                let missing: Vec<_> = ctorv_set_data
                    .difference(&ctorv_set_arm)
                    .cloned()
                    .collect();
                bool_test(unexpected.is_empty() && missing.is_empty(), || {
                    self.span()
                        .make(InconsistentMatchers { unexpected, missing })
                })?;
                // branch consistency check
                let mut ty_opt: Option<Type> = None;
                for ty in &ty_arms {
                    if let Some(ty_opt) = &ty_opt {
                        ty_opt.eqv(ty, || {
                            self.span()
                                .make(InconsistentBranches(ty_arms.clone()))
                        })?;
                    } else {
                        ty_opt = Some(ty.clone());
                    }
                }
                // empty match
                let Some(ty) = ty_opt else {
                    Err(self.span().make(InconsistentBranches(vec![])))?
                };
                Step::Done(ty)
            }
            TermComputation::CoMatch(_) => Err(self
                .span()
                .make(NeedAnnotation { content: format!("comatch") }))?,
            TermComputation::Dtor(Dtor { body, dtor, args }) => {
                let ty_app = body.syn(ctx.clone())?.head_reduction()?;
                let TCtor::Var(tvar) = ty_app.tctor else {
                    Err(self.span().make(TypeExpected {
                        context: format!("dtor"),
                        expected: format!("codata type"),
                        found: ty_app.clone().into(),
                    }))?
                };
                let Codata { name, params, dtors } =
                    ctx.coda_ctx.get(&tvar).cloned().ok_or_else(|| {
                        self.span().make(
                            NameResolveError::UnboundTypeVariable { tvar }
                                .into(),
                        )
                    })?;
                // arity check on codata type
                let diff = Env::init(&params, &ty_app.args, || {
                    self.span().make(ArityMismatch {
                        context: format!(
                            "codata type `{}` instiantiation",
                            name
                        ),
                        expected: params.len(),
                        found: ty_app.args.len(),
                    })
                })?;
                let CodataBr(_, tys, ty) = dtors
                    .into_iter()
                    .find(|CodataBr(dtorv, _, _)| dtorv == dtor)
                    .ok_or_else(|| {
                        body.span().make(
                            NameResolveError::UnknownDestructor {
                                dtor: dtor.clone(),
                            }
                            .into(),
                        )
                    })?;
                bool_test(args.len() == tys.len(), || {
                    self.span().make(ArityMismatch {
                        context: format!("dtor"),
                        expected: tys.len(),
                        found: args.len(),
                    })
                })?;
                for (arg, ty) in args.iter().zip(tys.iter()) {
                    arg.ana(
                        ty.inner_ref().to_owned().subst(diff.clone()),
                        ctx.clone(),
                    )?;
                }
                Step::Done(ty.inner_ref().to_owned().subst(diff))
            }
        })
    }
    fn ana_step(
        &self, typ: Self::Out, mut ctx: Self::Ctx,
    ) -> Result<Step<(Self::Ctx, &Self), Self::Out>, Span<TypeCheckError>> {
        let span = self.span();
        span.make(span.make(typ.clone()).syn(ctx.clone())?)
            .ensure(&Kind::CType, "ana computation")?;
        Ok(match self.inner_ref() {
            TermComputation::Ret(Ret(v)) => {
                let ty_app = typ.head_reduction()?;
                let ty_body = ty_app.elim_ret().ok_or_else(|| {
                    self.span().make(TypeExpected {
                        context: format!("ret"),
                        expected: format!("ret a"),
                        found: typ.clone(),
                    })
                })?;
                v.ana(ty_body, ctx)?;
                Step::Done(typ)
            }
            TermComputation::Force(Force(v)) => {
                let kd = self.span().make(typ.clone()).syn(ctx.clone())?;
                self.span().make(kd).ensure(&Kind::CType, "force")?;
                v.ana(
                    Type::make_thunk(rc!(self.span().make(typ.clone()))),
                    ctx,
                )?;
                Step::Done(typ)
            }
            TermComputation::Let(Let { var, def, body }) => {
                let ty_def = def.syn(ctx.clone())?;
                let kd = self.span().make(ty_def.clone()).syn(ctx.clone())?;
                self.span().make(kd).ensure(&Kind::VType, "let")?;
                ctx.term_ctx.insert(var.to_owned(), ty_def.clone());
                Step::AnaMode((ctx, body), typ)
            }
            TermComputation::Do(Do { var, comp, body }) => {
                let ty_comp = comp.syn(ctx.clone())?;
                let kd = self.span().make(ty_comp.clone()).syn(ctx.clone())?;
                self.span().make(kd).ensure(&Kind::CType, "do")?;
                let ty_app = ty_comp.head_reduction()?;
                let ty_val = ty_app.elim_ret().ok_or_else(|| {
                    self.span().make(TypeExpected {
                        context: format!("do"),
                        expected: format!("ret a"),
                        found: ty_comp.clone(),
                    })
                })?;
                ctx.term_ctx.insert(var.to_owned(), ty_val);
                Step::AnaMode((ctx, body), typ)
            }
            TermComputation::Rec(Rec { var, body }) => {
                ctx.term_ctx.insert(
                    var.to_owned(),
                    Type::make_thunk(rc!(self.span().make(typ.clone()))),
                );
                Step::AnaMode((ctx, body), typ)
            }
            TermComputation::Match(Match { scrut, arms }) => {
                let ty_scrut = scrut.syn(ctx.clone())?;
                let kd = self.span().make(ty_scrut.clone()).syn(ctx.clone())?;
                self.span().make(kd).ensure(&Kind::VType, "match")?;
                let ty_app = ty_scrut.head_reduction()?;
                let TCtor::Var(tvar) = ty_app.tctor else {
                    Err(self.span().make(TypeExpected {
                        context: format!("match"),
                        expected: format!("codata type"),
                        found: ty_app.clone().into(),
                    }))?
                };
                let Data { name, params, ctors } =
                    ctx.data_ctx.get(&tvar).cloned().ok_or_else(|| {
                        self.span().make(
                            NameResolveError::UnboundTypeVariable { tvar }
                                .into(),
                        )
                    })?;
                // arity check on data type
                let diff = Env::init(&params, &ty_app.args, || {
                    self.span().make(ArityMismatch {
                        context: format!("data type `{}` instiantiation", name),
                        expected: params.len(),
                        found: ty_app.args.len(),
                    })
                })?;
                let ctors: HashMap<_, _> = ctors
                    .into_iter()
                    .map(|DataBr(ctor, tys)| (ctor, tys))
                    .collect();
                let mut unexpected = Vec::new();
                let mut ctorv_set_arm: HashSet<CtorV> = HashSet::new();
                for Matcher { ctor, vars, body } in arms {
                    let Some(tys) = ctors.get(ctor) else {
                        unexpected.push(ctor.to_owned());
                        continue;
                    };
                    ctorv_set_arm.insert(ctor.to_owned());
                    let tys = tys.into_iter().map(|ty| {
                        ty.inner_ref().to_owned().subst(diff.clone())
                    });
                    let mut ctx = ctx.clone();
                    for (var, ty) in vars.iter().zip(tys) {
                        ctx.term_ctx.insert(var.to_owned(), ty);
                    }
                    body.ana(typ.clone(), ctx.clone())?;
                }
                let ctorv_set_data: HashSet<CtorV> =
                    ctors.keys().cloned().collect();
                let missing: Vec<_> = ctorv_set_data
                    .difference(&ctorv_set_arm)
                    .cloned()
                    .collect();
                bool_test(unexpected.is_empty() && missing.is_empty(), || {
                    self.span()
                        .make(InconsistentMatchers { unexpected, missing })
                })?;
                Step::Done(typ)
            }
            TermComputation::CoMatch(CoMatch { arms }) => {
                let ty_app = typ.head_reduction()?;
                let TCtor::Var(tvar) = ty_app.tctor else {
                    Err(self.span().make(TypeExpected {
                        context: format!("comatch"),
                        expected: format!("codata type"),
                        found: typ.clone(),
                    }))?
                };
                let Codata { name, params, dtors } =
                    ctx.coda_ctx.get(&tvar).cloned().ok_or_else(|| {
                        self.span().make(
                            NameResolveError::UnboundTypeVariable { tvar }
                                .into(),
                        )
                    })?;
                // arity check on codata type
                let diff = Env::init(&params, &ty_app.args, || {
                    self.span().make(ArityMismatch {
                        context: format!(
                            "codata type `{}` instantiation",
                            name
                        ),
                        expected: params.len(),
                        found: ty_app.args.len(),
                    })
                })?;
                let dtors: HashMap<_, _> = dtors
                    .into_iter()
                    .map(|CodataBr(dtor, tys, ty)| (dtor, (tys, ty)))
                    .collect();
                let mut unexpected = Vec::new();
                let mut dtorv_set_arm: HashSet<DtorV> = HashSet::new();
                for CoMatcher { dtor, vars, body } in arms {
                    let Some((tys, ty)) = dtors.get(dtor).cloned() else {
                        unexpected.push(dtor.to_owned());
                        continue;
                    };
                    dtorv_set_arm.insert(dtor.to_owned());
                    let tys = tys.into_iter().map(|ty| {
                        ty.inner_ref().to_owned().subst(diff.clone())
                    });
                    let ty = ty.inner_ref().to_owned().subst(diff.clone());
                    let mut ctx = ctx.clone();
                    for (var, ty) in vars.iter().zip(tys) {
                        ctx.term_ctx.insert(var.to_owned(), ty);
                    }
                    body.ana(ty, ctx)?;
                }
                let dtorv_set_coda: HashSet<DtorV> =
                    dtors.keys().cloned().collect();
                let missing: Vec<_> = dtorv_set_coda
                    .difference(&dtorv_set_arm)
                    .cloned()
                    .collect();
                bool_test(unexpected.is_empty() && missing.is_empty(), || {
                    self.span()
                        .make(InconsistentCoMatchers { unexpected, missing })
                })?;
                Step::Done(typ)
            }
            TermComputation::TermAnn(_) | TermComputation::Dtor(_) => {
                let typ_syn = self.syn(ctx)?;
                typ.eqv(&typ_syn, || self.span().make(Subsumption))?;
                Step::Done(typ)
            }
        })
    }
}
