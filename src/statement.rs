use crate::{cil::CILOp, r#type::TyCache};
use rustc_middle::{
    mir::{Body, CopyNonOverlapping, NonDivergingIntrinsic, Statement, StatementKind},
    ty::{Instance, TyCtxt},
};
pub fn handle_statement<'tcx>(
    statement: &Statement<'tcx>,
    tyctx: TyCtxt<'tcx>,
    method: &Body<'tcx>,
    method_instance: Instance<'tcx>,
    type_cache: &mut TyCache,
) -> Vec<CILOp> {
    let kind = &statement.kind;
    let mut res = match kind {
        StatementKind::StorageLive(_local) => {
            vec![]
        }
        StatementKind::StorageDead(_local) => {
            vec![]
        }
        StatementKind::Assign(palce_rvalue) => {
            let place = palce_rvalue.as_ref().0;
            let rvalue = &palce_rvalue.as_ref().1;
            let rvalue_ops = rustc_middle::ty::print::with_no_trimmed_paths! {crate::rvalue::handle_rvalue(
                rvalue,
                tyctx,
                &place,
                method,
                method_instance,
                type_cache,
            )};
            let mut res = crate::place::place_set(
                &place,
                tyctx,
                rvalue_ops,
                method,
                method_instance,
                type_cache,
            );
            if crate::TRACE_STATEMENTS {
                use crate::r#type::Type;
                rustc_middle::ty::print::with_no_trimmed_paths! {res.extend(CILOp::debug_msg(&format!("{statement:?}")))};
                let place_ty = type_cache.type_from_cache(
                    crate::utilis::monomorphize(
                        &method_instance,
                        place.ty(method, tyctx).ty,
                        tyctx,
                    ),
                    tyctx,
                    Some(method_instance),
                );
                match place_ty {
                    Type::Bool => rustc_middle::ty::print::with_no_trimmed_paths! {{
                        res.extend(CILOp::debug_msg_no_nl(&format!("{place:?}:")));
                        res.extend(crate::place::place_get(
                            &place,
                            tyctx,
                            method,
                            method_instance,
                            type_cache
                        ));
                        res.push(CILOp::debug_bool());
                        res.extend(CILOp::debug_msg(&""));
                    }},
                    Type::I32 => rustc_middle::ty::print::with_no_trimmed_paths! {{
                        res.extend(CILOp::debug_msg_no_nl(&format!("{place:?}:")));
                        res.extend(crate::place::place_get(
                            &place,
                            tyctx,
                            method,
                            method_instance,
                            type_cache
                        ));
                        res.push(CILOp::debug_i32());
                        res.extend(CILOp::debug_msg(&""));
                    }},
                    Type::USize => rustc_middle::ty::print::with_no_trimmed_paths! {{
                        res.extend(CILOp::debug_msg_no_nl(&format!("{place:?}:")));
                        res.extend(crate::place::place_get(
                            &place,
                            tyctx,
                            method,
                            method_instance,
                            type_cache
                        ));
                        res.push(CILOp::ConvU64(false));
                        res.push(CILOp::debug_u64());
                        res.extend(CILOp::debug_msg(&""));
                    }},
                    Type::ISize => rustc_middle::ty::print::with_no_trimmed_paths! {{
                        res.extend(CILOp::debug_msg_no_nl(&format!("{place:?}:")));
                        res.extend(crate::place::place_get(
                            &place,
                            tyctx,
                            method,
                            method_instance,
                            type_cache
                        ));
                        res.push(CILOp::ConvU64(false));
                        res.push(CILOp::debug_u64());
                        res.extend(CILOp::debug_msg(&""));
                    }},
                    Type::Ptr(_) => rustc_middle::ty::print::with_no_trimmed_paths! {{
                        res.extend(CILOp::debug_msg_no_nl(&format!("{place:?}:")));
                        res.extend(crate::place::place_get(
                            &place,
                            tyctx,
                            method,
                            method_instance,
                            type_cache
                        ));
                        res.push(CILOp::ConvU64(false));
                        res.push(CILOp::debug_u64());
                        res.extend(CILOp::debug_msg(&""));
                    }},
                    Type::F32 => rustc_middle::ty::print::with_no_trimmed_paths! {{
                        res.extend(CILOp::debug_msg_no_nl(&format!("{place:?}:")));
                        res.extend(crate::place::place_get(
                            &place,
                            tyctx,
                            method,
                            method_instance,
                            type_cache
                        ));
                        res.push(CILOp::debug_f32());
                        res.extend(CILOp::debug_msg(&""));
                    }},
                    _ => (),
                }
            };
            res
        }
        StatementKind::Intrinsic(non_diverging_intirinsic) => {
            match non_diverging_intirinsic.as_ref() {
                NonDivergingIntrinsic::Assume(_) => vec![],
                NonDivergingIntrinsic::CopyNonOverlapping(CopyNonOverlapping {
                    src,
                    dst,
                    count,
                }) => {
                    let dst_op = crate::operand::handle_operand(
                        dst,
                        tyctx,
                        method,
                        method_instance,
                        type_cache,
                    );
                    let src_op = crate::operand::handle_operand(
                        src,
                        tyctx,
                        method,
                        method_instance,
                        type_cache,
                    );
                    let count_op = crate::operand::handle_operand(
                        count,
                        tyctx,
                        method,
                        method_instance,
                        type_cache,
                    );
                    let src_ty = src.ty(method, tyctx);
                    let src_ty = crate::utilis::monomorphize(&method_instance, src_ty, tyctx);
                    let ptr_type = type_cache.type_from_cache(src_ty, tyctx, Some(method_instance));
                    let pointed = if let crate::r#type::Type::Ptr(pointed) = ptr_type {
                        pointed
                    } else {
                        rustc_middle::ty::print::with_no_trimmed_paths! { panic!("Copy nonoverlaping called with non-pointer type {src_ty:?}")};
                    };
                    let mut res: Vec<_> =
                        [dst_op, src_op, count_op].into_iter().flatten().collect();
                    res.push(CILOp::SizeOf(pointed));
                    res.push(CILOp::Mul);
                    res.push(CILOp::CpBlk);
                    if crate::TRACE_STATEMENTS {
                        rustc_middle::ty::print::with_no_trimmed_paths! {res.extend(CILOp::debug_msg(&format!("{statement:?}")))};
                    }
                    res
                }
                _ => {
                    rustc_middle::ty::print::with_no_trimmed_paths! {todo!("Can't handle non-diverging intrinsics {non_diverging_intirinsic:?} yet!")}
                }
            }
        }
        _ => {
            rustc_middle::ty::print::with_no_trimmed_paths! {todo!("Unsuported statement kind {kind:?}")}
        }
    };

    res
}
