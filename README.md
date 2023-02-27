# plates

Esoteric, imperative, stack-based programming language.

## Instructions

- `PUSH <value>`: pushes a word onto the stack.
    - If an unsigned 32-bit integer is provided, that value is pushed onto the stack.
    - If a function name is provided, that function is pushed onto the stack.
    - If the token `^` is provided, the value at the top of the stack is copied.
    - If the token `*` is provided, a random byte (from a uniform distribution) is generated.
- `DEFN <function-name> (<instructions>)`: defines a function.
- `CALLIF`: checks the data word at the top of the stack. If it is nonzero, the function below that is executed. Otherwise, the function is not called. In either case, the two words at the top of the stack are discarded.
- `EXIT`: terminates the program.

## Functions

Functions can be pushed onto the stack and then called. When called, they can modify the state of the stack.

### Built-in subroutines

- `__print__`: displays the data words starting at the top of the stack and continuing downwards until it reaches a zero word. Each word is interpreted as a UTF-32. The printed data will remain on the stack.
- `__input__`: reads one line of input from stdin and places each character onto the stack (with the most recently read character on top). The characters are represented in UTF-32.
- `__swap__`: swaps words. The data word at the top of the stack determines the target of the swap. For example:
    ```
    BEFORE                      AFTER

    [top of the stack]

    | __swap__ |
    |        2 |
    |        0 | <-- index 0    | 6 |
    |        3 | <-- index 1    | 3 |
    |        6 | <-- index 2    | 0 |
    |        9 |                | 9 |

    [bottom of the stack]
    ```
- `__nand__`: performs bitwise NAND on the two data words at the top of the stack.

## Comments

When `//` is encountered, everything until the end of that line is treated as a comment.
