/*
* Test files are sorted by how they should be run
* - non-zero-exit-code/ holds tests that are OS programs don't read
*   anything from stdin, stdout is ignored and the exit code must be
*   0 to succeed
*
* - check-only/ holds tests that should typecheck as OS programs but
*   are not executed
*
* - pure/ holds tests that consist of a single expression of type
*   Ret(a) and pass if the test runs without error
*
* - custom/ holds tests that need custom I/O mocking to execute.
*/

use std::{io::Read, path::PathBuf};
use zydeco_lang::{
    dynamics::syntax as ds,
    statics::syntax as ss,
    utils::fmt::FmtArgs,
    zydeco::{ZydecoExpr, ZydecoFile},
};

struct IOMatch {
    args: Vec<String>,
    input: String,
    correct_answer: String,
}

fn wrapper<T>(r: Result<T, String>) {
    match r {
        Ok(_) => {}
        Err(e) => panic!("{}", e),
    }
}

fn pure_test(f: &str) -> Result<(), String> {
    let mut path = PathBuf::from("tests/pure");
    path.push(f);
    let mut buf = String::new();
    std::fs::File::open(path)
        .map_err(|e| e.to_string())?
        .read_to_string(&mut buf)
        .map_err(|e| e.to_string())?;
    let mut zydeco_expr = ZydecoExpr::new();
    match ZydecoExpr::parse(&buf) {
        Err(_) => Err("Didn't parse".to_string())?,
        Ok(term) => match ZydecoExpr::elab(term) {
            Err(_) => Err("Didn't elaborate".to_string())?,
            Ok(term) => match term.inner {
                ss::Term::Value(_) => Err("Expecting a computation, found a value".to_string())?,
                ss::Term::Computation(comp) => {
                    let comp = term.info.make(comp);
                    zydeco_expr.tyck_computation(comp.clone())?;
                    let comp = ZydecoExpr::link_computation(comp.inner_ref());
                    let comp = zydeco_expr.eval_ret_computation(comp);
                    match comp {
                        ds::ProgKont::Ret(value) => {
                            println!("{}", value.fmt());
                        }
                        ds::ProgKont::ExitCode(i) => Err(format!("exited with code {}", i))?,
                    }
                }
            },
        },
    };
    Ok(())
}

fn batch_test(f: &str) -> Result<(), String> {
    let mut path = PathBuf::from("tests/nonzero-exit-code");
    path.push(f);
    let m = ZydecoFile::parse(path)?;
    let m = ZydecoFile::elab(m)?;
    ZydecoFile::tyck(m.clone())?;
    let m = ZydecoFile::link(m.inner)?;

    let mut input = std::io::empty();
    let mut output = std::io::sink();
    let ds::ProgKont::ExitCode(exit_code) =
        ZydecoFile::eval_virtual_os(m, &mut input, &mut output, &[]).entry else {
            Err("Expected ExitCode".to_string())?
        };
    if exit_code != 0 {
        Err(format!("Non-zero exit code: {}", exit_code))?
    }

    Ok(())
}

fn io_test(f: &str, iomatch: &IOMatch) -> Result<(), String> {
    let mut path = PathBuf::from("tests/custom");
    path.push(f);
    let m = ZydecoFile::parse(path)?;
    let m = ZydecoFile::elab(m)?;
    ZydecoFile::tyck(m.clone())?;
    let m = ZydecoFile::link(m.inner)?;

    let mut input = std::io::Cursor::new(iomatch.input.as_str());
    let mut output: Vec<u8> = Vec::new();
    let args = iomatch.args.as_slice();

    let ds::ProgKont::ExitCode(exit_code) =
        ZydecoFile::eval_virtual_os(m, &mut input, &mut output, args).entry else {
            Err("Expected ExitCode".to_string())?
        };
    if exit_code != 0 {
        Err(format!("Non-zero exit code: {}", exit_code))?
    }

    let s = std::str::from_utf8(&output).unwrap();
    assert_eq!(iomatch.correct_answer, s, "Output is not correct.");

    Ok(())
}

