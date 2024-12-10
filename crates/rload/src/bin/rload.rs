fn main() -> Result<(), anyhow::Error> {
  let report = rload::cli::run()?;
  println!("{}", report);
  Ok(())
}