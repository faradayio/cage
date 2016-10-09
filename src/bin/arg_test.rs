#[macro_use]
extern crate clap;

use clap::App;

fn main() {
    let yaml = load_yaml!("cli.yml");
    let cli = App::from_yaml(yaml)
        .version(crate_version!());
    //cli.gen_completions_to("cage", Shell::Fish, &mut std::io::stdout());
    let matches = cli.get_matches();
    println!("{:?}", &matches);
}
