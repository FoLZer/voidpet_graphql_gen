use std::collections::{HashMap, HashSet};

use cached::proc_macro::cached;
use lazy_static::lazy_static;
use reqwest::blocking::Client;
use rslint_parser::{parse_text, ast::{CallExpr, Expr, PropName, ObjectProp, ExprOrSpread, Stmt, Pattern}, AstNode};

lazy_static! {
    static ref CLIENT: Client = Client::new();
}

#[cached]
pub fn get_graphql_file() -> String {
    let links = get_all_links();
    let path = links.get("5308").unwrap();
    let r = CLIENT.get(format!("https://voidpet.com/_next/{}", path)).send().unwrap();
    let t = r.text().unwrap();
    return t;
}

#[cached]
fn get_all_links() -> HashMap<String, String> {
    let manifest = get_build_manifest();
    let mut set = HashSet::new();
    for (_key, value) in manifest {
        for s in value {
            set.insert(s);
        }
    }
    return set.into_iter().map(|v| {
        (v.split("/").last().unwrap().rsplit_once("-").unwrap().0.to_string(), v)
    }).collect();
}

#[cached]
fn get_build_manifest() -> HashMap<String, Vec<String>> {
    let build_id = get_current_build_id();
    let r = CLIENT.get(format!("https://voidpet.com/_next/static/{}/_buildManifest.js", build_id)).send().unwrap();
    let t = r.text().unwrap();
    let parsed = parse_text(&t, 0);
    let func = {
        let f = parsed.syntax().first_child().unwrap().first_child().unwrap().first_child().unwrap().last_child().unwrap();
        let f = CallExpr::cast(f).unwrap();
        f
    };
    let bodyfn = {
        let b = func.callee().unwrap();
        if let Expr::FnExpr(b) = b {
            b
        } else {
            panic!()
        }
    };
    let mut a = bodyfn.parameters().unwrap().parameters();
    let l = func.arguments().unwrap().args().map(|v| {
        if let Expr::Literal(v) = v {
            let a = a.next().unwrap();
            if let Pattern::SinglePattern(a) = a {
                (a.name().unwrap().to_string(), v.inner_string_text().unwrap().to_string())
            } else {
                panic!()
            }
            
        } else {
            panic!()
        }
    }).collect::<HashMap<String, String>>();
    
    let body = bodyfn.body().unwrap();
    let ret = {
        let f = body.stmts().next().unwrap();
        if let Stmt::ReturnStmt(f) = f {
            f
        } else {
            panic!()
        }
    };
    let obj = {
        let o = ret.value().unwrap();
        if let Expr::ObjectExpr(o) = o {
            o
        } else {
            panic!()
        }
    };
    let mut map = HashMap::new();
    for prop in obj.props() {
        if let ObjectProp::LiteralProp(prop) = prop {
            let key = {
                let k = prop.key().unwrap();
                if let PropName::Literal(k) = k {
                    k.inner_string_text().unwrap().to_string()
                } else {
                    continue;
                }
            };
            let value = {
                let v = prop.value().unwrap();
                if let Expr::ArrayExpr(v) = v {
                    let m = v.elements().map(|el| {
                        if let ExprOrSpread::Expr(el) = el {
                            match el {
                                Expr::NameRef(el) => {
                                    let v = l.get(&el.to_string()).unwrap().clone();
                                    v
                                },
                                Expr::Literal(el) => {
                                    let v = el.inner_string_text().unwrap().to_string();
                                    v
                                },
                                _ => panic!()
                            }
                        } else {
                            panic!()
                        }
                    }).collect::<Vec<String>>();
                    m
                } else {
                    panic!()
                }
            };
            map.insert(key, value);
        }
    }

    return map;
}

#[cached]
fn get_current_build_id() -> String {
    let r = CLIENT.get("https://voidpet.com/").send().unwrap();
    let t = r.text().unwrap();
    let mut a = t.find("buildId").unwrap();
    loop {
        if t.chars().nth(a).unwrap() == ':' {
            break;
        }
        a += 1;
    }
    
    let (start, end) = {
        let start;
        let end;
        let mut i = a+1;
        loop {
            if t.chars().nth(i).unwrap() == '"' {
                start = i+1;
                break;
            }
            i += 1;
        }
        i = start+1;
        loop {
            if t.chars().nth(i).unwrap() == '"' {
                end = i;
                break;
            }
            i += 1;
        }
        (start, end)
    };
    let mut i = t.chars();
    i.advance_by(start).unwrap();
    let build_id = i.take(end-start).collect();
    return build_id;
}