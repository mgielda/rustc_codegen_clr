use crate::BaseIR;
use crate::FunctionSignature;
use crate::IString;
use crate::VariableType;
use rustc_index::IndexVec;
use rustc_middle::mir::{Body, CastKind, Local, LocalDecl};
use rustc_middle::mir::interpret::Scalar;
use rustc_middle::{
    mir::{
        interpret::ConstValue, AggregateKind, BinOp, ConstantKind, Operand, Rvalue, Statement,
        StatementKind, Terminator, TerminatorKind,
    },
    ty::{IntTy, TyCtxt, TyKind},
};
macro_rules! sign_cast{
    ($var:ident,$src:ty,$dest:ty)=>{
        (<$dest>::from_ne_bytes(($var as $src).to_ne_bytes()))
    };
}
#[derive(Debug)]
pub(crate) struct CLRMethod {
    ops: Vec<BaseIR>,
    locals: Vec<VariableType>,
    sig: FunctionSignature,
    name: IString,
    curr_bb:u32,
    //bbs:
}
enum LocalPlacement {
    Arg(u32),
    Var(u32),
}
impl CLRMethod {
    pub fn begin_bb(&mut self){
        self.ops.push(BaseIR::BBLabel{bb_id:self.curr_bb});
        self.curr_bb += 1;
    }
    fn name(&self) -> &str {
        &self.name
    }
    pub(crate) fn add_locals(&mut self, locals: &IndexVec<Local, LocalDecl>) {
        let mut new_locals: Vec<VariableType> = Vec::with_capacity(locals.len());
        for local in locals {
            new_locals.push(VariableType::from_ty(local.ty));
        }
        self.locals = new_locals;
        //todo!();
    }

    pub(crate) fn locals_init(&self) -> IString {
        if self.locals.is_empty() {
            return "".into();
        }
        let mut locals = String::new();
        let mut locals_iter = self.locals.iter().enumerate();
        match locals_iter.next() {
            Some((index, first)) => locals.push_str(&format!(
                "\n\t\t[{index}] {loc_type}",
                loc_type = first.il_name()
            )),
            None => (),
        }
        for (index, local) in locals_iter {
            locals.push_str(&format!(
                ",\n\t\t[{index}] {loc_type}",
                loc_type = local.il_name()
            ))
        }
        format!("\t.locals init({locals}\n\t)").into()
    }
    pub(crate) fn into_il_ir(&self) -> String {
        let output = self.sig.output().il_name();
        let mut arg_list = String::new();
        let mut arg_iter = self.sig.inputs.iter();
        match arg_iter.next() {
            Some(start) => arg_list.push_str(&start.il_name()),
            None => (),
        }
        for arg in arg_iter {
            arg_list.push(',');
            arg_list.push_str(&arg.il_name());
        }
        let mut ops_ir = String::new();
        for op in &self.ops {
            ops_ir.push_str(&op.clr_ir());
        }
        format!(
            ".method public static {output} {name}({arg_list}){{\n{locals_init}\n{ops_ir}}}\n",
            name = self.name(),
            locals_init = self.locals_init()
        )
    }
    fn argc(&self) -> u32 {
        self.sig.inputs().len() as u32
    }
    fn has_return(&self) -> bool {
        true
    }
    
