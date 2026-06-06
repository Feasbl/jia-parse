# jia-parse

[![coverage](https://img.shields.io/endpoint?url=https%3A%2F%2Ffeasbl-cdn.t3.tigrisfiles.io%2Fcoverage%2Fjia-parse%2Fcoverage-badge.json&cacheSeconds=300)](https://feasbl-cdn.t3.tigrisfiles.io/coverage/jia-parse/html/index.html)

`jia-parse` is a command-line parser for planning and modelling files used by Jia.

It currently parses:

- PDDL domain and problem files
- `.jia` / `.jiacp` model files

The crate also exposes a Rust library API for embedding the parsers in other tools.

## Run

Parse PDDL:

```bash
jia-parse pddl --domain domain.pddl --problem problem.pddl
```

Print PDDL parse statistics:

```bash
jia-parse pddl --domain domain.pddl --problem problem.pddl --stats
```

Emit PDDL AST JSON:

```bash
jia-parse pddl --domain domain.pddl --problem problem.pddl --json
```

The JSON output is a single object:

```json
{
  "domain": {},
  "problem": {}
}
```

Parse a `.jia` model:

```bash
jia-parse jia model.jia
```

Emit `.jia` AST JSON:

```bash
jia-parse jia model.jia --json
```

Validate only:

```bash
jia-parse pddl --domain domain.pddl --problem problem.pddl --validate
jia-parse jia model.jia --validate
```

## Examples

Inspectable examples live under `examples/`:

- `examples/pddl/delivery/` - a compact typed PDDL domain and problem
- `examples/pddl/advanced/` - durative actions, numeric fluents, derived predicates, preferences, and constraints
- `examples/jia/job_shop.jia` - CP scheduling model
- `examples/jia/resource_schedule.jia` - CP scheduling model with optional tasks, real bounds, and resource demand
- `examples/jia/production_lp.jia` - LP-style `.jia` model

## Compile

Install from GitHub:

```bash
cargo install --git https://github.com/Feasbl/jia-parse.git --locked
```

After the crate is published to crates.io:

```bash
cargo install jia-parse --locked
```

Build from source:

```bash
cargo build --release
```

The binary will be written to:

```bash
target/release/jia-parse
```

Run tests:

```bash
cargo test
```

Install locally from a checkout:

```bash
cargo install --path .
```

## Library

Use the parser from Rust:

```rust
let domain = jia_parse::pddl::parse_domain_str(domain_source)?;
let problem = jia_parse::pddl::parse_problem_str(problem_source)?;
let model = jia_parse::jia::parse_model_str(jia_source)?;
```
