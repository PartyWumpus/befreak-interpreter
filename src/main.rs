use array2d::{Array2D, Error};

// for file read
use std::fs::File;
use std::io::{self, BufRead};
use std::path::Path;

enum Direction {
    North,
    South,
    East,
    West,
}

impl Direction {
    fn step_location(location: (usize, usize)) -> (usize, usize) {
        match self.direction {
            Direction::North => (location.0, location.1 - 1),
            Direction::South => (location.0, location.1 + 1),
            Direction::East => (location.0 + 1, location.1),
            Direction::West => (location.0 - 1, location.1),
        }
    }
}

struct State {
    stack: Vec<u8>,
    control_stack: Vec<u8>,
    location: (usize, usize),
    direction: Direction,
    reversed: bool,
    ascii: bool,
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
                reversed: false,
                code,
            },
        }
    }

    fn get_instruction(&self, location: (usize, usize)) -> &char {
        return self
            .code
            .get(location.0, location.1)
            .expect("position should not exit the code");
    }

    fn step(&mut self) {
        self.location = Direction::step_location(self.location);
        let instruction = self.get_instruction(self.location);
        if !self.reversed {
            // http://tunes.org/~iepos/befreak.html#reference
            match instruction {
                // FIXME: the reference says 1-9, this appears to be a mistake
                '0' | '1' | '2' | '3' | '4' | '5' | '6' | '7' | '8' | '9' => todo!(),
                '(' => self.stack.push(0),
                ')' => { if self.stack.pop() != Some(0) { panic!("popped non-zero value") } },
                '[' => todo!(),
                ']' => todo!(),
                '$' => todo!(),
                'w' => todo!(),
                'r' => todo!(),
                '\'' => todo!(),
                '`' => todo!(),
                '+' => todo!(),
                '-' => todo!(),
                '%' => todo!(),
                '*' => todo!(),
                '~' => todo!(),
                '&' => todo!(),
                '|' => todo!(),
                '#' => todo!(),
                '{' => todo!(),
                '}' => todo!(),
                '!' => todo!(),
                '=' => todo!(),
                'l' => todo!(),
                'g' => todo!(),
                's' => todo!(),
                'd' => todo!(),
                'b' => todo!(),
                'f' => todo!(),
                'c' => todo!(),
                'o' => todo!(),
                'u' => todo!(),
                ':' => todo!(),
                ';' => todo!(),
                '"' => todo!(),
                '?' => todo!(),
                '@' => todo!(),
                '\\' => todo!(),
                '/' => todo!(),
                '>' => todo!(),
                '<' => todo!(),
                'v' => todo!(),
                '^' => todo!(),
                _ => unreachable!(),
            }
        }
    }
}

fn main() {
    let x = read_lines("helloworld.txt");
    println!("Hello, world! {:?}", x);
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
