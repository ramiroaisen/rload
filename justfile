rload *args: 
  cargo run --release --bin rload -- {{args}}

server *args:
  cargo run --release --bin bench-server -- {{args}}