use std::{fs::File, io::Read, process::exit, time::Instant};

mod compiler;
mod mem;
mod opcode;
mod vm;

fn timeit<F: Fn() -> T, T>(title: &'static str, f: F) -> T {
    let time = Instant::now();
    let result = f();
    println!("{title} took {} ms", time.elapsed().as_millis());
    result
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        eprintln!("usage: bf-jit <program>");
        exit(1);
    }

    let mut file = File::open(args[1].clone()).expect("Requires a valid file");
    let mut buffer = String::new();
    file.read_to_string(&mut buffer).unwrap();

    let compiled = timeit("compilation", || compiler::compile(&buffer));
    let func = timeit("native code generation", || {
        let native_code = compiler::compile_to_native(&compiled).unwrap();
        mem::write_function(native_code)
    });

    // timeit("vm execution", || vm::execute(&compiled));
    timeit("native code execution", || {
        let mut stack = vec![0u8; 512];
        func(stack.as_mut_ptr());
    });
}