    pub(crate) fn new(sig: FunctionSignature, name: &str) -> Self {
        Self {
            locals: Vec::new(),
            sig,
            name: name.into(),
            ops: Vec::with_capacity(0x100),
            curr_bb:0,
        }
    }
    fn var_live(&mut self, _local: u32) {
        //TODO: use variable lifetimes!
    }
    fn var_dead(&mut self, _local: u32) {
        //TODO: use variable lifetimes!
    }
    fn local_id_placement(&self, local: u32) -> LocalPlacement {
        // I assume local 0 is supposed to be the return value. TODO: check if this is always the case.
        if self.has_return() {
            if local == 0 {
                LocalPlacement::Var(0)
            } else if local - 1 < self.argc() {
                LocalPlacement::Arg(local - 1)
            } else {
                LocalPlacement::Var(local - self.argc())
            }
        } else {
            panic!("Can't handle void functions yet. Diagnose MIR to determine what happens to the return var(0)!");
        }
    }
    fn load(&mut self, local: u32) {
        self.ops.push(match self.local_id_placement(local) {
            LocalPlacement::Arg(arg_id) => BaseIR::LDArg(arg_id),
            LocalPlacement::Var(var_id) => BaseIR::LDLoc(var_id),
        })
    }
    fn store(&mut self, local: u32) {
        self.ops.push(match self.local_id_placement(local) {
            LocalPlacement::Arg(arg_id) => BaseIR::STArg(arg_id),
            LocalPlacement::Var(var_id) => BaseIR::STLoc(var_id),
        })
    }
    fn process_constant(&mut self, constant: ConstantKind) {
        match constant {
            ConstantKind::Val(value, r#type) => match value {
                    ConstValue::Scalar(scalar) =>{
                        let value:u128 = if let Scalar::Int(scalar) = scalar{
                            scalar.try_to_uint(scalar.size()).expect("IMPOSSIBLE. Size of scalar was not equal to itself.")
                        }else{
                            panic!("Can't support pointers quite yet!");
                        };
                        self.load_constant_primitive(&VariableType::from_ty(r#type),value);
                    }
                    _ => todo!("Unhanled constant value {value:?}"),
                },
                _ => todo!("Unhanled constant {constant:?}"),
        };
    }
    // Makes so the top of the stack is the value of RValue
    fn process_operand(&mut self, operand: &Operand) {
        match operand {
            Operand::Copy(place) => self.load(place.local.into()),
            //TODO:Do moves need to be treated any diffrently forom copies in the context of CLR?
            Operand::Move(place) => self.load(place.local.into()),
            Operand::Constant(const_val) => {
                self.process_constant(const_val.literal);
            }
        }
    }
    fn convert(&mut self, src: &VariableType, dest: &VariableType) {
        match (src, dest) {
            (VariableType::F32 | VariableType::I8 | VariableType::I16  | VariableType::I32 | VariableType::U8 | VariableType::U16 |  VariableType::U32, VariableType::I32) => self.ops.push(BaseIR::ConvI32),
            (VariableType::F64 | VariableType::I64 | VariableType::U64 | VariableType::ISize | VariableType::USize, VariableType::I32) => self.ops.push(BaseIR::ConvI32Checked),
            (VariableType::F32, VariableType::I8) => self.ops.push(BaseIR::ConvI8),
            (VariableType::I32, VariableType::F32) => self.ops.push(BaseIR::ConvF32),
            _ => todo!("Can't convert type {src:?} to {dest:?}"),
        }
    }
    // Makes so the top of the stack is the value of RValue
    fn process_rvalue<'ctx>(
        &mut self,
        rvalue: &Rvalue<'ctx>,
        body: &Body<'ctx>,
        tyctx: &TyCtxt<'ctx>,
    ) {
        match rvalue {
            Rvalue::Use(operand) => self.process_operand(operand),
            Rvalue::BinaryOp(binop, operands) => {
                let (a, b): (_, _) = (&operands.0, &operands.1);
                self.process_operand(a);
                self.process_operand(b);
                self.ops.push(match binop {
                    BinOp::Add => BaseIR::Add,
                    BinOp::Sub => BaseIR::Sub,
                    BinOp::Mul => BaseIR::Mul,
                    BinOp::Shl => BaseIR::Shl,
                    BinOp::Shr => BaseIR::Shr,
                    BinOp::Eq => BaseIR::Eq,
                    BinOp::Gt => BaseIR::Gt,
                    BinOp::Rem => BaseIR::Rem,
                    _ => todo!("Unknown binop:{binop:?}"),
                });
            }
            Rvalue::Cast(
                CastKind::IntToInt
                | CastKind::FloatToFloat
                | CastKind::FloatToInt
                | CastKind::IntToFloat,
                operand,
                target,
            ) => {
                self.process_operand(operand);
                self.convert(
                    &VariableType::from_ty(operand.ty(body, *tyctx)),
                    &VariableType::from_ty(*target),
                );
            }
            Rvalue::Aggregate(kind, operands) => {
                match kind.as_ref() {
                    AggregateKind::Adt(def_id, variant_idx, subst_ref, utai, fidx) => {
                        //let (def_id,variant_idx,subst_ref,utai,fidx) = *adt;
                        panic!("def_id:{def_id:?},variant_idx:{variant_idx:?},subst_ref:{subst_ref:?},utai:{utai:?},fidx:{fidx:?}");
                    }
                    _ => todo!(
                        "Can't yet handle the aggregate of kind {kind:?} and operands:{operands:?}"
                    ),
                }
            }
            _ => todo!("Can't yet handle a rvalue of type {rvalue:?}"),
        }
    }
    pub(crate) fn add_statement<'ctx>(
        &mut self,
        statement: &Statement<'ctx>,
        body: &Body<'ctx>,
        tyctx: &TyCtxt<'ctx>,
    ) {
        println!("statement:{statement:?}");
        match &statement.kind {
            StatementKind::Assign(asign_box) => {
                let (place, rvalue) = (asign_box.0, &asign_box.1);
                self.process_rvalue(rvalue, body, tyctx);
                self.store(place.local.into());
                //panic!("place:{place:?},rvalue:{rvalue:?}");
            }
            StatementKind::StorageLive(local) => {
                self.var_live((*local).into());
            }
            StatementKind::StorageDead(local) => {
                self.var_dead((*local).into());
            }
            _ => todo!("Unhanded statement:{:?}", statement.kind),
        }
    }
    pub(crate) fn load_constant_primitive(&mut self, var_type:&VariableType,value:u128){
        match (var_type){
            VariableType::I8=>self.ops.push(BaseIR::LDConstI8(sign_cast!(value,u8,i8))),
            VariableType::I32=>self.ops.push(BaseIR::LDConstI32(sign_cast!(value,u32,i32))),
            VariableType::I64=>self.ops.push(BaseIR::LDConstI64(sign_cast!(value,u64,i64))),
            VariableType::Bool=>self.ops.push(BaseIR::LDConstI8((value != 0) as u8 as i8)),
            _=>todo!("Can't yet load constant primitives of type {var_type:?}!"),
        }
    }
    pub(crate) fn add_terminator<'ctx>(&mut self, terminator: &Terminator<'ctx>,
         body: &Body<'ctx>,
        tyctx: &TyCtxt<'ctx>,
    ) {
        match &terminator.kind {
            TerminatorKind::Return => {
                if self.has_return() {
                    self.load(0);
                    self.ops.push(BaseIR::Return);
                } else {
                    todo!("Can't yet return from a void method!");
                }
            }
            TerminatorKind::SwitchInt{discr,targets}=>{
                for (value,target) in targets.iter(){
                    self.process_operand(&discr);
                    self.load_constant_primitive(&VariableType::from_ty(discr.ty(body,*tyctx)),value);
                    self.ops.push(BaseIR::BEq{target:target.into()});
                }
                self.ops.push(BaseIR::GoTo{target:(*targets.all_targets().last().unwrap()).into()});
            }
            TerminatorKind::Goto{target}=>{
                self.ops.push(BaseIR::GoTo{target:(*target).into()});
            },
            _ => todo!("Unknown terminator type {terminator:?}"),
        }
    }
}