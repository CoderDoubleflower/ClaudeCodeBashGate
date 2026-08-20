# BashGate：Rust 实现计划

> 目标：从 ClaudeCodeRev 中提炼 Bash 命令解析能力，用 Rust 实现一个独立 `BashGate`。  
> **只负责解析 Bash，不实现 permission 系统。**

参考：

```text
CoderDoubleflower/ClaudeCodeRev
commit: ffe4eab7876ae20ca8f3d7d71b783fa30032fd12
```

重点源码：

```text
src/utils/bash/bashParser.ts
src/utils/bash/parser.ts
src/utils/bash/ast.ts
src/utils/bash/ParsedCommand.ts
src/utils/bash/treeSitterAnalysis.ts
src/utils/bash/commands.ts
```

---

## 1. BashGate 的职责

输入：

```bash
FOO=bar cat "a b.txt" | grep foo >> result.txt
```

输出结构化结果：

```json
{
  "kind": "parsed",
  "commands": [
    {
      "text": "FOO=bar cat \"a b.txt\"",
      "argv": ["cat", "a b.txt"],
      "env": [{"name":"FOO","value":"bar"}],
      "redirects": []
    },
    {
      "text": "grep foo >> result.txt",
      "argv": ["grep", "foo"],
      "env": [],
      "redirects": [
        {"op":">>","target":"result.txt"}
      ]
    }
  ],
  "operators": ["|"]
}
```

如果无法可靠解析：

```bash
for x in *; do eval "$x"; done
```

返回：

```json
{
  "kind": "too_complex",
  "reason": "unsupported_node",
  "node_type": "for_statement"
}
```

### BashGate 不负责

```text
allow / ask / deny
permission rule
read/write 判定
path policy
Git/sed/find 安全分类
UI
Pi plugin
classifier
命令执行
```

---

# 2. 最重要原则：Fail Closed

直接复刻 ClaudeCodeRev `ast.ts` 的核心思想：

```text
能可靠理解
    -> Parsed

不能可靠理解
    -> TooComplex
```

不要：

```text
解析失败
    -> 猜一个 argv
```

Rust：

```rust
pub enum GateResult {
    Parsed(ParsedProgram),
    TooComplex(TooComplex),
    ParseError(ParseError),
}
```

`TooComplex` 不是异常，而是正常结果。

---

# 3. 技术选择

使用：

```text
tree-sitter
tree-sitter-bash
serde
serde_json
thiserror
```

不要逐行翻译 ClaudeCodeRev 的 TypeScript parser。

Rust 直接使用 `tree-sitter-bash` grammar。

真正需要复刻的是：

```text
ClaudeCodeRev ast.ts 的安全 walker
+
ParsedCommand.ts 的结构提取
+
treeSitterAnalysis.ts 的关键分析方式
```

---

# 4. 工程结构

第一版保持简单：

```text
bashgate/
├── Cargo.toml
└── src/
    ├── lib.rs
    ├── main.rs
    ├── model.rs
    ├── parser.rs
    ├── walker.rs
    ├── argument.rs
    ├── redirect.rs
    ├── structure.rs
    └── limits.rs
```

---

# 5. 核心 IR

```rust
pub struct ParsedProgram {
    pub source: String,
    pub commands: Vec<SimpleCommand>,
    pub operators: Vec<OperatorOccurrence>,
}
```

```rust
pub struct SimpleCommand {
    pub text: String,
    pub argv: Vec<String>,
    pub env: Vec<EnvAssignment>,
    pub redirects: Vec<Redirect>,
    pub span: Span,
}
```

```rust
pub struct EnvAssignment {
    pub name: String,
    pub value: String,
}
```

```rust
pub struct Span {
    pub start_byte: usize,
    pub end_byte: usize,
}
```

必须使用 **UTF-8 byte offset**。

Tree-sitter offset 是 byte offset。

---

# 6. Operator

支持：

```rust
pub enum ShellOperator {
    And,        // &&
    Or,         // ||
    Pipe,       // |
    PipeBoth,   // |&
    Sequence,   // ;
    Background, // &
    Newline,
}
```

例如：

```bash
a && b || c | d
```

解析：

```text
commands:
  a
  b
  c
  d

operators:
  &&
  ||
  |
```

这里只解析结构，不执行 shell control-flow。

---

# 7. Redirect

对照 ClaudeCodeRev `ast.ts`：

```text
>
>>
<
<<
>&
>|
<&
&>
&>>
<<<
```

Rust：

```rust
pub struct Redirect {
    pub op: RedirectOp,
    pub target: String,
    pub fd: Option<u32>,
}
```

例如：

```bash
echo hi 2>> error.log
```

得到：

```text
argv:
  echo
  hi

redirect:
  fd = 2
  op = >>
  target = error.log
```

---

# 8. AST Walker

核心要求：

> 使用 node allowlist，而不是“遇到什么 node 都尽量解释”。

