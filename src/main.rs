use compiler::builder::*;
use compiler::lexer::*;
use compiler::parser::*;

fn main() {
    let y = Token::load_file("/home/keller-polk/Desktop/compiler/src/test.lang");
    //let y = Token::lexer("[0,1,2]");
    let mut t = TokenParse::convert_from_lex(y);
    //println!("{:?}",t.tokens);
    let program = t.parse();
    println!("{:?}", program);
    program.build();
}
