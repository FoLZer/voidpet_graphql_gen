#![feature(iter_advance_by)]

mod types;
mod queries;

use std::{fs::File, io::Write};

use queries::get_graphql_file;
use rslint_parser::{parse_text, SyntaxKind, ast::{LiteralProp}, AstNode};
use types::{UnionOrObjectType, parse_types};

fn main() {
    let queries = get_graphql_queries();
    let mut f = File::create("queries.graphql").unwrap();
    f.write_all(queries.join("\n\n").as_bytes()).unwrap();
    let types = get_graphql_types();
    let mut f = File::create("schema.graphql").unwrap();
    f.write_all(types.into_iter().map(|v| v.to_string()).collect::<String>().as_bytes()).unwrap();
}

fn get_graphql_queries() -> Vec<String> {
    let s = get_graphql_file();
    let parsed = parse_text(&s, 0);
    let root = parsed.syntax().last_child().unwrap();
    let module_obj = root.first_child().unwrap().children().nth(1).unwrap().first_child().unwrap().children().nth(1).unwrap();
    let mut mutations = Vec::new();
    let mo = module_obj.first_child().unwrap().children().nth(1).unwrap().last_child().unwrap();
    let mut mo_it = mo.children();
    mo_it.advance_by(2).unwrap();
    loop {
        let m = mo_it.next().unwrap();
        if m.first_token().unwrap().kind() == SyntaxKind::BANG {
            break;
        }
        let a = m.children().nth(2).unwrap().first_child().unwrap().first_child().unwrap().children().nth(1).unwrap().children().nth(1).unwrap().first_child().unwrap().first_child().unwrap();
        mutations.push(a.text().to_string().replace("\"", "").replace("\\n", "\n").replace("  ", "\t").trim().to_string());
    }
    return mutations;
}

fn get_graphql_types() -> Vec<UnionOrObjectType> {
    let s = get_graphql_file();
    let parsed = parse_text(&s, 0);
    let root = parsed.syntax().last_child().unwrap();
    let module_obj = root.first_child().unwrap().children().nth(1).unwrap().first_child().unwrap().children().nth(1).unwrap();
    let mo = module_obj.first_child().unwrap().children().nth(1).unwrap().last_child().unwrap();
    let schema = mo.last_child().unwrap().first_child().unwrap().last_child().unwrap().first_child().unwrap().last_child().unwrap();
    let types = schema.children().nth(3).unwrap();
    let types = LiteralProp::cast(types).unwrap();
    let types = parse_types(types);
    return types;
}