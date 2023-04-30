use anyhow::{anyhow, Error};
use rand::{rngs::ThreadRng, Rng};
use std::{collections::HashMap, fmt::Display, io::Write};

use crate::parser::Instruction;

const ERR_UNDERFLOW: &str = "Runtime error: Stack underflow.";
const ERR_UNDEFINED: &str = "Runtime error: Undefined argument or function.";
const ERR_TYPE: &str = "Runtime error: Wrong type.";
const ERR_UTF32: &str = "Runtime error: Invalid UTF-32 code point.";
const ERR_STDOUT: &str = "Environment error: Failed to flush stdout.";
const ERR_STDIN: &str = "Environment error: Failed to read from stdin.";

#[derive(Clone, Debug, Eq, PartialEq)]
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

#[derive(Clone, Debug)]
pub struct Runtime {
    value_stack: Vec<Word>,
    function_table: HashMap<String, (u32, Vec<Instruction>)>,
    rng: ThreadRng,
    instruction_stack: Vec<Instruction>,
    args_array: Vec<Word>,
}

impl PartialEq for Runtime {
    fn eq(&self, other: &Self) -> bool {
        self.value_stack == other.value_stack
            && self.function_table == other.function_table
            && self.instruction_stack == other.instruction_stack
            && self.args_array == other.args_array
    }
}

impl Runtime {
    pub fn new() -> Self {
        Runtime {
            value_stack: Vec::new(),
            function_table: HashMap::new(),
            rng: rand::thread_rng(),
            instruction_stack: Vec::new(),
            args_array: Vec::new(),
        }
    }

    pub fn stack_to_string(&mut self) -> String {
        let words = self
            .value_stack
            .iter()
            .map(|w| w.to_string())
            .collect::<Vec<_>>();
        format!("[{}]  <-- top", words.join(", "))
    }

    /// Returns true iff the program should exit.
    pub fn run(&mut self, instruction: Instruction) -> Result<bool, Error> {
        self.instruction_stack.push(instruction);

        loop {
            match self.instruction_stack.pop() {
                None => return Ok(false),
                Some(instruction) => match self.run_instruction(instruction) {
                    Err(e) => {
                        self.instruction_stack.clear();
                        return Err(e);
                    }
                    Ok(true) => return Ok(true),
                    Ok(false) => continue,
                },
            };
        }
    }

    fn run_instruction(&mut self, instruction: Instruction) -> Result<bool, Error> {
        match instruction {
            Instruction::Exit => Ok(true),
            Instruction::PushData(n) => self.run_pushdata(n),
            Instruction::PushFunction(f) => self.run_pushfunction(f),
            Instruction::PushRandom => self.run_pushrandom(),
            Instruction::PushArg(n) => self.run_pusharg(n),
            Instruction::Define(f, arg_count, body) => self.run_define(f, arg_count, body),
            Instruction::CallIf => self.run_callif(),
        }
    }

    fn run_pushdata(&mut self, n: u32) -> Result<bool, Error> {
        self.value_stack.push(Word::Data(n));
        Ok(false)
    }

    fn run_pushfunction(&mut self, f: String) -> Result<bool, Error> {
        self.value_stack.push(Word::Function(f));
        Ok(false)
    }

    fn run_pushrandom(&mut self) -> Result<bool, Error> {
        let n = self.rng.gen();
        self.value_stack.push(Word::Data(n));
        Ok(false)
    }

    fn run_pusharg(&mut self, n: usize) -> Result<bool, Error> {
        let value = match self.args_array.get(n) {
            None => return Err(anyhow!(ERR_UNDEFINED)),
            Some(x) => x.clone(),
        };
        self.value_stack.push(value);
        Ok(false)
    }

    fn run_define(
        &mut self,
        f: String,
        arg_count: u32,
        body: Vec<Instruction>,
    ) -> Result<bool, Error> {
        self.function_table.insert(f, (arg_count, body));
        Ok(false)
    }