第一版 structural nodes：

```text
program
list
pipeline
redirected_statement
command
declaration_command
```

separator：

```text
&&
||
|
|&
;
&
newline
```

基础 argument：

```text
word
string
raw_string
number
command_name
variable_assignment
```

未明确支持的 node：

```text
TooComplex
```

---

# 9. 第一版先拒绝的结构

先返回 `TooComplex`：

```text
subshell
compound_statement
for_statement
while_statement
until_statement
if_statement
case_statement
function_definition
test_command

process_substitution
复杂 command substitution
复杂 parameter expansion
arithmetic expansion
brace expansion

无法可靠解析的 heredoc/herestring
ERROR node
未知 node
```

后续逐个增加支持。

---

# 10. argv 不能用 split_whitespace

错误：

```rust
command.split_whitespace()
```

例如：

```bash
echo "hello world"
```

必须得到：

```json
["echo", "hello world"]
```

```bash
cat 'a b.txt'
```

必须得到：

```json
["cat", "a b.txt"]
```

argv 必须从 AST node 构造。

---

# 11. Env Assignment

支持：

```bash
FOO=bar BAR="hello world" node app.js
```

解析：

```text
env:
  FOO=bar
  BAR=hello world

argv:
  node
  app.js
```

不要把 `FOO=bar` 当 executable。

---

# 12. Pipeline

MVP 必须支持：

```bash
cat a | grep foo | head -10
```

得到：

```text
command 1:
  cat a

command 2:
  grep foo

command 3:
  head -10

operators:
  |
  |
```

完全根据 AST 分段。

不要 regex split。

---

# 13. Compound Command

MVP 支持：

```bash
a && b
a || b
a ; b
a &
a
b
```

例如：

```bash
git status && echo ok
```

得到：

```json
{
  "commands": [
    {"argv":["git","status"]},
    {"argv":["echo","ok"]}
  ],
  "operators":["&&"]
}
```

---

# 14. Pipeline + Compound 混合

必须测试：

```bash
a | b && c | d
a && b || c
a ; b | c
```

所有 command 和 operator 必须按 source order 输出。

ClaudeCodeRev `ParsedCommand.ts` 曾专门处理：

```text
AST DFS 顺序 != source 顺序
```

Rust 统一按：

```text
start_byte
```

排序即可。

---

# 15. Quotes

至少正确处理：

```bash
echo "a | b"
echo 'a && b'
printf '%s\n' "a;b"
echo ">"
```

这些引号内部内容：

```text
不能被识别成 operator / redirect
```

第一版至少支持：

```text
single quote
double quote
unquoted word
```

---

# 16. Expansion

这是后面最难的一块。

MVP 先保守：

```bash
echo $UNTRUSTED
echo ${FOO}
echo $((1+2))
echo *.txt
echo {a,b}
```

如果无法保证：

```text
BashGate argv == Bash 实际 argv
```

就返回：

```text
TooComplex
```

典型危险：

```bash
VAR="-rf /"
rm $VAR
```

真实 Bash 可能 word-split 成：

```text
rm
-rf
/
```

不能错误解析成：

```text
rm
"-rf /"
```

---

# 17. Command Substitution

MVP：

```bash
echo "$(git status)"
```

直接：

```text
TooComplex(command_substitution)
```

第二阶段再考虑像 ClaudeCodeRev 一样递归解析：

```text
inner:
  git status

outer:
  echo __CMDSUB_OUTPUT__
```

不要在 MVP 做。

---

# 18. Heredoc

MVP 只要求识别：

```text
存在 heredoc
```

复杂 body expansion 可以返回：

```text
TooComplex
```

特别注意：

```bash
cat <<EOF
$(cmd)
EOF
```

和：

```bash
cat <<'EOF'
$(literal)
EOF
```

语义不同。

不要简单文本扫描。

---

# 19. Parser Limits

借鉴 ClaudeCodeRev：

```text
MAX_COMMAND_LENGTH ≈ 10,000
PARSE_TIMEOUT ≈ 50ms
MAX_NODES ≈ 50,000
```

Rust 建议：

```rust
pub struct ParseLimits {
    pub max_command_bytes: usize,
    pub max_nodes: usize,
    pub max_commands: usize,
    pub max_depth: usize,
}
```

超过限制：

```text
TooComplex(limit_exceeded)
```

不要 panic。

---

# 20. Parser Differential Pre-check

ClaudeCodeRev 特别防：

```text
Tree-sitter 认为的 token
!=
真实 shell token
```

MVP 至少检查：

```text
NUL / control chars
异常 CR
Unicode 隐形/特殊 whitespace
backslash + whitespace
backslash-newline word joining
```

发现可疑：

```text
TooComplex
```

---

# 21. CLI

第一版只做：

```bash
bashgate parse 'git status && echo ok'
```

输出 JSON。

也支持 stdin：

```bash
echo 'git status && echo ok' | bashgate parse
```

