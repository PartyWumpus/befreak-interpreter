use array2d::{Array2D, Error};

// for file read
use std::fs::File;
use std::io::{self, BufRead};
use std::path::Path;

#[derive(Clone, Copy, Debug)]
enum Direction {
    North,
    South,
    East,
    West,
}

impl Direction {
    const fn step_location(direction: Self, location: (usize, usize)) -> (usize, usize) {
        match direction {
            Direction::North => (location.0, location.1 - 1),
            Direction::South => (location.0, location.1 + 1),
            Direction::East => (location.0 + 1, location.1),
            Direction::West => (location.0 - 1, location.1),
        }
    }
}

#[derive(Debug)]
struct State {
    stack: Vec<i64>,
    control_stack: Vec<i64>,
    location: (usize, usize),
    direction: Direction,
    output_stack: Vec<i64>,
    direction_reversed: bool, // TODO:
    inverse_mode: bool,
    ascii_mode: bool,
    current_number: Option<i64>,
    code: Array2D<char>,
}

impl State {
    fn new(code: Array2D<char>) -> Self {
        let mut start: Option<(usize, usize)> = None;
        for (index_y, mut row) in code.rows_iter().enumerate() {
            if let Some(index_x) = row.position(|x| *x == '@') {
                start = Some((index_x, index_y));
                break;
            }
        }
        // TODO: check for more than one start pos?
        match start {
            None => panic!("No start position"),
            Some(location) => Self {
                stack: vec![],
                control_stack: vec![],
                location,
                direction: Direction::East,
                output_stack: vec![],
                direction_reversed: false,
                inverse_mode: false,
                ascii_mode: false,
                current_number: None,
                code,
            },
        }
    }

    fn get_instruction(&self, location: (usize, usize)) -> &char {
        return self
            .code
            .get(location.1, location.0)
            .expect("position should not exit the code");
    }

