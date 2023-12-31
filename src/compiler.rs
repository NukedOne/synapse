use crate::parser::{
    AssignExpression, BinaryExpression, BinaryExpressionKind, BlockStatement, BreakStatement,
    CallExpression, ContinueStatement, Expression, ExpressionStatement, FnStatement, ForStatement,
    GetExpression, IfStatement, ImplStatement, Literal, LiteralExpression, LogicalExpression,
    PrintStatement, ReturnStatement, Statement, StructExpression, StructInitializerExpression,
    StructStatement, SubscriptExpression, UnaryExpression, VariableExpression, VecExpression,
    WhileStatement,
};
use crate::tokenizer::Token;
use anyhow::{bail, Result};
use std::collections::HashMap;
use std::rc::Rc;

const CAPACITY_MIN: usize = 1024;

pub struct Compiler<'src> {
    bytecode: Bytecode<'src>,
    functions: HashMap<&'src str, Function<'src>>,
    locals: Vec<&'src str>,
    pops: Vec<usize>,
    structs: HashMap<&'src str, Blueprint<'src>>,
    breaks: Vec<usize>,
    loop_starts: Vec<usize>,
    loop_depths: Vec<usize>,
    depth: usize,
}

impl Default for Compiler<'_> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'src> Compiler<'src> {
    pub fn new() -> Self {
        Compiler {
            bytecode: Bytecode::default(),
            functions: HashMap::with_capacity(CAPACITY_MIN),
            locals: Vec::with_capacity(CAPACITY_MIN),
            structs: HashMap::with_capacity(CAPACITY_MIN),
            pops: Vec::with_capacity(CAPACITY_MIN),
            breaks: Vec::with_capacity(CAPACITY_MIN),
            loop_starts: Vec::with_capacity(CAPACITY_MIN),
            loop_depths: Vec::with_capacity(CAPACITY_MIN),
            depth: 0,
        }
    }

    pub fn compile(&mut self, ast: &[Statement<'src>]) -> Result<&Bytecode<'src>> {
        for statement in ast {
            statement.codegen(self)?;
        }

        match self.functions.get("main").cloned() {
            Some(f) => {
                self.emit_opcodes(&[Opcode::Call(0)]);
                self.emit_opcodes(&[Opcode::Jmp(f.location)]);
                self.emit_opcodes(&[Opcode::Pop(1)]);
            }
            None => bail!("compiler: main fn was not defined"),
        }

        self.emit_opcodes(&[Opcode::Halt]);

        Ok(&self.bytecode)
    }

    fn compile_variable_assignment(
        &mut self,
        assign_expr: AssignExpression<'src>,
        variable_expr: VariableExpression<'src>,
        is_specialized: bool,
        operator: Token<'src>,
    ) -> Result<()> {
        let (idx, fresh) = self.resolve_local(variable_expr.value);

        if is_specialized {
            self.emit_opcodes(&[Opcode::Deepget(idx)]);

            assign_expr.rhs.codegen(self)?;
            self.handle_specialized_operator(operator);
        } else {
            assign_expr.rhs.codegen(self)?;
        }

        if !fresh {
            self.emit_opcodes(&[Opcode::Deepset(idx)]);
        } else {
            match self.pops.last_mut() {
                Some(last) => *last += 1,
                None => bail!("compiler: tried to pop an empty stack."),
            }
        }

        Ok(())
    }

    fn compile_unary_assignment(
        &mut self,
        unary_expr: UnaryExpression<'src>,
        rhs: Expression<'src>,
        is_specialized: bool,
        operator: Token<'src>,
    ) -> Result<()> {
        unary_expr.expr.codegen(self)?;

        if is_specialized {
            rhs.codegen(self)?;
            self.handle_specialized_operator(operator);
        } else {
            rhs.codegen(self)?;
        }

        self.emit_opcodes(&[Opcode::DerefSet]);

        Ok(())
    }

    fn compile_get_assignment(
        &mut self,
        get_expr: GetExpression<'src>,
        rhs: Expression<'src>,
        is_specialized: bool,
        operator: Token<'src>,
    ) -> Result<()> {
        get_expr.expr.codegen(self)?;

        if get_expr.op == Token::Arrow {
            self.emit_opcodes(&[Opcode::Deref]);
        }

        if is_specialized {
            self.emit_opcodes(&[Opcode::Getattr(get_expr.member.to_owned().into())]);

            rhs.codegen(self)?;
            self.handle_specialized_operator(operator);
        } else {
            rhs.codegen(self)?;
        }

        self.emit_opcodes(&[Opcode::Setattr(get_expr.member.to_owned().into())]);

        self.emit_opcodes(&[Opcode::Pop(1)]);

        Ok(())
    }

    fn compile_subscript_assignment(
        &mut self,
        sub_expr: SubscriptExpression<'src>,
        e: AssignExpression<'src>,
        is_specialized: bool,
        operator: Token<'src>,
    ) -> Result<()> {
        sub_expr.expr.codegen(self)?;
        sub_expr.index.codegen(self)?;

        if is_specialized {
            e.lhs.codegen(self)?;
            e.rhs.codegen(self)?;
            self.handle_specialized_operator(operator);
        } else {
            e.rhs.codegen(self)?;
        }

        self.emit_opcodes(&[Opcode::VecSet]);

        Ok(())
    }

    fn handle_specialized_operator(&mut self, operator: Token<'src>) {
        match operator {
            Token::PlusEqual => self.emit_opcodes(&[Opcode::Add]),
            Token::MinusEqual => self.emit_opcodes(&[Opcode::Sub]),
            Token::StarEqual => self.emit_opcodes(&[Opcode::Mul]),
            Token::SlashEqual => self.emit_opcodes(&[Opcode::Div]),
            Token::PercentEqual => self.emit_opcodes(&[Opcode::Mod]),
            Token::AmpersandEqual => self.emit_opcodes(&[Opcode::BitAnd]),
            Token::PipeEqual => self.emit_opcodes(&[Opcode::BitOr]),
            Token::CaretEqual => self.emit_opcodes(&[Opcode::BitXor]),
            Token::LessLessEqual => self.emit_opcodes(&[Opcode::BitShl]),
            Token::GreaterGreaterEqual => self.emit_opcodes(&[Opcode::BitShr]),
            _ => unreachable!(),
        };
    }

    fn add_string(&mut self, s: &'src str) -> usize {
        match self.bytecode.sp.iter().position(|&x| x == s) {
            Some(idx) => idx,
            None => {
                self.bytecode.sp.push(s);
                self.bytecode.sp.len() - 1
            }
        }
    }

    fn emit_opcodes(&mut self, opcodes: &[Opcode]) -> usize {
        for opcode in opcodes {
            self.bytecode.code.push(opcode.clone());
        }
        self.bytecode.code.len() - opcodes.len()
    }

    fn emit_u32(&mut self, value: u32) {
        self.bytecode
            .code
            .push(Opcode::Raw(((value >> 24) & 0xFF) as u8));
        self.bytecode
            .code
            .push(Opcode::Raw(((value >> 16) & 0xFF) as u8));
        self.bytecode
            .code
            .push(Opcode::Raw(((value >> 8) & 0xFF) as u8));
        self.bytecode.code.push(Opcode::Raw((value & 0xFF) as u8));
    }

    fn emit_stack_cleanup(&mut self) {
        let popcount = self.pops.last().copied().unwrap();
        self.emit_opcodes(&[Opcode::Pop(popcount)]);
    }

    // clean up the stack and locals,
    // that is everything declared within the loop
    fn emit_loop_cleanup(&mut self) {
        if let Some(&last_depth) = self.loop_depths.last() {
            for i in last_depth + 1..=self.depth {
                self.emit_opcodes(&[Opcode::Pop(self.pops[i])]);
            }
        }
    }

    fn resolve_local(&mut self, name: &'src str) -> (usize, bool) {
        match self.locals.iter().position(|&local| local == name) {
            Some(idx) => (idx, false),
            None => {
                self.locals.push(name);
                (self.locals.len() - 1, true)
            }
        }
    }

    fn patch_jmp(&mut self, idx: usize) {
        let v = self.bytecode.code.len() - 1;
        if let Some(opcode) = self.bytecode.code.get_mut(idx) {
            match opcode {
                Opcode::Jmp(addr) | Opcode::Jz(addr) => {
                    *addr = v;
                }
                _ => unreachable!(),
            }
        }
    }
}

impl<'src> Codegen<'src> for Statement<'src> {
    fn codegen(&self, compiler: &mut Compiler<'src>) -> Result<()> {
        match self {
            Statement::Print(print_statement) => print_statement.codegen(compiler)?,
            Statement::Fn(fn_statement) => fn_statement.codegen(compiler)?,
            Statement::Return(return_statement) => return_statement.codegen(compiler)?,
            Statement::If(if_statement) => if_statement.codegen(compiler)?,
            Statement::While(while_statement) => while_statement.codegen(compiler)?,
            Statement::For(for_statement) => for_statement.codegen(compiler)?,
            Statement::Break(break_statement) => break_statement.codegen(compiler)?,
            Statement::Continue(continue_statement) => continue_statement.codegen(compiler)?,
            Statement::Expression(expr_statement) => expr_statement.codegen(compiler)?,
            Statement::Block(block_statement) => block_statement.codegen(compiler)?,
            Statement::Struct(struct_statement) => struct_statement.codegen(compiler)?,
            Statement::Impl(impl_statement) => impl_statement.codegen(compiler)?,
            Statement::Dummy => {}
        }

        Ok(())
    }
}

