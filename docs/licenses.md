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
| `chrono-tz` (bundled timezone rules) | 0.10.4 | MIT OR Apache-2.0 |
| `csv` | 1.4.0 | MIT OR Unlicense |
| `pdf-extract` | 0.12.1 | MIT |
| `lopdf` | 0.42.0 | MIT |
| `ttf-parser` | 0.25.1 | MIT OR Apache-2.0 |
| `tempfile` (now runtime export publication) | 3.27.0 | MIT OR Apache-2.0 |
| Bundled Open Sans Regular font | Debian fonts-open-sans asset, Google 2010–2011 | Apache-2.0 |

For dual-licensed components, select and record the license under which the release distributes them and preserve the applicable notices. The repository's own source currently has no public redistribution license (`UNLICENSED` in the frontend package); that does not replace third-party license obligations or define commercial end-user terms.

`chrono-tz` adds locked `phf`/`phf_shared` 0.12.1 (MIT) alongside existing 0.13.1 entries; timezone source data is bundled for offline America/Chicago dates. Package metadata was inspected; full transitive notices remain part of release review.

## Native components and packaging

- `libsqlite3-sys` 0.38.2 declares MIT; its bundled SQLCipher `LICENSE` carries Zetetic's BSD-style three-clause redistribution conditions. Binary distribution must reproduce that notice, conditions, and disclaimer.
- `openssl-src` 300.6.1+3.6.3 declares MIT/Apache-2.0 for its build wrapper; bundled OpenSSL 3.6.3 has its own Apache-2.0 license file. Preserve the native-library obligations as well as Rust wrapper notices.
- Microsoft WebView2 is a separate runtime redistributable. The intended offline installer includes its installer; review Microsoft's current redistribution terms and relevant notices before commercial release. This inventory does not establish that this review has been completed.
- NSIS and Windows installer tooling, all transitive Rust/npm dependencies, and the exact packaged native binaries still need a distribution inventory and notices review.
- CSV parsing uses `csv` 1.4.0 and `csv-core` 0.1.13 (MIT OR Unlicense), with `ryu` 1.0.23 (Apache-2.0 OR BSL-1.0) added by the locked CSV dependency. Metadata was inspected; complete commercial notices review remains open. Text-PDF parsing uses the above MIT crates plus Adobe CMap/type1 encoding parsers (MIT), cff-parser (MIT OR Apache-2.0), postscript (Apache-2.0 OR MIT), and euclid (MIT OR Apache-2.0); source metadata was inspected. Their complete transitive/native artifact notices still require release review. Internal PDF rendering reuses `lopdf`, embedded Open Sans and `ttf-parser` metrics; local vector charts need no chart dependency. The bundled font license/attribution are under `src-tauri/assets/fonts`. OCR dependencies remain unselected.
- Interface fonts use the operating system's font stack; no third-party font file is bundled in this foundation. Recheck if branded fonts or other assets are introduced.

## Before distribution

Generate a complete software bill of materials from both lockfiles and the Windows artifact; review transitive licenses, vulnerability results, bundled assets, and runtime terms; include third-party notices in the installer; and obtain appropriate legal review for branding, end-user terms, privacy documentation, and commercial licensing. Dependency upgrades must refresh this inventory. No conclusion about all dependencies' commercial suitability is made here.

## Local original-document inspection (0.50.0)

`pdfjs-dist` 6.4.299 is pinned in both frontend manifests (Apache-2.0). The
original-document panel bundles its PDF canvas renderer, worker, Adobe CMaps and
standard font assets locally. It does not use the viewer application, scripting
sandbox, annotation/link/form UI, external fonts or a remote document service.
Wasm and optional Node canvas are not used in the browser renderer. Existing Rust
PDF intake validation and source retention remain authoritative.

The upstream Apache license, Adobe CMap notice, Foxit standard-font notice and
Liberation font notice are copied unchanged to `public/licenses/pdfjs/` and are
included by the normal frontend build. These files cover the specifically bundled
viewer assets; they do not replace the full release SBOM/transitive/native review.
`@napi-rs/canvas` is an upstream optional Node dependency, not imported into the
browser bundle. The renderer's asset map allows only bundled font/CMap filenames.