现在先不做 daemon。

等以后真接 Pi，再加：

```text
serve --stdio
```

---

# 22. 实现阶段

## Phase 1 — Parser + IR

实现：

```text
tree-sitter-bash
ParsedProgram
SimpleCommand
Span
word/string/raw_string
quote-aware argv
```

测试：

```bash
echo hello
echo "hello world"
git status
cat 'a b.txt'
```

---

## Phase 2 — Env + Redirect

实现：

```text
VAR=x command

>
>>
<
2>
2>>
&>
```

测试：

```bash
FOO=bar node app.js
cat a > b
echo x 2>> error.log
```

---

## Phase 3 — Compound + Pipeline

实现：

```text
&&
||
;
|
|&
&
newline
```

测试：

```bash
git status && echo ok
cat a | grep x
a | b && c | d
```

完成后就是 **BashGate MVP**。

---

## Phase 4 — Fail Closed

加入：

```text
unknown AST
ERROR node
command/node/depth limits
control chars
Unicode suspicious whitespace
parser failure
```

统一输出：

```text
TooComplex
```

---

## Phase 5 — Expansion

逐个增加：

```text
safe quoted variable
tracked env assignment
command substitution recursion
quoted heredoc
```

每增加一种语法：

```text
必须同时有正常测试 + parser differential 测试
```

---

## Phase 6 — ClaudeCodeRev Parity

从：

```text
ast.ts
parser.ts
ParsedCommand.ts
treeSitterAnalysis.ts
bashSecurity.ts
```

抽测试案例。

只比较：

```text
command boundary
argv
env
redirect
operator
Parsed / TooComplex
```

不要测试 permission decision。

---

# 23. MVP 测试清单

建议：

```text
tests/
  basic.rs
  quotes.rs
  env.rs
  redirects.rs
  pipeline.rs
  compound.rs
  unicode.rs
  parser_error.rs
  limits.rs
```

特别测试：

```bash
echo "a | b"
echo 'a && b'
printf '%s\n' "a;b"
echo ">"
```

确保引号里的 operator 不被拆。

---

# 24. Fuzz

用：

```text
cargo-fuzz
```

target：

```rust
BashGate::parse(input)
```

最低 property：

```text
任意输入：
- 不 panic
- 不 hang
- 不 OOM
- 不越界 slice
- parser failure 不伪装成 Parsed
```

---

# 25. MVP 完成标准

必须完成：

```text
[x] Rust + tree-sitter-bash
[x] SimpleCommand
[x] argv
[x] env assignment
[x] UTF-8 byte span
[x] redirect
[x] && || ; | |& & newline
[x] pipeline / compound boundary
[x] quote-aware
[x] unknown AST -> TooComplex
[x] ERROR -> TooComplex
[x] parser limits
[x] JSON CLI
[x] unit tests
```

明确不做：

```text
[ ] permission
[ ] allow / ask / deny
[ ] wildcard permission rule
[ ] readonly/write classification
[ ] path policy
[ ] command capability database
[ ] Git flag analysis
[ ] Pi plugin
```

---

# 26. 可直接发给另一个会话的 Prompt

```text
请实现一个 Rust 项目 `BashGate`。

目标只是从
CoderDoubleflower/ClaudeCodeRev@ffe4eab7876ae20ca8f3d7d71b783fa30032fd12
中提炼 Bash 命令结构解析能力，不实现 permission system。

重点阅读：

src/utils/bash/bashParser.ts
src/utils/bash/parser.ts
src/utils/bash/ast.ts
src/utils/bash/ParsedCommand.ts
src/utils/bash/treeSitterAnalysis.ts
src/utils/bash/commands.ts

使用 Rust `tree-sitter` + `tree-sitter-bash`。

输入一条 Bash command，输出：

- SimpleCommand[]
- argv
- env assignments
- redirects
- UTF-8 source byte spans
- && || ; | |& & newline operator
- Parsed / TooComplex / ParseError

核心原则复刻 ClaudeCodeRev ast.ts：

只有能可靠提取 shell structure 和 argv 时才返回 Parsed。
未知 AST、动态语义无法可靠解释、ERROR、limit exceeded
全部返回 TooComplex，绝不能猜。

第一阶段完成：

1. Rust 项目骨架
2. tree-sitter-bash
3. SimpleCommand IR
4. quote-aware argv
5. env assignment
6. redirect
7. compound / pipeline segmentation
8. UTF-8 byte span
9. fail-closed
10. parser limits
11. JSON CLI
12. 单元测试

不要实现：

permission
allow/ask/deny
rule matcher
read/write 判定
path policy
Git/sed/find security
Pi plugin integration

先只把 BashGate parser 做稳定。
```

---

## 一句话定位

```text
Bash source
    ↓
tree-sitter-bash
    ↓
fail-closed AST walker
    ↓
可信的 command / argv / env / redirect / operator IR
```

这就是 BashGate。
