use explonz_shared::icons::LabelIcon;
use leptos::prelude::*;

/// 根据 LabelIcon 枚举值渲染对应的 SVG 图标组件。
/// icons crate 中的组件无法在运行时动态构造，故用 match 静态分发。
#[component]
pub fn LabelIconView(icon: LabelIcon) -> impl IntoView {
    match icon {
        LabelIcon::Tag           => view! { <icons::Tag /> }.into_any(),
        LabelIcon::Users         => view! { <icons::Users /> }.into_any(),
        LabelIcon::Star          => view! { <icons::Star /> }.into_any(),
        LabelIcon::MapPin        => view! { <icons::MapPin /> }.into_any(),
        LabelIcon::Flame         => view! { <icons::Flame /> }.into_any(),
        LabelIcon::Coffee        => view! { <icons::Coffee /> }.into_any(),
        LabelIcon::Camera        => view! { <icons::Camera /> }.into_any(),
        LabelIcon::Wifi          => view! { <icons::Wifi /> }.into_any(),
        LabelIcon::Clock         => view! { <icons::Clock /> }.into_any(),
        LabelIcon::Mountain      => view! { <icons::Mountain /> }.into_any(),
        LabelIcon::TreePine      => view! { <icons::TreePine /> }.into_any(),
        LabelIcon::Waves         => view! { <icons::Waves /> }.into_any(),
        LabelIcon::Baby          => view! { <icons::Baby /> }.into_any(),
        LabelIcon::PawPrint      => view! { <icons::PawPrint /> }.into_any(),
        LabelIcon::Bike          => view! { <icons::Bike /> }.into_any(),
        LabelIcon::Tent          => view! { <icons::Tent /> }.into_any(),
        LabelIcon::Sunset        => view! { <icons::Sunset /> }.into_any(),
        LabelIcon::Accessibility => view! { <icons::Accessibility /> }.into_any(),
    }
}