    fn run_callif(&mut self) -> Result<bool, Error> {
        let f = self.pop_function_from_stack()?;

        let top_data = self.pop_data_from_stack()?;

        if top_data == 0 {
            Ok(false)
        } else {
            self.call_function(&f)
        }
    }

    /// This does not run the function in its entirety, it just pushes the body of the function onto the stack.
    fn call_function(&mut self, f: &str) -> Result<bool, Error> {
        // Clear the args array so that args from a previous function call don't leak to a subsequent function call.
        // Clearing the array before each call should be enough to guarantee this since it is a syntax error to use
        // arguments outside a function.
        self.args_array.clear();

        if f.starts_with("__") {
            self.call_builtin_function(f)
        } else {
            self.call_custom_function(f)
        }
    }

    fn call_builtin_function(&mut self, f: &str) -> Result<bool, Error> {
        match f {
            "__print__" => self.call_print(),
            "__input__" => self.call_input(),
            "__nand__" => self.call_nand(),
            // TODO: Replace left and right shift with rotate right
            "__shift_left__" => self.call_shift_left(),
            "__shift_right__" => self.call_shift_right(),
            _ => Err(anyhow!(ERR_UNDEFINED)),
        }
    }

    fn call_custom_function(&mut self, f: &str) -> Result<bool, Error> {
        let (arg_count, body) = match self.function_table.get(f) {
            None => return Err(anyhow!(ERR_UNDEFINED)),
            Some(body) => body,
        };

        for _ in 0..*arg_count {
            let n = match self.value_stack.pop() {
                None => return Err(anyhow!(ERR_UNDERFLOW)),
                Some(x) => x,
            };

            self.args_array.push(n);
        }

        for instruction in body.iter().rev() {
            self.instruction_stack.push(instruction.clone());
        }

        Ok(false)
    }

    fn call_print(&mut self) -> Result<bool, Error> {
        loop {
            let n = self.pop_data_from_stack()?;

            if n == 0 {
                if std::io::stdout().flush().is_err() {
                    return Err(anyhow!(ERR_STDOUT));
                }
                return Ok(false);
            }

            let c = match char::from_u32(n) {
                None => return Err(anyhow!(ERR_UTF32)),
                Some(c) => c,
            };

            print!("{c}");
        }
    }

    fn call_input(&mut self) -> Result<bool, Error> {
        let mut line = String::new();
        if std::io::stdin().read_line(&mut line).is_err() {
            return Err(anyhow!(ERR_STDIN));
        }

        for c in line.chars().rev() {
            let n = c as u32;
            self.value_stack.push(Word::Data(n));
        }

        Ok(false)
    }

    fn call_nand(&mut self) -> Result<bool, Error> {
        // Use !(a & b)
        let a = self.pop_data_from_stack()?;
        let b = self.pop_data_from_stack()?;

        let result = !(a & b);
        self.value_stack.push(Word::Data(result));

        Ok(false)
    }

    fn call_shift_left(&mut self) -> Result<bool, Error> {
        let n = self.pop_data_from_stack()?;

        let result = n << 1;
        self.value_stack.push(Word::Data(result));

        Ok(false)
    }

    fn call_shift_right(&mut self) -> Result<bool, Error> {
        let n = self.pop_data_from_stack()?;

        let result = n >> 1;
        self.value_stack.push(Word::Data(result));

        Ok(false)
    }

    fn pop_data_from_stack(&mut self) -> Result<u32, Error> {
        match self.value_stack.pop() {
            None => Err(anyhow!(ERR_UNDERFLOW)),
            Some(Word::Function(_)) => Err(anyhow!(ERR_TYPE)),
            Some(Word::Data(n)) => Ok(n),
        }
    }

