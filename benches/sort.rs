use criterion::{Criterion, black_box, criterion_group, criterion_main};
use sort_package_json::sort_package_json;

fn bench_small_package(c: &mut Criterion) {
    let input = include_str!("../tests/fixtures/package.json");
    c.bench_function("sort small package.json", |b| {
        b.iter(|| sort_package_json(black_box(input)));
    });
}

fn bench_already_sorted(c: &mut Criterion) {
    let input = include_str!("../tests/fixtures/package.json");
    let sorted = sort_package_json(input).unwrap();
    c.bench_function("sort already sorted package.json", |b| {
        b.iter(|| sort_package_json(black_box(&sorted)));
    });
}

fn bench_minimal_package(c: &mut Criterion) {
    let input = r#"{
  "version": "1.0.0",
  "name": "test",
  "description": "A test package"
}"#;
    c.bench_function("sort minimal package.json", |b| {
        b.iter(|| sort_package_json(black_box(input)));
    });
}

fn bench_large_package(c: &mut Criterion) {
    let input = r#"{
  "version": "1.0.0",
  "dependencies": {
    "react": "^18.0.0",
    "axios": "^1.0.0",
    "lodash": "^4.17.21",
    "express": "^4.18.0",
    "typescript": "^5.0.0",
    "webpack": "^5.0.0",
    "babel-loader": "^9.0.0",
    "eslint": "^8.0.0",
    "prettier": "^3.0.0",
    "jest": "^29.0.0"
  },
  "devDependencies": {
    "@types/react": "^18.0.0",
    "@types/node": "^20.0.0",
    "@types/express": "^4.17.0",
    "ts-node": "^10.0.0",
    "nodemon": "^3.0.0",
    "concurrently": "^8.0.0"
  },
  "scripts": {
    "test": "jest",
    "build": "webpack",
    "lint": "eslint .",
    "format": "prettier --write .",
    "dev": "nodemon",
    "start": "node dist/index.js",
    "pretest": "npm run lint",
    "postbuild": "echo 'Build complete'"
  },
  "name": "large-package",
  "description": "A larger test package",
  "keywords": ["test", "large", "package", "example", "benchmark"],
  "author": "Test Author <test@example.com>",
  "license": "MIT",
  "repository": {
    "type": "git",
    "url": "https://github.com/test/test"
  },
  "bugs": {
    "url": "https://github.com/test/test/issues"
  },
  "homepage": "https://github.com/test/test#readme",
  "main": "./dist/index.js",
  "types": "./dist/index.d.ts",
  "files": ["dist", "README.md", "LICENSE"],
  "engines": {
    "node": ">=18.0.0",
    "npm": ">=8.0.0"
  }
}"#;
    c.bench_function("sort large package.json", |b| {
        b.iter(|| sort_package_json(black_box(input)));
    });
}

criterion_group!(
    benches,
    bench_small_package,
    bench_already_sorted,
    bench_minimal_package,
    bench_large_package
);
criterion_main!(benches);
