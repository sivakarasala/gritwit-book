// Chapter 11: Reusable Components
// Spotlight: Ownership & Borrowing Deep Dive
//
// DeleteModal — information hiding: doesn't know what it's deleting.

use leptos::prelude::*;

#[component]
pub fn DeleteModal(
    show: RwSignal<bool>,
    title: &'static str,
    message: String,
    on_confirm: impl Fn() + Clone + 'static,
) -> impl IntoView {
    let on_confirm = on_confirm.clone();

    let handle_confirm = move |_| {
        on_confirm();
        show.set(false);
    };

    let handle_cancel = move |_| {
        show.set(false);
    };

    view! {
        <Show when=move || show.get()>
            <div class="modal-overlay" on:click=handle_cancel.clone()>
                <div class="modal" on:click=|e| e.stop_propagation()>
                    <h3>{title}</h3>
                    <p>{message.clone()}</p>
                    <div class="modal-actions">
                        <button class="btn btn--cancel" on:click=handle_cancel.clone()>
                            "Cancel"
                        </button>
                        <button class="btn btn--danger" on:click=handle_confirm.clone()>
                            "Delete"
                        </button>
                    </div>
                </div>
            </div>
        </Show>
    }
}
