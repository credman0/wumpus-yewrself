use wasm_bindgen_futures::JsFuture;
use yew::prelude::*;
use wasm_bindgen_futures::spawn_local;
use reqwest::Client;
use reqwest::header;
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;
use web_sys::window;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Card {
    name: String,
    rarity: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
struct ApiResponse {
    object: String,
    total_cards: u32,
    has_more: bool,
    next_page: Option<String>,
    data: Vec<Card>,
}

async fn sleep(ms: u32) {
    let promise = js_sys::Promise::new(&mut |resolve, _| {
        let window = window().unwrap();
        window.set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, ms as i32).unwrap();
    });
    JsFuture::from(promise).await.unwrap();
}

#[function_component(App)]
fn app() -> Html {
    let cube_name = use_state(|| "my_cube".to_string());
    let without_rarity = use_state(|| false);
    let sets = use_state(|| "akh,dom,war,stx,znr".to_string());
    let cube = use_state(Vec::new);

    let fetch_cube = {
        let cube = cube.clone();
        let cube_name = cube_name.clone();
        let without_rarity = *without_rarity;
        let sets = sets.clone();

        Callback::from(move |_| {
            let cube = cube.clone();
            let sets = sets.clone();
            let cube_name = cube_name.clone();
            let without_rarity = without_rarity;

            spawn_local(async move {
                let mut headers = header::HeaderMap::new();

                let client = Client::builder()
                    .default_headers(headers)
                    .build()
                    .unwrap();

                let sets: Vec<&str> = sets.split(',').collect();
                let query = format!("-t%3ABasic+AND+game%3Apaper+AND+({})",
                    sets.iter().map(|s| format!("set%3A{}", s)).collect::<Vec<_>>().join("+OR+"));
                let base_url = format!("https://api.scryfall.com/cards/search?order=name&format=json&q=({})&page=1", query);

                let mut new_cube = Vec::new();
                let mut url = base_url.clone();

                while let Ok((response, next_page_url)) = fetch_page(&client, &url).await {
                    if response.status().is_success() {
                        let json_response = response.text().await.unwrap();
                        let json_response: ApiResponse = serde_json::from_str(&json_response).unwrap();
                        let with_rarity = !without_rarity;
                        new_cube.extend(process_json(&json_response.data, with_rarity));
                        log(&format!("Next page {:?}", next_page_url));
                        if let Some(next_page) = json_response.next_page {
                            url = next_page;
                            sleep(100).await;
                        } else {
                            break;
                        }
                    } else {
                        log::error!("Request failed with status: {}", response.status());
                        break;
                    }
                }

                cube.set(new_cube);
            });
        })
    };

    html! {
        <div>
            <input type="text" value={(*cube_name).clone()} oninput={Callback::from(move |e: InputEvent| cube_name.set(e.to_string().into()))} placeholder="Cube Name" />
            <input type="text" value={(*sets).clone()} oninput={Callback::from(move |e: InputEvent| sets.set(e.to_string().into()))} placeholder="Sets (comma-separated)" />
            <input type="checkbox" checked={*without_rarity} onclick={Callback::from(move |_| without_rarity.set(!*without_rarity))} />{" Without Rarity"}
            <button onclick={fetch_cube}>{"Fetch Cube"}</button>

            <div style="max-height: 400px; overflow-y: scroll; border: 1px solid #ccc;">
                { for cube.iter().map(|card| html! { <p>{ format!("{:?}", card) }</p> }) }
            </div>
        </div>
    }
}

async fn fetch_page(client: &Client, url: &str) -> Result<(reqwest::Response, Option<String>), reqwest::Error> {
    let response = client.get(url).send().await?;
    let next_page_url = response.headers().get("X-Scryfall-Next-Page")
        .map(|v| v.to_str().unwrap_or("").to_string());
    Ok((response, next_page_url))
}

fn process_json(data: &[Card], with_rarity: bool) -> Vec<Vec<String>> {
    data.iter().map(|card| {
        if with_rarity {
            vec![card.name.clone(), card.rarity.clone().unwrap_or_else(|| "unknown".to_string())]
        } else {
            vec![card.name.clone()]
        }
    }).collect()
}

fn main() {
    yew::Renderer::<App>::new().render();
}
