use array2d::Array2D;
use phf::phf_map;

use instant::Instant;
use std::future::Future;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::time::Duration;

// for file read
use std::fs::File;
use std::io::{self, BufRead};
use std::path::Path;

// TODO: add file editing:
// changing individual characters
// changing grid size

static PRESETS: phf::Map<&'static str, &'static str> = phf_map! {
"hello world 1" =>r#"
/"Hello world!"01\
\(13v     `wsv)@(/
    \(=13=13)/    "#,

"hello world 2" =>r#"
/"Hello world!"01\
\(13vws`v     )@(/
    (   )         
    =   3         
        1         
    \13=/         "#,

"hello world 3" => r#"
/"Hello world!"\
\(13:vwd` v@(10/
     \=(=)/     "#,

"primes 1" => r#"
    /1)@(1\         
    >)1=1(<         
    \'(v?)/         
       >'%s(\       
     ^ >*s)=/       
     >=<            
     (              
/s'0v^?w23(v`s]:(48\
[   (      )       +
)   =      =       4
0   c      c       8
1   =      =       )
%   )      (       w
\01(^      ^)01*01(/"#,

"primes 2" => r#"
    /2)@(2\         
    >)2=2(<         
    \'(v?)/         
       s            
       (            
       1            
       >(1=1\       
       )            
       1    o       
       {    *       
       1    b       
       (    l       
       >)u%d/       
       c            
       >b'%s(= \    
     ^ >dc=c*s)/    
     >=<            
     d              
     (              
/s'0v^?w23(v`s]:(48\
[   (      )       +
)   =      =       4
0   c      c       8
1   =      =       )
%   )      (       w
\01(^      ^)01*01(/"#,
};

#[derive(Clone, Copy, Debug)]
enum Direction {
    North,
    South,
    East,
    West,
}

#[derive(Debug)]
enum BefreakError {
    InvalidPosition,
    EmptyMainStack,
    EmptyControlStack,
    EmptyOutputStack,
    NonBoolInControlStack,
    InvalidUnduplicate,
    InvalidPopZero,
    InvalidUnder,
}

impl std::error::Error for BefreakError {}

impl std::fmt::Display for BefreakError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let str = match self {
            Self::InvalidPosition => "Attempted to enter a position outside the grid",
            Self::EmptyMainStack => "Attempted to pop off the stack but it was empty",
            Self::EmptyControlStack => "Attempted to pop off the control stack but it was empty",
            Self::EmptyOutputStack => "Attempted to pop off the output stack but it was empty",
            Self::NonBoolInControlStack => {
                "Tried to use the control stack but a non boolean value was at the top"
            }
            Self::InvalidUnduplicate => {
                "Tried to unduplicate the top two values but they were not identical"
            }
            Self::InvalidPopZero => "Tried to pop a value off the stack but it was not a zero",
            Self::InvalidUnder => {
                "Tried to do under but the top and third values were not identical"
            }
        };
        write!(f, "{str}")
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
    number_stack: Vec<char>,

    finished: bool,
    step: u64,

    // constants
    code: Array2D<char>,
}

pub struct AppState {
    state: State,
    speed: f32,
    //cursor_position: (usize, usize),
    paused: bool,
    previous_instant: Instant,
    //extra: bool,
    error: Option<BefreakError>,
    text_channel: (Sender<String>, Receiver<String>),
}

impl AppState {
    /// Called once before the first frame.
    pub fn new(__cc: &eframe::CreationContext<'_>) -> Self {
        // This is also where you can customize the look and feel of egui using
        // `cc.egui_ctx.set_visuals` and `cc.egui_ctx.set_fonts`.

        Self {
            state: State::new_empty(),
            //state: State::new_load_file("primes1"),
            text_channel: channel(),
            //cursor_position: (0, 0), //TODO: use this for editing
            previous_instant: Instant::now(),
            paused: true,
            //extra: false, //TODO: use this for displaying more info
            error: None,
            speed: 5.0,
        }
    }
}

impl AppState {
    fn handle_err(&mut self, err: Result<(), BefreakError>) {
        match err {
            Ok(..) => (),
            Err(err) => self.error = Some(err),
        }
    }

    fn reset(&mut self) {
        self.error = None;
        self.state.reset();
    }

    fn step(&mut self) {
        if self.error.is_none() {
            let x = self.state.step();
            self.handle_err(x);
        }
        if self.state.finished {
            self.paused = true;
        }
    }

    fn reverse_direction(&mut self) {
        let x = self.state.reverse_direction();
        self.handle_err(x);
    }
}

impl eframe::App for AppState {
    /// Called each time the UI needs repainting, which may be many times per second.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Put your widgets into a `SidePanel`, `TopBottomPanel`, `CentralPanel`, `Window` or `Area`.
        // For inspiration and more examples, go to https://emilk.github.io/egui
        //
        if let Ok(text) = self.text_channel.1.try_recv() {
            self.state = State::new_from_string(&text);
        }

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            // The top panel is often a good place for a menu bar:

            egui::menu::bar(ui, |ui| {
                // NOTE: no File->Quit on web pages!
                let is_web = cfg!(target_arch = "wasm32");
                ui.menu_button("File", |ui| {
                    if !is_web {
                        if ui.button("Quit").clicked() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                    }
                    if ui.button("New File").clicked() {
                        self.error = None;
                        self.state = State::new_empty();
                    }
                    if ui.button("📂 Open text file").clicked() {
                        let sender = self.text_channel.0.clone();
                        let task = rfd::AsyncFileDialog::new().pick_file();
                        // Context is wrapped in an Arc so it's cheap to clone as per:
                        // > Context is cheap to clone, and any clones refers to the same mutable data (Context uses refcounting internally).
                        // Taken from https://docs.rs/egui/0.24.1/egui/struct.Context.html
                        let ctx = ui.ctx().clone();
                        execute(async move {
                            let file = task.await;
                            if let Some(file) = file {
                                let text = file.read().await;
                                let _ = sender.send(String::from_utf8_lossy(&text).to_string());
                                ctx.request_repaint();
                            }
                        });
                    }

                    if ui.button("💾 Save text to file").clicked() {
                        let task = rfd::AsyncFileDialog::new().save_file();
                        let contents = self.state.serialize();
                        execute(async move {
                            let file = task.await;
                            if let Some(file) = file {
                                _ = file.write(contents.as_bytes()).await;
                            }
                        });
                    }
                });

                ui.menu_button("Presets", |ui| {
                    for key in PRESETS.keys() {
                        if ui.button(*key).clicked() {
                            match PRESETS.get(key) {
                                None => todo!(),
                                Some(data) => self.state = State::new_from_string(data),
                            }
                        }
                    }
                });
                
                ui.add_space(16.0);

                egui::widgets::global_dark_light_mode_buttons(ui);
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            let time_per_step = Duration::from_millis((500.0 - 49.0 * self.speed) as u64);
            if !self.paused && self.error.is_none() {
                if self.previous_instant.elapsed() >= time_per_step {
                    if self.speed == 10.00 {
                        // if speed is max, go double speed
                        self.step();
                    }
                    self.step();
                    self.previous_instant = Instant::now();
                }

                ui.ctx().request_repaint_after(time_per_step);
            }

            // The central panel the region left after adding TopPanel's and SidePanel's
            ui.horizontal(|ui| {
                ui.heading("Befreak interpreter");
                ui.label("step: ");
                ui.label(self.state.step.to_string());
                if let Some(error) = &self.error {
                    ui.label(egui::RichText::new(error.to_string()).color(egui::Color32::RED));
                };
            });

            ui.horizontal(|ui| {
                if ui.button("step").clicked() {
                    self.step();
                };
                if ui
                    .button(if self.paused { "unpause" } else { "pause" })
                    .clicked()
                    && self.error.is_none()
                {
                    self.paused = !self.paused;
                };
                if ui.button("reverse").clicked() {
                    self.reverse_direction();
                }
                if ui.button("restart").clicked() {
                    self.reset();
                }
                ui.add(egui::Slider::new(&mut self.speed, 1.0..=10.0).text("speed"));
            });

            ui.separator();

            ui.horizontal(|ui| {
                ui.columns(3, |cols| {
                    cols[0].vertical_centered_justified(|ui| {
                        ui.label("output");
                        let output = &self
                            .state
                            .output_stack
                            .iter()
                            .map(|x| *x as u8 as char)
                            .collect::<String>();
                        ui.label(output);
                    });
                    //TODO: these don't fit if the stack is too full
                    cols[1].vertical_centered_justified(|ui| {
                        ui.label("primary stack");
                        ui.horizontal(|ui| {
                            for value in &self.state.stack {
                                ui.label(value.to_string());
                            }
                        })
                    });
                    cols[2].vertical_centered_justified(|ui| {
                        ui.label("control stack");
                        ui.horizontal(|ui| {
                            for value in &self.state.control_stack {
                                ui.label(value.to_string());
                            }
                        })
                    });
                });
            });

            ui.separator();

            egui::Grid::new("letter_grid")
                .spacing([0.0, 0.0])
                .show(ui, |ui| {
                    ui.spacing_mut().item_spacing.x = 0.0;
                    for (index_y, row) in self.state.code.rows_iter().enumerate() {
                        for (index_x, c) in row.enumerate() {
                            if self.state.location == (index_x, index_y) {
                                ui.label(
                                    egui::RichText::new(*c).background_color(egui::Color32::RED),
                                );
                            } else {
                                ui.label(c.to_string());
                            }
                        }
                        ui.end_row();
                    }
                });

            ui.separator();

            ui.add(egui::github_link_file!(
                "github.com/PartyWumpus/befreak-interpreter",
                "Source code."
            ));

            ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                powered_by_egui_and_eframe(ui);
                egui::warn_if_debug_build(ui);
            });
        });
    }
}

