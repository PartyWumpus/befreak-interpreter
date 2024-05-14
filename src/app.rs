use array2d::Array2D;
use phf::phf_map;

use instant::Instant;
use std::future::Future;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::time::Duration;

// for file read
// use std::fs::File;
// use std::io::{self, BufRead};
// use std::path::Path;

// TODO:
// changing grid size
// make pasting with newlines functional maybe
// split stuff into several files
// fix adding/removing start points
// figure out a better way to format the stack ui so they don't overflow

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

"error test" => r#"
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
       >)u%b/       
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
    InvalidOperation,
    EmptyMainStack,
    EmptyControlStack,
    EmptyOutputStack,
    NonBoolInControlStack,
    InvalidUnduplicate,
    InvalidPopZero,
    InvalidUnder,
    InvalidStringRemoval,
}

impl std::error::Error for BefreakError {}

impl std::fmt::Display for BefreakError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let str = match self {
            Self::InvalidPosition => "Tried to enter a position outside the grid",
            Self::InvalidOperation => "Tried to run an invalid operator",
            Self::EmptyMainStack => "Tried to pop off the stack but it was empty",
            Self::EmptyControlStack => "Tried to pop off the control stack but it was empty",
            Self::EmptyOutputStack => "Tried to pop off the output stack but it was empty",
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
            Self::InvalidStringRemoval => "Tried to remove a string but it did not match",
        };
        write!(f, "{str}")
    }
}

#[derive(Debug)]
enum ExecutionState {
    NotStarted,
    Running,
    Done,
    Error(BefreakError),
}

#[derive(Debug)]
struct BefreakState {
    stack: Vec<i64>,
    control_stack: Vec<i64>,
    location: (usize, usize),
    direction: Direction,
    output_stack: Vec<i64>,
    direction_reversed: bool,
    inverse_mode: bool,
    string_mode: bool,
    number_stack: Vec<char>,

    start_pos: (usize, usize),
    state: ExecutionState,
    step: u64,

    // constants
    code: Array2D<char>,
}

pub struct AppState {
    befreak_state: BefreakState,
    speed: f32,
    cursor_position: (usize, usize),
    paused: bool,
    time_since_step: Instant,
    time_since_cursor: Instant,
    show_cursor: bool,
    extra: bool,
    text_channel: (Sender<String>, Receiver<String>),
}

impl AppState {
    /// Called once before the first frame.
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        // This is also where you can customize the look and feel of egui using
        // `cc.egui_ctx.set_visuals` and `cc.egui_ctx.set_fonts`.

        Self {
            befreak_state: BefreakState::new_empty(),
            //state: State::new_load_file("primes1"),
            text_channel: channel(),
            cursor_position: (0, 0),
            time_since_step: Instant::now(),
            time_since_cursor: Instant::now(),
            paused: true,
            show_cursor: true,
            extra: false,
            speed: 5.0,
        }
    }

    fn reset(&mut self) {
        self.befreak_state.reset();
    }

    fn step(&mut self) {
        self.befreak_state.checked_step();

        if !matches!(self.befreak_state.state, ExecutionState::Running) {
            self.paused = true;
        }
    }

    fn load(&mut self, data: &str) {
        self.befreak_state = BefreakState::new_from_string(data);
        self.paused = true;
    }

    fn new_file(&mut self) {
        self.befreak_state = BefreakState::new_empty();
        self.paused = true;
    }

    fn reverse_direction(&mut self) {
        self.befreak_state.checked_reverse_direction();
    }
}

impl eframe::App for AppState {
    /// Called each time the UI needs repainting, which may be many times per second.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Put your widgets into a `SidePanel`, `TopBottomPanel`, `CentralPanel`, `Window` or `Area`.
        // For inspiration and more examples, go to https://emilk.github.io/egui
        //
        if let Ok(text) = self.text_channel.1.try_recv() {
            self.load(&text);
        }

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            // The top panel is often a good place for a menu bar:

