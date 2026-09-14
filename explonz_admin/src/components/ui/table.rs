use leptos::prelude::*;
use leptos_ui::clx;

mod components {
    use super::*;

    clx! {TableHeader, thead, "[&_tr]:border-b"}
    clx! {TableBody, tbody, "[&_tr:last-child]:border-0"}
    clx! {TableFooter, tfoot, "bg-muted/50 font-medium [&>tr]:last:border-b-0"}
    clx! {TableRow, tr, "border-b transition-colors hover:bg-muted/50 data-[state=selected]:bg-muted"}
    clx! {TableHead, th, "h-10 px-4 text-left align-middle font-medium text-muted-foreground"}
    clx! {TableCell, td, "px-4 py-3 align-middle"}
    clx! {TableCaption, caption, "mt-4 text-sm text-muted-foreground"}
}

#[component]
pub fn Table(#[prop(into, optional)] class: String, children: Children) -> impl IntoView {
    let merged = tw_merge::tw_merge!("w-full caption-bottom text-sm", class);
    view! {
        <div class="relative w-full overflow-auto">
            <table class=merged>
                {children()}
            </table>
        </div>
    }
}

pub use components::*;
