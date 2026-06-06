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

#[component]
pub fn HomeIcon(class: String) -> Element {
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
                d: "m2.25 12 8.954-8.955c.44-.439 1.152-.439 1.591 0L21.75 12M4.5 9.75v10.125c0 .621.504 1.125 1.125 1.125H9.75v-4.875c0-.621.504-1.125 1.125-1.125h2.25c.621 0 1.125.504 1.125 1.125V21h4.125c.621 0 1.125-.504 1.125-1.125V9.75M8.25 21h8.25",
            }
        }
    }
}

#[component]
pub fn Squares2x2(class: String) -> Element {
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
                d: "M3.75 6A2.25 2.25 0 0 1 6 3.75h2.25A2.25 2.25 0 0 1 10.5 6v2.25a2.25 2.25 0 0 1-2.25 2.25H6a2.25 2.25 0 0 1-2.25-2.25V6ZM3.75 15.75A2.25 2.25 0 0 1 6 13.5h2.25a2.25 2.25 0 0 1 2.25 2.25V18a2.25 2.25 0 0 1-2.25 2.25H6A2.25 2.25 0 0 1 3.75 18v-2.25ZM13.5 6a2.25 2.25 0 0 1 2.25-2.25H18A2.25 2.25 0 0 1 20.25 6v2.25A2.25 2.25 0 0 1 18 10.5h-2.25a2.25 2.25 0 0 1-2.25-2.25V6ZM13.5 15.75a2.25 2.25 0 0 1 2.25-2.25H18a2.25 2.25 0 0 1 2.25 2.25V18A2.25 2.25 0 0 1 18 20.25h-2.25A2.25 2.25 0 0 1 13.5 18v-2.25Z",
            }
        }
    }
}

#[component]
pub fn ArrowsRightLeft(class: String) -> Element {
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
                d: "M7.5 21 3 16.5m0 0L7.5 12M3 16.5h13.5m0-13.5L21 7.5m0 0L16.5 12M21 7.5H7.5",
            }
        }
    }
}

#[component]
pub fn UserIcon(class: String) -> Element {
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
                d: "M15.75 6a3.75 3.75 0 1 1-7.5 0 3.75 3.75 0 0 1 7.5 0ZM4.501 20.118a7.5 7.5 0 0 1 14.998 0A17.933 17.933 0 0 1 12 21.75c-2.676 0-5.216-.584-7.499-1.632Z",
            }
        }
    }
}

/// GitHub Invertocat mark. Uses `fill` (not stroke) — pass a `text-*` color class.
#[component]
pub fn GitHubIcon(class: String) -> Element {
    rsx! {
        svg {
            xmlns: "http://www.w3.org/2000/svg",
            "viewBox": "0 0 24 24",
            fill: "currentColor",
            class: "{class}",
            path { d: "M12 .297c-6.63 0-12 5.373-12 12 0 5.303 3.438 9.8 8.205 11.385.6.113.82-.258.82-.577 0-.285-.01-1.04-.015-2.04-3.338.724-4.042-1.61-4.042-1.61C4.422 18.07 3.633 17.7 3.633 17.7c-1.087-.744.084-.729.084-.729 1.205.084 1.838 1.236 1.838 1.236 1.07 1.835 2.809 1.305 3.495.998.108-.776.417-1.305.76-1.605-2.665-.3-5.466-1.332-5.466-5.93 0-1.31.465-2.38 1.235-3.22-.135-.303-.54-1.523.105-3.176 0 0 1.005-.322 3.3 1.23.96-.267 1.98-.399 3-.405 1.02.006 2.04.138 3 .405 2.28-1.552 3.285-1.23 3.285-1.23.645 1.653.24 2.873.12 3.176.765.84 1.23 1.91 1.23 3.22 0 4.61-2.805 5.625-5.475 5.92.42.36.81 1.096.81 2.22 0 1.606-.015 2.896-.015 3.286 0 .315.21.69.825.57C20.565 22.092 24 17.592 24 12.297c0-6.627-5.373-12-12-12" }
        }
    }
}

#[component]
pub fn AdjustmentsHorizontal(class: String) -> Element {
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
                d: "M10.5 6h9.75M10.5 6a1.5 1.5 0 1 1-3 0m3 0a1.5 1.5 0 1 0-3 0M3.75 6H7.5m3 12h9.75m-9.75 0a1.5 1.5 0 0 1-3 0m3 0a1.5 1.5 0 0 0-3 0m-3.75 0H7.5m9-6h3.75m-3.75 0a1.5 1.5 0 0 1-3 0m3 0a1.5 1.5 0 0 0-3 0m-9.75 0h9.75",
            }
        }
    }
}
