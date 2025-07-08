#[cfg(feature = "mimalloc")]
#[global_allocator]
static ALLOC: mimalloc::MiMalloc = mimalloc::MiMalloc;

#[cfg(feature = "mimalloc")]
#[global_allocator]
static ALLOC: mimalloc::MiMalloc = mimalloc::MiMalloc;

fn main() -> Result<(), anyhow::Error> {
  let report = rload::cli::run()?;
  eprintln!("{}", report.to_string().replace('\n', "\nB | "));
  Ok(())
}