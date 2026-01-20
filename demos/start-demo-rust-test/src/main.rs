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

const HORIZONTAL: &str = "-h";
const VERTICAL: &str = "-v";

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
            select_pane(&OsString::from(i.to_string()))?;
            split_window_with_command(direction, commands[n])?;

            n += 1;
        }
        split_round += 1;

    }
    //tmux -L "$session" select-layout -t "$session" tiled
    //tmux -L "$session" attach-session -t $session
    select_layout()?;
    attach_session()?;
    Ok(())
}

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