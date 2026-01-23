use atc_router::ast::{Type, Value};
use atc_router::context::Context;
use atc_router::router::Router;
use atc_router::schema::Schema;
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use uuid::Uuid;

// To run this benchmark, execute the following command:
// ```shell
// cargo bench --bench match_mix
// ```

const N: usize = 100_000;

fn make_uuid(a: usize) -> String {
    format!("8cb2a7d0-c775-4ed9-989f-{:012}", a)
}

fn criterion_benchmark(c: &mut Criterion) {
    let mut schema = Schema::default();
    schema.add_field("http.path", Type::String);
    schema.add_field("http.version", Type::String);
    schema.add_field("a", Type::Int);

    let mut router = Router::new(&schema);

    for i in 0..N {
        let expr = format!(
            r#"(http.path ~ "^hello{}$" && http.version == "1.1") || {} || {} || {}"#,
            i, "!((a == 2) && (a == 9))", "!(a == 1)", "(a == 3 && a == 4) && !(a == 5)"
        );

        let uuid = make_uuid(i);
        let uuid = Uuid::try_from(uuid.as_str()).unwrap();

        router.add_matcher(N - i, uuid, &expr).unwrap();
    }

    let mut ctx = Context::new(&schema);

    // match benchmark
    ctx.add_value("http.path", Value::String("hello49999".to_string()));
    ctx.add_value("http.version", Value::String("1.1".to_string()));
    ctx.add_value("a", Value::Int(3_i64));

    c.bench_function("Match", |b| {
        b.iter(|| {
            let is_match = router.execute(&mut ctx);
            assert!(is_match);
        });
    });

    ctx.reset();

    // not match benchmark
    ctx.add_value("http.path", Value::String("hello49999".to_string()));
    ctx.add_value("http.version", Value::String("1.1".to_string()));
    ctx.add_value("a", Value::Int(5_i64)); // not match

    c.bench_function("Doesn't Match", |b| {
        b.iter(|| {
            let not_match = !router.execute(&mut ctx);
            assert!(not_match);
        });
    });
}

fn regex_and_equals_expr(options: usize) -> (String, String) {
    const POSSIBLE_CHARS: &str = "abcdefghijklmnopqrstuvwxyz";
    const PREFIX: &str = "/foo/bar";

    assert_ne!(options, 0);

    // question mark means there's one option in addition to the extra chars
    let possible_chars = &POSSIBLE_CHARS[..options - 1];

    let additional_regex_suffix = if possible_chars.is_empty() {
        String::new()
    } else {
        format!("[{}]?", possible_chars)
    };
    let regex_expr = format!(r##"http.path ~ r#"^{PREFIX}/{additional_regex_suffix}$"#"##);
    let mut equals_expr = format!("http.path == \"{PREFIX}/\"");
    for ch in possible_chars.chars() {
        equals_expr.push_str(format!(" || http.path == \"{PREFIX}/{ch}\"").as_str());
    }

    (regex_expr, equals_expr)
}

fn regex_vs_equals(c: &mut Criterion) {
    let mut schema = Schema::default();
    schema.add_field("http.path", Type::String);
    schema.add_field("a", Type::String);
    let mut g = c.benchmark_group("optional slash");
    let mut ctx = Context::new(&schema);
    ctx.add_value("http.path", Value::String(String::from("/foo/bar/!")));

    for i in [1, 2, 3, 4, 5] {
        let mut regex_router = Router::new(&schema);
        let mut equals_router = Router::new(&schema);
        let (regex_expr, equals_expr) = regex_and_equals_expr(i);
        dbg!((&regex_expr, &equals_expr));
        regex_router
            .add_matcher(0, Uuid::default(), &regex_expr)
            .unwrap();
        equals_router
            .add_matcher(0, Uuid::default(), &equals_expr)
            .unwrap();
        g.bench_with_input(BenchmarkId::new("regex", i), &regex_router, |b, router| {
            b.iter(|| {
                let matches = router.execute(&mut ctx);
                assert!(!matches);
            });
        });
        g.bench_with_input(
            BenchmarkId::new("equals", i),
            &equals_router,
            |b, router| {
                b.iter(|| {
                    let matches = router.execute(&mut ctx);
                    assert!(!matches);
                })
            },
        );
    }
    g.finish();
}

criterion_group!(benches, criterion_benchmark, regex_vs_equals);
criterion_main!(benches);
