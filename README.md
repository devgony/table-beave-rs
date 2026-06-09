<div align="center">
  <img src="images/beave-rs-icon.png" alt="A beaver gnawing a data table" width="120" />
  <h1>Table<sub>Beave-rs</sub></h1>
  <p>Cute beavers gnaw your Unicode &amp; ASCII box tables into Markdown pipe tables.</p>
  <p>
    <a href="https://table.beave.workers.dev"><strong>🔗 Live demo</strong></a> ·
    <a href="https://github.com/devgony/table-beave-rs"><strong>Source</strong></a>
  </p>
</div>

---

`+---+`, `┌───┐` 같은 **유니코드/ASCII 박스 표**를 붙여넣으면, GitHub·Notion·Obsidian에서 바로 쓸 수 있는 **Markdown 파이프 표**(`| a | b |`)로 변환해 줍니다. 파서는 Rust로 작성되었고, WebAssembly로 컴파일되어 **브라우저 안에서 전부 동작**합니다. 서버는 없습니다.

```text
┌────────────┬───────────────┐                | Animal     | Role          |
│ Animal     │ Role          │                | ---        | ---           |
├────────────┼───────────────┤   ──gnaw──▶    | Box beaver | Parser mascot |
│ Box beaver │ Parser mascot │                | Data otter | Reviewer      |
│ Data otter │ Reviewer      │
└────────────┴───────────────┘
```

## 주요 기능

- **유니코드 박스 + ASCII 박스 모두 지원** — `┌─┬─┐ │ ├─┼─┤ └─┴─┘` (light/heavy/double) 와 `+---+ | =` 를 한 파서로 처리
- **문서 통째로 변환** — 제목(`# ...`)이나 본문 문단은 그대로 두고, 표만 골라 표 단위로 변환
- **멀티라인 셀** — 여러 줄에 걸친 셀을 `<br>` 로 합침
- **헤더 옵션** — 첫 행을 헤더로 쓰거나, 없으면 `Column 1, Column 2 …` 를 자동 생성
- **라이브 미리보기** — 변환된 Markdown을 실시간 HTML 표로 렌더링
- **클립보드 복사 / 컬럼·행 수 표시 / 경고 메시지**

## 파싱은 어떻게 했나 — 외부 라이브러리 없이 자체 구현

> 표 파싱에는 **어떤 외부 크레이트도 쓰지 않았습니다.** `src/parser.rs` 의 손으로 짠 파서가 전부입니다.
> 의존하는 `pulldown-cmark` 는 **표 파싱과 무관**하며, 변환 결과 Markdown을 *미리보기 HTML로 렌더링*할 때만 씁니다.

박스 표는 형식이 제각각(코너 문자, 선 굵기, 구분자 종류, 들여쓰기, 멀티라인 셀)이라 범용 파서로는 깔끔히 안 잡힙니다. 그래서 줄 단위로 분류·그룹핑하는 작은 파서를 직접 만들었습니다. 흐름은 다음과 같습니다.

1. **세그먼트 분리** — `split_segments`
   문서를 *표 블록*(연속된 표 줄 묶음)과 *통과 줄*(제목·문단·빈 줄, 원문 그대로 유지)로 나눕니다. 덕분에 제목과 여러 표가 섞인 문서도 표만 골라 변환하고 나머지는 보존합니다.

2. **줄 분류** — 두 종류의 줄을 문자 단위로 판별
   - `is_horizontal_rule` : 구분선 판별. ASCII(`+ - = :`)와 유니코드 박스 드로잉 문자(코드포인트 `U+2500‒U+257F`)를 함께 인식하고, "선 문자 + 코너/박스 문자"가 있어야 구분선으로 봅니다.
   - `detect_cell_delimiter` : 내용 줄의 구분자 판별. 줄이 `|`, `│`(U+2502), `┃`(U+2503), `║`(U+2551) 중 하나로 시작·끝나고 2개 이상 있으면 셀 줄로 봅니다 → ASCII 파이프뿐 아니라 **유니코드 세로선(가는/굵은/이중)** 까지 지원.

3. **행 그룹핑** — `collect_row_groups`
   구분선 사이의 셀 줄들을 하나의 "논리적 행"으로 묶습니다. 구분선이 있으면 그 사이가 한 행(멀티라인 셀 지원), 구분선이 전혀 없으면 셀 줄 하나가 곧 한 행입니다.

4. **셀 정리** — `collapse_group`
   각 행 그룹을 구분자로 쪼개 셀별로 trim 하고, 여러 줄에 걸친 셀 조각을 `<br>` 로 합칩니다. 셀 안의 `|` 는 `\|` 로 이스케이프합니다.

5. **Markdown 출력** — `render_markdown_table`
   GitHub 스타일 파이프 표(헤더 행 + `---` 구분 행 + 본문 행)를 만듭니다. 첫 행 헤더 옵션이 꺼져 있으면 `Column N` 헤더를 합성합니다.