impl<'src> Codegen<'src> for PrintStatement<'src> {
    fn codegen(&self, compiler: &mut Compiler<'src>) -> Result<()> {
        self.expression.codegen(compiler)?;
        compiler.emit_opcodes(&[Opcode::Print]);

        Ok(())
    }
}

impl<'src> Codegen<'src> for FnStatement<'src> {
    fn codegen(&self, compiler: &mut Compiler<'src>) -> Result<()> {
        let jmp_idx = compiler.emit_opcodes(&[Opcode::Jmp(0xFFFFFFFF)]);

        let arguments: Vec<&'src str> = self
            .arguments
            .iter()
            .map(|&token| token.get_value())
            .collect();

        let name = self.name.get_value();

        let f = Function {
            name,
            localscount: 0,
            location: jmp_idx,
            paramcount: arguments.len(),
        };
        compiler.functions.insert(name, f.clone());

        for argument in &self.arguments {
            compiler.locals.push(argument.get_value());
        }

        compiler.pops.push(compiler.locals.len());

        if let Statement::Block(block) = &*self.body {
            block.codegen(compiler)?;
        }

        compiler.patch_jmp(jmp_idx);

        if let Some(func) = compiler.functions.get_mut(f.name) {
            func.localscount = compiler.locals.len();
        }

        compiler.locals.clear();
        compiler.pops.clear();

        Ok(())
    }
}