fn check_test(f: &str) -> Result<(), String> {
    let mut path = PathBuf::from("tests/check-only");
    path.push(f);
    let m = ZydecoFile::parse(path)?;
    let m = ZydecoFile::elab(m)?;
    ZydecoFile::tyck(m)?;

    Ok(())
}

macro_rules! mk_pure_test {
    ($test_name:ident, $file_name:expr) => {
        #[test]
        fn $test_name() {
            wrapper(pure_test($file_name))
        }
    };
}

macro_rules! mk_batch_test {
    ($test_name:ident, $file_name:expr) => {
        #[test]
        fn $test_name() {
            wrapper(batch_test($file_name))
        }
    };
}

macro_rules! mk_check_test {
    ($test_name:ident, $file_name:expr) => {
        #[test]
        fn $test_name() {
            wrapper(check_test($file_name))
        }
    };
}

macro_rules! mk_io_test {
    ($test_name:ident, $file_name:expr, $iomatch:expr) => {
        #[test]
        fn $test_name() {
            wrapper(io_test($file_name, $iomatch))
        }
    };
}

mod batch_tests {
    // Note: to use rust-analyzer's debug feature on tests, you can replace
    // the file name with full path to the test file and click `Debug`
    use super::*;
    mk_batch_test!(defunctionalization, "defunctionalization.zydeco");
    mk_batch_test!(dpa, "deterministic-pushdown-automaton.zydeco");
    mk_batch_test!(interpreter, "interpreter.zydeco");
    mk_batch_test!(list, "list.zydeco");
    mk_batch_test!(y, "Y.zydeco");
    mk_batch_test!(unit, "unit.zy");
    mk_batch_test!(num, "num.zy");
    mk_batch_test!(eo_data, "even-odd-data.zy");
    mk_batch_test!(eo_rec, "even-old-rec.zy");
    mk_batch_test!(eo_coda, "even-odd-codata.zy");
    mk_batch_test!(nat, "nat.zy");
    mk_batch_test!(add, "add.zy");
    mk_batch_test!(regex, "regex.zy");
    mk_batch_test!(listm, "listm.zydeco");
    mk_batch_test!(fn_opt, "fn-opt.zy");
    mk_batch_test!(abort, "abort.zy");
    mk_batch_test!(choice, "choice.zy");
    mk_batch_test!(forall, "forall.zy");
    mk_batch_test!(exists, "exists.zy");
    mk_batch_test!(partial_ann, "partial-annotation.zy");
    mk_batch_test!(oo, "oo.zydeco");
    mk_batch_test!(ret, "ret.zydeco");
}
mod pure_tests {
    use super::*;
    mk_pure_test!(bindings, "bindings.zy");
    mk_pure_test!(booleans, "booleans.zy");
    mk_pure_test!(comments, "comments.zy");
    mk_pure_test!(fn1, "fn.zy");
    mk_pure_test!(fn2, "fn'.zy");
    mk_pure_test!(thunk, "thunk.zy");
    mk_pure_test!(bigmac, "bigmac.zy");
}
mod chk_tests {
    use super::*;
    mk_check_test!(r#loop, "loop.zydeco");
    mk_check_test!(explosion, "explosion.zy");
    mk_check_test!(iota, "iota.zy");
    mk_check_test!(alias, "alias.zy");
}

mod io_tests {
    use super::*;
    mk_io_test!(
        echo_once,
        "echo_once.zydeco",
        &IOMatch {
            args: vec![],
            input: "hello\n".to_string(),
            correct_answer: "hello\n".to_string(),
        }
    );

    mk_io_test!(
        print_args,
        "print_args.zydeco",
        &IOMatch {
            args: vec!["hello".to_string(), "world".to_string()],
            input: String::new(),
            correct_answer: "hello\nworld\n".to_string(),
        }
    );

    mk_io_test!(
        print_list,
        "print_list.zydeco",
        &IOMatch {
            args: vec![],
            input: "hello\n".to_string(),
            correct_answer: "hello world\n5 4 3 2 1".to_string(),
        }
    );
}

mod custom_tests {}
