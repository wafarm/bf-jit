#[derive(Debug, PartialEq)]
pub enum OpCode {
    AlterValue(i16),
    AlterPointer(i16),
    JumpForward(usize),
    JumpBackward(usize),
    AddMul(i16, u8),
    SetZero,
    Nop,
    PutChar,
}
