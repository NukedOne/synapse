use assert_cmd;
use rstest::*;
use std::io::Write;
use std::{collections::VecDeque, path::Path};
use synapse::vm::Object;

macro_rules! object_vec {
    ( $($obj:expr),* ) => {{
        let mut v: Vec<Object> = vec![];
        $(
            v.push($obj.into());
        )*
        v
    }}
}

fn fetch_stdout(path: impl AsRef<Path>) -> (VecDeque<String>, VecDeque<String>) {
    let mut cmd = assert_cmd::Command::cargo_bin("synapse").unwrap();
    let assert = cmd.arg(path.as_ref()).assert();
    let output = assert.get_output();
    let stdout = String::from_utf8(output.stdout.clone()).unwrap();
    let stdout_split: VecDeque<String> = stdout
        .split('\n')
        .filter_map(|l| (!l.is_empty()).then_some(l.to_owned()))
        .collect();
    let filtered: VecDeque<String> = stdout_split
        .iter()
        .filter_map(|l| (l.starts_with("dbg:")).then_some(l.to_owned()))
        .collect();
    (stdout_split, filtered)
}

fn fetch_stderr(path: &str) -> VecDeque<String> {
    let mut cmd = assert_cmd::Command::cargo_bin("synapse").unwrap();
    let assert = cmd.arg(path).assert();
    let output = assert.get_output();
    let stderr = String::from_utf8(output.stderr.clone()).unwrap();
    let split: VecDeque<String> = stderr
        .split('\n')
        .filter_map(|l| (!l.is_empty()).then_some(l.to_owned()))
        .collect();
    split
}

macro_rules! run_test {
    ($path:expr, $expected:expr) => {{
        let (mut stdout, mut filtered) = fetch_stdout($path);
        for e in $expected {
            assert!(filtered.pop_front().unwrap() == format!("dbg: {:?}", e));
        }
        assert!(stdout.pop_back().unwrap() == "current instruction: Halt");
        assert!(stdout.pop_back().unwrap() == "stack: []");
    }};
}

macro_rules! run_test_error {
    ($type:tt, $path:expr, $expected:expr) => {{
        let mut stderr = fetch_stderr($path);
        assert!(
            stderr.pop_back().unwrap() == format!("synapse: {}: {}", stringify!($type), $expected)
        );
    }};
}

#[test]
fn add() {
    let (path, expected) = ("tests/cases/add.syn", object_vec![15.0]);
    run_test!(path, expected);
}

#[test]
fn add_error() {
    let (path, expected) = ("tests/cases/add_error.syn", "only numbers can be +");
    run_test_error!(vm, path, expected);
}

#[test]
fn sub() {
    let (path, expected) = ("tests/cases/sub.syn", object_vec![2.0]);
    run_test!(path, expected);
}

#[test]
fn sub_neg() {
    let (path, expected) = ("tests/cases/sub_neg.syn", object_vec![6.0]);
    run_test!(path, expected);
}

#[test]
fn mul() {
    let (path, expected) = ("tests/cases/mul.syn", object_vec![360.0]);
    run_test!(path, expected);
}

#[test]
fn div() {
    let (path, expected) = ("tests/cases/div.syn", object_vec![20.0]);
    run_test!(path, expected);
}

#[test]
fn eq() {
    let (path, expected) = (
        "tests/cases/eq.syn",
        object_vec![
            true, false, false, false, false, true, true, true, false, true, true, true, true,
            false, false, false
        ],
    );
    run_test!(path, expected);
}

#[test]
fn relational() {
    let (path, expected) = (
        "tests/cases/relational.syn",
        object_vec![true, false, true, false],
    );
    run_test!(path, expected);
}

#[test]
fn relational_error() {
    let (path, expected) = (
        "tests/cases/relational_error.syn",
        "only numbers can be: <, >, <=, >=",
    );
    run_test_error!(vm, path, expected);
}

#[test]
fn fib10() {
    let (path, expected) = ("tests/cases/fib10.syn", object_vec![55.0]);
    run_test!(path, expected);
}

#[test]
fn fizzbuzz() {
    let (path, expected) = (
        "tests/cases/fizzbuzz.syn",
        object_vec![
            "fizzbuzz", 1.0, 2.0, "fizz", 4.0, "buzz", "fizz", 7.0, 8.0, "fizz", "buzz", 11.0,
            "fizz", 13.0, 14.0, "fizzbuzz", 16.0, 17.0, "fizz", 19.0
        ],
    );
    run_test!(path, expected);
}

