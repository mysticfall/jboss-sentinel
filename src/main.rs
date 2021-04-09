#[macro_use]
extern crate chan;

use std::cmp::max;
use std::collections::HashMap;
use std::process::Command;

use dotproperties::parse_from_file;
use regex::Regex;
use sysinfo::{Pid, ProcessExt, RefreshKind, System, SystemExt};

fn main() {
    let config: HashMap<_, _> = parse_from_file("jboss-sentinel.properties")
        .map(|c| c.into_iter().collect())
        .expect("Missing jboss-sentinel.properties file.");

    let interval = config.get("interval")
        .and_then(|v| str::parse::<u32>(v).ok())
        .map(|v| max(v, 1))
        .unwrap_or(10);

    let command = config.get("command")
        .expect("Missing 'command' property.");

    println!("Starting JBoss Sentinel");
    println!(" - watch interval (seconds): {:?}", interval);
    println!(" - command: {:?}", command);

    let mut system = System::new_with_specifics(RefreshKind::new().with_processes());
    let name_pattern = Regex::new("jboss\\.home\\.dir").unwrap();

    let timer = chan::tick_ms(interval * 1000);

    let mut pid: Option<Pid> = None;

    loop {
        chan_select! {
            timer.recv() => match check_server(&mut system, &name_pattern) {
                Some(p) if pid.filter(|v| &p == v).is_some() => {},
                Some(p) => {
                    pid = Some(p);

                    println!("JBoss process found: {:?}", p)
                },
                None => {
                    println!("No JBoss process found.");

                    match &Command::new("cmd").arg("/C").arg("start").arg("cmd").arg("/C").arg(command).spawn() {
                        Ok(_) => println!("Spawned new server instance: {:?}", command),
                        Err(error) => panic!("Failed to restart the server: {:?}", error)
                    };
                },
            },
        }
    }
}

fn check_server(system: &mut System, name_pattern: &Regex) -> Option<Pid> {
    system.refresh_processes();

    system.get_processes().values()
        .find(|p| p.cmd().to_vec().iter().any(|a| name_pattern.find(a.as_str()).is_some()))
        .map(|p| p.pid())
}
