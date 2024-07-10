# Aos Operator

### Run operator

```shell
cargo build --features nitro-enclaves --release

./target/release/operator-runer -i postgres://postgres:hetu@0.0.0.0:5432/operator_db

./target/release/operator-runer  -c ./docs/template/config-operator.yaml
```