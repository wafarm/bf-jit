use iced_x86::{code_asm::*, IcedError};

use crate::opcode::OpCode;

fn is_jump_backward(op: &OpCode) -> bool {
    if let OpCode::JumpBackward(_) = op {
        true
    } else {
        false
    }
}

pub fn compile(source: &String) -> Vec<OpCode> {
    let mut compiled = Vec::new();

    // step 1: remove extra characters
    let mut minified = String::new();
    for c in source.chars() {
        match c {
            '+' | '-' | '>' | '<' | '[' | ']' | '.' => {
                minified.push(c);
            }
            _ => (),
        }
    }

    // step 2: generate byte codes
    let mut iter = minified.chars().peekable();
    let mut index = 0;
    let mut stack = Vec::new();
    while iter.peek().is_some() {
        let c = iter.next().unwrap();
        match c {
            '+' | '-' => {
                let mut v = if c == '+' { 1 } else { -1 };

                loop {
                    let peeked = iter.peek();
                    if let Some('+') = peeked {
                        v += 1;
                        iter.next();
                    } else if let Some('-') = peeked {
                        v -= 1;
                        iter.next();
                    } else {
                        break;
                    }
                }

                if v == 0 {
                    continue;
                }

                compiled.push(OpCode::AlterValue(v));
            }
            '>' | '<' => {
                let mut v = if c == '>' { 1 } else { -1 };

                loop {
                    let peeked = iter.peek();
                    if let Some('>') = peeked {
                        v += 1;
                        iter.next();
                    } else if let Some('<') = peeked {
                        v -= 1;
                        iter.next();
                    } else {
                        break;
                    }
                }

                if v == 0 {
                    continue;
                }

                compiled.push(OpCode::AlterPointer(v));
            }
            '.' => {
                compiled.push(OpCode::PutChar);
            }
            '[' => {
                stack.push(index);
                compiled.push(OpCode::JumpForward(0));
            }
            ']' => {
                let to = stack.pop().unwrap();
                compiled.push(OpCode::JumpBackward(to));
                compiled[to] = OpCode::JumpForward(index);
            }
            _ => (),
        }

        index += 1;
    }

    if stack.len() != 0 {
        panic!("Unmatched '['");
    }

    // step 3: optimize code
    let mut index = 0;
    while index != compiled.len() {
        match compiled[index] {
            OpCode::JumpForward(_) => 'label: {
                // case 1: [-] is equivalent to setting cell to 0
                if compiled.len() > index + 2 && is_jump_backward(&compiled[index + 2]) {
                    if compiled[index + 1] == OpCode::AlterValue(-1) {
                        compiled[index] = OpCode::SetZero;
                        compiled[index + 1] = OpCode::Nop;
                        compiled[index + 2] = OpCode::Nop;
                        break 'label;
                    }
                }

                // case 2: [->(n)+(k)<(n)] is *(pointer + n) += *pointer * k; *pointer = 0
                if compiled.len() > index + 5 && is_jump_backward(&compiled[index + 5]) {
                    if compiled[index + 1] == OpCode::AlterValue(-1)
                        && compiled[index + 3] == OpCode::AlterValue(1)
                    {
                        if let OpCode::AlterPointer(n) = compiled[index + 2] {
                            if OpCode::AlterPointer(-n) == compiled[index + 4] {
                                compiled[index] = OpCode::AddMul(n, 1);
                                compiled[index + 1] = OpCode::SetZero;
                                for i in 2..=5 {
                                    compiled[index + i] = OpCode::Nop;
                                }
                                break 'label;
                            }
                        }
                    }
                }

                // case 3: [>(n)+(k)<(n)-] is *(pointer + n) += *pointer * k; *pointer = 0
                if compiled.len() > index + 5 && is_jump_backward(&compiled[index + 5]) {
                    if let OpCode::AlterPointer(n) = compiled[index + 1] {
                        if OpCode::AlterPointer(-n) == compiled[index + 3] {
                            if let OpCode::AlterValue(k) = compiled[index + 2] {
                                if OpCode::AlterValue(-1) == compiled[index + 4] {
                                    compiled[index] = OpCode::AddMul(n, k as u8);
                                    compiled[index + 1] = OpCode::SetZero;
                                    for i in 2..=5 {
                                        compiled[index + i] = OpCode::Nop;
                                    }
                                    break 'label;
                                }
                            }
                        }
                    }
                }
            }
            _ => (),
        }
        index += 1;
    }

    compiled
}

