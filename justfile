rload *args: 
  cargo run --release --bin rload {{args}}

build *args: 
  cargo build --release --bin rload {{args}} 

server *args:
  cargo run --release --bin bench-server -- {{args}}

build-server *args:
  cargo build --release --bin bench-server {{args}}

build-all *args: 
  cargo build --release {{args}} 

all-feat cmd *args: 
  cargo {{cmd}} -p rload --no-default-features --features=h1 {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h2 {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h1,h2 {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h1,tls {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h2,tls {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h1,h2,tls {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h1,timeout {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h2,timeout {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h1,h2,timeout {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h1,tls,timeout {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h2,tls,timeout {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h1,h2,tls,timeout {{args}}

check-all-feat *args:
  @just all-feat check {{args}}

build-all-feat *args:
  @just all-feat build {{args}}

test-all-feat *args:
  @just all-feat test {{args}}