fn powered_by_egui_and_eframe(ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        ui.label("Powered by ");
        ui.hyperlink_to("egui", "https://github.com/emilk/egui");
        ui.label(" and ");
        ui.hyperlink_to(
            "eframe",
            "https://github.com/emilk/egui/tree/master/crates/eframe",
        );
        ui.label(". Built from the ");
        ui.hyperlink_to(
            "eframe template",
            "https://github.com/emilk/eframe_template/blob/main/",
        );
        ui.label(".");
    });
}

impl Default for State {
    fn default() -> Self {
        Self {
            stack: vec![],
            control_stack: vec![],
            location: (0, 0),
            direction: Direction::East,
            output_stack: vec![],
            direction_reversed: false,
            inverse_mode: false,
            ascii_mode: false,
            number_stack: vec![],
            step: 0,
            finished: false,
            code: Array2D::filled_with(' ', 10, 10),
        }
    }
}

impl State {
    fn _new_load_file<P>(path: P) -> Self
    where
        P: AsRef<Path>,
    {
        let file = File::open(path).unwrap();
        let mut lines = vec![];
        for line in io::BufReader::new(file).lines().flatten() {
            lines.push(line.chars().collect());
        }
        let code = Array2D::from_rows(&lines).unwrap();

        match Self::get_start_pos(&code) {
            None => panic!("No start position"),
            Some(location) => Self {
                location,
                code,
                ..Self::default()
            },
        }
    }