#[test]
fn _while() {
    let (path, expected) = (
        "tests/cases/while.syn",
        object_vec![5.0, 4.0, 3.0, 2.0, 1.0, 0.0],
    );
    run_test!(path, expected);
}

#[test]
fn _for() {
    let (path, expected) = (
        "tests/cases/for.syn",
        object_vec![0.0, 1.0, 2.0, 3.0, 4.0, 5.0],
    );
    run_test!(path, expected);
}

#[test]
fn _while_pop() {
    let (path, expected) = (
        "tests/cases/while_pop.syn",
        object_vec![5.0, 4.0, 3.0, 2.0, 1.0, 0.0],
    );
    run_test!(path, expected);
}

#[test]
fn _break() {
    let (path, expected) = ("tests/cases/break.syn", object_vec![0.0, 1.0, 2.0, 3.0]);
    run_test!(path, expected);
}

#[test]
fn break_complex() {
    let (path, expected) = (
        "tests/cases/break_complex.syn",
        object_vec![
            1.0,
            "Hello, world!",
            "Hello, world!",
            "Hello, world!",
            "Hello, world!",
            "Hello, world!",
            3.0
        ],
    );
    run_test!(path, expected);
}

#[test]
fn _continue() {
    let (path, expected) = (
        "tests/cases/continue.syn",
        object_vec![0.0, 1.0, 2.0, 3.0, 4.0],
    );
    run_test!(path, expected);
}

#[test]
fn strcat() {
    let (path, expected) = (
        "tests/cases/strcat.syn",
        object_vec!["Hello, world!".to_string()],
    );
    run_test!(path, expected);
}

#[test]
fn neg() {
    let (path, expected) = ("tests/cases/neg.syn", object_vec![-5.0]);
    run_test!(path, expected);
}

#[test]
fn minus_number() {
    let (path, expected) = ("tests/cases/minus_number.syn", object_vec![3.14]);
    run_test!(path, expected);
}

#[test]
fn neg_error() {
    let (path, expected) = ("tests/cases/neg_error.syn", "only numbers can be -");
    run_test_error!(vm, path, expected);
}

#[test]
fn not() {
    let (path, expected) = ("tests/cases/not.syn", object_vec![true]);
    run_test!(path, expected);
}

#[test]
fn not_error() {
    let (path, expected) = ("tests/cases/not_error.syn", "only bools can be !");
    run_test_error!(vm, path, expected);
}

#[test]
fn tokenizer_error() {
    let (path, expected) = ("tests/cases/tokenizer_error.syn", "unexpected token: $");
    run_test_error!(tokenizer, path, expected);
}

#[test]
fn grouping() {
    let (path, expected) = ("tests/cases/grouping.syn", object_vec![14.0]);
    run_test!(path, expected);
}

#[test]
fn structs() {
    let (path, expected) = ("tests/cases/structs.syn", object_vec!["Hello, world!"]);
    run_test!(path, expected);
}

#[test]
fn linked_list() {
    let (path, expected) = (
        "tests/cases/linked_list.syn",
        object_vec![3.14, false, "Hello, world!"],
    );
    run_test!(path, expected);
}

#[test]
fn struct_error01() {
    let (path, expected) = (
        "tests/cases/struct_error01.syn",
        "struct 'spam' has no member 'a'",
    );
    run_test_error!(vm, path, expected);
}

#[test]
fn strcat_error() {
    let (path, expected) = (
        "tests/cases/strcat_error.syn",
        "only strings can be concatenated",
    );
    run_test_error!(vm, path, expected);
}

#[test]
fn parser_error_expected_decl() {
    let (path, expected) = (
        "tests/cases/parser_error_expected_decl.syn",
        "expected a declaration (like 'fn' or 'struct')",
    );
    run_test_error!(parser, path, expected);
}

#[test]
fn parser_error_expected_identifier_when_instantiating_struct() {
    let (path, expected) = (
        "tests/cases/parser_error_expected_identifier_when_instantiating_struct.syn",
        "expected: number, string, (, true, false, null, identifier",
    );
    run_test_error!(parser, path, expected);
}

#[test]
fn parser_error_expected_identifier_after_struct_keyword() {
    let (path, expected) = (
        "tests/cases/parser_error_expected_identifier_after_struct_keyword.syn",
        "expected identifier after 'struct' keyword, got: 1",
    );
    run_test_error!(parser, path, expected);
}

