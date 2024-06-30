use crate::{compiler::Opcode, vm::VM};

pub fn disassemble<'src, 'bytecode>(vm: &mut VM<'src, 'bytecode>)
where
    'bytecode: 'src,
{
    vm.ip = vm.bytecode.code.as_mut_ptr();
    while vm.ip < unsafe { vm.bytecode.code.as_mut_ptr().add(vm.bytecode.code.len()) } {
        let opcode = Opcode::from(unsafe { *vm.ip });
        match opcode {
            Opcode::Const => {
                let n = vm.read_f64();
                println!("{:?} (const: {})", opcode, n);
            }
            Opcode::Str => {
                let idx = vm.read_u32();
                let s = vm.bytecode.sp[idx as usize];
                println!("{:?} (str: {})", opcode, s);
            }
            Opcode::Jmp | Opcode::Jz => {
                let addr = vm.read_u32();
                println!("{:?} (addr: {})", opcode, addr);
            }
            Opcode::Call => {
                let argcount = vm.read_u32();
                println!("{:?} (argcount: {})", opcode, argcount);
            }
            Opcode::CallMethod => {
                let method_name_idx = vm.read_u32();
                let argcount = vm.read_u32();
                let method_name = vm.bytecode.sp[method_name_idx as usize];
                println!(
                    "{:?} (method: {}, argcount: {})",
                    opcode, method_name, argcount
                );
            }
            Opcode::Deepget | Opcode::DeepgetPtr | Opcode::Deepset => {
                let idx = vm.read_u32();
                println!("{:?} (idx: {})", opcode, idx);
            }
            Opcode::Getattr | Opcode::GetattrPtr | Opcode::Setattr => {
                let idx = vm.read_u32();
                let attr = vm.bytecode.sp[idx as usize];
                println!("{:?} (attr: {})", opcode, attr);
            }
            Opcode::Struct => {
                let name_idx = vm.read_u32();
                let name = vm.bytecode.sp[name_idx as usize];
                println!("{:?} (struct: {})", opcode, name);
            }
            Opcode::StructBlueprint | Opcode::Impl => {
                /* TODO */
                println!("{:?}", opcode);
            }
            Opcode::Vec => {
                let elemcount = vm.read_u32();
                println!("{:?} (elemcount: {})", opcode, elemcount);
            }
            Opcode::Pop => {
                let popcount = vm.read_u32();
                println!("{:?} (popcount: {})", opcode, popcount);
            }
            _ => {
                println!("{:?}", opcode);
            }
        }
        unsafe { vm.ip = vm.ip.add(1) };
    }
}
