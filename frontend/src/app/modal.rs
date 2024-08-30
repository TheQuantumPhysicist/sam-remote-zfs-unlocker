use leptos::{
    component, create_effect, create_node_ref, ev::MouseEvent, html::Dialog, logging::warn, view,
    Fragment, IntoView, Signal, SignalGet,
};
use wasm_bindgen_futures::wasm_bindgen::JsCast;

#[component]
pub fn Modal<C>(
    /// A signal that determines whether the modal is currently open.
    #[prop(into)]
    open: Signal<bool>,
    /// A callback to be called when the modal is closed.
    on_close: C,
    /// A link to be used instead of a close button.
    #[prop(optional, into)]
    close_link: Option<String>,
    /// Content of the modal.
    children: impl FnOnce() -> Fragment + 'static,
) -> impl IntoView
where
    C: Fn() + Clone + 'static,
{
    let dialog_el = create_node_ref::<Dialog>();
    let on_click = move |ev: MouseEvent| {
        let rect = dialog_el
            .get()
            .expect("dialog to have been created")
            .get_bounding_client_rect();
        let click_is_in_dialog = rect.top() <= ev.client_y() as f64
            && ev.client_y() as f64 <= rect.top() + rect.height()
            && rect.left() <= ev.client_x() as f64
            && ev.client_x() as f64 <= rect.left() + rect.width();
        if !click_is_in_dialog {
            ev.target()
                .unwrap()
                .unchecked_into::<web_sys::HtmlDialogElement>()
                .close();
        }
    };

    create_effect(move |_| {
        if let Some(dialog) = dialog_el.get() {
            if open.get() {
                if dialog.show_modal().is_err() {
                    warn!("<Modal/> error while calling HTMLDialogElement.showModal()");
                    dialog.set_open(true);
                }
            } else {
                dialog.close();
            }
        }
    });

    view! {
        <dialog
            _ref=dialog_el
            open
            class="border-0 shadow-lg rounded-lg"
            // call the on_close callback when the close event fires
            on:close={
                let on_close = on_close.clone();
                move |_| (on_close.clone())()
            }
            // clicking on ::backdrop should dismiss modal
            on:click=on_click
        >
            <main class="Modal-content">{children()}</main>
            <div class="flex justify-end border-b">
                <form method="dialog" on:submit=move |_| (on_close.clone())()>
                    <a href=close_link.unwrap_or_default()>
                        <button class="Modal-header-close" aria-label="Close">
                            "Close"
                        </button>
                    </a>
                </form>
            </div>
        </dialog>
    }
}
