use criterion::{Criterion, black_box, criterion_group, criterion_main};
use tefas::parse_document;

fn parse_batch_sequential(docs: &[&str]) -> usize {
    docs.iter()
        .map(|html| {
            let (grouped, _) = parse_document(html);
            grouped.as_object().map(|m| m.len()).unwrap_or(0)
        })
        .sum()
}

fn parse_batch_parallel_blocking(
    docs: &[&'static str],
    runtime: &tokio::runtime::Runtime,
) -> usize {
    runtime.block_on(async {
        let mut handles = Vec::with_capacity(docs.len());
        for html in docs {
            let html = *html;
            handles.push(tokio::task::spawn_blocking(move || {
                let (grouped, _) = parse_document(html);
                grouped.as_object().map(|m| m.len()).unwrap_or(0)
            }));
        }

        let mut total = 0usize;
        for h in handles {
            total += h.await.expect("spawn_blocking task should not panic");
        }
        total
    })
}

fn bench_fundpage_pipeline(c: &mut Criterion) {
    let docs: [&'static str; 3] = [
        include_str!("../../../datasets/tefas/fundpage/html/AC5.html"),
        include_str!("../../../datasets/tefas/fundpage/html/AFT.html"),
        include_str!("../../../datasets/tefas/fundpage/html/TLY.html"),
    ];

    c.bench_function("fundpage_parse_batch/sequential", |b| {
        b.iter(|| black_box(parse_batch_sequential(&docs)))
    });

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("tokio runtime should build");

    c.bench_function("fundpage_parse_batch/spawn_blocking", |b| {
        b.iter(|| black_box(parse_batch_parallel_blocking(&docs, &runtime)))
    });
}

criterion_group!(benches, bench_fundpage_pipeline);
criterion_main!(benches);
