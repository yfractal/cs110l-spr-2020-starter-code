use crate::debugger_command::DebuggerCommand;
use crate::dwarf_data::{DwarfData, Error as DwarfError};
use crate::inferior::{Inferior, Status};
use nix::sys::signal;
use rustyline::error::ReadlineError;
use rustyline::Editor;
use std::collections::HashMap;

pub struct Debugger {
    target: String,
    history_path: String,
    readline: Editor<()>,
    inferior: Option<Inferior>,
    running: bool,
    debug_data: DwarfData,
    breakpoints: Vec<usize>,
    breakpoint_map: HashMap<u64, u8>,
}

impl Debugger {
    /// Initializes the debugger.
    pub fn new(target: &str) -> Debugger {
        // TODO (milestone 3): initialize the DwarfData

        let history_path = format!("{}/.deet_history", std::env::var("HOME").unwrap());
        let mut readline = Editor::<()>::new();
        let _ = readline.load_history(&history_path);

        let debug_data = match DwarfData::from_file(target) {
            Ok(val) => val,
            Err(DwarfError::ErrorOpeningFile) => {
                println!("Could not open file {}", target);
                std::process::exit(1);
            }
            Err(DwarfError::DwarfFormatError(err)) => {
                println!("Could not debugging symbols from {}: {:?}", target, err);
                std::process::exit(1);
            }
        };

        println!("[debug]: debug_data");
        debug_data.print();

        Debugger {
            target: target.to_string(),
            history_path,
            readline,
            inferior: None,
            running: false,
            debug_data,
            breakpoints: Vec::new(),
            breakpoint_map: HashMap::new(),
        }
    }

    pub fn run(&mut self) {
        loop {
            match self.get_next_command() {
                DebuggerCommand::Run(args) => {
                    if self.running {
                        let inferior = self.inferior.as_mut().unwrap();
                        match inferior.kill() {
                            Ok(status) => {
                                println!(
                                    "child process is killed pid={}, status={}",
                                    inferior.pid(),
                                    status
                                );
                            }
                            Err(error) => {
                                println!("Can't kill child, error={}", error)
                            }
                        }

                        self.running = false;
                    }

                    if let Some(inferior) = Inferior::new(&self.target, &args) {
                        self.inferior = Some(inferior);

                        self.running = true;

                        for breakpoint in self.breakpoints.iter() {
                            let orig_byte = self
                                .inferior
                                .as_mut()
                                .unwrap()
                                .breakpoint(*breakpoint)
                                .unwrap();
                            self.breakpoint_map.insert(*breakpoint as u64, orig_byte);
                        }

                        let status = self.inferior.as_ref().unwrap().cont().unwrap();

                        match status {
                            Status::Stopped(signal, rip) => {
                                println!("Child stopped (signal {})", signal);

                                let line =
                                    self.debug_data.get_line_from_addr(rip as usize).unwrap();
                                println!("Stopped at {}:{}", line.file, line.number);

                                // TODO: checking (%rip - 1) matches a breakpoint address
                                if signal == signal::Signal::SIGTRAP {
                                    // TODO: some many self.inferior.as_ref().unwrap() self.inferior.as_mut().unwrap()
                                    let prev_rip = self.inferior.as_ref().unwrap().getrip() - 1;
                                    println!("[debug][breakpoint] prev_rip={}", prev_rip);

                                    // write origin back
                                    let orig_byte = self.breakpoint_map.get(&prev_rip).unwrap();
                                    self.inferior
                                        .as_mut()
                                        .unwrap()
                                        .write_byte(prev_rip as usize, *orig_byte)
                                        .unwrap();

                                    self.inferior.as_ref().unwrap().go_back_one_step().unwrap();
                                    self.inferior.as_ref().unwrap().step().unwrap();
                                    let wait_status =
                                        self.inferior.as_ref().unwrap().wait(None).unwrap();

                                    let current_rip = self.inferior.as_ref().unwrap().getrip();
                                    println!(
                                        "[debug][breakpoing]: .... wait_status={:?}, rip={}",
                                        wait_status, current_rip
                                    );

                                    self.inferior
                                        .as_mut()
                                        .unwrap()
                                        .write_byte(prev_rip as usize, 204)
                                        .unwrap();

                                    self.inferior.as_ref().unwrap().cont().unwrap();
                                }
                            }
                            other => {
                                println!("Child stopped as {:?}", other)
                            }
                        }
                    } else {
                        println!("Error starting subprocess");
                    }
                }
                DebuggerCommand::Continue => self.handle_cont_command(),
                DebuggerCommand::Backtrace => self.handle_backtrace_command(),
                DebuggerCommand::Breakpoint(raw_addr) => self.handle_breakpoint(&raw_addr),
                DebuggerCommand::Quit => self.handle_quit(),
            }
        }
    }

