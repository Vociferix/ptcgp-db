use dioxus::prelude::*;

use crate::components::nav::NavLayout;
use crate::pages::{
    AnalysisPage, CardDetailPage, CatalogPage, ImportExportPage, ProfileManagerPage, SettingsPage,
    SummaryPage, TradePage,
};

/// App routes. Web builds use hash routing (e.g. `/#/catalog`); desktop/mobile use the default
/// native history. Route names match their corresponding page component functions.
#[derive(Clone, Routable, Debug, PartialEq)]
#[allow(clippy::enum_variant_names)]
pub enum Route {
    #[layout(NavLayout)]
    #[route("/")]
    SummaryPage {},
    #[route("/catalog")]
    CatalogPage {},
    #[route("/catalog/:card_id")]
    CardDetailPage { card_id: usize },
    #[route("/analysis")]
    AnalysisPage {},
    #[route("/trade")]
    TradePage {},
    #[route("/profiles")]
    ProfileManagerPage {},
    #[route("/import-export")]
    ImportExportPage {},
    #[route("/settings")]
    SettingsPage {},
}
