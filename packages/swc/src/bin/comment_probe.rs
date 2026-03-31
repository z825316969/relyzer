use swc_common::{comments::{Comments, SingleThreadedComments}, sync::Lrc, FileName, SourceMap, Spanned};
use swc_ecma_ast::*;
use swc_ecma_parser::{lexer::Lexer, Parser, StringInput, Syntax, TsConfig};

fn dump_comments(comments: &SingleThreadedComments, label: &str, pos: swc_common::BytePos) {
    let leading = comments.get_leading(pos);
    println!("{} @ {:?}", label, pos);
    match leading {
        Some(items) => {
            for (i, c) in items.iter().enumerate() {
                println!("  [{}] kind={:?} text={:?}", i, c.kind, c.text);
            }
        }
        None => println!("  <no leading comments>"),
    }
}

fn main() {
    let code = r#"
/** @component */
export function helper() {
  return <Panel />;
}
"#;

    let cm: Lrc<SourceMap> = Default::default();
    let comments = SingleThreadedComments::default();
    let fm = cm.new_source_file(FileName::Custom("probe.tsx".into()).into(), code.to_string());

    let lexer = Lexer::new(
        Syntax::Typescript(TsConfig {
            tsx: true,
            decorators: true,
            dts: false,
            no_early_errors: true,
            disallow_ambiguous_jsx_like: false,
        }),
        EsVersion::Es2022,
        StringInput::from(&*fm),
        Some(&comments),
    );

    let mut parser = Parser::new_from(lexer);
    let module = parser.parse_module().expect("parse module");

    println!("module body len = {}", module.body.len());

    for item in &module.body {
        match item {
            ModuleItem::ModuleDecl(ModuleDecl::ExportDecl(export_decl)) => {
                println!("ExportDecl span: {:?}", export_decl.span);
                dump_comments(&comments, "ExportDecl.lo", export_decl.span.lo);
                match &export_decl.decl {
                    Decl::Fn(fn_decl) => {
                        println!("FnDecl ident span: {:?}", fn_decl.ident.span);
                        println!("Function span: {:?}", fn_decl.function.span);
                        dump_comments(&comments, "FnDecl.ident.lo", fn_decl.ident.span.lo);
                        dump_comments(&comments, "Function.lo", fn_decl.function.span.lo);
                        dump_comments(&comments, "Function.hi", fn_decl.function.span.hi);
                    }
                    _ => {}
                }
            }
            ModuleItem::Stmt(stmt) => {
                println!("Stmt span: {:?}", stmt.span());
                dump_comments(&comments, "Stmt.lo", stmt.span().lo);
            }
            _ => {}
        }
    }
}