impl<'src> Codegen<'src> for IfStatement<'src> {
    fn codegen(&self, compiler: &mut Compiler<'src>) -> Result<()> {
        self.condition.codegen(compiler)?;

        let jz_idx = compiler.emit_opcodes(&[Opcode::Jz(0xFFFFFFFF)]);

        self.if_branch.codegen(compiler)?;

        let else_idx = compiler.emit_opcodes(&[Opcode::Jmp(0xFFFFFFFF)]);

        compiler.patch_jmp(jz_idx);

        self.else_branch.codegen(compiler)?;
        compiler.patch_jmp(else_idx);

        Ok(())
    }
}

impl<'src> Codegen<'src> for WhileStatement<'src> {
    fn codegen(&self, compiler: &mut Compiler<'src>) -> Result<()> {
        let loop_start = compiler.bytecode.code.len() - 1;

        compiler.loop_starts.push(loop_start);
        let break_count = compiler.breaks.len();

        self.condition.codegen(compiler)?;

        let jz_idx = compiler.emit_opcodes(&[Opcode::Jz(0xFFFFFFFF)]);

        compiler.loop_depths.push(compiler.depth);

        self.body.codegen(compiler)?;

        compiler.loop_depths.pop();

        compiler.emit_opcodes(&[Opcode::Jmp(loop_start)]);

        let pop = compiler.breaks.len() - break_count;
        for _ in 0..pop {
            let break_jump = compiler.breaks.pop().unwrap();
            compiler.patch_jmp(break_jump);
        }

        compiler.loop_starts.pop();

        compiler.patch_jmp(jz_idx);

        Ok(())
    }
}