#[test]
fn parser_error_wrong_struct_decl() {
    let (path, expected) = (
        "tests/cases/parser_error_wrong_struct_decl.syn",
        "structs should be declared as: `struct s { x, y, z, }`",
    );
    run_test_error!(parser, path, expected);
}

#[test]
fn compiler_error_struct_not_defined() {
    let (path, expected) = (
        "tests/cases/compiler_error_struct_not_defined.syn",
        "struct 'egg' is not defined",
    );
    run_test_error!(compiler, path, expected);
}

#[test]
fn compiler_error_wrong_initializer_count() {
    let (path, expected) = (
        "tests/cases/compiler_error_wrong_initializer_count.syn",
        "struct 'spam' has 3 members",
    );
    run_test_error!(compiler, path, expected);
}

#[test]
fn compiler_error_no_main() {
    let (path, expected) = (
        "tests/cases/compiler_error_no_main.syn",
        "main fn was not defined",
    );
    run_test_error!(compiler, path, expected);
}

#[test]
fn compiler_error_wrong_params() {
    let (path, expected) = (
        "tests/cases/compiler_error_wrong_params.syn",
        "function 'f' takes 3 arguments",
    );
    run_test_error!(compiler, path, expected);
}

#[test]
fn compiler_error_fn_not_defined() {
    let (path, expected) = (
        "tests/cases/compiler_error_fn_not_defined.syn",
        "function 'f' is not defined",
    );
    run_test_error!(compiler, path, expected);
}

#[test]
fn compiler_error_invalid_assignment() {
    let (path, expected) = (
        "tests/cases/compiler_error_invalid_assignment.syn",
        "invalid assignment",
    );
    run_test_error!(compiler, path, expected);
}

#[test]
fn ptr() {
    let (path, expected) = ("tests/cases/ptr.syn", object_vec![3.0]);
    run_test!(path, expected);
}

#[test]
fn ptr02() {
    let (path, expected) = ("tests/cases/ptr02.syn", object_vec![3.0]);
    run_test!(path, expected);
}

#[test]
fn ptr03() {
    let (path, expected) = ("tests/cases/ptr03.syn", object_vec![3.0]);
    run_test!(path, expected);
}

#[test]
fn _loop() {
    let (path, expected) = ("tests/cases/loop.syn", object_vec![1.0, 2.0, 3.0, 4.0, 5.0]);
    run_test!(path, expected);
}

#[test]
fn method() {
    let (path, expected) = ("tests/cases/method.syn", object_vec!["Hello, John."]);
    run_test!(path, expected);
}

#[rstest]
#[case(32, 64, 32, "&&", 64, "Run!")]
#[case(32, 64, 16, "&&", 64, "Shouldn't run!")]
#[case(32, 64, 32, "&&", 16, "Shouldn't run!")]
#[case(32, 64, 128, "&&", 256, "Shouldn't run!")]
#[case(32, 64, 32, "||", 64, "Run!")]
#[case(32, 64, 16, "||", 64, "Run!")]
#[case(32, 64, 32, "||", 16, "Run!")]
#[case(32, 64, 128, "||", 256, "Shouldn't run!")]
fn logical(
    #[case] a: usize,
    #[case] b: usize,
    #[case] if_a_equals: usize,
    #[case] op: &str,
    #[case] if_b_equals: usize,
    #[case] expected: &str,
) {
    let source = format!(
        r#"
        fn main() {{
            x = {};
            y = {};
            if x == {} {} y == {} {{
                print "Run!";
            }} else {{
                print "Shouldn't run!";
            }}
            return 0;
        }}
        "#,
        a, b, if_a_equals, op, if_b_equals
    );

    let random = rand::random::<u64>();
    let filename = format!("input_logical_{}.syn", random);
    let dir = std::env::temp_dir();
    let input_file_path = dir.join(filename);
    let mut file = std::fs::File::create(&input_file_path).expect("create test file failed");
    writeln!(file, "{}", source).expect("write test file failed");

    run_test!(input_file_path.as_path(), object_vec!(expected));
}

