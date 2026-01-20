use std::{ffi::{OsStr, OsString}, fs::OpenOptions, io::Error, path::{Path, PathBuf}, process::{Child, Command, ExitCode, ExitStatus, Stdio, exit}, thread, time};
use clap::Parser;
use anyhow::{Context, Result};
use tracing::{error, info, instrument::WithSubscriber};
use tracing_subscriber::{EnvFilter, fmt};

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
    pub commands: Vec<String>,

    #[arg(short, long)]
    pub log_path: PathBuf,
}
/*
#[derive(Debug, Default)]
pub struct CommandSpec {
    program: OsString,
    args: Vec<OsString>,
    dir: Option<PathBuf>,
    stdin: Option<Stdio>,
    stdout: Option<Stdio>,
    stderr: Option<Stdio>,
}

impl CommandSpec {
    fn new(program: impl Into<OsString>) -> Self {
        Self {
            program: program.into(),
            ..Default::default()
        }
    }

    fn arg(mut self, arg: impl Into<OsString>) -> Self {
        self.args.push(arg.into());
        self
    }

    fn args<I, S>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<OsString>,
    {
        self.args.extend(args.into_iter().map(Into::into));
        self
    }

    fn dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.dir = Some(dir.into());
        self
    }
} */

const HORIZONTAL: &str = "-h";
const VERTICAL: &str = "-v";

macro_rules! select_pane_args {
    ($session:expr, $pane_number:expr) => {
        &[OsStr::new("-L"), $session, "select-pane", "-t", $pane_number]
    };
}

macro_rules! split_with_command {
    ($session:expr, $direction:expr, $command:expr) => {
        &[OsStr::new("-L"), $session, "split-window", $direction, $command]
    };
}

// set up tracing
fn init_tracing(path: &Path) {
    let log_file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .expect("failed to open log file");

    let subscriber = fmt::Subscriber::builder()
        .with_env_filter(EnvFilter::from_default_env())
        .with_writer(log_file)
        .with_target(true)
        .with_level(true)
        .finish();

    tracing::subscriber::set_global_default(subscriber)
        .expect("failed to set tracing subscriber");
}

// run a Command using status()
fn run_status(command: &mut Command) -> Result<()> {
    tracing::info!(command = ?command, "running command");

    let status = command
        .status()
        .with_context(|| format!("failed to spawn {:?}", command))?;

    if !status.success() {
        anyhow::bail!("command: {:?}: exited with {}", command, status);
    }

    Ok(())
}

fn spawn(command: &mut Command) -> Result<Child> {
    command
        .spawn()
        .with_context(|| format!("failed to spawn {:?} from dir={:?}", command, command.get_current_dir()))
/* let ax_handle = Command::new("ax")
        .current_dir(dir)
        .arg("run")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn(); */
}

fn spawn_logged(command: &mut Command) -> Result<Child> {
    tracing::info!(command = ?command, "spawning command");
    spawn(command)
}

fn tmux(args: &[&OsStr], dir: Option<&OsStr>) -> Result<()> {
    run_status(Command::new("tmux")
        .current_dir(dir.unwrap_or_else(|| Path::new(".").as_os_str()))
        .args(args))
}

//fn tmux_new_detached_session
/*
macro_rules! select_pane_args {
    ($session:expr, $pane_number:expr) => {
        &[OsStr::new("-L"), $session, "select-pane", "-t", $pane_number]
    };
}

*/

fn tmux1(args: &[&str], dir: Option<&OsStr>) -> Result<ExitStatus, Error> {
    Command::new("tmux")
        .current_dir(dir.unwrap_or_else(|| Path::new(".").as_os_str()))
        .args(args)
        .status()
}

fn rm(args: &[&OsStr], dir: Option<&OsStr>) -> Result<()> {
    run_status(
        Command::new("rm")
            .current_dir(dir.unwrap_or_else(|| Path::new(".").as_os_str()))
            .stderr(Stdio::null())
            .args(args)
    )
}

fn pkill(arg: &OsStr) -> Result<()> {
    run_status(
        Command::new("pkill")
            .arg(arg)
    )
}

