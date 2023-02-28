use rand::{rngs::ThreadRng, Rng};
use std::{collections::HashMap, fmt::Display, io::Write};

use crate::parser::Instruction;

#[derive(Clone)]
enum Word {
    Data(u32),
    Function(String),
}

impl Display for Word {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Word::Data(n) => write!(formatter, "{n}"),
            Word::Function(f) => write!(formatter, "function {f}"),
        }
    }
}

pub struct Runtime {
    value_stack: Vec<Word>,
    function_table: HashMap<String, Vec<Instruction>>,
    rng: ThreadRng,
    instruction_stack: Vec<Instruction>,
}

impl Runtime {
    pub fn new() -> Self {
        Runtime {
            value_stack: Vec::new(),
            function_table: HashMap::new(),
            rng: rand::thread_rng(),
            instruction_stack: Vec::new(),
        }
    }

    pub fn show_stack(&mut self) {
        let words = self
            .value_stack
            .iter()
            .map(|w| w.to_string())
            .collect::<Vec<_>>();
        println!("[{}]  <-- top", words.join(", "))
    }

    /// Returns true iff the program should exit.
    pub fn run(&mut self, instruction: Instruction) -> Result<bool, String> {
        match instruction {
            Instruction::Exit => Ok(true),
            Instruction::PushData(n) => {
                self.value_stack.push(Word::Data(n));
                Ok(false)
            }
            Instruction::PushFunction(f) => {
                self.value_stack.push(Word::Function(f));
                Ok(false)
            }
            Instruction::PushCopy => match self.value_stack.last() {
                None => Err(format!(
                    "Runtime error: cannot push copy when the stack is empty."
                )),
                Some(w) => {
                    self.value_stack.push(w.clone());
                    Ok(false)
                }
            },
            Instruction::PushRandom => {
                let n = self.rng.gen();
                self.value_stack.push(Word::Data(n));
                Ok(false)
            }
            Instruction::Define(f, body) => {
                self.function_table.insert(f, body);
                Ok(false)
            }
            Instruction::CallIf => self.run_callif(),
        }
    }

    fn run_callif(&mut self) -> Result<bool, String> {
        let top_data = match self.pop_data_from_stack() {
            Err(msg) => return Err(msg),
            Ok(n) => n,
        };

        if top_data == 0 {
            // TODO: What if there's nothing to pop?
            // In general, be more clear about the state of the stack after a runtime error.
            self.value_stack.pop();
            return Ok(false);
        }

        match self.pop_function_from_stack() {
            Err(msg) => Err(msg),
            Ok(f) => self.call_function(&f),
        }
    }

    fn call_function(&mut self, f: &str) -> Result<bool, String> {
        if f.starts_with("__") {
            return self.call_builtin(f);
        }

        let body = match self.function_table.get(f) {
            None => return Err(format!("Runtime error: function '{}' is not defined.", f)),
            Some(body) => body,
        };

        for instruction in body.iter().rev() {
            self.instruction_stack.push(instruction.clone());
        }

        loop {
            match self.instruction_stack.pop() {
                None => return Ok(false),
                Some(instruction) => {
                    match self.run(instruction) {
                        Err(msg) => return Err(msg),
                        Ok(true) => return Ok(true),
                        Ok(false) => {}
                    };
                }
            };
        }
    }

    pub fn call_builtin(&mut self, f: &str) -> Result<bool, String> {
        match f {
            "__print__" => self.call_print(),
            "__input__" => self.call_input(),
            "__swap__" => self.call_swap(),
            "__nand__" => self.call_nand(),
            _ => Err(format!(
                "Runtime error: unrecognized built-in function '{}'.",
                f
            )),
        }
    }

    fn call_print(&mut self) -> Result<bool, String> {
        loop {
            let n = match self.pop_data_from_stack() {
                Err(msg) => return Err(msg),
                Ok(0) => {
                    std::io::stdout().flush().expect("Failed to flush stdout.");
                    return Ok(false);
                }
                Ok(n) => n,
            };

            let c = match char::from_u32(n) {
                None => return Err(format!("Runtime error: {n} is not a valid code point.")),
                Some(c) => c,
            };

            print!("{c}");
        }
    }

    fn call_input(&mut self) -> Result<bool, String> {
        let mut line = String::new();
        match std::io::stdin().read_line(&mut line) {
            Err(_) => return Err(String::from("Runtime error: failed to read from stdin.")),
            Ok(_) => {}
        };

        for c in line.chars().rev() {
            let n = c as u32;
            self.value_stack.push(Word::Data(n));
        }

        Ok(false)
    }

    fn call_swap(&mut self) -> Result<bool, String> {
        let i = match self.pop_data_from_stack() {
            Err(msg) => return Err(msg),
            Ok(n) => n,
        };

        let i = match i.try_into() {
            Ok(i) => i,
            Err(_) => return Err(format!("Runtime error: {i} is not a valid index.")),
        };

        let top_index = self.value_stack.len() - 1;
        if top_index < i {
            return Err(format!(
                "Runtime error: cannot swap to index {} in stack of size {}.",
                i,
                top_index + 1
            ));
        }

        self.value_stack.swap(top_index, top_index - i);
        Ok(false)
    }

    fn call_nand(&mut self) -> Result<bool, String> {
        // Use !(a & b)
        let a = match self.pop_data_from_stack() {
            Err(msg) => return Err(msg),
            Ok(n) => n,
        };
        let b = match self.pop_data_from_stack() {
            Err(msg) => return Err(msg),
            Ok(n) => n,
        };

        let result = !(a & b);
        self.value_stack.push(Word::Data(result));
        return Ok(false);
    }

    fn pop_data_from_stack(&mut self) -> Result<u32, String> {
        match self.value_stack.pop() {
            None => Err(format!("Runtime error: cannot pop from empty stack.")),
            Some(Word::Function(f)) => Err(format!(
                "Runtime error: expected data but received function '{}'.",
                f
            )),
            Some(Word::Data(n)) => Ok(n),
        }
    }

    fn pop_function_from_stack(&mut self) -> Result<String, String> {
        match self.value_stack.pop() {
            None => Err(format!("Runtime error: cannot pop from empty stack.")),
            Some(Word::Data(n)) => Err(format!(
                "Runtime error: expected function but received data '{}'.",
                n
            )),
            Some(Word::Function(f)) => Ok(f),
        }
    }
}
