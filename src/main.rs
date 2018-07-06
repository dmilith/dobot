// extern crate futures;
extern crate tokio_core;
extern crate tokio_process;
extern crate tokio_timer;
extern crate irc;
extern crate everust;
extern crate tempfile;


use irc::client::prelude::*;
use std::{default::Default, process::Command as Cmd}; // time::Duration
// use futures::Future;
// use tokio_timer::Timer;
use tokio_core::reactor::Core;
use tokio_process::CommandExt;


const CMD_TIMEOUT: u32 = 30; /* command timeout in seconds */


pub enum Modes {
    Shell(String),
    Rust(String),
}

use Modes::*;


fn get_result(mode: Modes, core: &mut Core) -> String {
    use std::error::Error;
    use tempfile::NamedTempFile;
    use std::io::Write;

    match mode {
        Shell(command) => {
            // create command wrapper:
            let wrapper = format!("#!/bin/sh\n{}\n", command);
            let wrapper_tmpfile = NamedTempFile::new().unwrap();
            let wrapper_path = &wrapper_tmpfile.path();
            match write!(&wrapper_tmpfile, "{}", wrapper) {
                Ok(_) => println!("Tmpfile: {:?}. Written data: {:?}", wrapper_tmpfile, wrapper),
                Err(why) => return format!("Couldn't write to {}: {}", wrapper_path.display(), why.description()),
            }

            let wrapper_contents = format!("#!/bin/sh\ntimeout {} sh {}\n", CMD_TIMEOUT, wrapper_path.display());
            let tmpfile = NamedTempFile::new().unwrap();
            let path = &tmpfile.path();
            match write!(&tmpfile, "{}", wrapper_contents) {
                Ok(_) => println!("Tmpfile: {:?}. Written data: {:?}", tmpfile, wrapper_contents),
                Err(why) => return format!("Couldn't write to {}: {}", path.display(), why.description()),
            }

            // spawn asynchronously
            let child = Cmd::new("sh").arg(path).output_async(&mut core.handle());
            match core.run(child) {
                Ok(output) => {
                    let stdout = String::from_utf8(output.stdout).unwrap_or_default();
                    let stdout = stdout.to_string().trim().replace("\n", " ");
                    let stderr = String::from_utf8(output.stderr).unwrap_or_default();
                    let stderr = stderr.to_string().trim().replace("\n", " ");
                    if stdout.len() == 0 && stderr.len() == 0 {
                        format!("Timeout or no output!")
                    } else if stderr.len() == 0 {
                        format!("StdOut: {}", stdout)
                    } else if stdout.len() == 0 {
                        format!("StdErr: {}", stderr)
                    } else {
                        format!("StdOut: {}, StdErr: {}", stdout, stderr)
                    }
                },
                Err(e) => format!("{}", e),
            }
        },

        Rust(command) => {
            use everust::eval;
            let code = format!(r##"{}"##, command);
            match eval(&code) {
                Ok(result) => format!("{}", result.to_string().replace("\n", " ")),
                Err(e) => format!("{}", e.to_string().replace("\n", " ")),
            }
        },

    }
}


fn main() {
    let config = Config {
        nickname: Some("dobot".to_owned()),
        nick_password: Some("alaniemakota666".to_owned()),
        server: Some("chat.freenode.net".to_owned()),
        channels: Some(vec![
           "#gynvaelstream".to_owned(),
           "#gynvaelstream-en".to_owned(),
           "#scala.pl".to_owned(),
        ]),
        use_ssl: Some(true),
        burst_window_length: Some(4),
        max_messages_in_burst: Some(2),
        ..Default::default()
    };

    loop {
        let mut core = Core::new().unwrap();
        let client = IrcClient::from_config(config.clone()).unwrap();
        client.identify().unwrap();
        client.for_each_incoming(
            |message| {
                print!("> {}", message);
                if let Command::PRIVMSG(ref target, ref msg) = message.command {
                    println!("MSG: {}, LEN: {}", msg, msg.len());
                    let eval_head = msg.chars().take(3).collect::<String>();
                    let eval_cmd = msg.chars().skip(3).collect::<String>();
                    match eval_head.as_ref() {
                        "sh:" => client.send_privmsg(
                                    message.response_target().unwrap_or(target),
                                    &format!("{}: {}",
                                         message.source_nickname().unwrap_or(target),
                                         get_result(Shell(eval_cmd), &mut core)
                                    )
                                ).unwrap_or_else(|_| println!("I should be able to send Sh response to IRC channel!")),

                        "rs:" => client.send_privmsg(
                                    message.response_target().unwrap_or(target),
                                    &format!("{}: {}",
                                         message.source_nickname().unwrap_or(target),
                                         get_result(Rust(eval_cmd), &mut core)
                                    )
                                ).unwrap_or_else(|_| println!("I should be able to send Rust response to IRC channel!")),

                        _ => ()
                    }
                    if msg.contains("cebul") {
                        client
                            .send_privmsg(target, "😋")
                            .unwrap_or_else(|_| println!("I should be able to send Cebula yum to IRC channel!"));
                    }
                }
            }
        ).map_err(|e| println!("{:?}", e)).unwrap_or_default();
    }
}