fn spawn_ax_run(dir: Option<&OsStr>) -> Result<Child> {
    spawn_logged(Command::new("ax")
        .current_dir(dir.unwrap_or_else(|| Path::new(".").as_os_str()))
        .arg("run")
        .stdout(Stdio::null())
        .stderr(Stdio::null()))
}

// cmds are bash commands. not wrapped in tmux stuff.
fn split_and_run(session: &OsStr, commands: &[&OsStr], dir: Option<&OsStr>) -> Result<()> {
    let mut n = 0;
    let mut split_round = 0;
    let base: i32 = 2;

    let new_detached_session = |command: &OsStr| -> Result<()> {
        tmux(&[OsStr::new("-L"), session, OsStr::new("new-session"), OsStr::new("-d"), OsStr::new("-s"), session, command], dir)
    };
    let select_pane = |pane: &OsStr| -> Result<()> {
        tmux(&[OsStr::new("-L"), session, OsStr::new("select-pane"), OsStr::new("-t"), pane], dir)
    };
    let split_window_with_command = |direction: &OsStr, command: &OsStr| -> Result<()> {
        tmux(&[OsStr::new("-L"), session, OsStr::new("split-window"), direction, command], dir)
    };
    let select_layout = || -> Result<()> {
        tmux(&[OsStr::new("-L"), session, OsStr::new("select-layout"), OsStr::new("-t"), session, OsStr::new("tiled")], dir)
    };
    let attach_session = || -> Result<()> {
        tmux(&[OsStr::new("-L"), session, OsStr::new("attach-session"), OsStr::new("-t"), session], dir)
    };

    //unimplemented!()

    while n < commands.len() {
        if n == 0 {
            //let args = [OsStr::new("-L"), session, OsStr::new("new-session"), OsStr::new("-d"), OsStr::new("-s"), session, commands[n]];
            //tmux(&args, dir)?;
            new_detached_session(commands[n])?;
            n += 1;
        }

        let direction = if split_round % 2 == 0 { OsStr::new(HORIZONTAL) } else { OsStr::new(VERTICAL) };

        for i in 0..base.pow(split_round) {
            if n >= commands.len() {
                break
            }
            // tmux -L "$session" select-pane -t "%$i"
            // tmux -L "$session" split-window "$split" "${cmds[n]}"
            //tmux(select_pane_args!(session, &i.to_string()), dir)?;
            //tmux(split_with_command!(session, direction, commands[n]), dir)?;
            select_pane(&OsString::from(i.to_string()))?;
            split_window_with_command(direction, commands[n])?;

            n += 1;
        }
        split_round += 1;

    }
    //tmux -L "$session" select-layout -t "$session" tiled
    //tmux -L "$session" attach-session -t $session
    //tmux(&["-L", session, "select-layout", "-t", session, "tiled"], dir)?;
    //tmux(&["-L", session, "attach-session", "-t", session], dir)?;
    select_layout()?;
    attach_session()?;
    Ok(())
}

/* fn split_and_run1(session: &str, commands: Vec<&str>, dir: Option<&OsStr>) -> Result<(), Error> {
    let mut n = 0;
    let mut split_round = 0;
    let base: i32 = 2;
    while n < commands.len() {
        if n == 0 {
            let args = ["-L", session, "new-session", "-d", "-s", session, commands[n]];
            let result_new_session = tmux1(&args, dir);
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
            let select_pane_result = tmux1(select_pane_args!(session, &i.to_string()), dir);
            if select_pane_result.is_err() {
                return Err(select_pane_result.unwrap_err())
            }
            let split_result = tmux1(split_with_command!(session, direction, commands[n]), dir);
            if split_result.is_err() {
                return Err(split_result.unwrap_err());
            }
            n += 1;
        }
        split_round += 1;

    }
    //tmux -L "$session" select-layout -t "$session" tiled
    //tmux -L "$session" attach-session -t $session
    let layout_result = tmux1(&["-L", session, "select-layout", "-t", session, "tiled"], dir);
    if layout_result.is_err() {
        return Err(layout_result.unwrap_err())
    }
    let attach_result = tmux1(&["-L", session, "attach-session", "-t", session], dir);
    if attach_result.is_err() {
        return Err(attach_result.unwrap_err())
    }

    Ok(())
}
 */
