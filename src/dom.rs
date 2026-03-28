use crate::error::BrowserError;
use anyhow::Result;
use scraper::{Html, Selector, ElementRef};
use serde::Serialize;
use std::collections::HashMap;

pub struct Document {
    html: Html,
}

#[derive(Serialize, Clone)]
pub struct Element {
    pub index: usize,
    pub text: String,
    pub html: String,
    pub tag: String,
    pub attributes: HashMap<String, String>,
}

#[derive(Serialize)]
pub struct Link {
    pub index: usize,
    pub href: String,
    pub text: String,
}

#[derive(Serialize)]
pub struct Form {
    pub index: usize,
    pub action: String,
    pub method: String,
    pub id: Option<String>,
    pub name: Option<String>,
    pub fields: Vec<FormField>,
}

#[derive(Serialize, Clone)]
pub struct FormField {
    pub name: String,
    #[serde(rename = "type")]
    pub field_type: String,
    pub value: String,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub required: bool,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub checked: bool,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub options: Vec<SelectOption>,
}

#[derive(Serialize, Clone)]
pub struct SelectOption {
    pub value: String,
    pub text: String,
    pub selected: bool,
}

#[derive(Serialize)]
pub struct Table {
    pub index: usize,
    pub headers: Vec<String>,
    pub rows: Vec<Vec<String>>,
}

impl Document {
    pub fn parse(html_str: &str) -> Self {
        Document {
            html: Html::parse_document(html_str),
        }
    }

    pub fn select(&self, selector_str: &str) -> Result<Vec<Element>> {
        let selector = Selector::parse(selector_str)
            .map_err(|_| BrowserError::InvalidSelector(selector_str.to_string()))?;

        let elements: Vec<Element> = self
            .html
            .select(&selector)
            .enumerate()
            .map(|(i, el)| element_from_ref(i, el))
            .collect();

        Ok(elements)
    }

    pub fn extract_text(&self, selector_str: &str) -> Result<String> {
        let selector = Selector::parse(selector_str)
            .map_err(|_| BrowserError::InvalidSelector(selector_str.to_string()))?;

        let text: Vec<String> = self
            .html
            .select(&selector)
            .map(|el| el.text().collect::<Vec<_>>().join(" "))
            .collect();

        Ok(text.join("\n"))
    }

    pub fn extract_links(&self, selector_str: &str, base_url: Option<&url::Url>) -> Result<Vec<Link>> {
        let selector = Selector::parse(selector_str)
            .map_err(|_| BrowserError::InvalidSelector(selector_str.to_string()))?;

        let links: Vec<Link> = self
            .html
            .select(&selector)
            .enumerate()
            .filter_map(|(i, el)| {
                let href = el.value().attr("href")?;
                let resolved = if let Some(base) = base_url {
                    base.join(href).map(|u| u.to_string()).unwrap_or_else(|_| href.to_string())
                } else {
                    href.to_string()
                };
                Some(Link {
                    index: i,
                    href: resolved,
                    text: el.text().collect::<Vec<_>>().join(" ").trim().to_string(),
                })
            })
            .collect();

        Ok(links)
    }

    pub fn extract_forms(&self) -> Result<Vec<Form>> {
        let form_sel = Selector::parse("form").unwrap();
        let input_sel = Selector::parse("input, select, textarea").unwrap();

        let forms: Vec<Form> = self
            .html
            .select(&form_sel)
            .enumerate()
            .map(|(i, form_el)| {
                let action = form_el.value().attr("action").unwrap_or("").to_string();
                let method = form_el
                    .value()
                    .attr("method")
                    .unwrap_or("GET")
                    .to_uppercase();
                let id = form_el.value().attr("id").map(|s| s.to_string());
                let name = form_el.value().attr("name").map(|s| s.to_string());

                let fields: Vec<FormField> = form_el
                    .select(&input_sel)
                    .filter_map(|input| {
                        let field_name = input.value().attr("name")?.to_string();
                        let tag = input.value().name();
                        let field_type = match tag {
                            "select" => "select".to_string(),
                            "textarea" => "textarea".to_string(),
                            _ => input
                                .value()
                                .attr("type")
                                .unwrap_or("text")
                                .to_string(),
                        };
                        let value = match tag {
                            "textarea" => input.text().collect::<Vec<_>>().join(""),
                            _ => input.value().attr("value").unwrap_or("").to_string(),
                        };
                        let required = input.value().attr("required").is_some();
                        let checked = input.value().attr("checked").is_some();

                        let options = if tag == "select" {
                            let opt_sel = Selector::parse("option").unwrap();
                            input
                                .select(&opt_sel)
                                .map(|opt| SelectOption {
                                    value: opt
                                        .value()
                                        .attr("value")
                                        .unwrap_or("")
                                        .to_string(),
                                    text: opt.text().collect::<Vec<_>>().join("").trim().to_string(),
                                    selected: opt.value().attr("selected").is_some(),
                                })
                                .collect()
                        } else {
                            vec![]
                        };

                        Some(FormField {
                            name: field_name,
                            field_type,
                            value,
                            required,
                            checked,
                            options,
                        })
                    })
                    .collect();

                Form {
                    index: i,
                    action,
                    method,
                    id,
                    name,
                    fields,
                }
            })
            .collect();

        Ok(forms)
    }

    pub fn extract_tables(&self, selector_str: Option<&str>) -> Result<Vec<Table>> {
        let sel_str = selector_str.unwrap_or("table");
        let table_sel = Selector::parse(sel_str)
            .map_err(|_| BrowserError::InvalidSelector(sel_str.to_string()))?;
        let tr_sel = Selector::parse("tr").unwrap();
        let th_sel = Selector::parse("th").unwrap();
        let td_sel = Selector::parse("td").unwrap();

        let tables: Vec<Table> = self
            .html
            .select(&table_sel)
            .enumerate()
            .map(|(i, table_el)| {
                let mut headers = Vec::new();
                let mut rows = Vec::new();

                for tr in table_el.select(&tr_sel) {
                    let ths: Vec<String> = tr
                        .select(&th_sel)
                        .map(|th| th.text().collect::<Vec<_>>().join(" ").trim().to_string())
                        .collect();

                    if !ths.is_empty() && headers.is_empty() {
                        headers = ths;
                        continue;
                    }

                    let tds: Vec<String> = tr
                        .select(&td_sel)
                        .map(|td| td.text().collect::<Vec<_>>().join(" ").trim().to_string())
                        .collect();

                    if !tds.is_empty() {
                        rows.push(tds);
                    } else if !ths.is_empty() {
                        rows.push(ths);
                    }
                }

                Table {
                    index: i,
                    headers,
                    rows,
                }
            })
            .collect();

        Ok(tables)
    }
}

fn element_from_ref(index: usize, el: ElementRef) -> Element {
    let text = el.text().collect::<Vec<_>>().join(" ").trim().to_string();
    let html = el.html();
    let tag = el.value().name().to_string();

    let mut attributes = HashMap::new();
    for attr in el.value().attrs() {
        attributes.insert(attr.0.to_string(), attr.1.to_string());
    }

    Element {
        index,
        text,
        html,
        tag,
        attributes,
    }
}