    fn new_from_string(data: &str) -> Self {
        let mut lines = vec![];
        for line in data.lines() {
            // skip lines with no content
            if !line.is_empty() {
                lines.push(line.chars().collect());
            }
        }
        let code = Array2D::from_rows(&lines).unwrap();

        match Self::get_start_pos(&code) {
            None => panic!("No start position"),
            Some(location) => Self {
                location,
                code,
                ..Default::default()
            },
        }
    }

    fn new_empty() -> Self {
        Default::default()
    }

    fn reset(&mut self) {
        match Self::get_start_pos(&self.code) {
            None => panic!("No start position"),
            Some(location) => {
                *self = Self {
                    location,
                    code: self.code.clone(),
                    ..Self::default()
                }
            }
        };
    }

    fn serialize(&self) -> String {
        let mut s: String = String::new();
        for line in self.code.rows_iter() {
            s.push_str(&line.collect::<String>());
            s.push('\n');
        }
        s
    }

    // TODO: check for more than one start pos and error?
    fn get_start_pos(code: &Array2D<char>) -> Option<(usize, usize)> {
        let mut start: Option<(usize, usize)> = None;
        for (index_y, mut row) in code.rows_iter().enumerate() {
            if let Some(index_x) = row.position(|x| *x == '@') {
                start = Some((index_x, index_y));
                break;
            }
        }
        start
    }

