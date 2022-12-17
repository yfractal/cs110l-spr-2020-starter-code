use crate::dwarf_data::DwarfData;
use nix::sys::ptrace;
use nix::sys::signal;
use nix::sys::wait::{waitpid, WaitPidFlag, WaitStatus};
use nix::unistd::Pid;
use std::mem::size_of;
use std::os::unix::process::CommandExt;
use std::process::Child;
use std::process::Command;

#[derive(Debug)]
pub enum Status {
    /// Indicates inferior stopped. Contains the signal that stopped the process, as well as the
    /// current instruction pointer that it is stopped at.
    Stopped(signal::Signal, usize),

    /// Indicates inferior exited normally. Contains the exit status code.
    Exited(i32),

    /// Indicates the inferior exited due to a signal. Contains the signal that killed the
    /// process.
    Signaled(signal::Signal),
}

/// This function calls ptrace with PTRACE_TRACEME to enable debugging on a process. You should use
/// pre_exec with Command to call this in the child process.
fn child_traceme() -> Result<(), std::io::Error> {
    println!("child_traceme is called");
    ptrace::traceme().or(Err(std::io::Error::new(
        std::io::ErrorKind::Other,
        "ptrace TRACEME failed",
    )))
}

pub struct Inferior {
    child: Child,
}

impl Inferior {
    /// Attempts to start a new inferior process. Returns Some(Inferior) if successful, or None if
    /// an error is encountered.
    pub fn new(target: &str, args: &Vec<String>) -> Option<Inferior> {
        println!("Inferior::new: target={}, args={:?}", target, args);

        let mut cmd = Command::new(target);
        // TODO: why need cmd2
        let cmd2 = cmd.args(args);
        unsafe {
            cmd2.pre_exec(child_traceme);
        }
        let child = cmd2.spawn().ok()?;
        let inferior = Inferior { child };

        match inferior.wait(None).unwrap() {
            Status::Stopped(signal, rip) => {
                println!("The program is stopped, signal={:?}, rip={:?}", signal, rip);
                Some(inferior)
            }
            other => {
                println!("The program is not stopped, return None");
                None
            }
        }
    }

    /// Returns the pid of this inferior.
    pub fn pid(&self) -> Pid {
        nix::unistd::Pid::from_raw(self.child.id() as i32)
    }

    /// Calls waitpid on this inferior and returns a Status to indicate the state of the process
    /// after the waitpid call.
    pub fn wait(&self, options: Option<WaitPidFlag>) -> Result<Status, nix::Error> {
        Ok(match waitpid(self.pid(), options)? {
            WaitStatus::Exited(_pid, exit_code) => Status::Exited(exit_code),
            WaitStatus::Signaled(_pid, signal, _core_dumped) => Status::Signaled(signal),
            WaitStatus::Stopped(_pid, signal) => {
                let regs = ptrace::getregs(self.pid())?;
                println!("[debug] the child is stopped, rip=#{}", regs.rip);
                Status::Stopped(signal, regs.rip as usize)
            }
            other => panic!("waitpid returned unexpected status: {:?}", other),
        })
    }

    pub fn cont(&self) -> Result<Status, nix::Error> {
        println!("inferior cont is called");
        ptrace::cont(self.pid(), None);
        self.wait(None)
    }

    pub fn kill(&mut self) {
        self.child.kill();
        println!("start waiting kill");
        self.child.wait().expect("Failed to wait on child process");
        println!("waiting kill end");
    }

    pub fn backtrace(&self, debug_data: &DwarfData) -> Result<Status, nix::Error> {
        // println!("backtrace is called");
        let regs = ptrace::getregs(self.pid())?;
        let mut instruction_ptr = regs.rip;
        // start of current frame

        // memory: hith ---> low
        // frame:  start_of_frame(frame top) ............ end_of_frame(bottom of frame)
        //              /\                                     /\
        //               |                                      |
        //              rbp                                    rsp
        let mut frame_ptr = regs.rbp;
        // TODO: consider better error handling
        loop {
            // println!("instruction_ptr={}", instruction_ptr);
            let line = debug_data
                .get_line_from_addr(instruction_ptr as usize)
                .unwrap();
            let func = debug_data
                .get_function_from_addr(instruction_ptr as usize)
                .unwrap();

            println!("{} ({}:{})", func, line.file, line.number);

            if func == "main" {
                break;
            }

            // get return address base on fp
            instruction_ptr =
                ptrace::read(self.pid(), (frame_ptr + 8) as ptrace::AddressType)? as u64;

            // walk back throuh frame pointer
            frame_ptr = ptrace::read(self.pid(), frame_ptr as ptrace::AddressType)? as u64;
        }

        self.wait(None)
    }
    pub fn getrip(&self) -> u64 {
        ptrace::getregs(self.pid()).unwrap().rip
    }

    pub fn breakpoint(&mut self, addr: usize) -> Result<u8, nix::Error> {
        self.write_byte(addr, 204)
    }

    // TODO: use gdb to go through this fun
    pub fn write_byte(&mut self, addr: usize, val: u8) -> Result<u8, nix::Error> {
        println!("[debug][write_byte] addr={}", addr);
        let aligned_addr = align_addr_to_word(addr);
        let byte_offset = addr - aligned_addr;
        let word = ptrace::read(self.pid(), aligned_addr as ptrace::AddressType)? as u64;
        let orig_byte = (word >> 8 * byte_offset) & 0xff;
        let masked_word = word & !(0xff << 8 * byte_offset);
        let updated_word = masked_word | ((val as u64) << 8 * byte_offset);
        ptrace::write(
            self.pid(),
            aligned_addr as ptrace::AddressType,
            updated_word as *mut std::ffi::c_void,
        )?;
        Ok(orig_byte as u8)
    }

    pub fn go_back_one_step(&self) -> Result<(), nix::Error> {
        let mut regs = ptrace::getregs(self.pid()).unwrap();
        regs.rip = regs.rip - 1;
        ptrace::setregs(self.pid(), regs)
    }

    pub fn step(&self) -> Result<(), nix::Error> {
        // TODO: check ptrace step wait None vs some trap signal...
        ptrace::step(self.pid(), None)
    }
}

fn align_addr_to_word(addr: usize) -> usize {
    addr & (-(size_of::<usize>() as isize) as usize)
}
