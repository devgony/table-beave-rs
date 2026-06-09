use leptos::prelude::*;

use crate::parser::{markdown_to_html, parse_ascii_table};

const SAMPLE_TABLE: &str = r#"# Unicode box table

┌────────────┬───────────────┐
│ Animal     │ Role          │
├────────────┼───────────────┤
│ Box beaver │ Parser mascot │
│ Data otter │ Reviewer      │
└────────────┴───────────────┘

# ASCII table

+------------+---------------+
| Animal     | Role          |
+------------+---------------+
| Box beaver | Parser mascot |
| Data otter | Reviewer      |
+------------+---------------+"#;

pub fn mount() {
    mount_to_body(App);
}

#[component]
fn App() -> impl IntoView {
    let (input, set_input) = signal(SAMPLE_TABLE.to_string());
    let (first_row_is_header, set_first_row_is_header) = signal(true);
    let parsed = Memo::new(move |_| parse_ascii_table(&input.get(), first_row_is_header.get()));

    let copy_output = move |_| {
        let text = parsed.get().markdown;
        if !text.is_empty() {
            copy_to_clipboard(&text);
        }
    };

    let load_sample = move |_| {
        set_input.set(SAMPLE_TABLE.to_string());
    };

    let clear_input = move |_| {
        set_input.set(String::new());
    };

    view! {
        <main class="app-shell">
            <section class="workspace">
                <header class="topbar">
                    <div class="brand">
                        <img
                            class="brand-icon"
                            src="images/beave-rs-icon.png"
                            alt="A beaver gnawing a data table"
                        />
                        <div>
                            <h1 class="brand-title">
                                <span class="brand-prefix">"Table"</span><span class="brand-name">"Beave"</span><span class="brand-suffix">"-rs"</span>
                            </h1>
                            <p>"Cute beavers gnaw your Unicode & ASCII box tables into Markdown pipe tables."</p>
                        </div>
                    </div>
                    <div class="status-strip">
                        <span>{move || format!("{} columns", parsed.get().column_count)}</span>
                        <span>{move || format!("{} data rows", parsed.get().row_count)}</span>
                    </div>
                </header>

                <div class="tool-grid">
                    <section class="panel">
                        <div class="panel-heading">
                            <h2>"Unicode / ASCII Box Input"</h2>
                            <div class="actions">
                                <button type="button" on:click=load_sample>"Sample"</button>
                                <button type="button" on:click=clear_input>"Clear"</button>
                            </div>
                        </div>

                        <textarea
                            class="editor"
                            aria-label="Unicode or ASCII box table input"
                            spellcheck="false"
                            prop:value=move || input.get()
                            on:input=move |ev| set_input.set(event_target_value(&ev))
                        ></textarea>

                        <label class="option-row">
                            <input
                                type="checkbox"
                                prop:checked=move || first_row_is_header.get()
                                on:change=move |ev| {
                                    let checked = event_target_checked(&ev);
                                    set_first_row_is_header.set(checked);
                                }
                            />
                            <span>"Use the first row as the Markdown header"</span>
                        </label>
                    </section>

                    <section class="panel">
                        <div class="panel-heading">
                            <h2>"Markdown Output"</h2>
                            <div class="actions">
                                <button type="button" on:click=copy_output>"Copy"</button>
                            </div>
                        </div>

                        <textarea
                            class="editor output"
                            aria-label="Markdown output"
                            readonly
                            spellcheck="false"
                            prop:value=move || parsed.get().markdown
                        ></textarea>

                        <Show
                            when=move || !parsed.get().warnings.is_empty()
                            fallback=|| view! { <p class="hint">"Ready for Notion, Obsidian, and GitHub Markdown."</p> }
                        >
                            <ul class="warnings">
                                {move || {
                                    parsed
                                        .get()
                                        .warnings
                                        .into_iter()
                                        .map(|warning| view! { <li>{warning}</li> })
                                        .collect::<Vec<_>>()
                                }}
                            </ul>
                        </Show>
                    </section>
                </div>

                <section class="panel preview-panel">
                    <div class="panel-heading">
                        <h2>"Markdown Preview"</h2>
                    </div>
                    <Show
                        when=move || !parsed.get().markdown.is_empty()
                        fallback=|| view! { <p class="hint">"Rendered tables appear here."</p> }
                    >
                        <div
                            class="preview"
                            inner_html=move || markdown_to_html(&parsed.get().markdown)
                        ></div>
                    </Show>
                </section>
            </section>
        </main>
    }
}

fn copy_to_clipboard(text: &str) {
    if let Some(window) = web_sys::window() {
        let clipboard = window.navigator().clipboard();
        let _ = clipboard.write_text(text);
    }
}