    fn get_instruction(&self, location: (usize, usize)) -> Result<&char, BefreakError> {
        self.code
            .get(location.1, location.0)
            .ok_or(BefreakError::InvalidPosition)
    }

    // TODO: fix reversing while processing a number
    // easiest solution is making it into a stack so stuff can just be popped off like everywhere
    // else :)
    fn reverse_direction(&mut self) -> Result<(), BefreakError> {
        self.direction_reversed = !self.direction_reversed;
        self.direction = match self.direction {
            Direction::North => Direction::South,
            Direction::South => Direction::North,
            Direction::East => Direction::West,
            Direction::West => Direction::East,
        };
        self.inverse_mode = !self.inverse_mode;
        self.process_instruction()?;
        Ok(())
    }

    fn step(&mut self) -> Result<(), BefreakError> {
        // http://tunes.org/~iepos/befreak.html#reference

        let Self { location, .. } = self;
        self.location = match self.direction {
            Direction::North => (location.0, location.1 - 1),
            Direction::South => (location.0, location.1 + 1),
            Direction::East => (location.0 + 1, location.1),
            Direction::West => (location.0 - 1, location.1),
        };

        if self.direction_reversed {
            self.step -= 1;
        } else {
            self.step += 1;
        }

        self.process_instruction()?;
        Ok(())
    }

