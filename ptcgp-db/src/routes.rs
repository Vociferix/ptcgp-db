use dioxus::prelude::*;

use crate::pages::{
    AnalysisPage, CatalogPage, ImportExportPage, ProfileManagerPage, SettingsPage, SummaryPage,
    TradePage,
};

/// App routes. Web builds use hash routing (e.g. `/#/catalog`); desktop/mobile use the default
/// native history. Route names match their corresponding page component functions.
#[derive(Clone, Routable, Debug, PartialEq)]
pub enum Route {
    #[route("/")]
    SummaryPage {},
    #[route("/catalog")]
    CatalogPage {},
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