#[allow(dead_code)]
pub fn compile_to_native(program: &Vec<OpCode>) -> Result<Vec<u8>, IcedError> {
    let mut a = CodeAssembler::new(64)?;

    // rbx will be our stack pointer
    let stack_pointer = rbx;

    // save rbx first
    a.push(rbx)?;
    if cfg!(unix) {
        a.mov(stack_pointer, rdi)?;
    } else if cfg!(windows) {
        a.mov(stack_pointer, rcx)?;
    } else {
        panic!("Unsupported platform");
    }

    let mut jump_stack = Vec::new();

    let mut index = 0;
    while index != program.len() {
        let op = &program[index];
        match *op {
            OpCode::AlterValue(v) => {
                if v == 1 {
                    a.inc(byte_ptr(stack_pointer))?;
                } else if v == -1 {
                    a.dec(byte_ptr(stack_pointer))?;
                } else if v > 0 {
                    a.add(byte_ptr(stack_pointer), v as i32)?;
                } else if v < 0 {
                    a.sub(byte_ptr(stack_pointer), (-v) as i32)?;
                } else {
                    // Should not reach here
                    panic!("Unreachable code");
                }
            }
            OpCode::AlterPointer(v) => {
                if v == 1 {
                    a.inc(stack_pointer)?;
                } else if v == -1 {
                    a.dec(stack_pointer)?;
                } else if v > 0 {
                    a.add(stack_pointer, v as i32)?;
                } else if v < 0 {
                    a.sub(stack_pointer, (-v) as i32)?;
                } else {
                    // Should not reach here
                    panic!("Unreachable code");
                }
            }
            OpCode::JumpForward(_) => {
                let mut backword_label = a.create_label();
                let forward_label = a.create_label();

                a.cmp(byte_ptr(stack_pointer), 0)?;
                a.je(forward_label)?;
                a.set_label(&mut backword_label)?;

                jump_stack.push(backword_label);
                jump_stack.push(forward_label);
            }
            OpCode::JumpBackward(_) => {
                let mut forward_label = jump_stack.pop().unwrap();
                let backword_label = jump_stack.pop().unwrap();

                a.cmp(byte_ptr(stack_pointer), 0)?;
                a.jne(backword_label)?;
                a.set_label(&mut forward_label)?;
            }
            OpCode::AddMul(target, n) => {
                if n == 1 {
                    a.mov(eax, byte_ptr(stack_pointer))?;
                } else {
                    a.mov(eax, n as u32)?;
                    a.mul(byte_ptr(stack_pointer))?;
                }
                a.add(byte_ptr(stack_pointer + target), al)?;
            },
            OpCode::SetZero => {
                a.mov(byte_ptr(stack_pointer), 0)?;
            }
            OpCode::Nop => (),
            OpCode::PutChar => {
                if cfg!(unix) {
                    a.movzx(edi, byte_ptr(stack_pointer))?;
                } else if cfg!(windows) {
                    a.movzx(ecx, byte_ptr(stack_pointer))?;
                } else {
                    panic!("Unsupported platform");
                }
                a.mov(rax, libc::putchar as u64)?;
                a.call(rax)?;
            }
        }

        index += 1;
    }

    // restore rbx since it's callee-saved
    a.pop(rbx)?;

    // return 0
    a.xor(eax, eax)?;
    a.ret()?;

    Ok(a.assemble(0).unwrap())
}
