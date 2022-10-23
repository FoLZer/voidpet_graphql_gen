#![feature(iter_advance_by)]

use std::{fs::File, io::Write, collections::{HashMap, HashSet}};

use reqwest::blocking::Client;
use rslint_parser::{parse_text, SyntaxKind, SyntaxNode, ast::{ObjectExpr, ExprOrSpread, LiteralProp, Expr, ObjectProp, PropName, CallExpr, Pattern, Stmt}, AstNode};

use cached::proc_macro::cached;
use lazy_static::lazy_static;

fn main() {
    /* 
    
    println!("{:?}", types);
    */
    //let links = get_all_links();
    let types = get_graphql_types();
    let mut f = File::create("schema.graphql").unwrap();
    for t in types {
        match t {
            UnionOrObjectType::Union(t) => {
                let s = format!("union {} = {}\n", t.name, t.possible_types.join(" | "));
                f.write(s.as_bytes()).unwrap();
                println!("union");
            },
            UnionOrObjectType::Object(t) => {
                let mut v = Vec::new();
                for f in t.fields {
                    v.push(format!("{}: {}", f.name, f.obj_type.to_string()));
                }
                let s = format!("type {} {{\n\t{}\n}}\n", t.name, v.join("\n\t"));
                f.write(s.as_bytes()).unwrap();
                println!("{:?}", v);
            },
        }
    }
}

fn get_graphql_types() -> Vec<UnionOrObjectType> {
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
    let s = mutations.join("\n\n");
    let mut f = File::create("queries.graphql").unwrap();
    f.write_all(s.as_bytes()).unwrap();

    let schema = mo.last_child().unwrap().first_child().unwrap().last_child().unwrap().first_child().unwrap().last_child().unwrap();
    let types = schema.children().nth(3).unwrap();
    let types = parse_types(types);
    return types;
}

fn parse_types(obj: SyntaxNode) -> Vec<UnionOrObjectType> {
    let mut types = Vec::new();
    //println!("{:?}", obj.kind());
    let ar = LiteralProp::cast(obj).unwrap();
    let ar = ar.value().unwrap();
    if let Expr::ArrayExpr(ar) = ar {
        for child in ar.elements() {
            if let ExprOrSpread::Expr(child) = child {
                if let Expr::ObjectExpr(child) = child {
                    let child = parse_type(child);
                    //println!("{:?}", child);
                    if let Some(o) = child {
                        types.push(o);
                    }
                }
            }
        }
    }
    return types;
}

#[derive(Debug)]
enum UnionOrObjectType {
    Union(Union),
    Object(Object)
}

#[derive(Debug)]
struct Union {
    name: String,
    possible_types: Vec<String>
}

#[derive(Debug)]
struct Object {
    name: String,
    fields: Vec<OType>
}

#[derive(Debug)]
struct OType {
    name: String,
    obj_type: TypeModif
}

#[derive(Debug)]
struct TypeModif {
    kind: String,
    name: Option<String>,
    of_type: Option<Box<TypeModif>>
}

impl TypeModif {
    pub fn to_string(&self) -> String {
        match self.kind.as_str() {
            "NON_NULL" => {
                format!("{}!", self.of_type.as_ref().unwrap().to_string())
            },
            "LIST" => {
                format!("[{}]", self.of_type.as_ref().unwrap().to_string())
            },
            "SCALAR" => {
                "SCALAR".to_string()
            },
            "OBJECT" => {
                self.name.as_ref().unwrap().clone()
            },
            "UNION" => {
                self.name.as_ref().unwrap().clone()
            }
            _ => panic!("Unknown type: {}", self.kind)
        }
    }
}