impl<'src> Codegen<'src> for ForStatement<'src> {
    fn codegen(&self, compiler: &mut Compiler<'src>) -> Result<()> {
        if let Expression::Assign(assignment) = self.initializer.clone() {
            if let Expression::Variable(variable) = &*assignment.lhs {
                compiler.locals.push(variable.value);
                assignment.rhs.codegen(compiler)?;

                let loop_start = compiler.bytecode.code.len() - 1;
                compiler.loop_starts.push(loop_start);
                let break_count = compiler.breaks.len();

                self.condition.codegen(compiler)?;

                let exit_jump = compiler.emit_opcodes(&[Opcode::Jz(0xFFFFFFFF)]);

                let jump_over_advancement = compiler.emit_opcodes(&[Opcode::Jmp(0xFFFFFFFF)]);

                let loop_continuation = compiler.bytecode.code.len() - 1;

                self.advancement.codegen(compiler)?;

                compiler.emit_opcodes(&[Opcode::Jmp(loop_start)]);

                compiler.patch_jmp(jump_over_advancement);

                if let Some(start) = compiler.loop_starts.last_mut() {
                    *start = loop_continuation;
                }

                compiler.loop_depths.push(compiler.depth);

                self.body.codegen(compiler)?;

                compiler.loop_depths.pop();

                compiler.emit_opcodes(&[Opcode::Jmp(loop_continuation)]);

                let pop = compiler.breaks.len() - break_count;
                for _ in 0..pop {
                    let break_jump = compiler.breaks.pop().unwrap();
                    compiler.patch_jmp(break_jump);
                }

                compiler.locals.pop();
                compiler.loop_starts.pop();

                compiler.patch_jmp(exit_jump);

                compiler.emit_opcodes(&[Opcode::Pop(1)]);
            }
        }

        Ok(())
    }
}

impl<'src> Codegen<'src> for BreakStatement {
    fn codegen(&self, compiler: &mut Compiler<'src>) -> Result<()> {
        if !compiler.loop_starts.is_empty() {
            compiler.emit_loop_cleanup();

            let break_jump = compiler.emit_opcodes(&[Opcode::Jmp(0xFFFFFFFF)]);

            compiler.breaks.push(break_jump);
        } else {
            bail!("compiler: break outside a loop");
        }

        Ok(())
    }
}

impl<'src> Codegen<'src> for ContinueStatement {
    fn codegen(&self, compiler: &mut Compiler<'src>) -> Result<()> {
        if !compiler.loop_starts.is_empty() {
            let loop_start = compiler.loop_starts.last().copied().unwrap();

            compiler.emit_loop_cleanup();

            compiler.emit_opcodes(&[Opcode::Jmp(loop_start)]);
        } else {
            bail!("compiler: continue outside a loop");
        }

        Ok(())
    }
}

impl<'src> Codegen<'src> for StructStatement<'src> {
    fn codegen(&self, compiler: &mut Compiler<'src>) -> Result<()> {
        let blueprint = Blueprint {
            members: self.members.clone(),
            name: self.name,
            methods: HashMap::new(),
        };
        compiler.structs.insert(self.name, blueprint.clone());

        compiler.emit_opcodes(&[Opcode::StructBlueprint]);

        let blueprint_name_idx = compiler.add_string(self.name);

        compiler.emit_u32(blueprint_name_idx as u32);
        compiler.emit_u32(blueprint.members.len() as u32);

        for member in blueprint.members {
            let member_name_idx = compiler.add_string(member);
            compiler.emit_u32(member_name_idx as u32);
        }

        Ok(())
    }
}