    /// This function prompts the user to enter a command, and continues re-prompting until the user
    /// enters a valid command. It uses DebuggerCommand::from_tokens to do the command parsing.
    ///
    /// You don't need to read, understand, or modify this function.
    fn get_next_command(&mut self) -> DebuggerCommand {
        loop {
            // Print prompt and get next line of user input
            match self.readline.readline("(deet) ") {
                Err(ReadlineError::Interrupted) => {
                    // User pressed ctrl+c. We're going to ignore it
                    println!("Type \"quit\" to exit");
                }
                Err(ReadlineError::Eof) => {
                    // User pressed ctrl+d, which is the equivalent of "quit" for our purposes
                    return DebuggerCommand::Quit;
                }
                Err(err) => {
                    panic!("Unexpected I/O error: {:?}", err);
                }
                Ok(line) => {
                    if line.trim().len() == 0 {
                        continue;
                    }
                    self.readline.add_history_entry(line.as_str());
                    if let Err(err) = self.readline.save_history(&self.history_path) {
                        println!(
                            "Warning: failed to save history file at {}: {}",
                            self.history_path, err
                        );
                    }
                    let tokens: Vec<&str> = line.split_whitespace().collect();
                    if let Some(cmd) = DebuggerCommand::from_tokens(&tokens) {
                        return cmd;
                    } else {
                        println!("Unrecognized command.");
                    }
                }
            }
        }
    }

    fn parse_address(&self, addr: &str) -> Option<usize> {
        let addr_without_0x = if addr.to_lowercase().starts_with("*0x") {
            &addr[3..]
        } else {
            &addr
        };

        match addr_without_0x.parse::<usize>() {
            Ok(line) => self.debug_data.get_addr_for_line(None, line), // doesn't work...
            _ => match self.debug_data.get_addr_for_function(None, addr_without_0x) {
                Some(addr) => Some(addr),
                _ => usize::from_str_radix(addr_without_0x, 16).ok(),
            },
        }
    }

    fn handle_cont_command(&self) {
        if !self.running {
            return println!("Please run the target program first!");
        }

        match self.inferior.as_ref().unwrap().cont() {
            // use :? for saving devlopment time, it's a toy project anyway
            Ok(status) => println!("Child stopped status={:?}", status),
            Err(err) => println!("error={}", err),
        }
    }

    fn handle_backtrace_command(&self) {
        if !self.running {
            return println!("Please run the target program first!");
        }

        let inferior = self.inferior.as_ref().unwrap();
        match inferior.backtrace(&self.debug_data) {
            Ok(status) => println!("Child stopped status={:?}", status),
            Err(err) => println!("error={}", err),
        }
    }

    fn handle_breakpoint(&mut self, raw_addr: &str) {
        let addr = self.parse_address(&raw_addr).unwrap();

        if !self.running {
            self.breakpoints.push(addr);
        } else {
            let inferior = self.inferior.as_mut().unwrap();
            let orig_byte = inferior.breakpoint(addr).unwrap();
            self.breakpoint_map.insert(addr as u64, orig_byte);
        }
    }

    fn handle_quit(&mut self) {
        if !self.running {
            return println!("Please run the target program first!");
        }
        println!(
            "Killing running inferior (pid {})",
            self.inferior.as_ref().unwrap().pid()
        );

        self.inferior.as_mut().unwrap().kill();
    }
}
