pub mod cfg;
pub mod cmds;
pub mod utils;
use clap::Command;
use cmds::SubCmd;

fn main() {
    let m = Command::new("sshp")
        .about("A CLI to Support SSH Dynamic Proxy.")
        .version("0.1.2")
        .subcommands(vec![
            cmds::dynamic_proxy::DynamicProxy::usage().display_order(1),
            cmds::multi_proxy::MultiDynamicProxy::usage().display_order(2),
        ])
        .arg_required_else_help(true)
        .get_matches();
    match m.subcommand() {
        Some(("dynamic_proxy", args)) => {
            if let Err(e) = cmds::dynamic_proxy::DynamicProxy::new().handler(args) {
                utils::print_with_color((e.to_string() + "\n").as_str(), 31, true);
                std::process::exit(1);
            }
        }
        Some(("multi_proxy", args)) => {
            if let Err(e) = cmds::multi_proxy::MultiDynamicProxy::new().handler(args) {
                utils::print_with_color((e.to_string() + "\n").as_str(), 31, true);
                std::process::exit(1);
            }
        }
        _ => {}
    };
}
