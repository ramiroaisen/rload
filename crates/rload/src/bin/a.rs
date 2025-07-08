#[cfg(feature = "jemalloc")]
#[global_allocator]
static ALLOC: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

#[cfg(feature = "mimalloc")]
#[global_allocator]
static ALLOC: mimalloc::MiMalloc = mimalloc::MiMalloc;

fn main() -> Result<(), anyhow::Error> {
  let report = rload::cli::run()?;
  eprintln!("{}", report.to_string().replace('\n', "\nA | "));
  Ok(())
}