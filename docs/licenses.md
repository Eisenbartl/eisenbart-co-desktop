# Initial dependency license inventory

This is a preliminary inventory of runtime dependencies, not a completed commercial distribution audit or legal opinion. Versions below were matched to the committed dependency locks and inspected package metadata. Native component licenses were also inspected in the downloaded source. A release must include the required notices and review the full dependency tree and actual installer contents.

## Direct runtime packages

| Component | Locked version | Declared license |
| --- | --- | --- |
| `@tauri-apps/api` | 2.12.3 | Apache-2.0 OR MIT |
| React | 19.3.0 | MIT |
| React DOM | 19.3.0 | MIT |
| Rust `tauri` | 2.12.3 | Apache-2.0 OR MIT |
| `rusqlite` | 0.40.2 | MIT |
| `argon2` | 0.5.3 | MIT OR Apache-2.0 |
| `rand` | 0.8.8 | MIT OR Apache-2.0 |
| `uuid` | 1.28.0 | Apache-2.0 OR MIT |
| `serde` | 1.0.229 | MIT OR Apache-2.0 |
| `serde_json` | 1.0.154 | MIT OR Apache-2.0 |
| `zeroize` | 1.9.1 | Apache-2.0 OR MIT |
| `chrono` | 0.4.45 | MIT OR Apache-2.0 |

For dual-licensed components, select and record the license under which the release distributes them and preserve the applicable notices. The repository's own source currently has no public redistribution license (`UNLICENSED` in the frontend package); that does not replace third-party license obligations or define commercial end-user terms.

## Native components and packaging

- `libsqlite3-sys` 0.38.2 declares MIT; its bundled SQLCipher `LICENSE` carries Zetetic's BSD-style three-clause redistribution conditions. Binary distribution must reproduce that notice, conditions, and disclaimer.
- `openssl-src` 300.6.1+3.6.3 declares MIT/Apache-2.0 for its build wrapper; bundled OpenSSL 3.6.3 has its own Apache-2.0 license file. Preserve the native-library obligations as well as Rust wrapper notices.
- Microsoft WebView2 is a separate runtime redistributable. The intended offline installer includes its installer; review Microsoft's current redistribution terms and relevant notices before commercial release. This inventory does not establish that this review has been completed.
- NSIS and Windows installer tooling, all transitive Rust/npm dependencies, and the exact packaged native binaries still need a distribution inventory and notices review.
- No document-parsing, OCR, PDF-report, or charting dependencies have been selected for later milestones. Their licensing must be reviewed before adoption.
- Interface fonts use the operating system's font stack; no third-party font file is bundled in this foundation. Recheck if branded fonts or other assets are introduced.

## Before distribution

Generate a complete software bill of materials from both lockfiles and the Windows artifact; review transitive licenses, vulnerability results, bundled assets, and runtime terms; include third-party notices in the installer; and obtain appropriate legal review for branding, end-user terms, privacy documentation, and commercial licensing. Dependency upgrades must refresh this inventory. No conclusion about all dependencies' commercial suitability is made here.
