use tree_sitter::{Language, Parser};

#[test]
fn dump_zero_descriptor_asts() {
    let mut parser = Parser::new();
    let language: Language = tree_sitter_bash::LANGUAGE.into();
    parser.set_language(&language).expect("grammar should load");

    let mut dumps = String::new();
    for source in [
        "cat 0<&3",
        "0<&3 cat",
        "cat 0>out",
        "0>out cat",
        "cat 10<&3",
        "cat <&3",
    ] {
        let tree = parser
            .parse(source.as_bytes(), None)
            .expect("source should produce an AST");
        dumps.push_str(&format!("{source:?}: {}\n", tree.root_node().to_sexp()));
    }

    panic!("\n{dumps}");
}
