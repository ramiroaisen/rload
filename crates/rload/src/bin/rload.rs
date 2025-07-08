#[global_allocator]
static ALLOC: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc; 

fn main() -> Result<(), anyhow::Error> {
  let report = rload::cli::run()?;
  eprintln!("{}", report);
  Ok(())
}