6. **정리** — `tidy_blank_lines`
   앞뒤 빈 줄을 없애고 연속 빈 줄을 하나로 합쳐 출력을 깔끔하게 다듬습니다.

> **유니코드 처리:** 박스 드로잉 감지는 `U+2500‒U+257F` 범위로, 세로 구분자는 가는/굵은/이중선을 모두 받습니다. Rust의 네이티브 `char` 순회를 쓰기 때문에 한글·원문자(①②③)·가운뎃점(·) 같은 멀티바이트 UTF-8도 정확히 처리됩니다. (관련 테스트가 `src/parser.rs` 에 포함되어 있습니다.)

## Rust 파서가 웹에서 도는 원리 — Leptos + WebAssembly

이 프로젝트에는 **서버가 없습니다.** Rust 코드를 WebAssembly로 컴파일해 브라우저에서 직접 실행합니다.

```text
src/*.rs ──(cargo, target=wasm32)──▶ .wasm
        ──(wasm-bindgen: JS 바인딩 생성)──▶ glue.js
        ──(wasm-opt -Oz: 용량 최적화)──▶ dist/
              │
              ▼
        index.html 이 init() 으로 wasm 로드
              │
              ▼
        main() → app::mount() → Leptos 컴포넌트를 <body>에 마운트
              │
              ▼
        타이핑할 때마다 parse_ascii_table() 가 브라우저에서 실행 (reactive)
```

- **조건부 컴파일** (`src/main.rs`)
  `cfg(target_arch = "wasm32")` 일 때만 Leptos 앱을 마운트합니다. 네이티브 빌드에서는 안내 문구만 출력하므로, `parser` 모듈을 `cargo test` 로 네이티브에서 단위 테스트할 수 있습니다. 즉 **파서 로직은 wasm/네이티브가 공유**합니다.

- **반응형 UI** (`src/app.rs`, Leptos 0.8 CSR)
  입력 `signal` → `Memo` 가 입력이 바뀔 때마다 `parse_ascii_table` 를 다시 돌리고, 출력 textarea·미리보기·상태표시가 자동 갱신됩니다. 클립보드 복사는 `web-sys` 로 처리합니다.

- **빌드 파이프라인** (Trunk)
  `index.html` 의 `<link data-trunk rel="rust" data-wasm-opt="z" />` 지시를 보고 Trunk이 ① `wasm32-unknown-unknown` 으로 컴파일 → ② `wasm-bindgen` 으로 JS 바인딩 생성 → ③ `wasm-opt -Oz` 로 용량 최적화 → ④ 해시 붙은 wasm/js/css를 `dist/` 로 번들합니다.

- **배포** (Cloudflare Workers 정적 자산)
  `wrangler.jsonc` 가 `dist/` 를 정적 자산으로 서빙합니다(서버 코드 `main` 없음, SPA fallback). `make deploy` 가 release 빌드 후 배포합니다.

## 의존성

| 크레이트 | 용도 | 비고 |
|---|---|---|
| `pulldown-cmark` 0.13 | 변환 결과 Markdown → 미리보기 HTML 렌더링 | 표 파싱과 무관. `html` 기능만 사용 |
| `leptos` 0.8 (`csr`) | 반응형 UI 컴포넌트 | `wasm32` 타깃 전용 |
| `web-sys` 0.3 | 클립보드 등 브라우저 API | `wasm32` 타깃 전용 |

## 개발 / 빌드 / 배포

사전 준비: Rust(+ `wasm32-unknown-unknown` 타깃), [Trunk](https://trunkrs.dev), Node(npx, wrangler 용).

```bash
make dev      # 로컬 개발 서버 (라이브 리로드)  →  trunk serve
make build    # 최적화 프로덕션 번들           →  trunk build --release  (dist/)
make test     # 파서 테스트                     →  cargo test
make deploy   # 빌드 후 Cloudflare 배포         →  build + wrangler deploy
make help     # 사용 가능한 타깃 목록
```

## 프로젝트 구조

```text
table-beave-rs/
├── index.html         # Trunk 엔트리(rust/css 지시) + Cloudflare Web Analytics beacon
├── style.css          # UI 스타일
├── src/
│   ├── main.rs        # 진입점. wasm일 때 Leptos 마운트 / 네이티브일 때 안내 출력
│   ├── app.rs         # Leptos CSR UI 컴포넌트 (반응형 상태, 클립보드)
│   └── parser.rs      # 박스 표 파서 (자체 구현) + 미리보기 렌더 + 테스트
├── images/            # 아이콘 등 정적 자산
├── Cargo.toml         # 의존성 / 릴리스 프로필(opt-level=z, lto)
├── Trunk 출력 → dist/ # 빌드 산출물 (git 무시)
├── wrangler.jsonc     # Cloudflare Workers 정적 자산 설정
└── Makefile           # dev / build / test / deploy 타깃
```
