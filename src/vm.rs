use crate::opcode::OpCode;

#[allow(dead_code)]
pub fn execute(code: &Vec<OpCode>) {
    let mut index = 0;
    let mut stack = vec![0u8; 512];
    let mut pointer = 0;
    let mut ops: u64 = 0;
    while index != code.len() {
        ops += 1;
        match code[index] {
            OpCode::AlterValue(v) => {
                stack[pointer] = (stack[pointer] as i32 + v as i32) as u8;
            }
            OpCode::AlterPointer(v) => {
                pointer = (pointer as isize + v as isize) as usize;
            }
            OpCode::JumpForward(to) => {
                if stack[pointer] == 0 {
                    index = to;
                    continue;
                }
            }
            OpCode::JumpBackward(to) => {
                if stack[pointer] != 0 {
                    index = to;
                    continue;
                }
            }
            OpCode::SetZero => {
                stack[pointer] = 0;
            }
            OpCode::AddMul(target, n) => {
                stack[(pointer as isize + target as isize) as usize] += n * stack[pointer];
            }
            OpCode::PutChar => {
                print!("{}", stack[pointer] as char);
            }
            OpCode::Nop => (),
        }

        index += 1;
    }

    println!("Ops executed: {}", ops);
}