#[rstest]
#[case(8, "|", 1, object_vec![9.0])]
#[case(15, "&", 1, object_vec![1.0])]
#[case(15, "^", 2, object_vec![13.0])]
#[case(1, "<<", 5, object_vec![32.0])]
#[case(64, ">>", 2, object_vec![16.0])]
fn bitwise(
    #[case] left: usize,
    #[case] op: &str,
    #[case] right: usize,
    #[case] expected: Vec<Object>,
) {
    let source = format!(
        r#"
        fn test_bitwise(a, b) {{
            return a {} b; 
        }}
        fn main() {{
            print test_bitwise({}, {});
            return 0;
        }}
        "#,
        op, left, right,
    );
    let random = rand::random::<u64>();
    let filename = format!("input_bitwise_{}.syn", random);
    let dir = std::env::temp_dir();
    let input_file_path = dir.join(filename);
    let mut file = std::fs::File::create(&input_file_path).expect("create test file failed");
    writeln!(file, "{}", source).expect("write test file failed");

    run_test!(input_file_path.as_path(), expected);
}

#[rstest]
#[case(0, "<", 10, 2, "+", object_vec![0.0, 2.0, 4.0, 6.0, 8.0])]
#[case(10, ">", 0, 2, "-", object_vec![10.0, 8.0, 6.0, 4.0, 2.0])]
#[case(1, "!=", 64, 2, "*", object_vec![1.0, 2.0, 4.0, 8.0, 16.0, 32.0])]
#[case(64, "!=", 1, 2, "/", object_vec![64.0, 32.0, 16.0, 8.0, 4.0, 2.0])]
#[case(128, "!=", 1, 1, ">>", object_vec![128.0, 64.0, 32.0, 16.0, 8.0, 4.0, 2.0])]
#[case(1, "!=", 128, 1, "<<", object_vec![1.0, 2.0, 4.0, 8.0, 16.0, 32.0, 64.0])]
fn compound_assignment(
    #[case] start: usize,
    #[case] cond_op: &str,
    #[case] end: usize,
    #[case] step: usize,
    #[case] op: &str,
    #[case] expected: Vec<Object>,
) {
    let source = format!(
        r#"
        fn test_compound_assignment() {{
            x = {};
            while (x {} {}) {{
            print x;
            x {}= {};
            }}
            return 0; 
        }}
        fn main() {{
            test_compound_assignment();
            return 0;
        }}
        "#,
        start, cond_op, end, op, step,
    );
    let random = rand::random::<u64>();
    let filename = format!("input_compound_{}.syn", random);
    let dir = std::env::temp_dir();
    let input_file_path = dir.join(filename);
    let mut file = std::fs::File::create(&input_file_path).expect("create test file failed");
    writeln!(file, "{}", source).expect("write test file failed");

    run_test!(input_file_path.as_path(), expected);
}

#[rstest]
#[case(32, "&", 2, object_vec![0.0])]
#[case(32, "|", 1, object_vec![33.0])]
#[case(32, "^", 1, object_vec![33.0])]
#[case(5, "%", 3, object_vec![2.0])]
fn more_compound_assignment(
    #[case] left: usize,
    #[case] op: &str,
    #[case] right: usize,
    #[case] expected: Vec<Object>,
) {
    let source = format!(
        r#"
        fn test_more_compound_assignment() {{
            x = {};
            x {}= {};
            return x; 
        }}
        fn main() {{
            print test_more_compound_assignment();
            return 0;
        }}
        "#,
        left, op, right,
    );

    let random = rand::random::<u64>();
    let filename = format!("input_more_compound_{}.syn", random);
    let dir = std::env::temp_dir();
    let input_file_path = dir.join(filename);
    let mut file = std::fs::File::create(&input_file_path).expect("create test file failed");
    writeln!(file, "{}", source).expect("write test file failed");

    run_test!(input_file_path.as_path(), expected);
}

#[test]
fn _vec() {
    let (path, expected) = (
        "tests/cases/vec.syn",
        object_vec![128.0, "Hello, world!", 11.0],
    );
    run_test!(path, expected);
}

#[test]
fn import_success() {
    let (path, expected) = ("tests/cases/import.syn", object_vec!(1.0));
    run_test!(path, expected);
}

#[test]
fn import_cycle() {
    let (path, expected) = ("tests/cases/cycle/a.syn", "cycle");
    run_test_error!(compiler, path, expected);
}

#[test]
fn import_cached() {
    let (path, expected) = (
        "tests/cases/cached/a.syn",
        "compiler: using cached import: tests/cases/cached/c.syn",
    );
    let (split, _filtered) = fetch_stdout(path);

    assert!(split.contains(&expected.to_owned()));
}
