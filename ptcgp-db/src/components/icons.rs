use dioxus::prelude::*;

#[component]
pub fn ChevronUp(class: String) -> Element {
    rsx! {
        svg {
            xmlns: "http://www.w3.org/2000/svg",
            fill: "none",
            "viewBox": "0 0 24 24",
            "stroke-width": "1.5",
            stroke: "currentColor",
            class: "{class}",
            path {
                "stroke-linecap": "round",
                "stroke-linejoin": "round",
                d: "m4.5 15.75 7.5-7.5 7.5 7.5",
            }
        }
    }
}

#[component]
pub fn ChevronDown(class: String) -> Element {
    rsx! {
        svg {
            xmlns: "http://www.w3.org/2000/svg",
            fill: "none",
            "viewBox": "0 0 24 24",
            "stroke-width": "1.5",
            stroke: "currentColor",
            class: "{class}",
            path {
                "stroke-linecap": "round",
                "stroke-linejoin": "round",
                d: "m19.5 8.25-7.5 7.5-7.5-7.5",
            }
        }
    }
}

#[component]
pub fn Bars3(class: String) -> Element {
    rsx! {
        svg {
            xmlns: "http://www.w3.org/2000/svg",
            fill: "none",
            "viewBox": "0 0 24 24",
            "stroke-width": "1.5",
            stroke: "currentColor",
            class: "{class}",
            path {
                "stroke-linecap": "round",
                "stroke-linejoin": "round",
                d: "M3.75 6.75h16.5M3.75 12h16.5m-16.5 5.25h16.5",
            }
        }
    }
}

#[component]
pub fn Check(class: String) -> Element {
    rsx! {
        svg {
            xmlns: "http://www.w3.org/2000/svg",
            fill: "none",
            "viewBox": "0 0 24 24",
            "stroke-width": "1.5",
            stroke: "currentColor",
            class: "{class}",
            path {
                "stroke-linecap": "round",
                "stroke-linejoin": "round",
                d: "m4.5 12.75 6 6 9-13.5",
            }
        }
    }
}

#[component]
pub fn Plus(class: String) -> Element {
    rsx! {
        svg {
            xmlns: "http://www.w3.org/2000/svg",
            fill: "none",
            "viewBox": "0 0 24 24",
            "stroke-width": "1.5",
            stroke: "currentColor",
            class: "{class}",
            path {
                "stroke-linecap": "round",
                "stroke-linejoin": "round",
                d: "M12 4.5v15m7.5-7.5h-15",
            }
        }
    }
}

#[component]
pub fn Minus(class: String) -> Element {
    rsx! {
        svg {
            xmlns: "http://www.w3.org/2000/svg",
            fill: "none",
            "viewBox": "0 0 24 24",
            "stroke-width": "1.5",
            stroke: "currentColor",
            class: "{class}",
            path {
                "stroke-linecap": "round",
                "stroke-linejoin": "round",
                d: "M5 12h14",
            }
        }
    }
}

#[component]
pub fn XMark(class: String) -> Element {
    rsx! {
        svg {
            xmlns: "http://www.w3.org/2000/svg",
            fill: "none",
            "viewBox": "0 0 24 24",
            "stroke-width": "1.5",
            stroke: "currentColor",
            class: "{class}",
            path {
                "stroke-linecap": "round",
                "stroke-linejoin": "round",
                d: "M6 18 18 6M6 6l12 12",
            }
        }
    }
}

#[component]
pub fn ArrowLeft(class: String) -> Element {
    rsx! {
        svg {
            xmlns: "http://www.w3.org/2000/svg",
            fill: "none",
            "viewBox": "0 0 24 24",
            "stroke-width": "1.5",
            stroke: "currentColor",
            class: "{class}",
            path {
                "stroke-linecap": "round",
                "stroke-linejoin": "round",
                d: "M10.5 19.5 3 12m0 0 7.5-7.5M3 12h18",
            }
        }
    }
}