    fn process_instruction(&mut self) -> Result<(), BefreakError> {
        if self.ascii_mode {
            let char = self.get_instruction(self.location)?;
            if *char == '"' {
                self.ascii_mode = false;
            } else {
                self.stack.push(*char as i64);
            }
            return Ok(());
        }

        let instruction = self.get_instruction(self.location)?;
        // TODO: allow reversing in the middle of a long number
        match instruction {
            '0' | '1' | '2' | '3' | '4' | '5' | '6' | '7' | '8' | '9' => {
                self.number_stack.push(*instruction);
                return Ok(());
            }
            _ => {
                if !self.number_stack.is_empty() {
                    let mut number: i64 = 0;
                    if self.inverse_mode {
                        for digit in self
                            .number_stack
                            .iter()
                            .map(|x| i64::from(x.to_digit(10).unwrap()))
                            .rev()
                        {
                            number = number * 10 + digit;
                        }
                    } else {
                        for digit in self
                            .number_stack
                            .iter()
                            .map(|x| i64::from(x.to_digit(10).unwrap()))
                        {
                            number = number * 10 + digit;
                        }
                    }
                    *self.stack.last_mut().unwrap() ^= number;
                    self.number_stack = vec![];
                }
            }
        };

        let mut instruction = *self.get_instruction(self.location)?;
        if self.inverse_mode {
            instruction = match instruction {
                '(' => ')',
                ')' => '(',

                '[' => ']',
                ']' => '[',

                'w' => 'w',
                'r' => ' ', // FIXME:

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
                if self.pop_main()? != 0 {
                    return Err(BefreakError::InvalidPopZero);
                }
            }

            // Transfer the top of main stack to control stack
            '[' => {
                let x = self.pop_main()?;
                self.control_stack.push(x);
            }
            // Transfer the top of control stack to the main stack
            ']' => {
                let x = self.pop_ctrl()?;
                self.stack.push(x);
            }

            // Swap the top item with the top of control stack
            '$' => {
                let main = self.pop_main()?;
                let control = self.pop_ctrl()?;
                self.stack.push(control);
                self.control_stack.push(main);
            }

            // Write the top item to stdout as a character
            'w' => {
                if self.inverse_mode {
                    match self.output_stack.pop() {
                        None => return Err(BefreakError::EmptyOutputStack),
                        Some(x) => self.stack.push(x),
                    };
                } else {
                    let x = self.pop_main()?;
                    self.output_stack.push(x);
                }
            }

            // Read a character from stdin to the top of stack
            'r' => todo!(),

            // Increment the top item
            '\'' => match self.stack.last_mut() {
                None => return Err(BefreakError::EmptyMainStack),
                Some(x) => *x = x.overflowing_add(1).0,
            },
            // Decrement the top item
            '`' => match self.stack.last_mut() {
                None => return Err(BefreakError::EmptyMainStack),
                Some(x) => *x = x.overflowing_sub(1).0,
            },

            // Add the top item to the next item
            '+' => {
                let top = self.pop_main()?;
                let next = self.pop_main()?;
                self.stack.push(next.overflowing_add(top).0);
                self.stack.push(top);
            }
            // Subtract the top item from the next item
            '-' => {
                let top = self.pop_main()?;
                let next = self.pop_main()?;
                self.stack.push(next.overflowing_sub(top).0);
                self.stack.push(top);
            }

            // Divide next by top, leaving a quotient and remainder
            // [y] [x] -> [y/x] [y%x] [x]
            '%' => {
                let x = self.pop_main()?;
                let y = self.pop_main()?;
                self.stack.push(y / x);
                self.stack.push(y % x);
                self.stack.push(x);
            }
            // Undo the effects of %, using multiplication
            '*' => {
                let top = self.pop_main()?;
                let remainder = self.pop_main()?;
                let quotient = self.pop_main()?;
                self.stack.push(quotient * top + remainder);
                self.stack.push(top);
            }

            // Bitwise NOT the top item
            '~' => match self.stack.last() {
                None => return Err(BefreakError::EmptyMainStack),
                Some(x) => *self.stack.last_mut().unwrap() = !x,
            },

            // Bitwise AND top two items, XOR'ing to the third
            // [z] [y] [x] -> [z^(y&x)] [y] [x]
            '&' => {
                let x = self.pop_main()?;
                let y = self.pop_main()?;
                let z = self.pop_main()?;
                self.stack.push(z ^ (y & x));
                self.stack.push(y);
                self.stack.push(x);
            }
            // Bitwise OR top two items, XOR'ing to the third
            // [z] [y] [x] -> [z^(y|x)] [y] [x]
            '|' => {
                let x = self.pop_main()?;
                let y = self.pop_main()?;
                let z = self.pop_main()?;
                self.stack.push(z ^ (y | x));
                self.stack.push(y);
                self.stack.push(x);
            }
            // Bitwise XOR the top item to the next item
            // [y] [x] -> [y^x] [x]
            '#' => {
                let x = self.pop_main()?;
                let y = self.pop_main()?;
                self.stack.push(y ^ x);
                self.stack.push(x);
            }

            // Rotate means shift with wrapping
            // Rotate "y" to the left "x" bits
            //[y] [x] -> [y'] [x]
            '{' => {
                let x = self.pop_main()?;
                let y = self.pop_main()?;
                // TODO: figure out how to make this work well with negative values of x
                // maybe do a manual conversion modulo 64 or similar?
                // also don't just unwrap it smh my head
                self.stack.push(y.rotate_left(u32::try_from(x).unwrap()));
                self.stack.push(x);
            }
            // Rotate "y" to the right "x" bits
            '}' => {
                let x = self.pop_main()?;
                let y = self.pop_main()?;
                self.stack.push(y.rotate_right(u32::try_from(x).unwrap()));
                self.stack.push(x);
            }

            // Toggle top of control stack (i.e., XOR it with 1)
            '!' => self.toggle_control_stack(),

            // If y equals x, toggle top of control stack
            '=' => {
                let top = self.pop_main()?;
                let next = self.pop_main()?;
                if next == top {
                    self.toggle_control_stack();
                }
                self.stack.push(next);
                self.stack.push(top);
            }

            // If y is less than x, toggle top of control stack
            'l' => {
                let top = self.pop_main()?;
                let next = self.pop_main()?;
                if next < top {
                    self.toggle_control_stack();
                }
                self.stack.push(next);
                self.stack.push(top);
            }

            // If y is greater than x, toggle top of control stack
            'g' => {
                let top = self.pop_main()?;
                let next = self.pop_main()?;
                if next > top {
                    self.toggle_control_stack();
                }
                self.stack.push(next);
                self.stack.push(top);
            }

            // Swap the top two items
            's' => {
                let top = self.pop_main()?;
                let next = self.pop_main()?;
                self.stack.push(top);
                self.stack.push(next);
            }

            // Dig the third item to the top
            // [z] [y] [x] -> [y] [x] [z]
            'd' => {
                let x = self.pop_main()?;
                let y = self.pop_main()?;
                let z = self.pop_main()?;
                self.stack.push(y);
                self.stack.push(x);
                self.stack.push(z);
            }
            // Bury the first item under the next two
            // [z] [y] [x] -> [x] [z] [y]
            'b' => {
                let x = self.pop_main()?;
                let y = self.pop_main()?;
                let z = self.pop_main()?;
                self.stack.push(x);
                self.stack.push(z);
                self.stack.push(y);
            }
            // Flip the order of the top three items
            // [z] [y] [x] -> [x] [y] [z]
            'f' => {
                let x = self.pop_main()?;
                let y = self.pop_main()?;
                let z = self.pop_main()?;
                self.stack.push(x);
                self.stack.push(y);
                self.stack.push(z);
            }
            // Swap the second and third items
            // [z] [y] [x] -> [y] [z] [x]
            'c' => {
                let x = self.pop_main()?;
                let y = self.pop_main()?;
                let z = self.pop_main()?;
                self.stack.push(y);
                self.stack.push(z);
                self.stack.push(x);
            }
            // "Over": dig copy of second item to the top
            // [y] [x] -> [y] [x] [y]
            'o' => {
                let x = self.pop_main()?;
                let y = self.pop_main()?;
                self.stack.push(y);
                self.stack.push(x);
                self.stack.push(y);
            }
            // "Under": the inverse of "over"
            // [y] [x] [y] -> [y] [x]
            'u' => {
                let y1 = self.pop_main()?;
                let x = self.pop_main()?;
                let y2 = self.pop_main()?;
                if y1 != y2 {
                    return Err(BefreakError::InvalidUnder);
                }
                self.stack.push(y1);
                self.stack.push(x);
            }
            // Duplicate the top item
            // [x] -> [x] [x]
            ':' => {
                let x = self.pop_main()?;
                self.stack.push(x);
                self.stack.push(x);
            }
            // Unduplicate the top two items
            // [x] [x] -> [x]
            ';' => {
                let x1 = self.pop_main()?;
                let x2 = self.pop_main()?;
                if x1 != x2 {
                    return Err(BefreakError::InvalidUnduplicate);
                }
                self.stack.push(x1);
            }
            // Enter string mode
            '"' => self.ascii_mode = true,
            // Toggle inverse mode
            // the doc says "toggle reverse mode", which doesn't make any sense, as a reverse
            // mode toggle would just undo the whole program back to the start
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

            // If going north, go east and push 1 (in reverse mode, push 0) ...
            // If going south, go east and push 0 (in reverse mode, push 1) ...
            // If going west, pop and go south if 0, north if 1. (opposite in reverse mode)
            // If going east, toggle top of control stack, toggle inverted mode, and go west.
            '>' => match self.direction {
                Direction::North => {
                    self.direction = Direction::East;
                    self.control_stack.push(i64::from(!self.inverse_mode));
                }
                Direction::South => {
                    self.direction = Direction::East;
                    self.control_stack.push(i64::from(self.inverse_mode));
                }
                Direction::West => {
                    let dir = self.control_stack.pop();
                    if dir == Some(i64::from(self.inverse_mode)) {
                        self.direction = Direction::South;
                    } else if dir == Some(i64::from(!self.inverse_mode)) {
                        self.direction = Direction::North;
                    } else {
                        return Err(BefreakError::NonBoolInControlStack);
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
                    self.control_stack.push(i64::from(self.inverse_mode));
                }
                Direction::South => {
                    self.direction = Direction::West;
                    self.control_stack.push(i64::from(!self.inverse_mode));
                }
                Direction::East => {
                    let dir = self.control_stack.pop();
                    if dir == Some(i64::from(self.inverse_mode)) {
                        self.direction = Direction::North;
                    } else if dir == Some(i64::from(!self.inverse_mode)) {
                        self.direction = Direction::South;
                    } else {
                        return Err(BefreakError::NonBoolInControlStack);
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
                    self.control_stack.push(i64::from(!self.inverse_mode));
                }
                Direction::West => {
                    self.direction = Direction::South;
                    self.control_stack.push(i64::from(self.inverse_mode));
                }
                Direction::North => {
                    let dir = self.control_stack.pop();
                    if dir == Some(i64::from(self.inverse_mode)) {
                        self.direction = Direction::West;
                    } else if dir == Some(i64::from(!self.inverse_mode)) {
                        self.direction = Direction::East;
                    } else {
                        return Err(BefreakError::NonBoolInControlStack);
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
                    self.control_stack.push(i64::from(self.inverse_mode));
                }
                Direction::West => {
                    self.direction = Direction::North;
                    self.control_stack.push(i64::from(!self.inverse_mode));
                }
                Direction::South => {
                    let dir = self.control_stack.pop();
                    if dir == Some(i64::from(self.inverse_mode)) {
                        self.direction = Direction::East;
                    } else if dir == Some(i64::from(!self.inverse_mode)) {
                        self.direction = Direction::West;
                    } else {
                        return Err(BefreakError::NonBoolInControlStack);
                    }
                }
                Direction::North => {
                    self.toggle_control_stack();
                    self.inverse_mode = !self.inverse_mode;
                    self.direction = Direction::South;
                }
            },
            ' ' => (),
            _ => unreachable!(),
        };
        Ok(())
    }

    fn pop_main(&mut self) -> Result<i64, BefreakError> {
        self.stack.pop().ok_or(BefreakError::EmptyMainStack)
    }

    fn pop_ctrl(&mut self) -> Result<i64, BefreakError> {
        self.control_stack
            .pop()
            .ok_or(BefreakError::EmptyControlStack)
    }

    fn toggle_control_stack(&mut self) {
        *self.control_stack.last_mut().unwrap() ^= 1;
    }

    fn end(&mut self) {
        self.finished = true;
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn execute<F: Future<Output = ()> + Send + 'static>(f: F) {
    // this is stupid... use any executor of your choice instead
    std::thread::spawn(move || futures::executor::block_on(f));
}

#[cfg(target_arch = "wasm32")]
fn execute<F: Future<Output = ()> + 'static>(f: F) {
    wasm_bindgen_futures::spawn_local(f);
}
