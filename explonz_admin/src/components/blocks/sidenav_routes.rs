use heck::ToTitleCase;
use strum::{AsRefStr, Display, EnumIter, EnumString, IntoStaticStr};

/* ========================================================== */
/*                     ✨ FUNCTIONS ✨                        */
/* ========================================================== */

#[derive(Clone, Copy, Display, AsRefStr, IntoStaticStr, EnumString, EnumIter, Debug, PartialEq)]
#[strum(serialize_all = "kebab-case")]
pub enum SidenavRoutes {
    Home,
}

impl SidenavRoutes {
    pub fn view_segment() -> &'static str {
        // * 💁‍♂️ "view" for the moment, I will switch back to "view" when all good.
        "admin"
    }

    /// Detect which sidenav route is active based on URL path.
    /// Returns `Sidenav01` as default if no match is found.
    pub fn from_path(path: &str) -> Self {
        use strum::IntoEnumIterator;
        // Iterate in reverse to match higher numbers first (Sidenav10 before Sidenav01)
        Self::iter()
            .rev()
            .find(|route| path.contains(route.as_ref()))
            .unwrap_or(Self::Home)
    }

    pub fn to_route(self) -> String {
        format!("{}/{}", Self::view_segment(), self.as_ref())
    }

    pub fn to_title(self) -> String {
        self.as_ref().to_title_case()
    }
}

/* ========================================================== */
/*                     ✨ FUNCTIONS ✨                        */
/* ========================================================== */

#[derive(Clone, Copy, Display, AsRefStr, IntoStaticStr, EnumString, EnumIter, Debug, PartialEq)]
#[strum(serialize_all = "kebab-case")]
pub enum ExplonzRoutes {
    Spots,
    Hooks,
}

impl ExplonzRoutes {
    pub fn base_segment() -> &'static str {
        "explonz"
    }

    pub fn to_title(self) -> String {
        self.as_ref().to_title_case()
    }
}

/* ========================================================== */
/*                     ✨ FUNCTIONS ✨                        */
/* ========================================================== */

#[derive(Clone, Copy, Display, AsRefStr, IntoStaticStr, EnumString, EnumIter, Debug, PartialEq)]
#[strum(serialize_all = "kebab-case")]
pub enum SpotsRoutes {
    Addition,
    Alert,
    AlertDialog,
    Button,
}

impl SpotsRoutes {
    pub fn base_segment() -> &'static str {
        "components"
    }

    // http://localhost:3000/view/{sidenav_route}/docs/components
    pub fn base_url_with_sidenav(sidenav: SidenavRoutes) -> String {
        format!(
            "/{}/{}/{}",
            sidenav.to_route(),
            ExplonzRoutes::base_segment(),
            ExplonzRoutes::Spots.as_ref()
        )
    }

    // http://localhost:3000/view/{sidenav_route}/docs/components/XXXXXXX
    pub fn to_route_with_sidenav(self, sidenav: SidenavRoutes) -> String {
        format!("{}/{}", Self::base_url_with_sidenav(sidenav), self.as_ref())
    }

    pub fn to_title(self) -> String {
        self.as_ref().to_title_case()
    }
}

/* ========================================================== */
/*                     ✨ FUNCTIONS ✨                        */
/* ========================================================== */

#[derive(Clone, Copy, Display, AsRefStr, IntoStaticStr, EnumString, EnumIter, Debug, PartialEq)]
#[strum(serialize_all = "kebab-case")]
pub enum HooksRoutes {
    UseCopyClipboard,
    UseLockBodyScroll,
    UseRandom,
}

impl HooksRoutes {
    pub fn base_segment() -> &'static str {
        "hooks"
    }

    // http://localhost:3000/view/{sidenav_route}/docs/hooks
    pub fn base_url_with_sidenav(sidenav: SidenavRoutes) -> String {
        format!(
            "/{}/{}/{}",
            sidenav.to_route(),
            ExplonzRoutes::base_segment(),
            ExplonzRoutes::Hooks.as_ref()
        )
    }

    // http://localhost:3000/view/{sidenav_route}/docs/hooks/XXXXXXX
    pub fn to_route_with_sidenav(self, sidenav: SidenavRoutes) -> String {
        format!("{}/{}", Self::base_url_with_sidenav(sidenav), self.as_ref())
    }

    pub fn to_title(self) -> String {
        self.as_ref().to_title_case()
    }
}
