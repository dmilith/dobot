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

    let mut core = Core::new().unwrap();

    let config = Config {
        nickname: Some("gsdhbfhugdf".to_owned()),
        server: Some("chat.freenode.net".to_owned()),
        channels: Some(vec![
           // "#scala.pl".to_owned(),
           "#verknowsys".to_owned()
        ]),
        use_ssl: Some(true),
        burst_window_length: Some(4),
        max_messages_in_burst: Some(2),
        ..Default::default()
    };

    // Create our own local event loop
    // use futures::future::{ok, loop_fn, Future, FutureResult, Loop};
    // let timer = Timer::default();
    // let _timer = timer.interval(Duration::from_millis(10000)).for_each(move |_| {
    //     println!("timeout");
    //     Ok(())
    // });
    // loop_fn(vec!(), |client| {
    // });
    // use std::collections::HashMap;
    // type HM = HashMap<char, usize>;
    // let mut dict = HM::new();
    // String::from("some string like this").chars().for_each(|ch| {
    //     let item = dict.entry(ch).or_insert(0);
    //     *item = *item + 1;
    // });
    // println!("{:?}", dict);

    let client = IrcClient::from_config(config).unwrap();
    client.identify().unwrap();
    client.for_each_incoming(|message| {
        print!("> {}", message);
        if let Command::PRIVMSG(ref target, ref msg) = message.command {
            println!("MSG: {}, LEN: {}", msg, msg.len());
            let eval_head = msg.chars().take(3).collect::<String>();
            let eval_cmd = msg.chars().skip(3).collect::<String>();
            match eval_head.as_ref() {
                "sh:" => client.send_privmsg(
                            message.response_target().unwrap_or(target),
                            &format!("Shell: {} -> {:?}", eval_cmd.clone(), get_result(Shell(eval_cmd), &mut core))
                        ).expect("I should be able to send Sh response to IRC channel"),

                "rs:" => client.send_privmsg(
                            message.response_target().unwrap_or(target),
                            &format!("Rust: {} -> {:?}", eval_cmd.clone(), get_result(Rust(eval_cmd), &mut core))
                        ).expect("I should be able to send Rust response to IRC channel!"),

                _ => ()
            }
            if msg.contains("cebul") {
                client.send_privmsg(target, "😋").unwrap();
            }
        }
    }).unwrap();
}
