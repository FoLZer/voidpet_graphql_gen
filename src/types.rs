use std::collections::HashMap;

use rslint_parser::{ast::{ObjectExpr, PropName, ObjectProp, Expr, ExprOrSpread, LiteralProp}, AstNode};

#[derive(Debug)]
pub enum UnionOrObjectType {
    Union(Union),
    Object(Object)
}

impl UnionOrObjectType {
    pub fn to_string(&self) -> String {
        match self {
            UnionOrObjectType::Union(t) => {
                return format!("union {} = {}\n", t.name, t.possible_types.join(" | "));
            },
            UnionOrObjectType::Object(t) => {
                let mut v = Vec::new();
                for f in &t.fields {
                    v.push(format!("{}: {}", f.name, f.obj_type.to_string()));
                }
                return format!("type {} {{\n\t{}\n}}\n", t.name, v.join("\n\t"));
            },
        }
    }
}

#[derive(Debug)]
pub struct Union {
    name: String,
    possible_types: Vec<String>
}

#[derive(Debug)]
pub struct Object {
    name: String,
    fields: Vec<OType>
}

#[derive(Debug)]
pub struct OType {
    name: String,
    obj_type: TypeModif
}

#[derive(Debug)]
pub struct TypeModif {
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

pub fn parse_of_type(v: &ObjectExpr) -> TypeModif {
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

pub fn parse_types(obj: LiteralProp) -> Vec<UnionOrObjectType> {
    let mut types = Vec::new();
    //println!("{:?}", obj.kind());
    let ar = obj.value().unwrap();
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