impl<'src> Codegen<'src> for ImplStatement<'src> {
    fn codegen(&self, compiler: &mut Compiler<'src>) -> Result<()> {
        if let Some(mut blueprint) = compiler.structs.get(self.name).cloned() {
            for statement in &self.methods {
                if let Statement::Fn(method) = statement {
                    let f = Function {
                        name: method.name.get_value(),
                        localscount: 0,
                        location: compiler.bytecode.code.len(),
                        paramcount: method.arguments.len(),
                    };
                    blueprint.methods.insert(method.name.get_value(), f);
                    method.codegen(compiler)?;
                }
            }

            let blueprint_name_idx = compiler.add_string(blueprint.name);

            compiler.emit_opcodes(&[Opcode::Impl]);
            compiler.emit_u32(blueprint_name_idx as u32);
            compiler.emit_u32(blueprint.methods.len() as u32);

            for (method_name, method) in blueprint.methods {
                let method_name_idx = compiler.add_string(method_name);
                compiler.emit_u32(method_name_idx as u32);
                compiler.emit_u32(method.paramcount as u32);
                compiler.emit_u32(method.location as u32);
            }
        } else {
            bail!("compiler: struct '{}' is not defined", self.name);
        }

        Ok(())
    }
}

impl<'src> Codegen<'src> for ExpressionStatement<'src> {
    fn codegen(&self, compiler: &mut Compiler<'src>) -> Result<()> {
        match &self.expression {
            Expression::Call(call_expr) => {
                call_expr.codegen(compiler)?;
                compiler.emit_opcodes(&[Opcode::Pop(1)]);
            }

            Expression::Assign(assign_expr) => {
                assign_expr.codegen(compiler)?;
            }
            _ => unreachable!(),
        }

        Ok(())
    }
}

impl<'src> Codegen<'src> for ReturnStatement<'src> {
    fn codegen(&self, compiler: &mut Compiler<'src>) -> Result<()> {
        self.expression.codegen(compiler)?;

        let mut deepset_no = compiler.locals.len().saturating_sub(1);
        for _ in 0..compiler.locals.len() {
            compiler.emit_opcodes(&[Opcode::Deepset(deepset_no)]);

            deepset_no = deepset_no.saturating_sub(1);
        }

        compiler.emit_opcodes(&[Opcode::Ret]);

        Ok(())
    }
}

impl<'src> Codegen<'src> for BlockStatement<'src> {
    fn codegen(&self, compiler: &mut Compiler<'src>) -> Result<()> {
        compiler.depth += 1;
        compiler.pops.push(0);

        for statement in &self.body {
            statement.codegen(compiler)?;
        }

        for _ in 0..*compiler.pops.last().unwrap() {
            compiler.locals.pop();
        }

        compiler.emit_stack_cleanup();
        compiler.pops.pop();

        compiler.depth -= 1;

        Ok(())
    }
}

impl<'src> Codegen<'src> for Expression<'src> {
    fn codegen(&self, compiler: &mut Compiler<'src>) -> Result<()> {
        match self {
            Expression::Literal(literal) => literal.codegen(compiler)?,
            Expression::Variable(varexp) => varexp.codegen(compiler)?,
            Expression::Binary(binexp) => binexp.codegen(compiler)?,
            Expression::Call(call) => call.codegen(compiler)?,
            Expression::Assign(assignment) => assignment.codegen(compiler)?,
            Expression::Logical(logicalexp) => logicalexp.codegen(compiler)?,
            Expression::Unary(unary) => unary.codegen(compiler)?,
            Expression::Get(getexp) => getexp.codegen(compiler)?,
            Expression::Struct(structexp) => structexp.codegen(compiler)?,
            Expression::StructInitializer(structinitexp) => structinitexp.codegen(compiler)?,
            Expression::Vec(vecexpr) => vecexpr.codegen(compiler)?,
            Expression::Sub(subscriptexpr) => subscriptexpr.codegen(compiler)?,
        }

        Ok(())
    }
}

