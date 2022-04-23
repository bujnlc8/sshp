pub mod cfg;
pub mod cmds;
use anyhow::Result;
use clap::Command;
use cmds::SubCmd;

fn main() -> Result<()> {
    let m = Command::new("sshp")
        .about("A CLI to Support SSH Dynamic Proxy.")
        .version("0.1.0")
        .subcommands(vec![
            cmds::dynamic_proxy::DynamicProxy::usage().display_order(1),
            cmds::multi_proxy::MultiDynamicProxy::usage().display_order(2),
        ])
        .arg_required_else_help(true)
        .get_matches();
    match m.subcommand() {
        Some(("dynamic_proxy", args)) => cmds::dynamic_proxy::DynamicProxy::handler(args)?,
        Some(("multi_proxy", args)) => cmds::multi_proxy::MultiDynamicProxy::handler(args)?,
        _ => {}
    }
    Ok(())
}