fn parse_type(obj: ObjectExpr) -> Option<UnionOrObjectType> {
    let mut map = HashMap::new();
    for prop in obj.props() {
        if let ObjectProp::LiteralProp(prop) = prop {
            let key = prop.key().unwrap();
            if let PropName::Ident(key) = key {
                let value = prop.value().unwrap();
                map.insert(key.text(), value);
            }
        }
    }
    let kind = map.get("kind").unwrap();
    if let Expr::Literal(kind) = kind {
        match kind.text().as_str() {
            "\"UNION\"" => {
                let name = {
                    let name = map.get("name").unwrap();
                    if let Expr::Literal(name) = name {
                        name.text().replace("\"", "")
                    } else {
                        panic!()
                    }
                };
                let possible_types = {
                    let possible_types = map.get("possibleTypes").unwrap();
                    if let Expr::ArrayExpr(possible_types) = possible_types {
                        possible_types.elements().map(|v| {
                            if let ExprOrSpread::Expr(v) = v {
                                if let Expr::ObjectExpr(v) = v {
                                    let mut map = HashMap::new();
                                    for prop in v.props() {
                                        if let ObjectProp::LiteralProp(prop) = prop {
                                            let key = prop.key().unwrap();
                                            if let PropName::Ident(key) = key {
                                                let value = prop.value().unwrap();
                                                if let Expr::Literal(value) = value {
                                                    map.insert(key.text(), value.text().replace("\"", ""));
                                                }
                                            }
                                        }
                                    }
                                    map.get("name").unwrap().clone()
                                } else {
                                    panic!()
                                }
                            } else {
                                panic!()
                            }
                        }).collect()
                    } else {
                        panic!()
                    }
                };
                return Some(UnionOrObjectType::Union(Union {
                    name,
                    possible_types
                }));
            },
            "\"OBJECT\"" => {
                let name = {
                    let name = map.get("name").unwrap();
                    if let Expr::Literal(name) = name {
                        name.text().replace("\"", "")
                    } else {
                        panic!()
                    }
                };
                let fields = {
                    let fields = map.get("fields").unwrap();
                    if let Expr::ArrayExpr(fields) = fields {
                        fields.elements().map(|v| {
                            if let ExprOrSpread::Expr(v) = v {
                                if let Expr::ObjectExpr(v) = v {
                                    let mut map = HashMap::new();
                                    for prop in v.props() {
                                        if let ObjectProp::LiteralProp(prop) = prop {
                                            let key = prop.key().unwrap();
                                            if let PropName::Ident(key) = key {
                                                let value = prop.value().unwrap();
                                                map.insert(key.text(), value);
                                            }
                                        }
                                    }
                                    let name = if let Expr::Literal(v) = map.get("name").unwrap() {
                                        v.text().replace("\"", "")
                                    } else {
                                        panic!()
                                    };
                                    let obj_type = if let Expr::ObjectExpr(v) = map.get("type").unwrap() {
                                        parse_of_type(v)
                                    } else {
                                        panic!()
                                    };
                                    OType {
                                        name,
                                        obj_type
                                    }
                                } else {
                                    panic!()
                                }
                            } else {
                                panic!()
                            }
                        }).collect::<Vec<OType>>()
                    } else {
                        panic!()
                    }
                };
                return Some(UnionOrObjectType::Object(Object {
                    name,
                    fields
                }));
            },
            _ => {
                return None;
            }
        }
    } else {
        panic!()
    }
}

fn parse_of_type(v: &ObjectExpr) -> TypeModif {
    let mut map = HashMap::new();
    for prop in v.props() {
        if let ObjectProp::LiteralProp(prop) = prop {
            let key = prop.key().unwrap();
            if let PropName::Ident(key) = key {
                let value = prop.value().unwrap();
                map.insert(key.text(), value);
            }
        }
    }
    let kind = if let Expr::Literal(v) = map.get("kind").unwrap() {
        v.text().replace("\"", "")
    } else {
        panic!()
    };
    let of_type = match map.get("ofType") {
        Some(v) => {
            if let Expr::ObjectExpr(v) = v {
                Some(Box::new(parse_of_type(v)))
            } else {
                None
            }
        },
        None => {
            None
        },
    };
    let name = match map.get("name") {
        Some(v) => {
            if let Expr::Literal(v) = v {
                Some(v.inner_string_text().unwrap().to_string())
            } else {
                None
            }
        },
        None => {
            None
        },
    };
    return TypeModif {
        kind,
        name,
        of_type
    }
}

#[cached]
fn get_graphql_file() -> String {
    let links = get_all_links();
    let path = links.get("5308").unwrap();
    let r = CLIENT.get(format!("https://voidpet.com/_next/{}", path)).send().unwrap();
    let t = r.text().unwrap();
    return t;
}

lazy_static! {
    static ref CLIENT: Client = Client::new();
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