use leptos::prelude::*;

use crate::server::spots::get_spots;

#[component]
pub fn SpotList() -> impl IntoView {
    let fetch_trigger = RwSignal::new(false);
    Effect::new(move |_| {
        fetch_trigger.set(true);
    });

    let spots = Resource::new(
        move || fetch_trigger.get(),
        move |ready| async move {
            if !ready {
                return Ok(vec![]);
            }
            get_spots(1, 20).await
        },
    );

    view! {
        <div>
            <h1>"Spots List"</h1>
        </div>
    }
}
