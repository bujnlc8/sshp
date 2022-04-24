pub mod dynamic_proxy;
pub mod multi_proxy;
use anyhow::Result;
use clap::{ArgMatches, Command};

pub trait SubCmd {
    fn usage<'a>() -> Command<'a>;
    fn handler(arg: &ArgMatches) -> Result<()>;
}
