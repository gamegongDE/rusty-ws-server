pub use clap::Parser;

#[derive(Parser, Debug)]
pub struct Args {
    #[clap(default_value_t = 80)]
    pub port: u32,
}
