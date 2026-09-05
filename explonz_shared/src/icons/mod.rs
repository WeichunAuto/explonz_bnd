use serde::{Deserialize, Serialize};
use strum::{Display, EnumIter, EnumString};

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Display, EnumString, EnumIter,
)]
pub enum LabelIcon {
    Tag,
    Users,
    Star,
    MapPin,
    Flame,
    Coffee,
    Camera,
    Wifi,
    Clock,
    Mountain,
    TreePine,
    Waves,
    Baby,
    PawPrint,
    Bike,
    Tent,
    Sunset,
    Accessibility,
}
