use std::{env::current_dir, ffi::OsStr, io::Error, path::{Path, PathBuf}, process::{Command, ExitStatus, Stdio}, thread, time};
use clap::Parser;

#[derive(Parser)]
#[command(version, about, long_about = None)]
pub struct Cli {
    // directory containing demo
    #[arg(short, long)]
    pub dir: PathBuf,

    // tmux session name
    #[arg(short, long)]
    pub session_name: String,

    // commands to run in tmux session. expected to be npm commands in dir
    #[arg(required=true)]
    pub commands: Vec<String>
}

fn tmux(args: &[&str], dir: Option<&OsStr>) -> Result<ExitStatus, Error> {
    Command::new("tmux")
        .current_dir(dir.unwrap_or_else(|| Path::new(".").as_os_str()))
        .args(args)
        .status()
}

const HORIZONTAL: &str = "-h";
const VERTICAL: &str = "-v";

macro_rules! select_pane_args {
    ($session:expr, $pane_number:expr) => {
        &["-L", $session, "select-pane", "-t", $pane_number]
    };
}

macro_rules! split_with_command {
    ($session:expr, $direction:expr, $command:expr) => {
        &["-L", $session, "split-window", $direction, $command]
    };
}

// cmds are bash commands. not wrapped in tmux stuff.
fn split_and_run(session: &str, commands: Vec<&str>, dir: Option<&OsStr>) -> Result<(), Error> {
    let mut n = 0;
    let mut split_round = 0;
    let base: i32 = 2;
    while n < commands.len() {
        if n == 0 {
            let args = ["-L", session, "new-session", "-d", "-s", session, commands[n]];
            let result_new_session = tmux(&args, dir);
            if result_new_session.is_err() {
                return Err(result_new_session.unwrap_err())
            }
            n += 1;
        }

        let direction = if split_round % 2 == 0 { HORIZONTAL } else { VERTICAL };

        for i in 0..base.pow(split_round) {
            if n >= commands.len() {
                break
            }
            // tmux -L "$session" select-pane -t "%$i"
            // tmux -L "$session" split-window "$split" "${cmds[n]}"
            let select_pane_result = tmux(select_pane_args!(session, &i.to_string()), dir);
            if select_pane_result.is_err() {
                return Err(select_pane_result.unwrap_err())
            }
            let split_result = tmux(split_with_command!(session, direction, commands[n]), dir);
            if split_result.is_err() {
                return Err(split_result.unwrap_err());
            }
            n += 1;
        }
        split_round += 1;

    }
    //tmux -L "$session" select-layout -t "$session" tiled
    //tmux -L "$session" attach-session -t $session
    let layout_result = tmux(&["-L", session, "select-layout", "-t", session, "tiled"], dir);
    if layout_result.is_err() {
        return Err(layout_result.unwrap_err())
    }
    let attach_result = tmux(&["-L", session, "attach-session", "-t", session], dir);
    if attach_result.is_err() {
        return Err(attach_result.unwrap_err())
    }

    Ok(())
}

// TODO: Log everything.
fn main() {
    let cli = Cli::parse();
    let dir = cli.dir.as_os_str();
    //let demo_script = cli.dir;
    let session_name = cli.session_name;
    let commands = cli.commands;
    //let script = demo_script.file_name().unwrap();
    //let dir = demo_script.parent().unwrap().as_os_str();

    /* let is_ax_running = match Command::new("ps")
        .current_dir(dir)
        .arg("-C")
        .arg("ax")
        .status() {
            Ok(exit_status) => exit_status.success(),
            _ => false
        }; */
    /* if is_ax_running {
        println!("HEJJJ starting ax");
        Command::new("pkill")
        Command::new("ax")
            .current_dir(dir)
            .arg("run");
    } */

    let haha = Command::new("pkill")
        .current_dir(dir)
        .arg("ax")
        .status();
    //println!("{:?}", haha);

    let _ = Command::new("rm")
        .current_dir(dir)
        .arg("-rf")
        .arg("ax-data")
        .status();

    let ax_handle = Command::new("ax")
        .current_dir(dir)
        .arg("run")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn();

    thread::sleep(time::Duration::from_secs(1));

    /* let _ = Command::new("bash")
        .current_dir(dir)
        .arg(script)
        .arg(session_name)
        .spawn(); */

    /* let commands = [
        "npm run start-transport -- WarehouseFactory; exec bash",
        "npm run start-door -- WarehouseFactory; exec bash",
        "npm run start-forklift -- WarehouseFactory; exec bash",
        "npm run start-factory-robot -- WarehouseFactory; exec bash"

    ]; */

    let _ = split_and_run(&session_name, commands.iter().map(|cmd| cmd.as_str()).collect(), Some(dir));

}


/* fn main() {
    let cli = Cli::parse();
    let demo_script = cli.demo_script;
    let session_name = cli.session_name;
    let script = demo_script.file_name().unwrap();
    let dir = demo_script.parent().unwrap().as_os_str();

    let commands = vec![
        "npm run start-transport -- WarehouseFactory; exec bash",
        "npm run start-door -- WarehouseFactory; exec bash",
        "npm run start-forklift -- WarehouseFactory; exec bash",
        "npm run start-factory-robot -- WarehouseFactory; exec bash",
    ].into_iter()
    .map(|c| )

    let tmux = Tmux::with_commands();
    //    .socket_name(session_name)


    let _ = Command::new("bash")
        .current_dir(dir)
        .arg(script)
        .arg(session_name)
        .spawn();

} */