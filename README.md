# plates

plates is an esoteric, imperative, stack-based programming language.

## Instructions

- `PUSH <value>`: pushes a word onto the stack.
    - If an unsigned 32-bit integer is provided, that value is pushed onto the stack.
    - If a function name is provided, that function is pushed onto the stack.
    - If the token `*` is provided, a random byte (from a uniform distribution) is generated.
- `DEFN <function-name> (<arg-count>) { <instructions> }`: defines a function. When this function is called, the top `arg-count` values on the stack will be popped. They can then be accessed as `$0` (for the value that was on top of the stack), `$1`, `$2`, and so on. Note that nested function calls will overwrite arguments.
- `CALLIF`: checks the data word at the top of the stack. If it is nonzero, the function below that is executed. Otherwise, the function is not called. In either case, the two words at the top of the stack are discarded.
- `EXIT`: terminates the program.

## Functions

Functions can be pushed onto the stack and then called. When called, they can modify the state of the stack.

### Built-in functions

- `__print__`: displays the data words starting at the top of the stack and continuing downwards until it reaches a zero word. Each word is interpreted as a UTF-32. The printed data will remain on the stack.
- `__input__`: reads one line of input from stdin and places each character onto the stack (with the first character read on top). The characters are represented in UTF-32.
- `__nand__`: performs bitwise NAND on the two data words at the top of the stack.
- `__shift_left__`: performs a bitwise left shift (by one bit) on the data word at the top of the stack.
- `__shift_right__`: performs a bitwise right shift (by one bit) on the data word at the top of the stack.

## Comments

When `//` is encountered, everything until the end of that line is treated as a comment.
