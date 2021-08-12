#![no_std]
#![no_main]

extern crate alloc;

#[macro_use]
extern crate user_lib;

const LF: u8 = 0x0au8;
const CR: u8 = 0x0du8;
const DL: u8 = 0x7fu8;
const BS: u8 = 0x08u8;

use alloc::string::String;
use alloc::vec::Vec;
use user_lib::{
    fork,
    exec,
    waitpid,
    flush,
    open,
    OpenFlags,
    close,
    dup,
};
use user_lib::console::getchar;
use crate::alloc::string::ToString;

#[no_mangle]
pub fn main() -> i32 {
    println!("Rust user shell");
    let mut line: String = String::new();
    print!(">> ");
    flush();
    loop {
        let c = getchar();
        match c {
            LF | CR => {
                println!("");
                if !line.is_empty() {
                    let pipes: Vec <_> = line.as_str().split('|').collect();
                    let mut ftmp1 = false;
                    let mut ftmp2 = false;
                    let nametmp1 = "ftmp1".to_string();
                    let nametmp2 = "ftmp2".to_string();
                    let odd = false; // 0
                    for (cidx, cmd_line) in pipes.iter().enumerate() {
                        println!("{}", cmd_line);
                        println!("{}", cidx);
                        let args: Vec<_> = cmd_line.split(' ').collect();
                        let mut args_copy: Vec<String> = args
                        .iter()
                        .map(|&arg| {
                            let mut string = String::new();
                            string.push_str(arg);
                            string
                        })
                        .collect();
    
                        args_copy
                        .iter_mut()
                        .for_each(|string| {
                            string.push('\0');
                        });
    
                        // redirect input
                        let mut input = String::new();
                        if let Some((idx, _)) = args_copy
                        .iter()
                        .enumerate()
                        .find(|(_, arg)| arg.as_str() == "<\0") {
                            input = args_copy[idx + 1].clone();
                            args_copy.drain(idx..=idx + 1);
                        }
    
                        // redirect output
                        let mut output = String::new();
                        if let Some((idx, _)) = args_copy
                        .iter()
                        .enumerate()
                        .find(|(_, arg)| arg.as_str() == ">\0") {
                            output = args_copy[idx + 1].clone();
                            args_copy.drain(idx..=idx + 1);
                        }
                        
                        if cidx % 2 == 0 { // 1 as in, 2 as out
                            if input.is_empty() && ftmp1 == true {
                                input = nametmp1.clone();
                            }
                            if output.is_empty() && cidx != (pipes.len() - 1) {
                                output = nametmp2.clone();
                                ftmp2 = true; // next time this file is prepared
                            } else {
                                ftmp2 = false; // next time this file is not prepared
                            }
                        } else if cidx % 2 == 1 { // 2 is in, 1 is out
                            if input.is_empty() && ftmp2 == true {
                                input = nametmp2.clone();
                            } 
                            if output.is_empty() && cidx != (pipes.len() - 1) { // to stdout
                                output = nametmp1.clone();
                                ftmp1 = true;
                            } else {
                                ftmp1 = false;
                            }
                        }
                        let mut args_addr: Vec<*const u8> = args_copy
                            .iter()
                            .map(|arg| arg.as_ptr())
                            .collect();
                        args_addr.push(0 as *const u8);
                        let pid = fork();
                        if pid == 0 {
                            // input redirection u8);
                        let pid = fork();
                            if !input.is_empty() {
                                let input_fd = open(input.as_str(), OpenFlags::RDONLY);
                                if input_fd == -1 {
                                    println!("Error when opening file {}", input);
                                    return -4;
                                }
                                let input_fd = input_fd as usize;
                                close(0);
                                assert_eq!(dup(input_fd), 0);
                                close(input_fd);
                            }
                            // output redirection
                            if !output.is_empty() {
                                let output_fd = open(
                                    output.as_str(),
                                    OpenFlags::CREATE | OpenFlags::WRONLY
                                );
                                if output_fd == -1 {
                                    println!("Error when opening file {}", output);
                                    return -4;
                                }
                                let output_fd = output_fd as usize;
                                close(1);
                                assert_eq!(dup(output_fd), 1);
                                close(output_fd);
                            }
                            // child process
                            if exec(args_copy[0].as_str(), args_addr.as_slice()) == -1 {
                                println!("Error when executing!");
                                return -4;
                            }
                            unreachable!();
                        } else {
                            let mut exit_code: i32 = 0;
                            let exit_pid = waitpid(pid as usize, &mut exit_code);
                            assert_eq!(pid, exit_pid);
                            println!("Shell: Process {} exited with code {}", pid, exit_code);
                        }
                        // cmd_line.clear();
                    }
                    line.clear();
                }
                print!(">> ");
                flush();
            }
            BS | DL => {
                if !line.is_empty() {
                    print!("{}", BS as char);
                    print!(" ");
                    print!("{}", BS as char);
                    flush();
                    line.pop();
                }
            }
            _ => {
                print!("{}", c as char);
                flush();
                line.push(c as char);
            }
        }
    }
}