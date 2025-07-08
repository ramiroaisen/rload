cfg_if::cfg_if! {
  if #[cfg(all(feature = "jemalloc", feature = "mimalloc"))] { 
    compile_error!("only one of features jemalloc or mimalloc can be enabled at a time");
  }
}

#[cfg(feature = "mimalloc")]
#[global_allocator]
static ALLOC: mimalloc::MiMalloc = mimalloc::MiMalloc;

#[cfg(feature = "jemalloc")]
#[global_allocator]
static ALLOC: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc; 

fn main() -> Result<(), anyhow::Error> {
  let report = rload::cli::run()?;
  eprintln!("{}", report);
  Ok(())
}