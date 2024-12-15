fn main() -> Result<(), anyhow::Error> {
  let report = rload::cli::run()?;
  eprintln!("{}", report.to_string().replace('\n', "\nA | "));
  Ok(())
}