impl<'src> Codegen<'src> for LiteralExpression<'src> {
    fn codegen(&self, compiler: &mut Compiler<'src>) -> Result<()> {
        match &self.value {
            Literal::Num(n) => {
                compiler.emit_opcodes(&[Opcode::Const(*n)]);
            }

            Literal::Bool(b) => match b {
                true => {
                    compiler.emit_opcodes(&[Opcode::False, Opcode::Not]);
                }
                false => {
                    compiler.emit_opcodes(&[Opcode::False]);
                }
            },

            Literal::String(s) => {
                compiler.emit_opcodes(&[Opcode::Str(s.to_string().into())]);
            }

            Literal::Null => {
                compiler.emit_opcodes(&[Opcode::Null]);
            }
        }

        Ok(())
    }
}

impl<'src> Codegen<'src> for VariableExpression<'src> {
    fn codegen(&self, compiler: &mut Compiler<'src>) -> Result<()> {
        let (idx, _) = compiler.resolve_local(self.value);
        compiler.emit_opcodes(&[Opcode::Deepget(idx)]);

        Ok(())
    }
}

impl<'src> Codegen<'src> for BinaryExpression<'src> {
    fn codegen(&self, compiler: &mut Compiler<'src>) -> Result<()> {
        self.lhs.codegen(compiler)?;
        self.rhs.codegen(compiler)?;

        match self.kind {
            BinaryExpressionKind::Add => {
                compiler.emit_opcodes(&[Opcode::Add]);
            }

            BinaryExpressionKind::Sub => {
                compiler.emit_opcodes(&[Opcode::Sub]);
            }

            BinaryExpressionKind::Mul => {
                compiler.emit_opcodes(&[Opcode::Mul]);
            }

            BinaryExpressionKind::Div => {
                compiler.emit_opcodes(&[Opcode::Div]);
            }

            BinaryExpressionKind::Mod => {
                compiler.emit_opcodes(&[Opcode::Mod]);
            }

            BinaryExpressionKind::Equality(negation) => {
                compiler.emit_opcodes(&[Opcode::Eq]);

                if negation {
                    compiler.emit_opcodes(&[Opcode::Not]);
                }
            }

            BinaryExpressionKind::Less => {
                compiler.emit_opcodes(&[Opcode::Lt]);
            }

            BinaryExpressionKind::Greater => {
                compiler.emit_opcodes(&[Opcode::Gt]);
            }

            BinaryExpressionKind::LessEqual => {
                compiler.emit_opcodes(&[Opcode::Gt, Opcode::Not]);
            }

            BinaryExpressionKind::GreaterEqual => {
                compiler.emit_opcodes(&[Opcode::Lt, Opcode::Not]);
            }

            BinaryExpressionKind::BitwiseAnd => {
                compiler.emit_opcodes(&[Opcode::BitAnd]);
            }

            BinaryExpressionKind::BitwiseOr => {
                compiler.emit_opcodes(&[Opcode::BitOr]);
            }

            BinaryExpressionKind::BitwiseXor => {
                compiler.emit_opcodes(&[Opcode::BitXor]);
            }

            BinaryExpressionKind::BitwiseShl => {
                compiler.emit_opcodes(&[Opcode::BitShl]);
            }

            BinaryExpressionKind::BitwiseShr => {
                compiler.emit_opcodes(&[Opcode::BitShr]);
            }

            BinaryExpressionKind::Strcat => {
                compiler.emit_opcodes(&[Opcode::Strcat]);
            }
        }

        Ok(())
    }
}

impl<'src> Codegen<'src> for CallExpression<'src> {
    fn codegen(&self, compiler: &mut Compiler<'src>) -> Result<()> {
        match &*self.callee {
            Expression::Variable(variable) => {
                let f = compiler.functions.get(&variable.value);

                if f.is_none() {
                    bail!("compiler: function '{}' is not defined", variable.value);
                }

                let f = f.unwrap();

                if f.paramcount != self.arguments.len() {
                    bail!(
                        "compiler: function '{}' takes {} arguments",
                        f.name,
                        f.paramcount
                    );
                }

                let addr = f.location;

                for argument in &self.arguments {
                    argument.codegen(compiler)?;
                }

                compiler.emit_opcodes(&[Opcode::Call(self.arguments.len())]);

                compiler.emit_opcodes(&[Opcode::Jmp(addr)]);
            }
            Expression::Get(getexpr) => {
                getexpr.expr.codegen(compiler)?;

                if getexpr.op == Token::Arrow {
                    compiler.emit_opcodes(&[Opcode::Deref]);
                }

                for argument in &self.arguments {
                    argument.codegen(compiler)?;
                }

                let method_name_idx = compiler.add_string(getexpr.member);

                compiler.emit_opcodes(&[Opcode::CallMethod]);
                compiler.emit_u32(method_name_idx as u32);
                compiler.emit_u32(self.arguments.len() as u32);
            }
            _ => unreachable!(),
        }
        Ok(())
    }
}