// delete a file or directory.
/* fn rm(args: &[&OsStr], current_dir: &OsStr) -> Result<ExitStatus, Error> {
    let _span = tracing::info_span!("rm").entered();
    tracing::info!("rm {:?} from {}", args.iter().map(|arg| arg.display()).collect::<Vec<_>>(), current_dir.display());
    let exit_status = Command::new("rm")
        .current_dir(current_dir)
        .args(args)
        .status();
    tracing::info!("rm {:?} from {}. exit status: {:?}", args.iter().map(|arg| arg.display()).collect::<Vec<_>>(), current_dir.display(), exit_status);
    exit_status
} */



/* fn run_status(program: &OsStr, args: &[&OsStr], dir: &OsStr) -> Result<ExitStatus, Error> {
    //let _span = tracing::info_span!("foo").entered();
    tracing::info!("{} {:?} from {}", program.display(), args.iter().map(|arg| arg.display()).collect::<Vec<_>>(), dir.display());
    let exit_status = Command::new(program)
        .current_dir(dir)
        .args(args)
        .status();
    tracing::info!("{} {:?} from {}. exit status: {:?}", program.display(), args.iter().map(|arg| arg.display()).collect::<Vec<_>>(), dir.display(), exit_status);
    exit_status
} */

macro_rules! error_and_exit {
    ($error:expr) => {
        tracing::info!("Exiting due to error: {:?}", error);
        exit(1)
    };
}

/* fn run(cmd: &mut Command) -> anyhow::Result<()> {
    let status = cmd.status()
        .with_context(|| format!("failed to run {:?}", cmd))?;

    if !status.success() {
        anyhow::bail!("command exited with {}", status);
    }

    Ok(())
} */

/* fn run_tmux(cmd: &mut Command) -> Result<()> {
    tracing::info!(command = ?cmd, "spawning command");

    let status = cmd
        // inherit stdout/stderr → visible in terminal / tmux
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .with_context(|| format!("failed to spawn {:?}", cmd))?;

    if !status.success() {
        anyhow::bail!("command {:?} exited with {}", cmd, status);
    }

    Ok(())
} */

fn start_demo(dir: &OsStr, session_name: &OsStr, commands: &[&OsStr]) -> Result<()> {
    tracing::info!("Starting demo. Running {:#?} from {}", commands, dir.display());
    pkill(OsStr::new("ax"))?;
    rm(&[OsStr::new("-rf"), OsStr::new("ax-data")], Some(dir))?;
    spawn_ax_run(Some(dir))?;
    thread::sleep(time::Duration::from_secs(1));
    split_and_run(session_name, commands, Some(dir))?;
    pkill(OsStr::new("ax"))?;
    info!("finished demo");
    Ok(())
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let dir = cli.dir.as_os_str();
    let session_name = OsString::from(cli.session_name);
    let commands = cli.commands.iter().map(|s: &String| OsStr::new(s)).collect::<Vec<_>>();
    let log_path = cli.log_path;

    init_tracing(&log_path);

    if let Err(err) = start_demo(dir, &session_name, &commands) {
        tracing::error!("{:#}", err);
        return ExitCode::from(1);
    }

    ExitCode::SUCCESS
}

// TODO: Log everything.
/* fn main() {
    let cli = Cli::parse();
    let dir = cli.dir.as_os_str();

    let session_name = cli.session_name;
    let commands = cli.commands;

    /* let pkill_result = Command::new("pkill")
        .current_dir(dir)
        .arg("ax")
        .status(); */
    /* let pkill_result = match run_status(OsStr::new("pkill"), &[OsStr::new("ax")], dir) {
        Ok(exit_status) => exit_status,
        Err(error) => exit(1),
    }; */



/*     let _ = Command::new("rm")
        .current_dir(dir)
        .arg("-rf")
        .arg("ax-data")
        .status();

 */

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

} */





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