    fn step(&mut self) {
        // http://tunes.org/~iepos/befreak.html#reference

        self.location = Direction::step_location(self.direction, self.location);

        /*println!(
            "{}, {:?}, {:?}, {}",
            self.get_instruction(self.location),
            self.stack,
            self.control_stack,
            self.inverse_mode
        );*/

        if self.ascii_mode {
            let char = self.get_instruction(self.location);
            if *char == '"' {
                self.ascii_mode = false;
            } else {
                self.stack.push(*char as i64)
            }
            return;
        }

        let instruction = self.get_instruction(self.location);
        match instruction {
            '0' | '1' | '2' | '3' | '4' | '5' | '6' | '7' | '8' | '9' => {
                let new_digit = instruction.to_digit(10).unwrap() as i64;
                self.current_number = match self.current_number {
                    None => Some(new_digit),
                    Some(num) => Some(num * 10 + new_digit),
                };
                return;
            }
            _ => {
                if let Some(number) = self.current_number {
                    *self.stack.last_mut().unwrap() ^= number;
                    self.current_number = None;
                }
            }
        };

        let mut instruction = self.get_instruction(self.location).clone();
        if self.inverse_mode {
            instruction = match instruction {
                '(' => ')',
                ')' => '(',

                '[' => ']',
                ']' => '[',

                // interpret as NOOP for now
                // FIXME:
                'w' => ' ',
                'r' => ' ',

                '\'' => '`',
                '`' => '\'',

                '+' => '-',
                '-' => '+',

                '%' => '*',
                '*' => '%',

                '{' => '}',
                '}' => '{',

                'd' => 'b',
                'b' => 'd',

                'o' => 'u',
                'u' => 'o',

                ':' => ';',
                ';' => ':',
                _ => instruction,
            }
        }

        match instruction {
            // Push a zero onto the stack
            '(' => self.stack.push(0),
            // Pop a zero from the stack
            ')' => {
                if self.stack.pop() != Some(0) {
                    panic!("popped non-zero value")
                }
            }

            // Transfer the top of main stack to control stack
            '[' => self
                .control_stack
                .push(self.stack.pop().expect("main stack shouldn't be empty")),
            // Transfer the top of control stack to the main stack
            ']' => self.stack.push(
                self.control_stack
                    .pop()
                    .expect("control stack shouldn't be empty"),
            ),

            // Swap the top item with the top of control stack
            '$' => {
                let main = self.stack.pop().expect("empty stack");
                let control = self.control_stack.pop().expect("empty stack");
                self.stack.push(control);
                self.control_stack.push(main);
            }

            // Write the top item to stdout as a character
            'w' => {
                let x = self.pop();
                self.output_stack.push(x);
                print!("{}", x as u8 as char);
            }
            // Read a character from stdin to the top of stack
            'r' => todo!(),

            //TODO: allow under/overflow in increments/decrement

            // Increment the top item
            '\'' => *self.stack.last_mut().expect("empty stack") += 1,
            // Decrement the top item
            '`' => *self.stack.last_mut().expect("empty stack") -= 1,

            // TODO: allow under/overflow in sum/minus

            // Add the top item to the next item
            '+' => {
                let top = self.stack.pop().expect("empty stack");
                let next = self.stack.pop().expect("empty stack");
                self.stack.push(next + top);
                self.stack.push(top);
            }
            // Subtract the top item from the next item
            '-' => {
                let top = self.stack.pop().expect("empty stack");
                let next = self.stack.pop().expect("empty stack");
                self.stack.push(next - top);
                self.stack.push(top);
            }

            // Divide next by top, leaving a quotient and remainder
            // [y] [x] -> [y/x] [y%x] [x]
            '%' => {
                let x = self.stack.pop().expect("empty stack");
                let y = self.stack.pop().expect("empty stack");
                self.stack.push(y / x);
                self.stack.push(y % x);
                self.stack.push(x);
            }
            // Undo the effects of %, using multiplication
            '*' => {
                let top = self.stack.pop().expect("empty stack");
                let remainder = self.stack.pop().expect("empty stack");
                let quotient = self.stack.pop().expect("empty stack");
                self.stack.push(quotient * top + remainder);
                self.stack.push(top);
            }

            // Bitwise NOT the top item
            '~' => *self.stack.last_mut().unwrap() = !self.stack.last().expect("empty stack"),
            // Bitwise AND top two items, XOR'ing to the third
            // [z] [y] [x] -> [z^(y&x)] [y] [x]
            '&' => {
                let x = self.stack.pop().expect("empty stack");
                let y = self.stack.pop().expect("empty stack");
                let z = self.stack.pop().expect("empty stack");
                self.stack.push(z ^ (y & x));
                self.stack.push(y);
                self.stack.push(x);
            }
            // Bitwise OR top two items, XOR'ing to the third
            // [z] [y] [x] -> [z^(y|x)] [y] [x]
            '|' => {
                let x = self.stack.pop().expect("empty stack");
                let y = self.stack.pop().expect("empty stack");
                let z = self.stack.pop().expect("empty stack");
                self.stack.push(z ^ (y | x));
                self.stack.push(y);
                self.stack.push(x);
            }
            // Bitwise XOR the top item to the next item
            // [y] [x] -> [y^x] [x]
            '#' => {
                let x = self.stack.pop().expect("empty stack");
                let y = self.stack.pop().expect("empty stack");
                self.stack.push(y ^ x);
                self.stack.push(x);
            }

            // Rotate means shift with wrapping
            // Rotate "y" to the left "x" bits
            //[y] [x] -> [y'] [x]
            '{' => {
                let x = self.stack.pop().expect("empty stack");
                let y = self.stack.pop().expect("empty stack");
                self.stack.push(y.rotate_left(x as u32));
                self.stack.push(x);
            }
            // Rotate "y" to the right "x" bits
            '}' => {
                let x = self.stack.pop().expect("empty stack");
                let y = self.stack.pop().expect("empty stack");
                self.stack.push(y.rotate_right(x as u32));
                self.stack.push(x);
            }

            // Toggle top of control stack (i.e., XOR it with 1)
            '!' => self.toggle_control_stack(),

            // If y equals x, toggle top of control stack
            '=' => {
                let top = self.stack.pop().expect("empty stack");
                let next = self.stack.pop().expect("empty stack");
                if next == top {
                    self.toggle_control_stack()
                }
                self.stack.push(next);
                self.stack.push(top);
            }

            // If y is less than x, toggle top of control stack
            'l' => {
                let top = self.stack.pop().expect("empty stack");
                let next = self.stack.pop().expect("empty stack");
                if next < top {
                    self.toggle_control_stack()
                }
                self.stack.push(next);
                self.stack.push(top);
            }

            // If y is greater than x, toggle top of control stack
            'g' => {
                let top = self.stack.pop().expect("empty stack");
                let next = self.stack.pop().expect("empty stack");
                if next > top {
                    self.toggle_control_stack()
                }
                self.stack.push(next);
                self.stack.push(top);
            }

            // Swap the top two items
            's' => {
                let top = self.stack.pop().expect("empty stack");
                let next = self.stack.pop().expect("empty stack");
                self.stack.push(top);
                self.stack.push(next);
            }

            // Dig the third item to the top
            // [z] [y] [x] -> [y] [x] [z]
            'd' => {
                let x = self.stack.pop().expect("empty stack");
                let y = self.stack.pop().expect("empty stack");
                let z = self.stack.pop().expect("empty stack");
                self.stack.push(y);
                self.stack.push(x);
                self.stack.push(z);
            }
            // Bury the first item under the next two
            // [z] [y] [x] -> [x] [z] [y]
            'b' => {
                let x = self.stack.pop().expect("empty stack");
                let y = self.stack.pop().expect("empty stack");
                let z = self.stack.pop().expect("empty stack");
                self.stack.push(x);
                self.stack.push(z);
                self.stack.push(y);
            }
            // Flip the order of the top three items
            // [z] [y] [x] -> [x] [y] [z]
            'f' => {
                let x = self.stack.pop().expect("empty stack");
                let y = self.stack.pop().expect("empty stack");
                let z = self.stack.pop().expect("empty stack");
                self.stack.push(x);
                self.stack.push(y);
                self.stack.push(z);
            }
            // Swap the second and third items
            // [z] [y] [x] -> [y] [z] [x]
            'c' => {
                let x = self.stack.pop().expect("empty stack");
                let y = self.stack.pop().expect("empty stack");
                let z = self.stack.pop().expect("empty stack");
                self.stack.push(y);
                self.stack.push(z);
                self.stack.push(x);
            }
            // "Over": dig copy of second item to the top
            // [y] [x] -> [y] [x] [y]
            'o' => {
                let x = self.stack.pop().expect("empty stack");
                let y = self.stack.pop().expect("empty stack");
                self.stack.push(y);
                self.stack.push(x);
                self.stack.push(y);
            }
            // "Under": the inverse of "over"
            // [y] [x] [y] -> [y] [x]
            'u' => {
                let y1 = self.stack.pop().expect("empty stack");
                let x = self.stack.pop().expect("empty stack");
                let y2 = self.stack.pop().expect("empty stack");
                if y1 != y2 {
                    panic!("invalid inverse of over");
                }
                self.stack.push(y1);
                self.stack.push(x);
            }
            // Duplicate the top item
            // [x] -> [x] [x]
            ':' => {
                let x = self.pop();
                self.stack.push(x);
                self.stack.push(x);
            }
            // Unduplicate the top two items
            // [x] [x] -> [x]
            ';' => {
                let x1 = self.pop();
                let x2 = self.pop();
                if x1 != x2 {
                    panic!("unduplicate called on non-duplicates");
                }
                self.stack.push(x1);
            }
            // Enter string mode
            '"' => self.ascii_mode = true,
            // THIS IS WRONG, IT TRIGGERS INVERSE MODE. Toggle reverse mode
            '?' => self.inverse_mode = !self.inverse_mode,
            // Halt. Also signals the entrance point for the program
            '@' => self.end(),
            // If going east or west, turn right; otherwise, turn left
            '\\' => {
                self.direction = match self.direction {
                    Direction::North => Direction::West,
                    Direction::South => Direction::East,
                    Direction::East => Direction::South,
                    Direction::West => Direction::North,
                }
            }
            // If going east or west, turn left; otherwise, turn right
            '/' => {
                self.direction = match self.direction {
                    Direction::North => Direction::East,
                    Direction::South => Direction::West,
                    Direction::East => Direction::North,
                    Direction::West => Direction::South,
                }
            }

            // TODO: THESE ARE TOTALLY BORKED IN REVERSE MODE!!!!!!1!!1!!

            // If going north, go east and push 1 (in reverse mode, push 0) ...
            // If going south, go east and push 0 (in reverse mode, push 1) ...
            // If going west, pop and go south if 0, north if 1. (opposite in reverse mode)
            // If going east, toggle top of control stack, toggle inverted mode, and go west.
            '>' => match self.direction {
                Direction::North => {
                    self.direction = Direction::East;
                    self.control_stack.push(!self.inverse_mode as i64);
                }
                Direction::South => {
                    self.direction = Direction::East;
                    self.control_stack.push(self.inverse_mode as i64);
                }
                Direction::West => {
                    let dir = self.control_stack.pop();
                    if dir == Some(self.inverse_mode as i64) {
                        self.direction = Direction::South;
                    } else if dir == Some(!self.inverse_mode as i64) {
                        self.direction = Direction::North;
                    } else {
                        panic!("invalid value in control stack");
                    }
                }
                Direction::East => {
                    self.toggle_control_stack();
                    self.inverse_mode = !self.inverse_mode;
                    self.direction = Direction::West;
                }
            },
            // If going north, go west and push 0 (in reverse mode, push 1) ...
            // If going south, go west and push 1 (in reverse mode, push 0) ...
            // If going east, pop and go north if 0, south if 1. (opposite in reverse mode)
            // If going west, toggle top of control stack, toggle inverted mode, and go east.
            '<' => match self.direction {
                Direction::North => {
                    self.direction = Direction::West;
                    self.control_stack.push(self.inverse_mode as i64);
                }
                Direction::South => {
                    self.direction = Direction::West;
                    self.control_stack.push(!self.inverse_mode as i64);
                }
                Direction::East => {
                    let dir = self.control_stack.pop();
                    if dir == Some(self.inverse_mode as i64) {
                        self.direction = Direction::North;
                    } else if dir == Some(!self.inverse_mode as i64) {
                        self.direction = Direction::South;
                    } else {
                        panic!("invalid value in control stack");
                    }
                }
                Direction::West => {
                    self.toggle_control_stack();
                    self.inverse_mode = !self.inverse_mode;
                    self.direction = Direction::East;
                }
            },
            // If going east, go south and push 1 (in reverse mode, push 0) ...
            // If going west, go south and push 0 (in reverse mode, push 1) ...
            // If going north, pop and go west if 0, east if 1. (opposite in reverse mode)
            // If going south, toggle top of control stack, toggle inverted mode, and go north.
            'v' => match self.direction {
                Direction::East => {
                    self.direction = Direction::South;
                    self.control_stack.push(!self.inverse_mode as i64);
                }
                Direction::West => {
                    self.direction = Direction::South;
                    self.control_stack.push(self.inverse_mode as i64);
                }
                Direction::North => {
                    let dir = self.control_stack.pop();
                    if dir == Some(self.inverse_mode as i64) {
                        self.direction = Direction::West;
                    } else if dir == Some(!self.inverse_mode as i64) {
                        self.direction = Direction::East;
                    } else {
                        panic!("invalid value in control stack");
                    }
                }
                Direction::South => {
                    self.toggle_control_stack();
                    self.inverse_mode = !self.inverse_mode;
                    self.direction = Direction::North;
                }
            },
            // If going east, go north and push 0 (in reverse mode, push 1) ...
            // If going west, go north and push 1 (in reverse mode, push 0) ...
            // If going south, pop and go east if 0, west if 1. (opposite in reverse mode)
            // If going north, toggle top of control stack, toggle inverted mode, and go south.
            '^' => match self.direction {
                Direction::East => {
                    self.direction = Direction::North;
                    self.control_stack.push(self.inverse_mode as i64);
                }
                Direction::West => {
                    self.direction = Direction::North;
                    self.control_stack.push(!self.inverse_mode as i64);
                }
                Direction::South => {
                    let dir = self.control_stack.pop();
                    if dir == Some(self.inverse_mode as i64) {
                        self.direction = Direction::East;
                    } else if dir == Some(!self.inverse_mode as i64) {
                        self.direction = Direction::West;
                    } else {
                        panic!("invalid value in control stack");
                    }
                }
                Direction::North => {
                    self.toggle_control_stack();
                    self.inverse_mode = !self.inverse_mode;
                    self.direction = Direction::South
                }
            },
            ' ' => (),
            _ => unreachable!(),
        }
    }

    fn pop(&mut self) -> i64 {
        self.stack
            .pop()
            .expect("should not pop when stack is empty")
    }

    fn toggle_control_stack(&mut self) {
        *self.control_stack.last_mut().unwrap() ^= 1
    }

    fn end(&mut self) -> ! {
        for char in self.output_stack.iter() {
            print!("{}", *char as u8 as char);
        }
        std::process::exit(0);
    }
}

fn main() {
    let x = read_lines("primes2");
    let mut state = State::new(x);
    loop {
        state.step();
    }
}

fn read_lines<P>(filename: P) -> Array2D<char>
where
    P: AsRef<Path>,
{
    let file = File::open(filename).unwrap();
    let mut lines = vec![];
    for maybe_line in io::BufReader::new(file).lines() {
        if let Ok(line) = maybe_line {
            lines.push(line.chars().collect())
        }
    }
    Array2D::from_rows(&lines).unwrap()
}