impl<'src> Codegen<'src> for AssignExpression<'src> {
    fn codegen(&self, compiler: &mut Compiler<'src>) -> Result<()> {
        let is_specialized = self.op != Token::Equal;
        match &*self.lhs {
            Expression::Variable(variable) => {
                compiler.compile_variable_assignment(
                    self.clone(),
                    variable.clone(),
                    is_specialized,
                    self.op,
                )?;
            }

            Expression::Unary(unary) => {
                compiler.compile_unary_assignment(
                    unary.clone(),
                    (*self.rhs).clone(),
                    is_specialized,
                    self.op,
                )?;
            }

            Expression::Get(getexp) => {
                compiler.compile_get_assignment(
                    getexp.clone(),
                    (*self.rhs).clone(),
                    is_specialized,
                    self.op,
                )?;
            }

            Expression::Sub(subexp) => {
                compiler.compile_subscript_assignment(
                    subexp.to_owned(),
                    self.clone(),
                    is_specialized,
                    self.op,
                )?;
            }

            _ => bail!("compiler: invalid assignment"),
        };

        Ok(())
    }
}

impl<'src> Codegen<'src> for LogicalExpression<'src> {
    fn codegen(&self, compiler: &mut Compiler<'src>) -> Result<()> {
        self.lhs.codegen(compiler)?;

        match self.op {
            Token::DoubleAmpersand => {
                let jz_idx = compiler.emit_opcodes(&[Opcode::Jz(0xFFFFFFFF)]);

                self.rhs.codegen(compiler)?;

                let jmp_idx = compiler.emit_opcodes(&[Opcode::Jmp(0xFFFFFFFF)]);

                compiler.patch_jmp(jz_idx);
                compiler.emit_opcodes(&[Opcode::False]);
                compiler.patch_jmp(jmp_idx);
            }

            Token::DoublePipe => {
                let jz_idx = compiler.emit_opcodes(&[Opcode::Jz(0xFFFFFFFF)]);

                compiler.emit_opcodes(&[Opcode::False, Opcode::Not]);

                let jmp_idx = compiler.emit_opcodes(&[Opcode::Jmp(0xFFFFFFFF)]);

                compiler.patch_jmp(jz_idx);

                self.rhs.codegen(compiler)?;

                compiler.patch_jmp(jmp_idx);
            }
            _ => unreachable!(),
        }

        Ok(())
    }
}

impl<'src> Codegen<'src> for UnaryExpression<'src> {
    fn codegen(&self, compiler: &mut Compiler<'src>) -> Result<()> {
        match self.op {
            Token::Minus => {
                self.expr.codegen(compiler)?;
                compiler.emit_opcodes(&[Opcode::Neg]);
            }

            Token::Bang => {
                self.expr.codegen(compiler)?;
                compiler.emit_opcodes(&[Opcode::Not]);
            }

            Token::Ampersand => match &*self.expr {
                Expression::Variable(var) => {
                    let (idx, _) = compiler.resolve_local(var.value);
                    compiler.emit_opcodes(&[Opcode::DeepgetPtr(idx)]);
                }

                Expression::Get(getexp) => {
                    getexp.expr.codegen(compiler)?;

                    if getexp.op == Token::Arrow {
                        compiler.emit_opcodes(&[Opcode::Deref]);
                    }

                    compiler.emit_opcodes(&[Opcode::GetattrPtr(getexp.member.to_string().into())]);
                }

                _ => bail!("compiler: expected variable"),
            },

            Token::Star => {
                self.expr.codegen(compiler)?;
                compiler.emit_opcodes(&[Opcode::Deref]);
            }

            Token::Tilde => {
                self.expr.codegen(compiler)?;
                compiler.emit_opcodes(&[Opcode::BitNot]);
            }

            _ => unreachable!(),
        }

        Ok(())
    }
}

