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
  cargo {{cmd}} -p rload --no-default-features --features=h1,latency {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h2,latency {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h1,h2,latency {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h1,tls,latency {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h2,tls,latency {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h1,h2,tls,latency {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h1,timeout {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h2,timeout {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h1,h2,timeout {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h1,tls,timeout {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h2,tls,timeout {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h1,h2,tls,timeout {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h1,latency,timeout {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h2,latency,timeout {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h1,h2,latency,timeout {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h1,tls,latency,timeout {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h2,tls,latency,timeout {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h1,h2,tls,latency,timeout {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h1,error-detail {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h2,error-detail {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h1,h2,error-detail {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h1,tls,error-detail {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h2,tls,error-detail {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h1,h2,tls,error-detail {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h1,latency,error-detail {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h2,latency,error-detail {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h1,h2,latency,error-detail {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h1,tls,latency,error-detail {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h2,tls,latency,error-detail {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h1,h2,tls,latency,error-detail {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h1,timeout,error-detail {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h2,timeout,error-detail {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h1,h2,timeout,error-detail {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h1,tls,timeout,error-detail {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h2,tls,timeout,error-detail {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h1,h2,tls,timeout,error-detail {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h1,latency,timeout,error-detail {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h2,latency,timeout,error-detail {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h1,h2,latency,timeout,error-detail {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h1,tls,latency,timeout,error-detail {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h2,tls,latency,timeout,error-detail {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h1,h2,tls,latency,timeout,error-detail {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h1,status-detail {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h2,status-detail {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h1,h2,status-detail {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h1,tls,status-detail {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h2,tls,status-detail {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h1,h2,tls,status-detail {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h1,latency,status-detail {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h2,latency,status-detail {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h1,h2,latency,status-detail {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h1,tls,latency,status-detail {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h2,tls,latency,status-detail {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h1,h2,tls,latency,status-detail {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h1,timeout,status-detail {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h2,timeout,status-detail {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h1,h2,timeout,status-detail {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h1,tls,timeout,status-detail {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h2,tls,timeout,status-detail {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h1,h2,tls,timeout,status-detail {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h1,latency,timeout,status-detail {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h2,latency,timeout,status-detail {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h1,h2,latency,timeout,status-detail {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h1,tls,latency,timeout,status-detail {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h2,tls,latency,timeout,status-detail {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h1,h2,tls,latency,timeout,status-detail {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h1,status-detail,error-detail {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h2,status-detail,error-detail {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h1,h2,status-detail,error-detail {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h1,tls,status-detail,error-detail {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h2,tls,status-detail,error-detail {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h1,h2,tls,status-detail,error-detail {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h1,latency,status-detail,error-detail {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h2,latency,status-detail,error-detail {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h1,h2,latency,status-detail,error-detail {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h1,tls,latency,status-detail,error-detail {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h2,tls,latency,status-detail,error-detail {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h1,h2,tls,latency,status-detail,error-detail {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h1,timeout,status-detail,error-detail {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h2,timeout,status-detail,error-detail {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h1,h2,timeout,status-detail,error-detail {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h1,tls,timeout,status-detail,error-detail {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h2,tls,timeout,status-detail,error-detail {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h1,h2,tls,timeout,status-detail,error-detail {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h1,latency,timeout,status-detail,error-detail {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h2,latency,timeout,status-detail,error-detail {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h1,h2,latency,timeout,status-detail,error-detail {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h1,tls,latency,timeout,status-detail,error-detail {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h2,tls,latency,timeout,status-detail,error-detail {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h1,h2,tls,latency,timeout,status-detail,error-detail {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h1,monoio {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h2,monoio {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h1,h2,monoio {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h1,tls,monoio {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h2,tls,monoio {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h1,h2,tls,monoio {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h1,latency,monoio {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h2,latency,monoio {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h1,h2,latency,monoio {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h1,tls,latency,monoio {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h2,tls,latency,monoio {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h1,h2,tls,latency,monoio {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h1,timeout,monoio {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h2,timeout,monoio {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h1,h2,timeout,monoio {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h1,tls,timeout,monoio {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h2,tls,timeout,monoio {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h1,h2,tls,timeout,monoio {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h1,latency,timeout,monoio {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h2,latency,timeout,monoio {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h1,h2,latency,timeout,monoio {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h1,tls,latency,timeout,monoio {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h2,tls,latency,timeout,monoio {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h1,h2,tls,latency,timeout,monoio {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h1,error-detail,monoio {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h2,error-detail,monoio {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h1,h2,error-detail,monoio {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h1,tls,error-detail,monoio {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h2,tls,error-detail,monoio {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h1,h2,tls,error-detail,monoio {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h1,latency,error-detail,monoio {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h2,latency,error-detail,monoio {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h1,h2,latency,error-detail,monoio {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h1,tls,latency,error-detail,monoio {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h2,tls,latency,error-detail,monoio {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h1,h2,tls,latency,error-detail,monoio {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h1,timeout,error-detail,monoio {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h2,timeout,error-detail,monoio {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h1,h2,timeout,error-detail,monoio {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h1,tls,timeout,error-detail,monoio {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h2,tls,timeout,error-detail,monoio {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h1,h2,tls,timeout,error-detail,monoio {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h1,latency,timeout,error-detail,monoio {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h2,latency,timeout,error-detail,monoio {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h1,h2,latency,timeout,error-detail,monoio {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h1,tls,latency,timeout,error-detail,monoio {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h2,tls,latency,timeout,error-detail,monoio {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h1,h2,tls,latency,timeout,error-detail,monoio {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h1,status-detail,monoio {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h2,status-detail,monoio {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h1,h2,status-detail,monoio {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h1,tls,status-detail,monoio {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h2,tls,status-detail,monoio {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h1,h2,tls,status-detail,monoio {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h1,latency,status-detail,monoio {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h2,latency,status-detail,monoio {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h1,h2,latency,status-detail,monoio {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h1,tls,latency,status-detail,monoio {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h2,tls,latency,status-detail,monoio {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h1,h2,tls,latency,status-detail,monoio {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h1,timeout,status-detail,monoio {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h2,timeout,status-detail,monoio {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h1,h2,timeout,status-detail,monoio {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h1,tls,timeout,status-detail,monoio {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h2,tls,timeout,status-detail,monoio {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h1,h2,tls,timeout,status-detail,monoio {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h1,latency,timeout,status-detail,monoio {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h2,latency,timeout,status-detail,monoio {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h1,h2,latency,timeout,status-detail,monoio {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h1,tls,latency,timeout,status-detail,monoio {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h2,tls,latency,timeout,status-detail,monoio {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h1,h2,tls,latency,timeout,status-detail,monoio {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h1,status-detail,error-detail,monoio {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h2,status-detail,error-detail,monoio {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h1,h2,status-detail,error-detail,monoio {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h1,tls,status-detail,error-detail,monoio {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h2,tls,status-detail,error-detail,monoio {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h1,h2,tls,status-detail,error-detail,monoio {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h1,latency,status-detail,error-detail,monoio {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h2,latency,status-detail,error-detail,monoio {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h1,h2,latency,status-detail,error-detail,monoio {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h1,tls,latency,status-detail,error-detail,monoio {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h2,tls,latency,status-detail,error-detail,monoio {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h1,h2,tls,latency,status-detail,error-detail,monoio {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h1,timeout,status-detail,error-detail,monoio {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h2,timeout,status-detail,error-detail,monoio {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h1,h2,timeout,status-detail,error-detail,monoio {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h1,tls,timeout,status-detail,error-detail,monoio {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h2,tls,timeout,status-detail,error-detail,monoio {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h1,h2,tls,timeout,status-detail,error-detail,monoio {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h1,latency,timeout,status-detail,error-detail,monoio {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h2,latency,timeout,status-detail,error-detail,monoio {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h1,h2,latency,timeout,status-detail,error-detail,monoio {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h1,tls,latency,timeout,status-detail,error-detail,monoio {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h2,tls,latency,timeout,status-detail,error-detail,monoio {{args}}
  cargo {{cmd}} -p rload --no-default-features --features=h1,h2,tls,latency,timeout,status-detail,error-detail,monoio {{args}}


check-all-feat *args:
  @just all-feat check {{args}}

build-all-feat *args:
  @just all-feat build {{args}}

test-all-feat *args:
  @just all-feat test {{args}}

ab *args:
  #!/usr/bin/env -S parallel --shebang --ungroup
  ./target/release/a {{args}} | sed "s/^/A | /"
  ./target/release/b {{args}} | sed "s/^/B | /"


internal-bench *args:
  #!/usr/bin/env -S parallel --shebang --ungroup
  wrk {{args}}   | sed "s/^/WRK   | /"
  rload {{args}} | sed "s/^/RLOAD | /"