            egui::menu::bar(ui, |ui| {
                // NOTE: no File->Quit on web pages!
                let is_web = cfg!(target_arch = "wasm32");
                ui.menu_button("File", |ui| {
                    if !is_web && ui.button("Quit").clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                    if ui.button("New File").clicked() {
                        self.new_file();
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
                        let contents = self.befreak_state.serialize();
                        execute(async move {
                            let file = task.await;
                            if let Some(file) = file {
                                _ = file.write(contents.as_bytes()).await;
                            }
                        });
                    }
                });

                ui.menu_button("Settings", |ui| {
                    ui.checkbox(&mut self.extra, "extra info");
                });

                ui.menu_button("Presets", |ui| {
                    for key in PRESETS.keys() {
                        if ui.button(*key).clicked() {
                            match PRESETS.get(key) {
                                None => unreachable!(),
                                Some(data) => self.load(data),
                            }
                        }
                    }
                });

                ui.add_space(16.0);

                egui::widgets::global_dark_light_mode_buttons(ui);
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            let mut direction = None;
            if ui.input(|e| e.key_pressed(egui::Key::ArrowDown)) {
                direction = Some(Direction::South);
            } else if ui.input(|e| e.key_pressed(egui::Key::ArrowUp)) {
                direction = Some(Direction::North);
            } else if ui.input(|e| e.key_pressed(egui::Key::ArrowLeft)) {
                direction = Some(Direction::West);
            } else if ui.input(|e| e.key_pressed(egui::Key::ArrowRight)) {
                direction = Some(Direction::East);
            } else {
                ui.input(|e| {
                    for event in e.filtered_events(&egui::EventFilter {
                        tab: true,
                        escape: false,
                        horizontal_arrows: true,
                        vertical_arrows: true,
                    }) {
                        match event {
                            egui::Event::Text(text) | egui::Event::Paste(text) => {
                                for char in text.chars() {
                                    let _ = self.befreak_state.code.set(
                                        self.cursor_position.1,
                                        self.cursor_position.0,
                                        char,
                                    );
                                    if self.cursor_position
                                        == (
                                            self.befreak_state.code.row_len() - 1,
                                            self.befreak_state.code.column_len() - 1,
                                        )
                                    {
                                        self.cursor_position = (0, 0);
                                    } else if self.cursor_position.0
                                        >= self.befreak_state.code.row_len() - 1
                                    {
                                        self.cursor_position = (0, self.cursor_position.1 + 1);
                                    } else {
                                        self.cursor_position =
                                            (self.cursor_position.0 + 1, self.cursor_position.1);
                                    }
                                }
                                self.show_cursor = true;
                                self.time_since_cursor = Instant::now();
                            }
                            _ => (),
                        }
                    }
                });
            }

            if let Some(direction) = direction {
                self.cursor_position = self
                    .befreak_state
                    .move_location(self.cursor_position, direction);
                self.show_cursor = true;
                self.time_since_cursor = Instant::now();
            }
            let time_per_step = Duration::from_millis((500.0 - 49.0 * self.speed) as u64);
            let elapsed = self.time_since_step.elapsed();
            if !self.paused {
                if elapsed >= time_per_step {
                    self.step();
                    self.time_since_step = Instant::now();
                }

                ui.ctx().request_repaint_after(time_per_step);
            }

            // The central panel the region left after adding TopPanel's and SidePanel's
            ui.horizontal(|ui| {
                ui.heading("Befreak interpreter");
                if let ExecutionState::Error(error) = &self.befreak_state.state {
                    ui.label(egui::RichText::new(error.to_string()).color(egui::Color32::RED));
                };
            });

            ui.horizontal(|ui| {
                ui.add_enabled_ui(
                    !matches!(self.befreak_state.state, ExecutionState::Error(..)),
                    |ui| {
                        if ui.button("step").clicked() {
                            self.step();
                            if matches!(self.befreak_state.state, ExecutionState::Running) {
                                self.paused = true;
                            }
                        };
                        if ui
                            .button(if self.paused { "unpause" } else { "pause" })
                            .clicked()
                        {
                            self.paused = !self.paused;
                        };
                    },
                );

                ui.add_enabled_ui(
                    !matches!(self.befreak_state.state, ExecutionState::NotStarted),
                    |ui| {
                        if ui
                            .button(if self.befreak_state.direction_reversed {
                                "go forwards"
                            } else {
                                "go backwards"
                            })
                            .clicked()
                        {
                            self.reverse_direction();
                        }
                    },
                );

                if ui.button("restart").clicked() {
                    self.reset();
                    self.paused = true;
                }
                ui.add(egui::Slider::new(&mut self.speed, 1.0..=10.0).text("speed"));
            });

            ui.separator();

            ui.horizontal(|ui| {
                ui.columns(3, |cols| {
                    cols[0].vertical_centered_justified(|ui| {
                        ui.label("output");
                        let output = &self
                            .befreak_state
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
                            for value in &self.befreak_state.stack {
                                ui.label(value.to_string());
                            }
                        })
                    });
                    cols[2].vertical_centered_justified(|ui| {
                        ui.label("control stack");
                        ui.horizontal(|ui| {
                            for value in &self.befreak_state.control_stack {
                                ui.label(value.to_string());
                            }
                        })
                    });
                });
            });

