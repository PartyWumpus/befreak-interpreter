use phf::phf_map;

use instant::Instant;
use std::future::Future;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::time::Duration;

use crate::befreak::{BefreakState, Direction, ExecutionState};

// for file read
// use std::fs::File;
// use std::io::{self, BufRead};
// use std::path::Path;

// TODO:
// changing grid size
// make pasting with newlines functional maybe
// make a CLI interface
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

#[cfg(not(target_arch = "wasm32"))]
fn execute<F: Future<Output = ()> + Send + 'static>(f: F) {
    // this is stupid... use any executor of your choice instead
    std::thread::spawn(move || futures::executor::block_on(f));
}

#[cfg(target_arch = "wasm32")]
fn execute<F: Future<Output = ()> + 'static>(f: F) {
    wasm_bindgen_futures::spawn_local(f);
}
