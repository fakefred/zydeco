use super::*;

impl Lub for () {
    type Ctx = Ctx;
    type Out = ();
    fn lub(self, _rhs: (), _ctx: Ctx, _span: &SpanInfo) -> Result<(), TyckError> {
        Ok(())
    }
}

impl<T> Lub for Seal<T> {
    type Ctx = ();
    type Out = Seal<T>;
    fn lub(self, _other: Self, _: Self::Ctx, _: &SpanInfo) -> Result<Self::Out, TyckError> {
        unreachable!()
    }
}

impl Kind {
    fn normalize(self) -> Self {
        match self {
            Kind::Base(_) => self,
            Kind::TypeArity(TypeArity { params, kd }) => {
                if params.is_empty() {
                    kd.inner()
                } else {
                    Kind::TypeArity(TypeArity { params, kd })
                }
            }
        }
    }
}

impl Lub for Kind {
    type Ctx = Ctx;
    type Out = Kind;
    fn lub(self, rhs: Kind, ctx: Ctx, span: &SpanInfo) -> Result<Kind, TyckError> {
        let lhs = self;
        let lhs = lhs.normalize();
        let rhs = rhs.normalize();
        match (lhs.clone(), rhs.clone()) {
            (Kind::Base(lhs), Kind::Base(rhs)) => {
                bool_test(lhs == rhs, || {
                    ctx.err(
                        span,
                        KindMismatch {
                            context: format!("lub"),
                            expected: lhs.into(),
                            found: rhs.into(),
                        },
                    )
                })?;
                Ok(lhs.into())
            }
            (Kind::TypeArity(lhs), Kind::TypeArity(rhs)) => {
                bool_test(lhs.params.len() == rhs.params.len(), || {
                    ctx.err(
                        span,
                        ArityMismatch {
                            context: format!("lub"),
                            expected: lhs.params.len(),
                            found: rhs.params.len(),
                        },
                    )
                })?;
                let mut params = Vec::new();
                for (l, r) in lhs.params.into_iter().zip(rhs.params.into_iter()) {
                    let span = l.span().clone();
                    let kd = l.inner().lub(r.inner(), ctx.clone(), &span)?;
                    params.push(span.make(kd))
                }
                let kd = Box::new(lhs.kd.try_map(|kd| kd.lub(rhs.kd.inner(), ctx, span))?);
                Ok(TypeArity { params, kd }.into())
            }
            _ => {
                Err(ctx
                    .err(span, KindMismatch { context: format!("lub"), expected: lhs, found: rhs }))
            }
        }
        // bool_test(self == rhs, || {
        //     ctx.err(span, KindMismatch { context: format!("lub"), expected: self, found: rhs })
        // })?;
    }
}

impl Lub for Type {
    type Ctx = Ctx;
    type Out = Type;
    fn lub(self, rhs: Type, mut ctx: Ctx, span: &SpanInfo) -> Result<Type, TyckError> {
        let lhs = self;
        let err = {
            let expected = lhs.clone();
            let found = rhs.clone();
            || ctx.err(span, TypeMismatch { context: format!("lub"), expected, found })
        };
        let lhs = ctx.resolve_alias(lhs, span)?;
        let lhs_syn = lhs.resolve()?;
        let rhs = ctx.resolve_alias(rhs, span)?;
        let rhs_syn = rhs.resolve()?;
        match (lhs_syn, rhs_syn) {
            // (SynType::Hole(_), SynType::Hole(_)) => Err(err())?,
            (SynType::Hole(_), _) => Ok(rhs),
            (_, SynType::Hole(_)) => Ok(lhs),
            (SynType::TypeApp(lhs), _) if ctx.type_env.contains_key(&lhs.tvar) => {
                // lhs is a type variable
                let ty = ctx.clone().type_env[&lhs.tvar].clone();
                ty.lub(rhs, ctx, span)
            }
            (_, SynType::TypeApp(rhs)) if ctx.type_env.contains_key(&rhs.tvar) => {
                // rhs is a type variable
                let ty = ctx.clone().type_env[&rhs.tvar].clone();
                lhs.lub(ty, ctx, span)
            }
            (SynType::TypeApp(lhs), SynType::TypeApp(rhs)) => {
                bool_test(lhs.tvar == rhs.tvar, err)?;
                let mut args = vec![];
                for (lhs, rhs) in (lhs.args.iter()).zip(rhs.args.iter()) {
                    let arg = Self::lub(lhs.inner_clone(), rhs.inner_clone(), ctx.clone(), span)?;
                    args.push(rc!(lhs.span().make(arg)));
                }
                Ok(TypeApp { tvar: lhs.tvar, args }.into())
            }
            (
                SynType::Forall(Forall { param: (tvar, kd), ty }),
                SynType::Forall(Forall { param: (tvar_, kd_), ty: ty_ }),
            ) => {
                let kd = kd.inner().lub(kd_.inner(), ctx.clone(), span)?;
                let abst_var = ctx.fresh(kd);
                let lhs_ty = (ty.inner_clone())
                    .subst(Env::from_iter([(tvar, abst_var.clone().into())]), &ctx)?;
                let rhs_ty =
                    ty_.inner_clone().subst(Env::from_iter([(tvar_, abst_var.into())]), &ctx)?;
                let _ty = lhs_ty.lub(rhs_ty, ctx, span)?;
                // HACK: needs revertable type subst
                // Ok(Forall { param: lhs.param.clone(), ty: rc!(lhs.ty.span().make(ty)) }.into())
                Ok(lhs)
            }
            (
                SynType::Exists(Exists { param: (tvar, kd), ty }),
                SynType::Exists(Exists { param: (tvar_, kd_), ty: ty_ }),
            ) => {
                let kd = kd.inner().lub(kd_.inner(), ctx.clone(), span)?;
                let abst_var = ctx.fresh(kd);
                let lhs_ty = (ty.inner_clone())
                    .subst(Env::from_iter([(tvar, abst_var.clone().into())]), &ctx)?;
                let rhs_ty =
                    (ty_.inner_clone()).subst(Env::from_iter([(tvar_, abst_var.into())]), &ctx)?;
                let _ty = lhs_ty.lub(rhs_ty, ctx, span)?;
                // HACK: needs revertable type subst
                // Ok(Exists { param: lhs.param.clone(), ty: rc!(lhs.ty.span().make(ty)) }.into())
                Ok(lhs)
            }
            (SynType::AbstVar(lhs), SynType::AbstVar(rhs)) => {
                bool_test(lhs == rhs, err)?;
                Ok(lhs.into())
            }
            (SynType::TypeApp(_), _)
            | (SynType::Forall(_), _)
            | (SynType::Exists(_), _)
            | (SynType::AbstVar(_), _) => Err(err()),
        }
    }
}