            ui.separator();

            if self.extra {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label("execution state");
                        ui.label(format!("{:?}", self.befreak_state.state));
                    });
                    ui.vertical(|ui| {
                        ui.label("step");
                        ui.label(self.befreak_state.step.to_string());
                    });
                    ui.vertical(|ui| {
                        ui.label("location");
                        ui.label(format!("{:?}", self.befreak_state.location));
                    });
                    ui.vertical(|ui| {
                        ui.label("direction");
                        ui.label(format!("{:?}", self.befreak_state.direction));
                    });
                    ui.vertical(|ui| {
                        ui.label("inverse mode");
                        ui.label(format!("{:?}", self.befreak_state.inverse_mode));
                    });
                    ui.vertical(|ui| {
                        ui.label("string mode");
                        ui.label(format!("{:?}", self.befreak_state.string_mode));
                    });
                    ui.vertical(|ui| {
                        ui.label("time diff");
                        ui.label(format!("{elapsed:.3?}"));
                    });
                    for value in &self.befreak_state.stack {
                        ui.label(String::from(*value as u8 as char));
                    }
                });
                ui.separator();
            }

            let position_color = match self.befreak_state {
                BefreakState {
                    inverse_mode: true,
                    string_mode: true,
                    ..
                } => egui::Color32::LIGHT_RED,
                BefreakState {
                    inverse_mode: true,
                    string_mode: false,
                    ..
                } => egui::Color32::RED,
                BefreakState {
                    inverse_mode: false,
                    string_mode: true,
                    ..
                } => egui::Color32::LIGHT_BLUE,
                BefreakState {
                    inverse_mode: false,
                    string_mode: false,
                    ..
                } => egui::Color32::BLUE,
            };

            let cursor_flash_delay = if self.show_cursor {
                Duration::from_millis(310u64)
            } else {
                Duration::from_millis(295u64)
            };
            let elapsed = self.time_since_cursor.elapsed();
            if elapsed >= cursor_flash_delay {
                self.show_cursor = !self.show_cursor;
                self.time_since_cursor = Instant::now();
            }

            ui.ctx().request_repaint_after(time_per_step);

            egui::Grid::new("letter_grid")
                .spacing([0.0, 0.0])
                .show(ui, |ui| {
                    ui.spacing_mut().item_spacing.x = 0.0;
                    for (index_y, row) in self.befreak_state.code.rows_iter().enumerate() {
                        for (index_x, c) in row.enumerate() {
                            if self.show_cursor && self.cursor_position == (index_x, index_y) {
                                ui.label(
                                    egui::RichText::new(*c)
                                        .background_color(egui::Color32::GRAY)
                                        .family(egui::FontFamily::Monospace),
                                );
                            } else if self.befreak_state.location == (index_x, index_y) {
                                ui.label(
                                    egui::RichText::new(*c)
                                        .background_color(position_color)
                                        .family(egui::FontFamily::Monospace),
                                );
                            } else {
                                ui.label(
                                    egui::RichText::new(*c).family(egui::FontFamily::Monospace),
                                );
                            }
                        }
                        ui.end_row();
                    }
                });

            ui.separator();

            ui.add(egui::github_link_file!(
                "https://github.com/PartyWumpus/befreak-interpreter",
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

impl BefreakState {
    /*
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
    }*/

    fn new(location: (usize, usize), code: Array2D<char>) -> Self {
        Self {
            location,
            code,
            start_pos: location,

            stack: vec![],
            control_stack: vec![],
            direction: Direction::East,
            output_stack: vec![],
            direction_reversed: false,
            inverse_mode: false,
            string_mode: false,
            number_stack: vec![],
            step: 0,
            state: ExecutionState::NotStarted,
        }
    }

    fn new_from_string(data: &str) -> Self {
        let mut lines = vec![];
        let max_length = data.lines().map(str::len).max().unwrap();
        for line in data.lines() {
            // TODO: this is dumb don't just
            // skip lines with no content
            if !line.is_empty() {
                let mut x = line.chars().collect::<Vec<char>>();
                x.resize(max_length, ' ');
                lines.push(x);
            }
        }
        let code = Array2D::from_rows(&lines).unwrap();

        match Self::get_start_pos(&code) {
            None => panic!("No start position"),
            Some(location) => Self::new(location, code),
        }
    }

    fn new_empty() -> Self {
        let mut code = Array2D::filled_with(' ', 10, 10);
        let _ = code.set(1, 1, '@');

        Self::new((1,1), code)
    }

    fn reset(&mut self) {
        // TODO: remove the clone here, maybe check if this is optimized out or not.
        *self = Self::new(self.start_pos, self.code.clone())
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

    fn checked_reverse_direction(&mut self) {
        let run_step = matches!(self.state, ExecutionState::Running);

        match self.reverse_direction(run_step) {
            Ok(..) => (),
            Err(err) => self.state = ExecutionState::Error(err),
        }
    }

    // TODO: fix reversing while processing a number
    // // possibly do by just having numbers processed all in one go
    // // would be nice if the number actually entered the stack BEFORE
    // // the next operation, which this would do
    fn reverse_direction(&mut self, run_step: bool) -> Result<(), BefreakError> {
        self.direction_reversed = !self.direction_reversed;
        self.direction = match self.direction {
            Direction::North => Direction::South,
            Direction::South => Direction::North,
            Direction::East => Direction::West,
            Direction::West => Direction::East,
        };
        self.inverse_mode = !self.inverse_mode;
        if matches!(self.state, ExecutionState::Error(..)) {
            self.state = ExecutionState::Running;
        };
        if run_step {
            self.recover_from_state();
            self.process_instruction()?;
        }
        Ok(())
    }

    fn move_location(&self, location: (usize, usize), direction: Direction) -> (usize, usize) {
        let loc;
        match direction {
            Direction::North => {
                if location.1 == 0 {
                    loc = (location.0, self.code.column_len() - 1);
                } else {
                    loc = (location.0, location.1 - 1);
                }
            }
            Direction::South => {
                if location.1 + 1 >= self.code.column_len() {
                    loc = (location.0, 0);
                } else {
                    loc = (location.0, location.1 + 1);
                }
            }
            Direction::West => {
                if location.0 == 0 {
                    loc = (self.code.row_len() - 1, location.1);
                } else {
                    loc = (location.0 - 1, location.1);
                }
            }
            Direction::East => {
                if location.0 + 1 >= self.code.row_len() {
                    loc = (0, location.1);
                } else {
                    loc = (location.0 + 1, location.1);
                }
            }
        };
        loc
    }

    // TODO: this is a shit name. rename it
    fn recover_from_state(&mut self) {
        match self.state {
            ExecutionState::Done => {
                if !self.direction_reversed {
                    self.reset();
                }
                self.state = ExecutionState::Running;
            }

            ExecutionState::NotStarted => {
                if self.direction_reversed {
                    self.reset();
                }
                self.state = ExecutionState::Running;
            }

            ExecutionState::Error(..) | ExecutionState::Running => (),
        }
    }

    fn checked_step(&mut self) {
        self.recover_from_state();

        if matches!(self.state, ExecutionState::Running) {
            match self.step() {
                Ok(..) => (),
                Err(err) => self.state = ExecutionState::Error(err),
            }
        }
    }

    fn step(&mut self) -> Result<(), BefreakError> {
        // http://tunes.org/~iepos/befreak.html#reference

        self.location = self.move_location(self.location, self.direction);

        if self.direction_reversed {
            self.step -= 1;
        } else {
            self.step += 1;
        }

        self.process_instruction()?;
        Ok(())
    }

    fn process_instruction(&mut self) -> Result<(), BefreakError> {
        if self.string_mode {
            let char = self.get_instruction(self.location)?;
            if *char == '"' {
                self.string_mode = false;
            } else if self.inverse_mode {
                let char = *char as i64;
                let current = self.pop_main()?;
                if char != current {
                    self.stack.push(current);
                    return Err(BefreakError::InvalidStringRemoval);
                };
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
                let x = self.pop_main()?;
                if x != 0 {
                    self.stack.push(x);
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
                let [top, next] = self.pop_many()?;
                self.stack.push(next.overflowing_add(top).0);
                self.stack.push(top);
            }
            // Subtract the top item from the next item
            '-' => {
                let [top, next] = self.pop_many()?;
                self.stack.push(next.overflowing_sub(top).0);
                self.stack.push(top);
            }

            // Divide next by top, leaving a quotient and remainder
            // [y] [x] -> [y/x] [y%x] [x]
            '%' => {
                let [x, y] = self.pop_many()?;
                self.stack.push(y / x);
                self.stack.push(y % x);
                self.stack.push(x);
            }
            // Undo the effects of %, using multiplication
            '*' => {
                let [top, remainder, quotient] = self.pop_many()?;
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
                let [x, y, z] = self.pop_many()?;
                self.stack.push(z ^ (y & x));
                self.stack.push(y);
                self.stack.push(x);
            }
            // Bitwise OR top two items, XOR'ing to the third
            // [z] [y] [x] -> [z^(y|x)] [y] [x]
            '|' => {
                let [x, y, z] = self.pop_many()?;
                self.stack.push(z ^ (y | x));
                self.stack.push(y);
                self.stack.push(x);
            }
            // Bitwise XOR the top item to the next item
            // [y] [x] -> [y^x] [x]
            '#' => {
                let [x, y] = self.pop_many()?;
                self.stack.push(y ^ x);
                self.stack.push(x);
            }

            // Rotate means shift with wrapping
            // Rotate "y" to the left "x" bits
            //[y] [x] -> [y'] [x]
            '{' => {
                let [x, y] = self.pop_many()?;
                // TODO: figure out how to make this work well with negative values of x
                // maybe do a manual conversion modulo 64 or similar?
                // also don't just unwrap it smh my head
                self.stack.push(y.rotate_left(u32::try_from(x).unwrap()));
                self.stack.push(x);
            }
            // Rotate "y" to the right "x" bits
            '}' => {
                let [x, y] = self.pop_many()?;
                self.stack.push(y.rotate_right(u32::try_from(x).unwrap()));
                self.stack.push(x);
            }

            // Toggle top of control stack (i.e., XOR it with 1)
            '!' => self.toggle_control_stack()?,

            // If y equals x, toggle top of control stack
            '=' => {
                let [top, next] = self.pop_many()?;
                if next == top {
                    self.toggle_control_stack()?;
                }
                self.stack.push(next);
                self.stack.push(top);
            }

            // If y is less than x, toggle top of control stack
            'l' => {
                let [top, next] = self.pop_many()?;
                if next < top {
                    self.toggle_control_stack()?;
                }
                self.stack.push(next);
                self.stack.push(top);
            }

            // If y is greater than x, toggle top of control stack
            'g' => {
                let [top, next] = self.pop_many()?;
                if next > top {
                    self.toggle_control_stack()?;
                }
                self.stack.push(next);
                self.stack.push(top);
            }

            // Swap the top two items
            's' => {
                let [top, next] = self.pop_many()?;
                self.stack.push(top);
                self.stack.push(next);
            }

            // Dig the third item to the top
            // [z] [y] [x] -> [y] [x] [z]
            'd' => {
                let [x, y, z] = self.pop_many()?;
                self.stack.push(y);
                self.stack.push(x);
                self.stack.push(z);
            }
            // Bury the first item under the next two
            // [z] [y] [x] -> [x] [z] [y]
            'b' => {
                let [x, y, z] = self.pop_many()?;
                self.stack.push(x);
                self.stack.push(z);
                self.stack.push(y);
            }
            // Flip the order of the top three items
            // [z] [y] [x] -> [x] [y] [z]
            'f' => {
                let [x, y, z] = self.pop_many()?;
                self.stack.push(x);
                self.stack.push(y);
                self.stack.push(z);
            }
            // Swap the second and third items
            // [z] [y] [x] -> [y] [z] [x]
            'c' => {
                let [x, y, z] = self.pop_many()?;
                self.stack.push(y);
                self.stack.push(z);
                self.stack.push(x);
            }
            // "Over": dig copy of second item to the top
            // [y] [x] -> [y] [x] [y]
            'o' => {
                let [x, y] = self.pop_many()?;
                self.stack.push(y);
                self.stack.push(x);
                self.stack.push(y);
            }
            // "Under": the inverse of "over"
            // [y] [x] [y] -> [y] [x]
            'u' => {
                let [y1, x, y2] = self.pop_many()?;
                if y1 != y2 {
                    self.stack.push(y2);
                    self.stack.push(x);
                    self.stack.push(y1);
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
                let [x1, x2] = self.pop_many()?;
                if x1 != x2 {
                    self.stack.push(x2);
                    self.stack.push(x1);
                    return Err(BefreakError::InvalidUnduplicate);
                }
                self.stack.push(x1);
            }
            // Enter string mode
            '"' => self.string_mode = true,
            // Toggle inverse mode
            // the doc says "toggle reverse mode", which doesn't make any sense, as a reverse
            // mode toggle would just undo the whole program back to the start
            '?' => self.inverse_mode = !self.inverse_mode,
            // Halt. Also signals the entrance point for the program
            '@' => {
                if self.direction_reversed {
                    self.state = ExecutionState::NotStarted;
                    self.reverse_direction(false)?;
                } else {
                    self.state = ExecutionState::Done;
                }
            }
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
                    let maybe_dir = self.control_stack.pop();
                    match maybe_dir {
                        None => {
                            return Err(BefreakError::EmptyControlStack);
                        }
                        Some(dir) => {
                            if dir == i64::from(self.inverse_mode) {
                                self.direction = Direction::South;
                            } else if dir == i64::from(!self.inverse_mode) {
                                self.direction = Direction::North;
                            } else {
                                self.control_stack.push(dir);
                                return Err(BefreakError::NonBoolInControlStack);
                            }
                        }
                    }
                }
                Direction::East => {
                    self.toggle_control_stack()?;
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
                    let maybe_dir = self.control_stack.pop();
                    match maybe_dir {
                        None => {
                            return Err(BefreakError::EmptyControlStack);
                        }
                        Some(dir) => {
                            if dir == i64::from(self.inverse_mode) {
                                self.direction = Direction::North;
                            } else if dir == i64::from(!self.inverse_mode) {
                                self.direction = Direction::South;
                            } else {
                                self.control_stack.push(dir);
                                return Err(BefreakError::NonBoolInControlStack);
                            }
                        }
                    }
                }
                Direction::West => {
                    self.toggle_control_stack()?;
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
                    let maybe_dir = self.control_stack.pop();
                    match maybe_dir {
                        None => {
                            return Err(BefreakError::EmptyControlStack);
                        }
                        Some(dir) => {
                            if dir == i64::from(self.inverse_mode) {
                                self.direction = Direction::West;
                            } else if dir == i64::from(!self.inverse_mode) {
                                self.direction = Direction::East;
                            } else {
                                return Err(BefreakError::NonBoolInControlStack);
                            }
                        }
                    }
                }
                Direction::South => {
                    self.toggle_control_stack()?;
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
                    let maybe_dir = self.control_stack.pop();
                    match maybe_dir {
                        None => {
                            return Err(BefreakError::EmptyControlStack);
                        }
                        Some(dir) => {
                            if dir == i64::from(self.inverse_mode) {
                                self.direction = Direction::East;
                            } else if dir == i64::from(!self.inverse_mode) {
                                self.direction = Direction::West;
                            } else {
                                self.control_stack.push(dir);
                                return Err(BefreakError::NonBoolInControlStack);
                            }
                        }
                    }
                }
                Direction::North => {
                    self.toggle_control_stack()?;
                    self.inverse_mode = !self.inverse_mode;
                    self.direction = Direction::South;
                }
            },

            ' ' => (),
            _ => return Err(BefreakError::InvalidOperation),
        };
        Ok(())
    }

    fn pop_main(&mut self) -> Result<i64, BefreakError> {
        self.stack.pop().ok_or(BefreakError::EmptyMainStack)
    }

    fn pop_many<const LENGTH: usize>(&mut self) -> Result<[i64; LENGTH], BefreakError> {
        // if this errored mid-way through popping it would become impossible to recover from
        if self.stack.len() < LENGTH {
            Err(BefreakError::EmptyMainStack)
        } else {
            Ok(core::array::from_fn(|_| self.stack.pop().unwrap()))
        }
    }

    fn pop_ctrl(&mut self) -> Result<i64, BefreakError> {
        self.control_stack
            .pop()
            .ok_or(BefreakError::EmptyControlStack)
    }

    fn toggle_control_stack(&mut self) -> Result<(), BefreakError> {
        if self.control_stack.is_empty() {
            return Err(BefreakError::EmptyControlStack);
        }
        *self.control_stack.last_mut().unwrap() ^= 1;
        Ok(())
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
