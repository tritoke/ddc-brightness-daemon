use ddc_brightness_daemon::BrightnessChange;
use std::{collections::HashMap, ops::Neg, process::ExitCode};
use zbus::{Connection, Result as ZResult, proxy};

const RED: &str = "\x1B[31m";
const RESET: &str = "\x1B[0m";

#[proxy(
    interface = "org.tritoke.Displays",
    default_service = "org.tritoke.Brightness1",
    default_path = "/org/tritoke/Displays"
)]
trait Displays {
    async fn get_display_metadata(&self) -> ZResult<Vec<HashMap<String, String>>>;
    async fn set_absolute(&self, display: usize, amount: u16) -> ZResult<()>;
    async fn change_relative(&self, display: usize, amount: i16) -> ZResult<()>;
    async fn list_brightness(&self) -> ZResult<Vec<u16>>;
}

#[tokio::main]
async fn main() -> ExitCode {
    let Args {
        action,
        display,
        list,
    } = match parse_args() {
        Ok(args) => args,
        Err(e) => {
            eprintln!("{RED}Failed to parse arguments: {e}{RESET}");
            return ExitCode::FAILURE;
        }
    };

    let connection = match Connection::session().await {
        Ok(conn) => conn,
        Err(e) => {
            eprintln!("{RED}Failed to connect to dbus: {e}{RESET}");
            return ExitCode::FAILURE;
        }
    };

    let proxy = &match DisplaysProxy::new(&connection).await {
        Ok(conn) => conn,
        Err(e) => {
            eprintln!("{RED}Failed to connect to org.tritoke.Brightness1: {e}{RESET}");
            eprintln!("Is the daemon running?");
            return ExitCode::FAILURE;
        }
    };

    if action.is_noop() && !list {
        return ExitCode::SUCCESS;
    }

    let context = match Context::try_new(proxy).await {
        Ok(ctx) => ctx,
        Err(e) => {
            eprintln!("{RED}Failed to query for display context: {e}{RESET}");
            return ExitCode::FAILURE;
        }
    };

    if list {
        println!("Detected displays:");
        for (i, meta) in context.metadata.into_iter().enumerate() {
            println!(
                "  - [{i}]: {name} - ({id1}:{id2}:{serial}), manufactured week {week} of {year}",
                name = meta["model_name"],
                id1 = meta["manufacturer_id"],
                id2 = meta["model_id"],
                serial = meta["serial"],
                week = meta["manufacture_week"],
                year = meta["manufacture_year"],
            );
        }

        return ExitCode::SUCCESS;
    }

    if let Some(n) = display {
        if n < context.metadata.len() {
            return action.execute(&context, n).await;
        } else {
            eprintln!("{RED}No display {n}{RESET}");
            return ExitCode::FAILURE;
        }
    }

    let mut exit_code = ExitCode::SUCCESS;
    for i in 0..context.metadata.len() {
        if action.execute(&context, i).await == ExitCode::FAILURE {
            exit_code = ExitCode::FAILURE;
        }
    }

    exit_code
}

#[derive(Clone, Copy)]
enum Action {
    Change(BrightnessChange),
    Get,
}

impl Action {
    fn is_noop(self) -> bool {
        matches!(self, Action::Change(BrightnessChange::Relative(0)))
    }

    async fn execute(self, context: &Context<'_>, on: usize) -> ExitCode {
        let model_name = context.metadata[on]["model_name"].as_str();
        let disp = format!("display {on} ({model_name})");
        let old_value = context.brightnesses[on];

        match self {
            Action::Change(change) => {
                let new_value = change.apply(old_value);
                if old_value == new_value {
                    println!("No change needed for {disp}");
                    return ExitCode::SUCCESS;
                }

                println!(
                    "Changing brightness of display {on} ({model_name}) from {old_value}% to {new_value}"
                );
                let res = match change {
                    BrightnessChange::Relative(by) => context.proxy.change_relative(on, by).await,
                    BrightnessChange::Absolute(to) => context.proxy.set_absolute(on, to).await,
                };

                if let Err(e) = res {
                    eprintln!("{RED}Failed to set brightness for {disp}: {e}{RESET}");
                    return ExitCode::FAILURE;
                }
            }
            Action::Get => {
                println!("display {on} ({model_name}) is set to {old_value}% brightness");
            }
        }

        ExitCode::SUCCESS
    }
}

struct Args {
    action: Action,
    display: Option<usize>,
    list: bool,
}

fn parse_args() -> Result<Args, lexopt::Error> {
    use lexopt::prelude::*;

    let mut parser = lexopt::Parser::from_env();
    let mut display = None;
    let mut action = Action::Get;
    let mut list = false;
    while let Some(arg) = parser.next()? {
        match arg {
            Short('d') | Long("display") => {
                display = Some(parser.value()?.parse()?);
            }
            Long("inc") => {
                action = Action::Change(BrightnessChange::Relative(parser.value()?.parse()?));
            }
            Long("dec") => {
                action = Action::Change(BrightnessChange::Relative(
                    parser.value()?.parse::<i16>()?.neg(),
                ));
            }
            Long("set") => {
                action = Action::Change(BrightnessChange::Absolute(parser.value()?.parse()?));
            }
            Long("get") => action = Action::Get,
            Short('l') | Long("list") => list = true,
            Short('v') | Long("version") => {
                println!("{} v{}", env!("CARGO_BIN_NAME"), env!("CARGO_PKG_VERSION"));
                std::process::exit(0);
            }
            Short('h') | Long("help") => {
                println!(
                    "Usage: ddc-brightness-ctl [-h|--help] [-v|--version] [-d|--display=NUM] [-l|--list] [--inc=NUM] [--dec=NUM] [--set=NUM]"
                );
                println!();
                println!("Options:");
                println!("  -d,    --display: optionally specify which display to change");
                println!("                    default operates on all displays");
                println!("  -l,       --list: list all detected displays and metadata");
                println!("  -v,    --version: get the program version");
                println!("  -h,       --help: print this help message");
                println!("             --get: get the current brightness");
                println!("             --set: set brightness to NUM percent");
                println!("             --inc: increase brightness by NUM percent");
                println!("             --dec: decrease brightness by NUM percent");
                std::process::exit(0);
            }
            _ => return Err(arg.unexpected()),
        }
    }

    Ok(Args {
        action,
        display,
        list,
    })
}

struct Context<'a> {
    metadata: Vec<HashMap<String, String>>,
    brightnesses: Vec<u16>,
    proxy: &'a DisplaysProxy<'a>,
}

impl<'a> Context<'a> {
    async fn try_new(proxy: &'a DisplaysProxy<'_>) -> ZResult<Self> {
        let all_metadata = proxy.get_display_metadata().await?;
        let brightnesses = proxy.list_brightness().await?;
        assert_eq!(
            all_metadata.len(),
            brightnesses.len(),
            "The daemon is fucked, good luck o7 (if you actually hit this please tell me...)"
        );

        Ok(Self {
            metadata: all_metadata,
            brightnesses,
            proxy,
        })
    }
}