    fn pop_function_from_stack(&mut self) -> Result<String, Error> {
        match self.value_stack.pop() {
            None => Err(anyhow!(ERR_UNDERFLOW)),
            Some(Word::Data(_)) => Err(anyhow!(ERR_TYPE)),
            Some(Word::Function(f)) => Ok(f),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    macro_rules! assert_ok_and_eq {
        ( $actual:expr, $expected:expr ) => {
            let actual_val = $actual;
            assert!(actual_val.is_ok());
            let actual_val = actual_val.unwrap();
            assert_eq!($expected, actual_val);
        };
    }

    macro_rules! assert_err_with_msg {
        ( $value:expr, $msg:expr ) => {
            match $value {
                Ok(x) => panic!("Expected an error but received 'Ok({x:?})'."),
                Err(e) => assert_eq!($msg, format!("{e}")),
            };
        };
    }

    #[test]
    fn new_runtime() {
        let expected = Runtime {
            value_stack: vec![],
            function_table: HashMap::new(),
            rng: rand::thread_rng(),
            instruction_stack: vec![],
            args_array: vec![],
        };
        assert_eq!(expected, Runtime::new());
    }

    #[test]
    fn push_data() {
        let mut actual = Runtime::new();
        let mut expected = Runtime::new();

        // Empty stack
        assert_ok_and_eq!(actual.run(Instruction::PushData(123)), false);
        expected.value_stack.push(Word::Data(123));
        assert_eq!(expected, actual);

        // Non-empty stack
        assert_ok_and_eq!(actual.run(Instruction::PushData(456)), false);
        expected.value_stack.push(Word::Data(456));
        assert_eq!(expected, actual);
    }

    #[test]
    fn push_undefined_function() {
        let mut runtime = Runtime {
            value_stack: vec![Word::Function("foo".to_owned())],
            ..Runtime::new()
        };
        let instruction = Instruction::PushFunction("bar".to_owned());
        let after = Runtime {
            value_stack: vec![
                Word::Function("foo".to_owned()),
                Word::Function("bar".to_owned()),
            ],
            ..Runtime::new()
        };

        assert_ok_and_eq!(runtime.run(instruction), false);
        assert_eq!(after, runtime);
    }

    #[test]
    fn push_defined_function() {
        let mut runtime = Runtime {
            function_table: HashMap::from([("foo".to_owned(), (0, vec![]))]),
            ..Runtime::new()
        };
        let instruction = Instruction::PushFunction("foo".to_owned());
        let after = Runtime {
            value_stack: vec![Word::Function("foo".to_owned())],
            function_table: HashMap::from([("foo".to_owned(), (0, vec![]))]),
            ..Runtime::new()
        };

        assert_ok_and_eq!(runtime.run(instruction), false);
        assert_eq!(after, runtime);
    }

    #[test]
    fn push_random() {
        let mut runtime = Runtime {
            value_stack: vec![Word::Function("foo".to_owned()), Word::Data(123)],
            ..Runtime::new()
        };
        let instruction = Instruction::PushRandom;
        let after = Runtime::new();

        assert_ok_and_eq!(runtime.run(instruction), false);

        // Compare runtime without value stack
        let updated_value_stack = runtime.value_stack;
        runtime.value_stack = after.value_stack.clone();
        assert_eq!(runtime, after);

        // Check value stack: right length, first few values untouched, new value is any data word
        assert_eq!(updated_value_stack.len(), 3);
        assert_eq!(
            vec![Word::Function("foo".to_owned()), Word::Data(123)],
            updated_value_stack[..2]
        );
        assert!(matches!(updated_value_stack[2], Word::Data(_)));
    }

    #[test]
    fn push_valid_args() {
        let mut runtime = Runtime {
            value_stack: vec![Word::Data(123), Word::Function("foo".to_owned())],
            function_table: HashMap::from([(
                "swap".to_owned(),
                (1, vec![Instruction::PushArg(0), Instruction::PushArg(1)]),
            )]),
            args_array: vec![Word::Data(0), Word::Data(1)],
            ..Runtime::new()
        };
        let mut after = runtime.clone();

        assert_ok_and_eq!(runtime.run(Instruction::PushArg(0)), false);
        after.value_stack.push(Word::Data(0));
        assert_eq!(after, runtime);

        assert_ok_and_eq!(runtime.run(Instruction::PushArg(1)), false);
        after.value_stack.push(Word::Data(1));
        assert_eq!(after, runtime);
    }

    #[test]
    fn push_invalid_args() {
        let mut runtime = Runtime {
            value_stack: vec![Word::Data(123), Word::Function("foo".to_owned())],
            function_table: HashMap::from([(
                "swap".to_owned(),
                (1, vec![Instruction::PushArg(0), Instruction::PushArg(1)]),
            )]),
            args_array: vec![Word::Data(0), Word::Data(1)],
            ..Runtime::new()
        };
        let after = runtime.clone();

        assert_err_with_msg!(runtime.run(Instruction::PushArg(2)), ERR_UNDEFINED);
        assert_eq!(after, runtime);
    }

    #[test]
    fn define() {
        let mut runtime = Runtime::new();
        let instruction = Instruction::Define(
            "foo".to_owned(),
            2,
            vec![Instruction::PushData(123), Instruction::PushData(456)],
        );
        let after = Runtime {
            function_table: HashMap::from([(
                "foo".to_owned(),
                (
                    2,
                    vec![Instruction::PushData(123), Instruction::PushData(456)],
                ),
            )]),
            ..Runtime::new()
        };

        assert_ok_and_eq!(runtime.run(instruction), false);
        assert_eq!(after, runtime);
    }

    #[test]
    fn callif_true() {
        let mut runtime = Runtime {
            value_stack: vec![
                Word::Function("bar".to_owned()),
                Word::Data(789),
                Word::Data(1),
                Word::Function("foo".to_owned()),
            ],
            function_table: HashMap::from([(
                "foo".to_owned(),
                (
                    0,
                    vec![Instruction::PushData(123), Instruction::PushData(456)],
                ),
            )]),
            ..Runtime::new()
        };
        let after = Runtime {
            value_stack: vec![
                Word::Function("bar".to_owned()),
                Word::Data(789),
                Word::Data(123),
                Word::Data(456),
            ],
            ..runtime.clone()
        };

        assert_ok_and_eq!(runtime.run(Instruction::CallIf), false);
        assert_eq!(after, runtime);
    }

    #[test]
    fn callif_false() {
        let mut runtime = Runtime {
            value_stack: vec![Word::Data(0), Word::Function("foo".to_owned())],
            function_table: HashMap::from([(
                "foo".to_owned(),
                (
                    1,
                    vec![Instruction::PushData(123), Instruction::PushData(456)],
                ),
            )]),
            ..Runtime::new()
        };
        let after = Runtime {
            value_stack: vec![],
            ..runtime.clone()
        };

        assert_ok_and_eq!(runtime.run(Instruction::CallIf), false);
        assert_eq!(after, runtime);
    }

    #[test]
    fn callif_undefined() {
        let mut runtime = Runtime {
            value_stack: vec![
                Word::Data(1),
                Word::Function("undefined".to_owned()),
                Word::Data(0),
                Word::Function("undefined".to_owned()),
            ],
            ..Runtime::new()
        };
        let mut expected = runtime.clone();

        // No problem if the function isn't called
        assert_ok_and_eq!(runtime.run(Instruction::CallIf), false);
        expected.value_stack.remove(3);
        expected.value_stack.remove(2);
        assert_eq!(expected, runtime);

        // Error if the function is called
        assert_err_with_msg!(runtime.run(Instruction::CallIf), ERR_UNDEFINED);
        expected.value_stack.remove(1);
        expected.value_stack.remove(0);
        assert_eq!(expected, runtime);
    }

    #[test]
    fn nested_functions_success() {
        let mut runtime = Runtime {
            value_stack: vec![
                Word::Data(3),
                Word::Data(1),
                Word::Function("foo".to_owned()),
            ],
            function_table: HashMap::from([
                (
                    "foo".to_owned(),
                    (
                        1,
                        vec![
                            Instruction::PushArg(0),
                            Instruction::PushData(1),
                            Instruction::PushFunction("bar".to_owned()),
                            Instruction::CallIf,
                            Instruction::PushData(123),
                        ],
                    ),
                ),
                (
                    "bar".to_owned(),
                    (
                        0,
                        vec![
                            Instruction::PushData(1),
                            Instruction::PushFunction("__shift_left__".to_owned()),
                            Instruction::CallIf,
                        ],
                    ),
                ),
            ]),
            ..Runtime::new()
        };
        let after = Runtime {
            value_stack: vec![Word::Data(6), Word::Data(123)],
            ..runtime.clone()
        };

        let res = runtime.run(Instruction::CallIf);
        println!("{res:?}");
        assert_ok_and_eq!(res, false);
        assert_eq!(after, runtime);
    }

    #[test]
    fn undefined_arg_after_nested_custom_function() {
        let mut runtime = Runtime {
            value_stack: vec![
                Word::Data(3),
                Word::Data(1),
                Word::Function("bad".to_owned()),
            ],
            function_table: HashMap::from([
                (
                    "bad".to_owned(),
                    (
                        1,
                        vec![
                            Instruction::PushArg(0),
                            Instruction::PushData(1),
                            Instruction::PushFunction("empty".to_owned()),
                            Instruction::CallIf,
                            Instruction::PushArg(0),
                        ],
                    ),
                ),
                ("empty".to_owned(), (0, vec![])),
            ]),
            ..Runtime::new()
        };
        let after = Runtime {
            value_stack: vec![Word::Data(3)],
            ..runtime.clone()
        };

        assert_err_with_msg!(runtime.run(Instruction::CallIf), ERR_UNDEFINED);
        assert_eq!(after, runtime);
    }

    #[test]
    fn undefined_arg_after_nested_builtin_function() {
        let mut runtime = Runtime {
            value_stack: vec![
                Word::Data(3),
                Word::Data(1),
                Word::Function("bad".to_owned()),
            ],
            function_table: HashMap::from([(
                "bad".to_owned(),
                (
                    1,
                    vec![
                        Instruction::PushArg(0),
                        Instruction::PushData(1),
                        Instruction::PushFunction("__shift_left__".to_owned()),
                        Instruction::CallIf,
                        Instruction::PushArg(0),
                    ],
                ),
            )]),
            ..Runtime::new()
        };
        let after = Runtime {
            value_stack: vec![Word::Data(6)],
            ..runtime.clone()
        };

        assert_err_with_msg!(runtime.run(Instruction::CallIf), ERR_UNDEFINED);
        assert_eq!(after, runtime);
    }

    #[test]
    fn undefined_arg_in_nested_function() {
        let mut runtime = Runtime {
            value_stack: vec![
                Word::Data(4),
                Word::Data(1),
                Word::Function("foo".to_owned()),
            ],
            function_table: HashMap::from([
                (
                    "foo".to_owned(),
                    (
                        1,
                        vec![
                            Instruction::PushData(1),
                            Instruction::PushFunction("bar".to_owned()),
                            Instruction::CallIf,
                        ],
                    ),
                ),
                ("bar".to_owned(), (0, vec![Instruction::PushArg(0)])),
            ]),
            ..Runtime::new()
        };
        let after = Runtime {
            value_stack: vec![],
            ..runtime.clone()
        };

        assert_err_with_msg!(runtime.run(Instruction::CallIf), ERR_UNDEFINED);
        // Ignore the args array: it doesn't matter whether or not it's cleared right away, as long as args don't leak
        // to the next function call (there should be a test for that, like `undefined_arg_after_error`).
        runtime.args_array = vec![];
        assert_eq!(after, runtime);
    }

    #[test]
    fn undefined_arg_in_successive_functions() {
        let mut runtime = Runtime {
            value_stack: vec![
                Word::Data(123),
                Word::Data(1),
                Word::Function("bar".to_owned()),
                Word::Data(456),
                Word::Data(1),
                Word::Function("foo".to_owned()),
            ],
            function_table: HashMap::from([
                ("foo".to_owned(), (1, vec![])),
                ("bar".to_owned(), (0, vec![Instruction::PushArg(0)])),
            ]),
            ..Runtime::new()
        };
        let after_foo = Runtime {
            value_stack: vec![
                Word::Data(123),
                Word::Data(1),
                Word::Function("bar".to_owned()),
            ],
            ..runtime.clone()
        };
        let after_bar = Runtime {
            value_stack: vec![Word::Data(123)],
            ..runtime.clone()
        };

        // First call (to foo): no problem
        assert_ok_and_eq!(runtime.run(Instruction::CallIf), false);
        // Ignore the args array: it doesn't matter whether or not it's cleared right away, as long as args don't leak
        // to the next function call (there should be a test for that, like `undefined_arg_after_error`).
        runtime.args_array = vec![];
        assert_eq!(after_foo, runtime);

        // Second call (to bar): args from foo should be cleared, so $1 is invalid
        assert_err_with_msg!(runtime.run(Instruction::CallIf), ERR_UNDEFINED);
        // Ignore the args array: it doesn't matter whether or not it's cleared right away, as long as args don't leak
        // to the next function call (there should be a test for that, like `undefined_arg_after_error`).
        runtime.args_array = vec![];
        assert_eq!(after_bar, runtime);
    }

    /// Checks that arguments don't leak from one function call to the next after an error.
    #[test]
    fn undefined_arg_after_error() {
        let mut runtime = Runtime {
            value_stack: vec![
                Word::Data(42),
                Word::Data(1),
                Word::Function("bar".to_owned()),
                Word::Data(123),
                Word::Data(1),
                Word::Function("foo".to_owned()),
            ],
            function_table: HashMap::from([
                ("foo".to_owned(), (1, vec![Instruction::PushArg(999)])),
                ("bar".to_owned(), (0, vec![Instruction::PushArg(0)])),
            ]),
            ..Runtime::new()
        };
        let after_foo = Runtime {
            value_stack: vec![
                Word::Data(42),
                Word::Data(1),
                Word::Function("bar".to_owned()),
            ],
            ..runtime.clone()
        };
        let after_bar = Runtime {
            value_stack: vec![Word::Data(42)],
            ..runtime.clone()
        };

        // foo should fail because it's accessing an an argument that doesn't exist
        assert_err_with_msg!(runtime.run(Instruction::CallIf), ERR_UNDEFINED);
        // Ignore the args array: it doesn't matter whether or not it's cleared right away, as long as args don't leak
        // to the next function call.
        runtime.args_array = vec![];
        assert_eq!(after_foo, runtime);

        // bar should fail because there's no argument 0 (even though there was an argument 0 in foo)
        assert_err_with_msg!(runtime.run(Instruction::CallIf), ERR_UNDEFINED);
        // Ignore the args array: it doesn't matter whether or not it's cleared right away, as long as args don't leak
        // to the next function call.
        runtime.args_array = vec![];
        assert_eq!(after_bar, runtime);
    }

    #[test]
    fn callif_exit() {
        let mut runtime = Runtime {
            value_stack: vec![Word::Data(1), Word::Function("exit".to_owned())],
            function_table: HashMap::from([(
                "exit".to_owned(),
                (
                    0,
                    vec![
                        Instruction::PushData(123),
                        Instruction::Exit,
                        Instruction::PushData(456),
                    ],
                ),
            )]),
            ..Runtime::new()
        };
        let after = Runtime {
            value_stack: vec![Word::Data(123)],
            instruction_stack: vec![Instruction::PushData(456)],
            ..runtime.clone()
        };

        assert_ok_and_eq!(runtime.run(Instruction::CallIf), true);
        assert_eq!(after, runtime);
    }

    #[test]
    fn callif_error() {
        let mut runtime = Runtime {
            value_stack: vec![Word::Data(1), Word::Function("evil".to_owned())],
            function_table: HashMap::from([(
                "evil".to_owned(),
                (
                    0,
                    vec![
                        Instruction::PushData(123),
                        Instruction::PushArg(1),
                        Instruction::PushData(456),
                    ],
                ),
            )]),
            ..Runtime::new()
        };
        let after = Runtime {
            value_stack: vec![Word::Data(123)],
            instruction_stack: vec![],
            ..runtime.clone()
        };

        assert_err_with_msg!(runtime.run(Instruction::CallIf), ERR_UNDEFINED);
        assert_eq!(after, runtime);
    }

    #[test]
    fn callif_empty_stack() {
        let mut runtime = Runtime::new();

        assert_err_with_msg!(runtime.run(Instruction::CallIf), ERR_UNDERFLOW);
        assert_eq!(Runtime::new(), runtime);
    }

    #[test]
    fn callif_almost_empty_stack() {
        let mut runtime = Runtime {
            value_stack: vec![Word::Function("foo".to_owned())],
            ..Runtime::new()
        };

        assert_err_with_msg!(runtime.run(Instruction::CallIf), ERR_UNDERFLOW);
        assert_eq!(Runtime::new(), runtime);
    }

    #[test]
    fn callif_data_first() {
        let mut runtime = Runtime {
            value_stack: vec![Word::Function("empty".to_owned()), Word::Data(1)],
            ..Runtime::new()
        };

        assert_err_with_msg!(runtime.run(Instruction::CallIf), ERR_TYPE);
        assert_eq!(Runtime::new(), runtime);
    }

    #[test]
    fn callif_function_second() {
        let mut runtime = Runtime {
            value_stack: vec![
                Word::Function("empty".to_owned()),
                Word::Function("empty".to_owned()),
            ],
            ..Runtime::new()
        };

        assert_err_with_msg!(runtime.run(Instruction::CallIf), ERR_TYPE);
        assert_eq!(Runtime::new(), runtime);
    }

    #[test]
    fn builtin_nand() {
        let mut runtime = Runtime {
            value_stack: vec![
                Word::Data(12),
                Word::Data(10),
                Word::Data(1),
                Word::Function("__nand__".to_owned()),
            ],
            ..Runtime::new()
        };
        let after = Runtime {
            value_stack: vec![Word::Data(4294967287)],
            ..Runtime::new()
        };

        assert_ok_and_eq!(runtime.run(Instruction::CallIf), false);
        assert_eq!(after, runtime);
    }

    #[test]
    fn builtin_nand_empty_stack() {
        let mut runtime = Runtime {
            value_stack: vec![Word::Data(1), Word::Function("__nand__".to_owned())],
            ..Runtime::new()
        };

        // Only one value on the stack after popping function and data: stack underflow
        assert_err_with_msg!(runtime.run(Instruction::CallIf), ERR_UNDERFLOW);
        assert_eq!(Runtime::new(), runtime);
    }

    #[test]
    fn builtin_nand_almost_empty_stack() {
        let mut runtime = Runtime {
            value_stack: vec![
                Word::Function("foo".to_owned()),
                Word::Data(1),
                Word::Function("__nand__".to_owned()),
            ],
            ..Runtime::new()
        };

        assert_err_with_msg!(runtime.run(Instruction::CallIf), ERR_UNDERFLOW);
        assert_eq!(Runtime::new(), runtime);
    }

    #[test]
    fn builtin_nand_function_first() {
        let mut runtime = Runtime {
            value_stack: vec![
                Word::Data(42),
                Word::Function("foo".to_owned()),
                Word::Data(1),
                Word::Function("__nand__".to_owned()),
            ],
            ..Runtime::new()
        };

        assert_err_with_msg!(runtime.run(Instruction::CallIf), ERR_TYPE);
        assert_eq!(Runtime::new(), runtime);
    }

    #[test]
    fn builtin_nand_function_second() {
        let mut runtime = Runtime {
            value_stack: vec![
                Word::Function("foo".to_owned()),
                Word::Data(42),
                Word::Data(1),
                Word::Function("__nand__".to_owned()),
            ],
            ..Runtime::new()
        };

        assert_err_with_msg!(runtime.run(Instruction::CallIf), ERR_TYPE);
        assert_eq!(Runtime::new(), runtime);
    }

    #[test]
    fn builtin_shift_left() {
        let mut runtime = Runtime {
            value_stack: vec![
                // 2^31 + 4 + 1
                Word::Data(2147483653),
                Word::Data(1),
                Word::Function("__shift_left__".to_owned()),
            ],
            ..Runtime::new()
        };
        let after = Runtime {
            value_stack: vec![Word::Data(10)],
            ..Runtime::new()
        };

        assert_ok_and_eq!(runtime.run(Instruction::CallIf), false);
        assert_eq!(after, runtime);
    }

    #[test]
    fn builtin_shift_left_empty_stack() {
        let mut runtime = Runtime {
            value_stack: vec![Word::Data(1), Word::Function("__shift_left__".to_owned())],
            ..Runtime::new()
        };

        assert_err_with_msg!(runtime.run(Instruction::CallIf), ERR_UNDERFLOW);
        assert_eq!(Runtime::new(), runtime);
    }

    #[test]
    fn builtin_shift_left_function_first() {
        let mut runtime = Runtime {
            value_stack: vec![
                Word::Function("foo".to_owned()),
                Word::Data(1),
                Word::Function("__shift_left__".to_owned()),
            ],
            ..Runtime::new()
        };

        assert_err_with_msg!(runtime.run(Instruction::CallIf), ERR_TYPE);
        assert_eq!(Runtime::new(), runtime);
    }

    #[test]
    fn builtin_shift_right() {
        let mut runtime = Runtime {
            value_stack: vec![
                // 2^31 + 4 + 1
                Word::Data(2147483653),
                Word::Data(1),
                Word::Function("__shift_right__".to_owned()),
            ],
            ..Runtime::new()
        };
        let after = Runtime {
            // 2^30 + 2
            value_stack: vec![Word::Data(1073741826)],
            ..Runtime::new()
        };

        assert_ok_and_eq!(runtime.run(Instruction::CallIf), false);
        assert_eq!(after, runtime);
    }

    #[test]
    fn builtin_shift_right_empty_stack() {
        let mut runtime = Runtime {
            value_stack: vec![Word::Data(1), Word::Function("__shift_right__".to_owned())],
            ..Runtime::new()
        };

        assert_err_with_msg!(runtime.run(Instruction::CallIf), ERR_UNDERFLOW);
        assert_eq!(Runtime::new(), runtime);
    }

    #[test]
    fn builtin_shift_right_function_first() {
        let mut runtime = Runtime {
            value_stack: vec![
                Word::Function("foo".to_owned()),
                Word::Data(1),
                Word::Function("__shift_right__".to_owned()),
            ],
            ..Runtime::new()
        };

        assert_err_with_msg!(runtime.run(Instruction::CallIf), ERR_TYPE);
        assert_eq!(Runtime::new(), runtime);
    }

    #[test]
    fn exit() {
        let mut runtime = Runtime::new();

        assert_ok_and_eq!(runtime.run(Instruction::Exit), true);
        assert_eq!(Runtime::new(), runtime);
    }
}