impl<'src> Codegen<'src> for GetExpression<'src> {
    fn codegen(&self, compiler: &mut Compiler<'src>) -> Result<()> {
        self.expr.codegen(compiler)?;

        if self.op == Token::Arrow {
            compiler.emit_opcodes(&[Opcode::Deref]);
        }

        compiler.emit_opcodes(&[Opcode::Getattr(self.member.to_string().into())]);

        Ok(())
    }
}

impl<'src> Codegen<'src> for StructExpression<'src> {
    fn codegen(&self, compiler: &mut Compiler<'src>) -> Result<()> {
        if let Some(s) = compiler.structs.get(self.name) {
            if s.members.len() != self.initializers.len() {
                bail!(
                    "compiler: struct '{}' has {} members",
                    self.name,
                    s.members.len()
                );
            }

            compiler.emit_opcodes(&[Opcode::Struct(self.name.to_string().into())]);

            for init in &self.initializers {
                init.codegen(compiler)?;
            }
        } else {
            bail!("compiler: struct '{}' is not defined", self.name);
        }

        Ok(())
    }
}

impl<'src> Codegen<'src> for StructInitializerExpression<'src> {
    fn codegen(&self, compiler: &mut Compiler<'src>) -> Result<()> {
        self.value.codegen(compiler)?;

        if let Expression::Variable(var) = &*self.member {
            compiler.emit_opcodes(&[Opcode::Setattr(var.value.to_string().into())]);
        } else {
            unreachable!();
        }

        Ok(())
    }
}

impl<'src> Codegen<'src> for VecExpression<'src> {
    fn codegen(&self, compiler: &mut Compiler<'src>) -> Result<()> {
        let mut elements = self.elements.clone();
        elements.reverse();
        for element in elements {
            element.codegen(compiler)?;
        }
        compiler.emit_opcodes(&[Opcode::Vec(self.elements.len())]);
        Ok(())
    }
}

impl<'src> Codegen<'src> for SubscriptExpression<'src> {
    fn codegen(&self, compiler: &mut Compiler<'src>) -> Result<()> {
        self.expr.codegen(compiler)?;
        self.index.codegen(compiler)?;

        compiler.emit_opcodes(&[Opcode::Subscript]);

        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Opcode {
    Print,
    Const(f64),
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    BitAnd,
    BitOr,
    BitXor,
    BitShl,
    BitShr,
    BitNot,
    False,
    Not,
    Neg,
    Null,
    Eq,
    Lt,
    Gt,
    Str(Rc<String>),
    Jmp(usize),
    Jz(usize),
    Call(usize),
    CallMethod,
    Ret,
    Deepget(usize),
    DeepgetPtr(usize),
    Deepset(usize),
    Deref,
    DerefSet,
    Getattr(Rc<String>),
    GetattrPtr(Rc<String>),
    Setattr(Rc<String>),
    Strcat,
    Struct(Rc<String>),
    StructBlueprint,
    Impl,
    Vec(usize),
    VecSet,
    Subscript,
    Pop(usize),
    Halt,
    Raw(u8),
}

trait Codegen<'src> {
    fn codegen(&self, _compiler: &mut Compiler<'src>) -> Result<()>;
}

#[derive(Debug, Clone, Default)]
pub struct Bytecode<'src> {
    pub code: Vec<Opcode>,
    pub cp: Vec<f64>,
    pub sp: Vec<&'src str>,
}

#[derive(Debug, Clone)]
pub struct Function<'src> {
    pub name: &'src str,
    pub location: usize,
    pub paramcount: usize,
    pub localscount: usize,
}

#[derive(Debug, Clone)]
pub struct Blueprint<'src> {
    pub name: &'src str,
    pub members: Vec<&'src str>,
    pub methods: HashMap<&'src str, Function<'src>>,
}
