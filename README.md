# BashGate

`BashGate` is a small, fail-closed Bash structure parser written in Rust. It
uses `tree-sitter-bash` to turn a Bash source string into a conservative IR:

- simple commands and quote-resolved `argv`
- leading environment assignments
- file redirects
- UTF-8 byte spans
- `&&`, `||`, `|`, `|&`, `;`, `&`, and newline operators

It deliberately does **not** implement a permission system, command execution,
path policy, read/write classification, or a command capability database.

The implementation follows the central safety property of ClaudeCodeRev's
Bash AST walker: return a structured result only when every relevant AST node
is explicitly understood. Unknown syntax, dynamic expansion, parser errors,
and resource-limit failures produce `too_complex` instead of a guessed `argv`.

## Status

The repository contains the BashGate MVP described in
[`docs/BashGate_Rust_Implementation_Plan.md`](docs/BashGate_Rust_Implementation_Plan.md).

Supported in the MVP:

- literal command names and arguments
- unquoted words, single quotes, double quotes, and static concatenation
- leading `NAME=value` assignments
- `>`, `>>`, `<`, `>&`, `>|`, `<&`, `&>`, and `&>>`
- pipelines and compound separators
- source-order command/operator output
- UTF-8 byte offsets
- configurable command/node/command-count/depth/time limits
- JSON CLI and Rust library API

Rejected conservatively:

- command, process, parameter, and arithmetic expansion
- glob, brace, and tilde expansion
- heredocs and here-strings
- subshells, functions, loops, conditionals, case statements, and tests
- malformed trees, `ERROR`/missing nodes, control characters, suspicious
  Unicode whitespace, and parser differentials involving escaped whitespace

## CLI

Build and run:

```bash
cargo run -- parse 'FOO=bar cat "a b.txt" | grep foo >> result.txt'
```

Read from standard input:

```bash
printf '%s\n' 'git status && echo ok' | cargo run -- parse
```

Example output:

```json
{
  "kind": "parsed",
  "source": "git status && echo ok",
  "commands": [
    {
      "text": "git status",
      "argv": ["git", "status"],
      "env": [],
      "redirects": [],
      "span": {"start_byte": 0, "end_byte": 10}
    },
    {
      "text": "echo ok",
      "argv": ["echo", "ok"],
      "env": [],
      "redirects": [],
      "span": {"start_byte": 14, "end_byte": 21}
    }
  ],
  "operators": ["&&"]
}
```

Unsupported input is a normal result:

```json
{
  "kind": "too_complex",
  "reason": "unsupported_node",
  "node_type": "for_statement"
}
```

## Library

```rust
use bashgate::{parse, GateResult};

match parse("cat a | grep x") {
    GateResult::Parsed(program) => {
        assert_eq!(program.commands[0].argv, ["cat", "a"]);
    }
    GateResult::TooComplex(reason) => eprintln!("not statically resolvable: {reason:?}"),
    GateResult::ParseError(error) => eprintln!("parser setup failed: {error:?}"),
}
```

For custom limits:

```rust
use bashgate::{BashGate, ParseLimits};

let gate = BashGate::new(ParseLimits {
    max_command_bytes: 4_096,
    ..ParseLimits::default()
});
let result = gate.parse("echo hello");
```

Default limits mirror the reference design where applicable:

| Limit | Default |
|---|---:|
| command bytes | 10,000 |
| AST nodes | 50,000 |
| simple commands | 1,024 |
| AST depth | 256 |
| parser timeout | 50 ms |

## Design notes

Tree-sitter offsets are UTF-8 byte offsets. BashGate uses byte ranges throughout
and slices the original `str` only after validating that each range is a valid
UTF-8 boundary.

Commands and operators are sorted by `start_byte` before the result is returned.
This avoids relying on recursive AST traversal order for mixed structures such
as `a | b && c | d`.

`OperatorOccurrence` retains a byte span in the Rust IR. Its JSON representation
is intentionally just the operator string so the CLI stays compact.

## Tests and fuzzing

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
```

The `fuzz/` package contains a `cargo-fuzz` target whose minimum property is that
arbitrary UTF-8 input never panics or escapes the `Parsed` / `TooComplex` /
`ParseError` result model.

```bash
cargo install cargo-fuzz
cargo fuzz run parse
```

## Reference

The design was derived from the Bash parsing modules in
`CoderDoubleflower/ClaudeCodeRev` at commit
`ffe4eab7876ae20ca8f3d7d71b783fa30032fd12`, especially:

- `src/utils/bash/ast.ts`
- `src/utils/bash/parser.ts`
- `src/utils/bash/bashParser.ts`
- `src/utils/bash/ParsedCommand.ts`
- `src/utils/bash/treeSitterAnalysis.ts`
- `src/utils/bash/commands.ts`

BashGate copies the parser's conservative structure-extraction principles, not
ClaudeCodeRev's permission decisions.
