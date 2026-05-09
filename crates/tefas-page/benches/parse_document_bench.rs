use criterion::{BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main};
use tefas_parser::fund_page::parse_document;

// ---------------------------------------------------------------------------
// Sample HTML fixtures
// ---------------------------------------------------------------------------

/// Completely empty input — exercises the fast no-op path.
const EMPTY: &str = "";

/// Real TEFAS fund detail pages. The files live in datasets/tefas/fundpage and are
/// embedded at compile time so file I/O does not pollute parser timings.
const FUND_PAGES: &[(&str, &str)] = &[
    (
        "AC5",
        include_str!("../../../datasets/tefas/fundpage/html/AC5.html"),
    ),
    (
        "ADE",
        include_str!("../../../datasets/tefas/fundpage/html/ADE.html"),
    ),
    (
        "AFT",
        include_str!("../../../datasets/tefas/fundpage/html/AFT.html"),
    ),
    (
        "AGC",
        include_str!("../../../datasets/tefas/fundpage/html/AGC.html"),
    ),
    (
        "DGF",
        include_str!("../../../datasets/tefas/fundpage/html/DGF.html"),
    ),
    (
        "DTZ",
        include_str!("../../../datasets/tefas/fundpage/html/DTZ.html"),
    ),
    (
        "EML",
        include_str!("../../../datasets/tefas/fundpage/html/EML.html"),
    ),
    (
        "MTV",
        include_str!("../../../datasets/tefas/fundpage/html/MTV.html"),
    ),
    (
        "TAR",
        include_str!("../../../datasets/tefas/fundpage/html/TAR.html"),
    ),
    (
        "TGR",
        include_str!("../../../datasets/tefas/fundpage/html/TGR.html"),
    ),
    (
        "TLY",
        include_str!("../../../datasets/tefas/fundpage/html/TLY.html"),
    ),
    (
        "VCY",
        include_str!("../../../datasets/tefas/fundpage/html/VCY.html"),
    ),
    (
        "ZFB",
        include_str!("../../../datasets/tefas/fundpage/html/ZFB.html"),
    ),
];

// ---------------------------------------------------------------------------
// Benchmarks
// ---------------------------------------------------------------------------

fn bench_empty(c: &mut Criterion) {
    c.bench_function("parse_document/empty", |b| {
        b.iter(|| parse_document(black_box(EMPTY)));
    });
}

fn bench_fund_pages(c: &mut Criterion) {
    let mut group = c.benchmark_group("parse_document");
    for (name, html) in FUND_PAGES {
        group.throughput(Throughput::Bytes(html.len() as u64));
        group.bench_with_input(BenchmarkId::from_parameter(name), html, |b, input| {
            b.iter(|| parse_document(black_box(input)));
        });
    }
    group.finish();
}

fn bench_fund_page_batch(c: &mut Criterion) {
    let total_bytes: u64 = FUND_PAGES.iter().map(|(_, html)| html.len() as u64).sum();
    let mut group = c.benchmark_group("parse_document_batch");
    group.throughput(Throughput::Bytes(total_bytes));
    group.bench_function("ALL_DATASET_PAGES", |b| {
        b.iter(|| {
            for (_, html) in FUND_PAGES {
                black_box(parse_document(black_box(html)));
            }
        });
    });
    group.finish();
}

/// Verifies the parse doesn't panic and produces non-null output.
/// Not a performance benchmark — used to confirm correctness before timing.
fn bench_sanity(c: &mut Criterion) {
    c.bench_function("parse_document/sanity_check", |b| {
        b.iter(|| {
            for (_, html) in FUND_PAGES {
                let (chart, info) = parse_document(black_box(html));
                // Prevent the compiler from optimising away the result.
                black_box(chart);
                black_box(info);
            }
        });
    });
}

criterion_group!(
    benches,
    bench_empty,
    bench_fund_pages,
    bench_fund_page_batch,
    bench_sanity
);
criterion_main!(benches);
