use kommandozeile::*;

fn main() -> Result<()> {
    let args = setup_clap::<Args>()
        .color_from(|a| a.color)
        .verbose_from(pkg_name!(), |a| a.verbose)
        .run()?;

    let color = concolor::get(concolor::Stream::Stdout).color();
    let filter = verbosity_filter!(args.verbose.verbosity());
    eprintln!("color: {color}, filter: {filter}");
    eprintln!("args: {args:#?}");

    Ok(())
}

#[derive(Debug, clap::Parser)]
struct Args {
    #[clap(flatten)]
    verbose: Verbose,

    #[clap(flatten)]
    color: Color,

    #[clap(short, long, parse(from_os_str), default_value = "-")]
    input: InputFile,

    #[clap(short, long, parse(from_os_str), default_value = "-")]
    output: OutputFile,
}
