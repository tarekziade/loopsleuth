use rustpython_parser::{parse, Mode};

fn main() {
    let code = r#"
def foo(x):
    return x * 2

class Bar:
    def method(self):
        pass
"#;

    match parse(code, Mode::Module, "<test>") {
        Ok(ast) => {
            println!("Parsed successfully!");
            println!("AST: {:#?}", ast);
        }
        Err(e) => {
            eprintln!("Parse error: {:?}", e);
        }
    }
}
