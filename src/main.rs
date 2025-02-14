use config::Config;

mod config;
mod db;

fn main() {
    let cli_args: Vec<String> = std::env::args().collect();
    let args: Vec<&str> = cli_args.iter().map(|arg| &arg[..]).collect();
    let config = Config::build(&args);
    let task = config.get_task();
    if !config.import_schema {
        config
            .get_excluded_schemas()
            .iter()
            .for_each(|s| println!("Found ignored table: {}", s.name));
    }
    task.execute();
}
