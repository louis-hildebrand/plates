use rand::{Rng, rngs::ThreadRng};
use std::collections::HashMap;

use crate::{parser::Instruction};

pub struct Runtime {
    data_stack: Vec<Word>,
    function_table: HashMap<String, Vec<Instruction>>,
    rng: ThreadRng,
    instruction_stack: Vec<Instruction>,
}

#[derive(Clone)]
enum Word {
    Data(u32),
    Function(String),
}

impl Runtime {
    pub fn new() -> Self {
        Runtime {
            data_stack: Vec::new(),
            function_table: HashMap::new(),
            rng: rand::thread_rng(),
            instruction_stack: Vec::new(),
        }
    }

    /// Returns true iff the program should exit.
    pub fn execute(&mut self, instruction: Instruction) -> Result<bool, String> {
        match instruction {
            Instruction::Exit => {
                Ok(true)
            },
            Instruction::PushData(n) => {
                self.data_stack.push(Word::Data(n));
                Ok(false)
            },
            Instruction::PushFunction(f) => {
                self.data_stack.push(Word::Function(f));
                Ok(false)
            },
            Instruction::PushCopy => {
                match self.data_stack.last() {
                    None => {
                        Err(format!("Runtime error: cannot push copy when the stack is empty."))
                    },
                    Some(w) => {
                        self.data_stack.push(w.clone());
                        Ok(false)
                    }
                }
            },
            Instruction::PushRandom => {
                let n = self.rng.gen();
                self.data_stack.push(Word::Data(n));
                Ok(false)
            },
            Instruction::Define(f, body) => {
                self.function_table.insert(f, body);
                Ok(false)
            },
            Instruction::CallIf => {
                self.execute_callif()
            }
        }
    }

    fn execute_callif(&mut self) -> Result<bool, String> {
        match self.data_stack.pop() {
            None => {
                return Err(format!("Runtime error: cannot pop from empty stack."));
            },
            Some(Word::Function(f)) => {
                return Err(format!("Runtime error: Expected data, but received function '{}'.", f));
            },
            Some(Word::Data(0)) => {
                // TODO: What if there's nothing to pop?
                self.data_stack.pop();
                return Ok(false);
            },
            Some(Word::Data(_)) => {},
        };

        match self.data_stack.pop() {
            None => Err(format!("Runtime error: cannot pop from empty stack.")),
            Some(Word::Data(n)) => Err(format!("Runtime error: expected function but received data '{}'.", n)),
            Some(Word::Function(f)) => self.call_function(&f),
        }
    }

    fn call_function(&mut self, f: &str) -> Result<bool, String> {
        if f.starts_with("__") {
            return self.try_call_builtin(f);
        }

        let body = match self.function_table.get(f) {
            None => return Err(format!("Runtime error: function '{}' is not defined.", f)),
            Some(body) => body,
        };

        for instruction in body {
            self.instruction_stack.push(instruction.clone());
        }

        loop {
            match self.instruction_stack.pop() {
                None => return Ok(false),
                Some(instruction) => {
                    match self.execute(instruction) {
                        Err(msg) => return Err(msg),
                        Ok(true) => return Ok(true),
                        Ok(false) => {},
                    };
                },
            };
        }
    }

    pub fn try_call_builtin(&mut self, f: &str) -> Result<bool, String> {
        match f {
            "__print__" => self.call_print(),
            "__input__" => self.call_input(),
            "__swap__" => self.call_swap(),
            "__nand__" => self.call_nand(),
            _ => Err(format!("Runtime error: unrecognized built-in function '{}'.", f)),
        }
    }
    
    fn call_print(&mut self) -> Result<bool, String> {
        todo!();
    }

    fn call_input(&mut self) -> Result<bool, String> {
        todo!();
    }

    fn call_swap(&mut self) -> Result<bool, String> {
        todo!();
    }

    fn call_nand(&mut self) -> Result<bool, String> {
        todo!();
    }
}
